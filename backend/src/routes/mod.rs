pub mod health {
    use axum::Json;
    use serde_json::json;

    pub async fn health() -> Json<serde_json::Value> {
        Json(json!({
            "status": "ok",
            "service": "oan-backend",
            "version": "0.1.0"
        }))
    }
}

pub mod tasks {
    use axum::{
        extract::{Path, Query, State},
        http::StatusCode,
        Json,
    };
    use crate::{AppState, models::*};
    use crate::db;
    use serde::Deserialize;
    use futures::FutureExt;

    #[derive(Deserialize)]
    pub struct TaskQuery {
        status: Option<String>,
    }

    pub async fn list_tasks(
        State(state): State<crate::SharedState>,
        Query(query): Query<TaskQuery>,
    ) -> Result<Json<Vec<Task>>, (StatusCode, String)> {
        let tasks = match query.status.as_deref() {
            Some("funded") => db::tasks::list_available(&state.db)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
            _ => db::tasks::list_all(&state.db)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        };
        
        Ok(Json(tasks))
    }

    pub async fn list_all_tasks(
        State(state): State<crate::SharedState>,
    ) -> Result<Json<Vec<Task>>, (StatusCode, String)> {
        let tasks = db::tasks::list_all(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
        Ok(Json(tasks))
    }

    pub async fn get_task(
        State(state): State<crate::SharedState>,
        Path(id): Path<String>,
    ) -> Result<Json<Task>, (StatusCode, String)> {
        let task = db::tasks::find_by_id(&state.db, &id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Task not found".to_string()))?;
        
        Ok(Json(task))
    }

    pub async fn create_task(
        State(state): State<crate::SharedState>,
        Json(req): Json<CreateTaskRequest>,
    ) -> Result<Json<CreateTaskResponse>, (StatusCode, String)> {
        let buyer_pubkey = req.buyer_pubkey.clone().unwrap_or_else(|| "anonymous".to_string());
        
        let task = db::tasks::create(&state.db, req.clone(), buyer_pubkey)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let invoice = match crate::lexe::create_escrow_invoice(&state, task.bounty_sats, &task.id).await {
            Ok(inv) => inv,
            Err(e) => {
                tracing::error!("Failed to create invoice: {}", e);
                return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create invoice: {}", e)));
            }
        };

        db::tasks::set_escrow(&state.db, &task.id, invoice.invoice.clone(), invoice.payment_hash.clone())
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let payment = db::payments::create(
            &state.db,
            Some(task.id.clone()),
            invoice.invoice.clone(),
            invoice.payment_hash.clone(),
            task.bounty_sats,
            crate::models::PaymentDirection::In,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if let Some(ref buyer_pubkey) = req.buyer_pubkey {
            tracing::info!("Auto-funding task {} for buyer {}", task.id, buyer_pubkey);
            
            db::payments::settle(&state.db, &payment.id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            
            db::tasks::update_status(&state.db, &task.id, TaskStatus::Funded)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            
            tracing::info!("Task {} auto-funded successfully", task.id);
        }

        Ok(Json(CreateTaskResponse {
            task_id: task.id,
            escrow_invoice: invoice.invoice,
            payment_hash: invoice.payment_hash,
            amount_sats: task.bounty_sats,
        }))
    }

    pub async fn claim_task(
        State(state): State<crate::SharedState>,
        Path(id): Path<String>,
        Json(req): Json<ClaimTaskRequest>,
    ) -> Result<Json<ClaimTaskResponse>, (StatusCode, String)> {
        let task = db::tasks::find_by_id(&state.db, &id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Task not found".to_string()))?;

        if task.status_enum() != TaskStatus::Funded {
            return Err((StatusCode::BAD_REQUEST, "Task is not available for claiming".to_string()));
        }

        let stake_invoice = if task.stake_sats > 0 {
            Some(format!("lnbc{}n1...", task.stake_sats))
        } else {
            None
        };

        db::tasks::claim(&state.db, &id, req.worker_pubkey, req.worker_invoice)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(Json(ClaimTaskResponse {
            claimed: stake_invoice.is_none(),
            stake_invoice,
        }))
    }

    pub async fn assign_task(
        State(state): State<crate::SharedState>,
        Path(id): Path<String>,
        Json(req): Json<AssignTaskRequest>,
    ) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
        let task = db::tasks::find_by_id(&state.db, &id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Task not found".to_string()))?;

        if task.status_enum() != TaskStatus::Funded {
            return Err((StatusCode::BAD_REQUEST, "Task is not available for assignment".to_string()));
        }

        db::tasks::assign(&state.db, &id, req.worker_pubkey.clone())
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let _ = db::activity::log_event(
            &state.db,
            &req.worker_pubkey,
            None,
            "task_assigned",
            Some(&format!("Task {} assigned to {}", &id, &req.worker_pubkey)),
            Some(&id),
        ).await;

        Ok(Json(serde_json::json!({
            "assigned": true,
            "task_id": id,
            "worker_pubkey": req.worker_pubkey,
        })))
    }

    pub async fn submit_task(
        State(state): State<crate::SharedState>,
        Path(id): Path<String>,
        Json(req): Json<SubmitTaskRequest>,
    ) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
        let task = db::tasks::find_by_id(&state.db, &id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Task not found".to_string()))?;

        if task.status_enum() != TaskStatus::Claimed {
            return Err((StatusCode::BAD_REQUEST, "Task is not in claimed state".to_string()));
        }

        db::tasks::submit_result(&state.db, &id, req.result)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let state_clone = state.clone();
        let task_id = id.clone();
        let handle = tokio::spawn(async move {
            tracing::info!("Verifier task spawned for {}", task_id);
            let result = std::panic::AssertUnwindSafe(
                crate::agents::verifier::verify_and_release(&state_clone, &task_id)
            ).catch_unwind().await;
            match result {
                Ok(Ok(())) => {
                    tracing::info!("Verifier completed successfully for {}", task_id);
                }
                Ok(Err(e)) => {
                    tracing::error!("Verification failed for task {}: {}", task_id, e);
                    if let Err(db_err) = db::tasks::fail(&state_clone.db, &task_id, &format!("Verifier error: {}", e)).await {
                        tracing::error!("Failed to mark task {} as failed: {}", task_id, db_err);
                    }
                }
                Err(panic_info) => {
                    tracing::error!("Verifier panicked for task {}: {:?}", task_id, panic_info);
                }
            }
        });
        tracing::info!("Spawned verifier task with handle id {:?}", handle.id());

        Ok(Json(serde_json::json!({ "status": "verifying" })))
    }

    pub async fn task_status(
        State(state): State<crate::SharedState>,
        Path(id): Path<String>,
    ) -> Result<Json<TaskStatusResponse>, (StatusCode, String)> {
        let task = db::tasks::find_by_id(&state.db, &id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Task not found".to_string()))?;
        
        Ok(Json(TaskStatusResponse {
            task_id: task.id,
            status: task.status,
            result: task.result,
            failure_reason: task.failure_reason,
            payout_tx: None,
        }))
    }

    pub async fn reset_task(
        State(state): State<crate::SharedState>,
        Path(id): Path<String>,
    ) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
        let task = db::tasks::find_by_id(&state.db, &id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Task not found".to_string()))?;

        if task.status_enum() != TaskStatus::Verifying {
            return Err((StatusCode::BAD_REQUEST, "Task is not in verifying status".to_string()));
        }

        db::tasks::update_status(&state.db, &id, TaskStatus::Claimed)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(Json(serde_json::json!({ "status": "reset", "task_id": id })))
    }
}

pub mod webhooks {
    use axum::{extract::State, http::StatusCode, Json};
    use serde::Deserialize;
    use crate::{AppState, db};

    #[derive(Deserialize)]
    pub struct PaymentWebhook {
        pub payment_hash: String,
        pub preimage: String,
        pub amount_sats: i64,
    }

    pub async fn payment_webhook(
        State(state): State<crate::SharedState>,
        Json(payload): Json<PaymentWebhook>,
    ) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
        tracing::info!("Payment webhook received: hash={}", payload.payment_hash);

        let payment = db::payments::find_by_hash(&state.db, &payload.payment_hash)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if let Some(payment) = payment {
            db::payments::settle(&state.db, &payment.id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            if let Some(task_id) = &payment.task_id {
                db::tasks::update_status(&state.db, task_id, crate::models::TaskStatus::Funded)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            }
        }

        Ok(Json(serde_json::json!({ "status": "ok" })))
    }
}

pub mod agent {
    use axum::{extract::State, http::StatusCode, Json};
    use crate::{models::*, db};

    pub async fn get_balance(
        State(state): State<crate::SharedState>,
    ) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
        let balance = crate::lexe::get_balance(&state)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
        let pending_payments = db::payments::list_pending(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
        let pending_sats: i64 = pending_payments.iter().map(|p| p.amount_sats).sum();
        
        let tasks = db::tasks::list_all(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
        let escrow_sats: i64 = tasks.iter()
            .filter(|t| matches!(t.status_enum(), TaskStatus::Funded | TaskStatus::Claimed | TaskStatus::Verifying))
            .map(|t| t.bounty_sats)
            .sum();
            
        Ok(Json(serde_json::json!({
            "balance_sats": balance,
            "pending_sats": pending_sats,
            "escrow_sats": escrow_sats,
            "available_sats": balance - escrow_sats,
            "total_earned": 0
        })))
    }

    pub async fn withdraw(
        State(state): State<crate::SharedState>,
        Json(req): Json<WithdrawRequest>,
    ) -> Result<Json<WithdrawResponse>, (StatusCode, String)> {
        let amount = req.amount_sats.unwrap_or(0);
        
        let payment_hash = crate::lexe::pay_to_lightning_address(&state, &req.lightning_address, amount)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            
        Ok(Json(WithdrawResponse {
            payment_hash,
            amount_sats: amount,
        }))
    }
}

pub mod agents {
    use axum::{
        extract::{Path, State},
        http::StatusCode,
        Json,
    };
    use crate::{AppState, models::*, db, SharedState};
    use std::sync::Arc;
    use tokio::task::JoinHandle;

    pub async fn list_agents(
        State(state): State<crate::SharedState>,
    ) -> Result<Json<Vec<Agent>>, (StatusCode, String)> {
        let agents = db::agents::list_all(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Ok(Json(agents))
    }

    pub async fn create_agent(
        State(state): State<crate::SharedState>,
        Json(req): Json<CreateAgentRequest>,
    ) -> Result<Json<Agent>, (StatusCode, String)> {
        if req.name.trim().is_empty() {
            return Err((StatusCode::BAD_REQUEST, "Agent name is required".to_string()));
        }

        let agent = db::agents::create(&state.db, req)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
        Ok(Json(agent))
    }

    #[derive(Clone)]
    pub struct AgentHandle {
        pub agent_pubkey: String,
        pub handle: Arc<tokio::sync::Mutex<Option<JoinHandle<()>>>>,
    }

    pub async fn spawn_agent(
        State(state): State<crate::SharedState>,
        Json(req): Json<SpawnAgentRequest>,
    ) -> Result<Json<SpawnAgentResponse>, (StatusCode, String)> {
        let agent = db::agents::find_by_pubkey(&state.db, &req.agent_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Agent not found".to_string()))?;

        db::agents::set_active(&state.db, &agent.pubkey, true)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let state_clone = state.clone();
        let pubkey = agent.pubkey.clone();
        let name = agent.name.clone();
        let ln_address = agent.lightning_address.clone();
        let agent_type = agent.agent_type.clone().unwrap_or_else(|| "worker".to_string());

        let handle = tokio::spawn(async move {
            run_agent_loop(state_clone, pubkey, name, ln_address, agent_type).await;
        });

        {
            let mut agents = state.running_agents.lock().await;
            agents.push(AgentHandle {
                agent_pubkey: req.agent_id.clone(),
                handle: Arc::new(tokio::sync::Mutex::new(Some(handle))),
            });
        }

        let agent = db::agents::find_by_pubkey(&state.db, &req.agent_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .unwrap();

        Ok(Json(SpawnAgentResponse {
            agent,
            message: "Agent spawned successfully".to_string(),
        }))
    }

    pub async fn stop_agent(
        State(state): State<crate::SharedState>,
        Path(agent_id): Path<String>,
    ) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
        db::agents::set_active(&state.db, &agent_id, false)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        {
            let mut agents = state.running_agents.lock().await;
            if let Some(pos) = agents.iter().position(|a| a.agent_pubkey == agent_id) {
                let agent_handle = agents.remove(pos);
                let handle_guard = agent_handle.handle.lock().await;
                if let Some(handle) = handle_guard.as_ref() {
                    handle.abort();
                }
            }
        }

        Ok(Json(serde_json::json!({ "status": "stopped", "agent_id": agent_id })))
    }

    async fn run_agent_loop(state: SharedState, pubkey: String, name: Option<String>, ln_address: Option<String>, agent_type: String) {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();

        let lightning_address = ln_address.unwrap_or_else(|| format!("{}@lexe.app", pubkey));
        let api_url = state.config.api_url.clone();
        let agent_name = name.clone().unwrap_or_else(|| pubkey.clone());

        let _ = db::activity::log_event(
            &state.db,
            &pubkey,
            name.as_deref(),
            "agent_spawned",
            Some(&format!("{} agent started", agent_name)),
            None,
        ).await;

        loop {
            let is_active = db::agents::find_by_pubkey(&state.db, &pubkey)
                .await
                .ok()
                .flatten()
                .and_then(|a| a.is_active)
                .unwrap_or(false);

            if !is_active {
                let _ = db::activity::log_event(
                    &state.db,
                    &pubkey,
                    name.as_deref(),
                    "agent_stopped",
                    Some(&format!("{} agent stopped", agent_name)),
                    None,
                ).await;
                tracing::info!("Agent {} stopped", pubkey);
                break;
            }

            match agent_type.as_str() {
                "worker" => {
                    if let Err(e) = worker_cycle(&client, &state, &pubkey, name.as_deref(), &lightning_address, &api_url).await {
                        tracing::error!("Worker {} cycle error: {}", pubkey, e);
                    }
                }
                "buyer" => {
                    if let Err(e) = buyer_cycle(&client, &state, &pubkey, name.as_deref(), &api_url).await {
                        tracing::error!("Buyer {} cycle error: {}", pubkey, e);
                    }
                }
                _ => {}
            }

            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    }

    async fn buyer_cycle(
        client: &reqwest::Client,
        state: &SharedState,
        pubkey: &str,
        name: Option<&str>,
        api_url: &str,
    ) -> anyhow::Result<()> {
        let all_tasks: Vec<Task> = client
            .get(format!("{}/api/tasks", api_url))
            .send()
            .await?
            .json()
            .await?;

        let my_tasks: Vec<&Task> = all_tasks.iter()
            .filter(|t| t.buyer_pubkey == pubkey)
            .collect();

        if my_tasks.len() < 3 {
            let prompts = vec![
                "Summarize the Bitcoin whitepaper in 3 sentences",
                "Write a Python function that calculates Fibonacci numbers",
                "Explain the concept of Lightning Network channels",
                "Create a simple React component for a counter",
                "Write a haiku about decentralized systems",
                "What are the benefits of using Rust for systems programming?",
                "Explain the difference between REST and GraphQL APIs",
            ];
            
            let prompt = prompts[chrono::Utc::now().timestamp() as usize % prompts.len()];
            let bounty = 50 + (chrono::Utc::now().timestamp() % 100) as i64;

            let _ = db::activity::log_event(
                &state.db,
                pubkey,
                name,
                "task_created",
                Some(&format!("Created task: {} ({} sats)", prompt, bounty)),
                None,
            ).await;

            let res: CreateTaskResponse = client
                .post(format!("{}/api/tasks", api_url))
                .json(&CreateTaskRequest {
                    prompt: prompt.to_string(),
                    bounty_sats: bounty,
                    stake_sats: None,
                    buyer_pubkey: Some(pubkey.to_string()),
                })
                .send()
                .await?
                .json()
                .await?;

            let _ = db::activity::log_event(
                &state.db,
                pubkey,
                name,
                "task_funded",
                Some(&format!("Auto-funded task {} with {} sats", res.task_id, res.amount_sats)),
                Some(&res.task_id),
            ).await;
        }

        Ok(())
    }

    async fn worker_cycle(
        client: &reqwest::Client,
        state: &SharedState,
        pubkey: &str,
        name: Option<&str>,
        ln_address: &str,
        api_url: &str,
    ) -> anyhow::Result<()> {
        let tasks: Vec<Task> = client
            .get(format!("{}/api/tasks", api_url))
            .send()
            .await?
            .json()
            .await?;

        if tasks.is_empty() {
            return Ok(());
        }

        let assigned_task = tasks.iter()
            .find(|t| t.worker_pubkey.as_ref() == Some(&pubkey.to_string()));

        let task = if let Some(t) = assigned_task {
            t
        } else {
            return Ok(());
        };

        if task.status != "claimed" {
            let claim_res: ClaimTaskResponse = client
                .post(format!("{}/api/tasks/{}/claim", api_url, task.id))
                .json(&ClaimTaskRequest {
                    worker_pubkey: pubkey.to_string(),
                    worker_invoice: ln_address.to_string(),
                })
                .send()
                .await?
                .json()
                .await?;

            if !claim_res.claimed {
                return Ok(());
            }

            let _ = db::activity::log_event(
                &state.db,
                pubkey,
                name,
                "task_claimed",
                Some(&format!("Claimed assigned task: {}", &task.prompt[..task.prompt.len().min(40)])),
                Some(&task.id),
            ).await;
        }

        let _ = db::activity::log_event(
            &state.db,
            pubkey,
            name,
            "task_found",
            Some(&format!("Found assigned task: {} ({} sats)", &task.prompt[..task.prompt.len().min(40)], task.bounty_sats)),
            Some(&task.id),
        ).await;

        let result = do_work_with_llm(&task.prompt, state).await?;

        let _ = db::activity::log_event(
            &state.db,
            pubkey,
            name,
            "task_completed",
            Some(&format!("Completed work: {} -> {} chars", &task.prompt[..task.prompt.len().min(40)], result.len())),
            Some(&task.id),
        ).await;

        client
            .post(format!("{}/api/tasks/{}/submit", api_url, task.id))
            .json(&SubmitTaskRequest { result })
            .send()
            .await?;

        Ok(())
    }

    async fn do_work_with_llm(prompt: &str, state: &AppState) -> anyhow::Result<String> {
        let api_key = state.config.fireworks_api_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("FIREWORKS_API_KEY required for LLM work"))?;

        let client = reqwest::Client::new();

        let response = client
            .post("https://api.fireworks.ai/inference/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({
                "model": state.config.fireworks_model,
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
            .ok_or_else(|| anyhow::anyhow!("No content in LLM response"))?;

        Ok(content.to_string())
    }
}

pub mod l402 {
    use axum::{extract::State, http::StatusCode, Json};
    use crate::{AppState, models::L402Challenge};

    #[derive(serde::Deserialize)]
    pub struct VerifyRequest {
        pub macaroon: String,
        pub preimage: String,
    }

    pub async fn verify_token(
        State(state): State<crate::SharedState>,
        Json(req): Json<VerifyRequest>,
    ) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
        let valid = crate::lexe::l402::verify_macaroon(&state, &req.macaroon, &req.preimage)
            .await
            .map_err(|e: anyhow::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(Json(serde_json::json!({ "valid": valid })))
    }

    pub async fn issue_challenge(
        State(state): State<crate::SharedState>,
        resource: &str,
        amount_sats: u64,
    ) -> Result<Json<L402Challenge>, (StatusCode, String)> {
        let challenge = crate::lexe::l402::create_challenge(&state, resource, amount_sats)
            .await
            .map_err(|e: anyhow::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(Json(challenge))
    }
}

pub mod activity {
    use axum::{
        extract::{Query, State},
        http::StatusCode,
        Json,
    };
    use crate::{AppState, models::ActivityEntry, db};
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct ActivityQuery {
        limit: Option<usize>,
    }

    pub async fn list_activity(
        State(state): State<crate::SharedState>,
        Query(query): Query<ActivityQuery>,
    ) -> Result<Json<Vec<ActivityEntry>>, (StatusCode, String)> {
        let limit = query.limit.unwrap_or(50);
        let entries = db::activity::list_recent(&state.db, limit)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Ok(Json(entries))
    }
}

pub mod wallet {
    use axum::{extract::State, http::StatusCode, Json};
    use crate::AppState;

    #[derive(serde::Deserialize)]
    pub struct ReceiveRequest {
        pub amount_sats: Option<i64>,
        pub description: Option<String>,
    }

    #[derive(serde::Deserialize)]
    pub struct SendRequest {
        pub destination: String,
        pub amount_sats: Option<i64>,
    }

    pub async fn get_balance(
        State(state): State<crate::SharedState>,
    ) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
        let balance = crate::lexe::get_balance(&state)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            
        Ok(Json(serde_json::json!({
            "balance_sats": balance
        })))
    }

    pub async fn receive(
        State(state): State<crate::SharedState>,
        Json(req): Json<ReceiveRequest>,
    ) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
        let amount = req.amount_sats.unwrap_or(0);
        let description = req.description.as_deref();
        
        let invoice = if amount > 0 {
            crate::lexe::create_escrow_invoice(&state, amount, "wallet-receive")
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        } else {
            crate::lexe::create_variable_invoice(&state, description)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        };
        
        Ok(Json(serde_json::json!({
            "invoice": invoice.invoice,
            "payment_hash": invoice.payment_hash,
            "amount_sats": invoice.amount_sats
        })))
    }

    pub async fn send_payment(
        State(state): State<crate::SharedState>,
        Json(req): Json<SendRequest>,
    ) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
        let payment_hash = crate::lexe::pay_to_lightning_address(&state, &req.destination, req.amount_sats.unwrap_or(0))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            
        Ok(Json(serde_json::json!({
            "payment_hash": payment_hash,
            "status": "sent"
        })))
    }
}

pub use health::health;
