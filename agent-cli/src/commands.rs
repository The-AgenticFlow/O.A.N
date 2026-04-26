use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct Config {
    pub api_url: String,
    pub fireworks_key: Option<String>,
    pub fireworks_model: String,
}

#[derive(Deserialize)]
struct Task {
    id: String,
    prompt: String,
    bounty_sats: i64,
    stake_sats: i64,
    status: String,
    worker_pubkey: Option<String>,
}

#[derive(Deserialize)]
struct CreateTaskResponse {
    task_id: String,
    escrow_invoice: String,
    payment_hash: String,
    amount_sats: i64,
}

#[derive(Serialize)]
struct CreateTaskRequest {
    prompt: String,
    bounty_sats: i64,
    stake_sats: Option<i64>,
}

#[derive(Deserialize)]
struct ClaimTaskResponse {
    claimed: bool,
    stake_invoice: Option<String>,
}

#[derive(Serialize)]
struct ClaimTaskRequest {
    worker_pubkey: String,
    worker_invoice: String,
}

#[derive(Serialize)]
struct SubmitTaskRequest {
    result: String,
}

#[derive(Deserialize)]
struct TaskStatusResponse {
    status: String,
    result: Option<String>,
}

pub async fn run_agent(mode: String, pubkey: Option<String>, ln_address: Option<String>, config: Config) -> Result<()> {
    tracing::info!("Starting agent in {} mode", mode);
    
    let agent_pubkey = pubkey.unwrap_or_else(|| format!("agent_{}", uuid::Uuid::new_v4()));
    let lightning_address = ln_address.unwrap_or_else(|| format!("{}@lexe.app", agent_pubkey));
    
    tracing::info!("Agent pubkey: {}", agent_pubkey);
    tracing::info!("Lightning address: {}", lightning_address);
    
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    
    loop {
        match mode.as_str() {
            "worker" => {
                if let Err(e) = worker_cycle(&client, &config, &agent_pubkey, &lightning_address).await {
                    tracing::error!("Worker cycle error: {}", e);
                }
            }
            "buyer" => {
                tracing::info!("Buyer mode - use 'create' command to post tasks");
            }
            _ => {
                anyhow::bail!("Unknown mode: {}", mode);
            }
        }
        
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

async fn worker_cycle(client: &reqwest::Client, config: &Config, pubkey: &str, ln_address: &str) -> Result<()> {
    tracing::debug!("Polling for available tasks...");
    
    let tasks: Vec<Task> = client
        .get(format!("{}/api/tasks", config.api_url))
        .send()
        .await?
        .json()
        .await?;
    
    let assigned_task = tasks.iter().find(|t| {
        t.status == "claimed" && t.worker_pubkey.as_deref() == Some(pubkey)
    });
    
    if let Some(task) = assigned_task {
        tracing::info!("Found assigned task {} with {} sats bounty", task.id, task.bounty_sats);
        let result = do_task_work(client, config, task, pubkey).await?;
        
        tracing::info!("Work complete, submitting result...");
        
        client
            .post(format!("{}/api/tasks/{}/submit", config.api_url, task.id))
            .json(&SubmitTaskRequest { result })
            .send()
            .await?;
        
        tracing::info!("Result submitted! Waiting for verification...");
        return Ok(());
    }
    
    let funded_task = tasks.iter().find(|t| t.status == "funded");
    
    let task = match funded_task {
        Some(t) => t,
        None => {
            tracing::debug!("No funded tasks available");
            return Ok(());
        }
    };
    
    tracing::info!("Found task {} with {} sats bounty", task.id, task.bounty_sats);
    
    let claim_resp = client
        .post(format!("{}/api/tasks/{}/claim", config.api_url, task.id))
        .json(&ClaimTaskRequest {
            worker_pubkey: pubkey.to_string(),
            worker_invoice: ln_address.to_string(),
        })
        .send()
        .await?;

    if !claim_resp.status().is_success() {
        let status = claim_resp.status();
        let err_text = claim_resp.text().await?;
        tracing::warn!("Task claim failed with status {}: {}", status, err_text);
        return Ok(());
    }

    let claim_res: ClaimTaskResponse = claim_resp.json().await?;
    
    if !claim_res.claimed {
        tracing::warn!("Task claim requires stake payment");
        return Ok(());
    }
    
    tracing::info!("Task claimed! Working on: {}", task.prompt);
    
    let result = do_task_work(client, config, task, pubkey).await?;
    
    tracing::info!("Work complete, submitting result...");
    
    client
        .post(format!("{}/api/tasks/{}/submit", config.api_url, task.id))
        .json(&SubmitTaskRequest { result })
        .send()
        .await?;
    
    tracing::info!("Result submitted! Waiting for verification...");
    
    Ok(())
}

async fn do_task_work(_client: &reqwest::Client, config: &Config, task: &Task, pubkey: &str) -> Result<String> {
    if let Some(api_key) = &config.fireworks_key {
        let llm_client = reqwest::Client::new();
        
        let response = llm_client
            .post("https://api.fireworks.ai/inference/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({
                "model": config.fireworks_model,
                "messages": [
                    {"role": "system", "content": "You are a helpful AI agent completing tasks. Be concise and accurate."},
                    {"role": "user", "content": &task.prompt}
                ],
                "max_tokens": 500,
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        
        if let Some(content) = response["choices"][0]["message"]["content"].as_str() {
            return Ok(content.to_string());
        }
    }
    
    Ok(format!("Worker {} completed: {}", pubkey, task.prompt))
}

async fn do_work_with_llm(prompt: &str, config: &Config) -> Result<String> {
    let api_key = config.fireworks_key.as_ref()
        .context("FIREWORKS_API_KEY required for LLM work")?;
    
    let client = reqwest::Client::new();
    
    let response = client
        .post("https://api.fireworks.ai/inference/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": config.fireworks_model,
            "messages": [
                {"role": "system", "content": "You are a helpful AI agent completing tasks. Be concise and accurate."},
                {"role": "user", "content": prompt}
            ],
            "max_tokens": 500,
        }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    
    let content = response["choices"][0]["message"]["content"]
        .as_str()
        .context("No content in LLM response")?;
    
    Ok(content.to_string())
}

pub async fn create_task(prompt: String, bounty: i64, stake: i64, config: Config) -> Result<()> {
    let client = reqwest::Client::new();
    
    let res: CreateTaskResponse = client
        .post(format!("{}/api/tasks", config.api_url))
        .json(&CreateTaskRequest {
            prompt,
            bounty_sats: bounty,
            stake_sats: if stake > 0 { Some(stake) } else { None },
        })
        .send()
        .await?
        .json()
        .await?;
    
    println!("Task created: {}", res.task_id);
    println!("Escrow invoice: {}", res.escrow_invoice);
    println!("Amount: {} sats", res.amount_sats);
    println!("\nPay the invoice to fund the task.");
    
    Ok(())
}

pub async fn list_tasks(config: Config) -> Result<()> {
    let client = reqwest::Client::new();
    
    let tasks: Vec<Task> = client
        .get(format!("{}/api/tasks", config.api_url))
        .send()
        .await?
        .json()
        .await?;
    
    if tasks.is_empty() {
        println!("No tasks available");
        return Ok(());
    }
    
    println!("Available Tasks:");
    println!("----------------");
    for task in tasks {
        println!("ID: {}", task.id);
        println!("Prompt: {}", task.prompt);
        println!("Bounty: {} sats", task.bounty_sats);
        println!("Stake: {} sats", task.stake_sats);
        println!("Status: {}", task.status);
        println!("---");
    }
    
    Ok(())
}

pub async fn claim_task(task_id: String, invoice: String, config: Config) -> Result<()> {
    let client = reqwest::Client::new();
    
    let pubkey = format!("agent_{}", uuid::Uuid::new_v4());
    
    let res: ClaimTaskResponse = client
        .post(format!("{}/api/tasks/{}/claim", config.api_url, task_id))
        .json(&ClaimTaskRequest {
            worker_pubkey: pubkey,
            worker_invoice: invoice,
        })
        .send()
        .await?
        .json()
        .await?;
    
    if res.claimed {
        println!("Task claimed successfully!");
    } else if let Some(stake_invoice) = res.stake_invoice {
        println!("Stake required. Pay this invoice:");
        println!("{}", stake_invoice);
    }
    
    Ok(())
}

pub async fn submit_task(task_id: String, result: String, config: Config) -> Result<()> {
    let client = reqwest::Client::new();
    
    client
        .post(format!("{}/api/tasks/{}/submit", config.api_url, task_id))
        .json(&SubmitTaskRequest { result })
        .send()
        .await?;
    
    println!("Result submitted! Task is now verifying.");
    
    Ok(())
}

pub async fn get_balance(config: Config) -> Result<()> {
    let client = reqwest::Client::new();
    
    let res: serde_json::Value = client
        .get(format!("{}/api/agent/balance", config.api_url))
        .send()
        .await?
        .json()
        .await?;
    
    println!("Balance: {} sats", res["balance_sats"]);
    println!("Pending: {} sats", res["pending_sats"]);
    println!("Total Earned: {} sats", res["total_earned"]);
    
    Ok(())
}
