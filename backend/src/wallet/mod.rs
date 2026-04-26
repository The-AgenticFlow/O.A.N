use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

use crate::{AppState, db, models::TaskStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdkResponse<T> {
    success: bool,
    data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdkBalanceData {
    balanceSats: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdkInvoiceResponse {
    pub invoice: String,
    #[serde(rename = "paymentHash")]
    pub payment_hash: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdkSendData {
    #[serde(rename = "paymentHash")]
    payment_hash: String,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdkReceiveRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    amountSats: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdkSendRequest {
    destination: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    amountMsat: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct MdkClient {
    base_url: String,
    client: Client,
}

impl MdkClient {
    pub fn new(port: u16) -> Self {
        Self {
            base_url: format!("http://localhost:{}", port),
            client: Client::new(),
        }
    }

    pub async fn balance(&self) -> Result<i64> {
        let resp = self.client
            .get(format!("{}/balance", self.base_url))
            .send()
            .await?
            .json::<MdkResponse<MdkBalanceData>>()
            .await?;
        Ok(resp.data.balanceSats)
    }

    pub async fn receive(&self, amount_sats: i64, description: Option<&str>) -> Result<MdkInvoiceResponse> {
        let body = MdkReceiveRequest {
            amountSats: Some(amount_sats),
            description: description.map(|s| s.to_string()),
        };
        
        tracing::debug!("MDK receive request: amountSats={}", amount_sats);
        
        let resp = self.client
            .post(format!("{}/receive", self.base_url))
            .json(&body)
            .send()
            .await?;
        
        let response_text = resp.text().await?;
        tracing::debug!("MDK receive response: {}", response_text);
        
        let mdk_resp: MdkResponse<MdkInvoiceResponse> = serde_json::from_str(&response_text)?;
        let invoice = &mdk_resp.data.invoice;
        
        // Check if invoice starts with "lni" (zero-amount) vs "lnbc" (has amount)
        if invoice.starts_with("lni") || invoice.starts_with("LNI") {
            tracing::warn!("MDK returned zero-amount invoice (lni prefix) when {} sats was requested", amount_sats);
        }
        
        Ok(mdk_resp.data)
    }

    pub async fn receive_variable(&self, description: Option<&str>) -> Result<MdkInvoiceResponse> {
        let body = MdkReceiveRequest {
            amountSats: None,
            description: description.map(|s| s.to_string()),
        };
        
        let resp = self.client
            .post(format!("{}/receive", self.base_url))
            .json(&body)
            .send()
            .await?
            .json::<MdkResponse<MdkInvoiceResponse>>()
            .await?;
        Ok(resp.data)
    }

    pub async fn send(&self, destination: &str, amount_sats: Option<i64>) -> Result<MdkSendResponse> {
        // Convert sats to millisatoshis (1 sat = 1000 msat)
        let amount_msat = amount_sats.map(|s| s * 1000);
        
        tracing::debug!(
            "MDK send request: destination={}, amount_sats={:?}, amount_msat={:?}",
            destination,
            amount_sats,
            amount_msat
        );
        
        // Check if destination looks like a Lightning address vs invoice
        if destination.contains('@') && !destination.starts_with("ln") {
            tracing::debug!("Destination appears to be a Lightning address (contains @, doesn't start with ln)");
            if amount_sats.is_none() || amount_sats == Some(0) {
                tracing::error!("Lightning address payment requires non-zero amount");
            }
        }
        
        // Check if invoice is zero-amount (starts with "lni" instead of "lnbc")
        if destination.starts_with("lni") || destination.starts_with("LNI") {
            tracing::warn!("Attempting to pay zero-amount invoice (lni prefix)");
            if amount_sats.is_none() {
                tracing::error!("Zero-amount invoice requires explicit amount");
            }
        }
        
        let body = MdkSendRequest {
            destination: destination.to_string(),
            amountMsat: amount_msat,
        };
        
        let resp = self.client
            .post(format!("{}/send", self.base_url))
            .json(&body)
            .send()
            .await?;
        
        let json: serde_json::Value = resp.json().await?;
        
        if json.get("success").and_then(|v| v.as_bool()) == Some(false) {
            let error_msg = json.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            return Err(anyhow::anyhow!("MDK send failed: {}", error_msg));
        }
        
        let data = json.get("data")
            .ok_or_else(|| anyhow::anyhow!("Missing data in response"))?;
        
        Ok(MdkSendResponse {
            payment_hash: data["paymentHash"].as_str().unwrap_or("").to_string(),
            status: data["status"].as_str().unwrap_or("unknown").to_string(),
        })
    }

    pub async fn payments(&self) -> Result<Vec<serde_json::Value>> {
        let resp = self.client
            .get(format!("{}/payments", self.base_url))
            .send()
            .await?
            .json::<MdkResponse<Vec<serde_json::Value>>>()
            .await?;
        Ok(resp.data)
    }

    pub async fn list_payments(&self) -> Result<Vec<MdkPayment>> {
        let resp = self.client
            .get(format!("{}/payments", self.base_url))
            .send()
            .await?
            .json::<MdkResponse<MdkPaymentsData>>()
            .await?;
        Ok(resp.data.payments)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdkPaymentsData {
    pub payments: Vec<MdkPayment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdkPayment {
    #[serde(rename = "paymentHash")]
    pub payment_hash: String,
    #[serde(rename = "amountSats")]
    pub amount_sats: i64,
    pub direction: String,
    pub timestamp: i64,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct MdkSendResponse {
    pub payment_hash: String,
    pub status: String,
}

pub async fn start_payment_poller(state: Arc<AppState>) {
    let mut ticker = interval(Duration::from_secs(10));
    
    tracing::info!("Payment poller started (checking every 10s)");
    
    loop {
        ticker.tick().await;
        
        if let Err(e) = check_pending_payments(state.clone()).await {
            tracing::error!("Payment poll error: {}", e);
        }
    }
}

async fn check_pending_payments(state: Arc<AppState>) -> Result<()> {
    let pending = db::payments::list_pending(&state.db).await?;
    
    if pending.is_empty() {
        return Ok(());
    }
    
    tracing::debug!("Checking {} pending payments", pending.len());
    
    let mdk_payments = state.wallet.list_payments().await?;
    
    tracing::debug!("MDK returned {} payments", mdk_payments.len());
    for p in &mdk_payments {
        tracing::debug!("MDK payment: hash={}, status={}, direction={}", p.payment_hash, p.status, p.direction);
    }
    
    for payment in pending {
        tracing::debug!("DB payment: id={}, hash={:?}", payment.id, payment.payment_hash);
        if let Some(ref payment_hash) = payment.payment_hash {
            if let Some(mdk_payment) = mdk_payments.iter().find(|p| p.payment_hash == *payment_hash) {
                if mdk_payment.status == "completed" && mdk_payment.direction == "inbound" {
                    tracing::info!(
                        "Payment settled: hash={}, amount={}sats",
                        payment_hash,
                        mdk_payment.amount_sats
                    );
                    
                    db::payments::settle(&state.db, &payment.id).await?;
                    
                    if let Some(task_id) = &payment.task_id {
                        db::tasks::update_status(&state.db, task_id, TaskStatus::Funded).await?;
                        tracing::info!("Task {} marked as funded", task_id);
                    }
                }
            }
        }
    }
    
    Ok(())
}
