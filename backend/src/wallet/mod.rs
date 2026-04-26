use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

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
    amountSats: Option<i64>,
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
        
        let resp = self.client
            .post(format!("{}/receive", self.base_url))
            .json(&body)
            .send()
            .await?
            .json::<MdkResponse<MdkInvoiceResponse>>()
            .await?;
        Ok(resp.data)
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
        let body = MdkSendRequest {
            destination: destination.to_string(),
            amountSats: amount_sats,
        };
        
        let resp = self.client
            .post(format!("{}/send", self.base_url))
            .json(&body)
            .send()
            .await?
            .json::<MdkResponse<MdkSendData>>()
            .await?;
        Ok(MdkSendResponse {
            payment_hash: resp.data.payment_hash,
            status: resp.data.status,
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
}

#[derive(Debug, Clone)]
pub struct MdkSendResponse {
    pub payment_hash: String,
    pub status: String,
}
