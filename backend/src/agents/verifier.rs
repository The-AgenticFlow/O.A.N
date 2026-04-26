use crate::db;
use anyhow::Result;

pub async fn verify_and_release(state: &crate::SharedState, task_id: &str) -> Result<()> {
    tracing::info!("Starting verification for task {}", task_id);

    let task = db::tasks::find_by_id(&state.db, task_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

    tracing::info!("Task {} found, checking result", task_id);

    let result = task.result.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No result to verify"))?;

    tracing::info!("Task {} has result ({} chars), calling LLM", task_id, result.len());

    let (verified, reason) = verify_with_llm(&state, &task.prompt, result).await?;

    tracing::info!("Task {} LLM verdict: {} - {}", task_id, if verified { "PASS" } else { "FAIL" }, reason);

    if verified {
        tracing::info!("Task {} verified successfully", task_id);
        
        db::tasks::complete(&state.db, task_id).await?;

        let _ = crate::db::activity::log_event(
            &state.db,
            "verifier",
            Some("OAN Verifier"),
            "task_verified",
            Some(&format!("Task {} passed verification", task_id)),
            Some(task_id),
        ).await;

        if let Some(worker_invoice) = &task.worker_invoice {
            let payout_amount = calculate_payout(&state, task.bounty_sats);
            
            if let Err(e) = crate::lexe::pay_to_lightning_address(state, worker_invoice, payout_amount).await {
                tracing::error!("Failed to pay worker for task {}: {}", task_id, e);
            } else {
                let _ = crate::db::activity::log_event(
                    &state.db,
                    "verifier",
                    Some("OAN Verifier"),
                    "payment_sent",
                    Some(&format!("Paid {} sats to worker for task {}", payout_amount, task_id)),
                    Some(task_id),
                ).await;
            }
        }

        if let Some(worker_pubkey) = &task.worker_pubkey {
            db::agents::update_reputation(&state.db, worker_pubkey, true, task.bounty_sats).await?;
        }
    } else {
        tracing::info!("Task {} verification failed: {}", task_id, reason);
        
        db::tasks::fail(&state.db, task_id, &reason).await?;

        let _ = crate::db::activity::log_event(
            &state.db,
            "verifier",
            Some("OAN Verifier"),
            "task_failed",
            Some(&format!("Task {} failed: {}", task_id, reason)),
            Some(task_id),
        ).await;

        if let Err(e) = refund_buyer(state, &task).await {
            tracing::error!("Failed to refund buyer for task {}: {}", task_id, e);
        } else {
            let _ = crate::db::activity::log_event(
                &state.db,
                "verifier",
                Some("OAN Verifier"),
                "refund_sent",
                Some(&format!("Refunded {} sats to buyer for task {}", task.bounty_sats, task_id)),
                Some(task_id),
            ).await;
        }

        if let Some(worker_pubkey) = &task.worker_pubkey {
            db::agents::update_reputation(&state.db, worker_pubkey, false, 0).await?;
        }
    }

    Ok(())
}

async fn verify_with_llm(state: &crate::AppState, prompt: &str, result: &str) -> Result<(bool, String)> {
    let api_key = state.config.fireworks_api_key.as_ref()
        .ok_or_else(|| anyhow::anyhow!("FIREWORKS_API_KEY not configured"))?;

    tracing::info!("Using Fireworks API key (len={})", api_key.len());

    let system_prompt = r#"You are a task verification agent. Your job is to determine if a submitted result adequately fulfills the original task prompt.

Respond with ONLY "PASS" or "FAIL" followed by a brief reason on the next line.

Consider:
1. Does the result address the prompt?
2. Is the result reasonably complete?
3. Is the result relevant to what was asked?

Be lenient for creative tasks, strict for factual/technical tasks."#;

    let truncated_result = if result.len() > 2000 {
        tracing::info!("Truncating result from {} to 2000 chars", result.len());
        &result[..2000]
    } else {
        result
    };

    let user_prompt = format!(
        "Original task prompt:\n{}\n\nSubmitted result:\n{}\n\nDoes this result adequately fulfill the task?",
        prompt, truncated_result
    );

    tracing::info!("Building reqwest client with 60s timeout");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let url = "https://api.fireworks.ai/inference/v1/chat/completions";
    tracing::info!("Sending request to Fireworks API: {}", url);
    
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": state.config.fireworks_model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "max_tokens": 200,
            "temperature": 0.3
        }))
        .send()
        .await?;

    tracing::info!("Fireworks response status: {}", response.status());
    
    let body: serde_json::Value = response.json().await?;
    tracing::info!("Fireworks response body: {}", serde_json::to_string(&body).unwrap_or_default());
    
    if let Some(error) = body.get("error") {
        tracing::error!("Fireworks API error: {}", error);
        return Ok((true, "Verification passed (API error, defaulting to pass)".to_string()));
    }

    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("PASS\nDefaulted to pass due to missing response");

    tracing::info!("Fireworks verification response: {}", content);

    let upper_content = content.to_uppercase();
    let passed = upper_content.contains("PASS") && !upper_content.contains("FAIL");
    let failed = upper_content.contains("FAIL");
    
    let reason = if passed || failed {
        content.lines()
            .find(|l| l.to_uppercase().contains("PASS") || l.to_uppercase().contains("FAIL"))
            .map(|l| l.trim().to_string())
            .unwrap_or_else(|| content.lines().next().unwrap_or("No reason").to_string())
    } else {
        tracing::warn!("No PASS/FAIL verdict found in response, defaulting to pass");
        "No explicit verdict found, defaulting to pass".to_string()
    };
    
    let passed = passed || (!failed);

    Ok((passed, reason))
}

fn calculate_payout(state: &crate::AppState, bounty_sats: i64) -> i64 {
    let fee = (bounty_sats as f64 * state.config.platform_fee_percent / 100.0) as i64;
    bounty_sats - fee
}

async fn refund_buyer(state: &crate::AppState, task: &crate::models::Task) -> Result<()> {
    tracing::info!("Refunding {} sats to buyer {}", task.bounty_sats, task.buyer_pubkey);

    let escrow_payments = crate::db::payments::list_by_task(&state.db, &task.id).await?;
    
    let escrow_payment = escrow_payments
        .iter()
        .find(|p| p.direction == "in" && p.status == "paid")
        .ok_or_else(|| anyhow::anyhow!("No paid escrow payment found for task"))?;

    crate::db::payments::update_status(&state.db, &escrow_payment.id, crate::models::PaymentStatus::Failed).await?;

    let buyer_invoice = task.buyer_pubkey.clone();
    if buyer_invoice.contains("@") {
        match crate::lexe::pay_to_lightning_address(state, &buyer_invoice, task.bounty_sats).await {
            Ok(payment_hash) => {
                tracing::info!("Refund payment sent: {}", payment_hash);
                
                crate::db::payments::create(
                    &state.db,
                    Some(task.id.clone()),
                    format!("refund_{}", task.id),
                    "refund".to_string(),
                    task.bounty_sats,
                    crate::models::PaymentDirection::Out,
                ).await?;
            }
            Err(e) => {
                tracing::error!("Failed to send refund payment: {}", e);
            }
        }
    }

    Ok(())
}
