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
        extract::{Path, State},
        http::StatusCode,
        Json,
    };
    use crate::{AppState, models::*};
    use crate::db;

    pub async fn list_tasks(
        State(state): State<crate::SharedState>,
    ) -> Result<Json<Vec<Task>>, (StatusCode, String)> {
        let tasks = db::tasks::list_available(&state.db)
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
        let buyer_pubkey = "placeholder_pubkey".to_string();
        
        let task = db::tasks::create(&state.db, req, buyer_pubkey)
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

        db::payments::create(
            &state.db,
            Some(task.id.clone()),
            invoice.invoice.clone(),
            invoice.payment_hash.clone(),
            task.bounty_sats,
            crate::models::PaymentDirection::In,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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
        tokio::spawn(async move {
            if let Err(e) = crate::agents::verifier::verify_and_release(&state_clone, &task_id).await {
                tracing::error!("Verification failed for task {}: {}", task_id, e);
            }
        });

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
            payout_tx: None,
        }))
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
    use crate::{AppState, models::*};

    pub async fn get_balance(
        State(state): State<crate::SharedState>,
    ) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
        let balance = crate::lexe::get_balance(&state)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            
        Ok(Json(serde_json::json!({
            "balance_sats": balance,
            "pending_sats": 0,
            "total_earned": 0
        })))
    }

    pub async fn withdraw(
        State(state): State<crate::SharedState>,
        Json(req): Json<WithdrawRequest>,
    ) -> Result<Json<WithdrawResponse>, (StatusCode, String)> {
        let amount = req.amount_sats.unwrap_or_else(|| {
            // Default to full balance if not specified
            0
        });
        
        let payment_hash = crate::lexe::pay_to_lightning_address(&state, &req.lightning_address, amount)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            
        Ok(Json(WithdrawResponse {
            payment_hash,
            amount_sats: amount,
        }))
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
