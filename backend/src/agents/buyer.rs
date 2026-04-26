use anyhow::Result;

pub struct BuyerAgent {
    pub pubkey: String,
}

impl BuyerAgent {
    pub fn new(pubkey: Option<String>) -> Self {
        let pubkey = pubkey.unwrap_or_else(|| format!("buyer_{}", uuid::Uuid::new_v4()));
        Self { pubkey }
    }
    
    pub async fn create_task(&self, api_url: &str, prompt: &str, bounty_sats: i64, stake_sats: Option<i64>) -> Result<crate::models::CreateTaskResponse> {
        let client = reqwest::Client::new();
        
        let res = client
            .post(format!("{}/api/tasks", api_url))
            .json(&crate::models::CreateTaskRequest {
                prompt: prompt.to_string(),
                bounty_sats,
                stake_sats,
                buyer_pubkey: Some(self.pubkey.clone()),
            })
            .send()
            .await?
            .json()
            .await?;
        
        Ok(res)
    }
}
