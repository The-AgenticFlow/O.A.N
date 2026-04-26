use anyhow::Result;

pub struct WorkerAgent {
    pub pubkey: String,
    pub lightning_address: String,
}

impl WorkerAgent {
    pub fn new(pubkey: Option<String>, ln_address: Option<String>) -> Self {
        let pubkey = pubkey.unwrap_or_else(|| format!("agent_{}", uuid::Uuid::new_v4()));
        let lightning_address = ln_address.unwrap_or_else(|| format!("{}@lexe.app", pubkey));
        
        Self { pubkey, lightning_address }
    }
    
    pub async fn run(&self, api_url: &str) -> Result<()> {
        tracing::info!("Worker agent {} starting", self.pubkey);
        
        loop {
            if let Err(e) = self.cycle(api_url).await {
                tracing::error!("Worker cycle error: {}", e);
            }
            
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    }
    
    async fn cycle(&self, api_url: &str) -> Result<()> {
        let client = reqwest::Client::new();
        
        let tasks: Vec<crate::models::Task> = client
            .get(format!("{}/api/tasks", api_url))
            .send()
            .await?
            .json()
            .await?;
        
        if tasks.is_empty() {
            return Ok(());
        }
        
        let task = &tasks[0];
        tracing::info!("Found task {} with {} sats bounty", task.id, task.bounty_sats);
        
        let res: crate::models::ClaimTaskResponse = client
            .post(format!("{}/api/tasks/{}/claim", api_url, task.id))
            .json(&crate::models::ClaimTaskRequest {
                worker_pubkey: self.pubkey.clone(),
                worker_invoice: self.lightning_address.clone(),
            })
            .send()
            .await?
            .json()
            .await?;
        
        if !res.claimed {
            return Ok(());
        }
        
        tracing::info!("Task claimed! Working on: {}", task.prompt);
        
        let result = format!("Completed: {}", task.prompt);
        
        client
            .post(format!("{}/api/tasks/{}/submit", api_url, task.id))
            .json(&serde_json::json!({ "result": result }))
            .send()
            .await?;
        
        tracing::info!("Result submitted!");
        
        Ok(())
    }
}
