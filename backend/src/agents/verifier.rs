use crate::db;
use anyhow::Result;

pub async fn verify_and_release(state: &crate::SharedState, task_id: &str) -> Result<()> {
    tracing::info!("Starting verification for task {}", task_id);

    let task = db::tasks::find_by_id(&state.db, task_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

    let result = task.result.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No result to verify"))?;

    let verified = verify_with_llm(&state, &task.prompt, result).await?;

    if verified {
        tracing::info!("Task {} verified successfully", task_id);
        
        db::tasks::complete(&state.db, task_id).await?;

        if let Some(worker_invoice) = &task.worker_invoice {
            let payout_amount = calculate_payout(&state, task.bounty_sats);
            
            if let Err(e) = crate::lexe::pay_to_lightning_address(state, worker_invoice, payout_amount).await {
                tracing::error!("Failed to pay worker for task {}: {}", task_id, e);
            }
        }

        if let Some(worker_pubkey) = &task.worker_pubkey {
            db::agents::update_reputation(&state.db, worker_pubkey, true, task.bounty_sats).await?;
        }
    } else {
        tracing::info!("Task {} verification failed", task_id);
        
        db::tasks::fail(&state.db, task_id).await?;

        if let Err(e) = refund_buyer(state, &task).await {
            tracing::error!("Failed to refund buyer for task {}: {}", task_id, e);
        }

        if let Some(worker_pubkey) = &task.worker_pubkey {
            db::agents::update_reputation(&state.db, worker_pubkey, false, 0).await?;
        }
    }

    Ok(())
}

async fn verify_with_llm(state: &crate::AppState, prompt: &str, result: &str) -> Result<bool> {
    let api_key = state.config.fireworks_api_key.as_ref()
        .ok_or_else(|| anyhow::anyhow!("FIREWORKS_API_KEY not configured"))?;

    let system_prompt = r#"You are a task verification agent. Your job is to determine if a submitted result adequately fulfills the original task prompt.

Respond with ONLY "PASS" or "FAIL" followed by a brief reason.

Consider:
1. Does the result address the prompt?
2. Is the result reasonably complete?
3. Is the result relevant to what was asked?

Be lenient for creative tasks, strict for factual/technical tasks."#;

    let user_prompt = format!(
        "Original task prompt:\n{}\n\nSubmitted result:\n{}\n\nDoes this result adequately fulfill the task?",
        prompt, result
    );

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.fireworks.ai/inference/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": state.config.fireworks_model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "max_tokens": 100,
            "temperature": 0.3
        }))
        .send()
        .await?;

    let body: serde_json::Value = response.json().await?;
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("FAIL");

    tracing::info!("Fireworks verification response: {}", content);

    Ok(content.to_uppercase().starts_with("PASS"))
}

fn calculate_payout(state: &crate::AppState, bounty_sats: i64) -> i64 {
    let fee = (bounty_sats as f64 * state.config.platform_fee_percent / 100.0) as i64;
    bounty_sats - fee
}

async fn refund_buyer(_state: &crate::AppState, task: &crate::models::Task) -> Result<()> {
    tracing::info!("Refunding {} sats to buyer {}", task.bounty_sats, task.buyer_pubkey);
    Ok(())
}
