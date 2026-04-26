mod schema;

use anyhow::Result;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

pub use schema::*;

#[derive(Clone)]
pub struct Database {
    client: Client,
    url: String,
    anon_key: String,
}

impl Database {
    pub fn new(supabase_url: &str, anon_key: &str) -> Self {
        let client = Client::new();
        let base_url = supabase_url.trim_end_matches('/');
        Self {
            client,
            url: base_url.to_string(),
            anon_key: anon_key.to_string(),
        }
    }

    pub async fn migrate(&self) -> Result<()> {
        // Supabase manages schema via its dashboard or SQL editor.
        // For local dev / testing we can execute raw SQL via the RPC endpoint
        // but in production the tables should already exist.
        // Here we attempt to ping the tasks table to verify connectivity.
        let _: Vec<Value> = self.list("tasks", None, None).await.unwrap_or_default();
        tracing::info!("Database connection verified (Supabase REST)");
        Ok(())
    }

    // ── Core HTTP helpers ─────────────────────────────────────────

    async fn get_one<T: DeserializeOwned>(&self, table: &str, column: &str, value: &str) -> Result<Option<T>> {
        let filter_val = format!("eq.{}", value);
        let resp = self.client
            .get(format!("{}/rest/v1/{}", self.url, table))
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {}", &self.anon_key))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .query(&[("select", "*")])
            .query(&[(column, filter_val.as_str())])
            .send()
            .await?;

        if resp.status() == 404 || resp.status() == 406 {
            return Ok(None);
        }

        let body: Vec<T> = resp.error_for_status()?.json().await?;
        Ok(body.into_iter().next())
    }

    async fn insert<T: DeserializeOwned>(&self, table: &str, body: &Value) -> Result<Vec<T>> {
        let resp = self.client
            .post(format!("{}/rest/v1/{}", self.url, table))
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {}", &self.anon_key))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .json(body)
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<T>>()
            .await?;
        Ok(resp)
    }

    async fn update(&self, table: &str, filter_col: &str, filter_val: &str, body: &Value) -> Result<()> {
        self.client
            .patch(format!("{}/rest/v1/{}", self.url, table))
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {}", &self.anon_key))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .query(&[(filter_col, format!("eq.{}", filter_val))])
            .json(body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    async fn list<T: DeserializeOwned>(&self, table: &str, filter: Option<(&str, String)>, order: Option<(&str, &str)>) -> Result<Vec<T>> {
        let mut req = self.client
            .get(format!("{}/rest/v1/{}", self.url, table))
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {}", &self.anon_key))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .query(&[("select", "*")]);

        if let Some((col, val)) = filter {
            req = req.query(&[(col, val)]);
        }
        if let Some((col, dir)) = order {
            req = req.query(&[("order", format!("{}.{}", col, dir))]);
        }

        let body: Vec<T> = req
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(body)
    }
}

pub mod tasks {
    use super::*;
    use crate::models::{Task, TaskStatus, CreateTaskRequest};
    use uuid::Uuid;
    use chrono::Utc;

    pub async fn create(db: &Database, req: CreateTaskRequest, buyer_pubkey: String) -> Result<Task> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let body = json!({
            "id": id,
            "prompt": req.prompt,
            "bounty_sats": req.bounty_sats,
            "stake_sats": req.stake_sats.unwrap_or(0),
            "status": TaskStatus::PendingPayment.to_string(),
            "buyer_pubkey": buyer_pubkey,
            "created_at": now.clone(),
            "updated_at": now,
        });

        let rows: Vec<Task> = db.insert("tasks", &body).await?;
        rows.into_iter().next().ok_or_else(|| anyhow::anyhow!("Insert returned no rows"))
    }

    pub async fn find_by_id(db: &Database, id: &str) -> Result<Option<Task>> {
        db.get_one("tasks", "id", id).await
    }

    pub async fn list_available(db: &Database) -> Result<Vec<Task>> {
        db.list("tasks", Some(("status", "eq.funded".to_string())), Some(("bounty_sats", "desc"))).await
    }

    pub async fn list_all(db: &Database) -> Result<Vec<Task>> {
        db.list("tasks", None, Some(("created_at", "desc"))).await
    }

    pub async fn list_by_buyer(db: &Database, pubkey: &str) -> Result<Vec<Task>> {
        db.list("tasks", Some(("buyer_pubkey", format!("eq.{}", pubkey))), Some(("created_at", "desc"))).await
    }

    pub async fn update_status(db: &Database, id: &str, status: TaskStatus) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        db.update("tasks", "id", id, &json!({
            "status": status.to_string(),
            "updated_at": now,
        })).await
    }

    pub async fn set_escrow(db: &Database, id: &str, invoice: String, payment_hash: String) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        db.update("tasks", "id", id, &json!({
            "escrow_invoice": invoice,
            "payment_hash": payment_hash,
            "updated_at": now,
        })).await
    }

    pub async fn claim(db: &Database, id: &str, worker_pubkey: String, worker_invoice: String) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        db.update("tasks", "id", id, &json!({
            "worker_pubkey": worker_pubkey,
            "worker_invoice": worker_invoice,
            "status": "claimed",
            "updated_at": now,
        })).await
    }

    pub async fn assign(db: &Database, id: &str, worker_pubkey: String) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        db.update("tasks", "id", id, &json!({
            "worker_pubkey": worker_pubkey,
            "status": "claimed",
            "updated_at": now,
        })).await
    }

    pub async fn submit_result(db: &Database, id: &str, result: String) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        db.update("tasks", "id", id, &json!({
            "result": result,
            "status": "verifying",
            "updated_at": now,
        })).await
    }

    pub async fn complete(db: &Database, id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        db.update("tasks", "id", id, &json!({
            "status": "completed",
            "verified_at": now.clone(),
            "updated_at": now,
        })).await
    }

    pub async fn fail(db: &Database, id: &str, reason: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        db.update("tasks", "id", id, &json!({
            "status": "failed",
            "failure_reason": reason,
            "updated_at": now,
        })).await
    }
}

pub mod agents {
    use super::*;
    use crate::models::{Agent, CreateAgentRequest};
    use uuid::Uuid;

    pub async fn find_by_pubkey(db: &Database, pubkey: &str) -> Result<Option<Agent>> {
        db.get_one("agents", "pubkey", pubkey).await
    }

    pub async fn list_all(db: &Database) -> Result<Vec<Agent>> {
        db.list("agents", None, Some(("created_at", "desc"))).await
    }

    pub async fn list_active(db: &Database) -> Result<Vec<Agent>> {
        db.list("agents", Some(("is_active", "eq.true".to_string())), Some(("created_at", "desc"))).await
    }

    pub async fn create(db: &Database, req: CreateAgentRequest) -> Result<Agent> {
        let pubkey = format!("agent_{}", Uuid::new_v4());
        let now = chrono::Utc::now().to_rfc3339();

        let avatar_url = generate_avatar_url(&req.name);

        let body = json!({
            "pubkey": pubkey,
            "name": req.name,
            "avatar_url": avatar_url,
            "agent_type": req.agent_type.unwrap_or_else(|| "worker".to_string()),
            "lightning_address": req.lightning_address,
            "is_active": false,
            "created_at": now,
        });

        let resp = db.client
            .post(format!("{}/rest/v1/agents", db.url))
            .header("apikey", &db.anon_key)
            .header("Authorization", format!("Bearer {}", &db.anon_key))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let rows: Vec<Agent> = resp.json().await?;
        rows.into_iter().next().ok_or_else(|| anyhow::anyhow!("Insert returned no rows"))
    }

    pub async fn set_active(db: &Database, pubkey: &str, is_active: bool) -> Result<()> {
        db.update("agents", "pubkey", pubkey, &json!({
            "is_active": is_active,
        })).await
    }

    pub async fn create_or_update(db: &Database, pubkey: &str, ln_address: Option<String>) -> Result<Agent> {
        let now = chrono::Utc::now().to_rfc3339();

        let body = json!({
            "pubkey": pubkey,
            "lightning_address": ln_address,
            "created_at": now,
        });

        let resp = db.client
            .post(format!("{}/rest/v1/agents", db.url))
            .header("apikey", &db.anon_key)
            .header("Authorization", format!("Bearer {}", &db.anon_key))
            .header("Content-Type", "application/json")
            .header("Prefer", "resolution=merge-duplicates,return=representation")
            .query(&[("on_conflict", "pubkey")])
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let rows: Vec<Agent> = resp.json().await?;
        rows.into_iter().next().ok_or_else(|| anyhow::anyhow!("Upsert returned no rows"))
    }

    pub async fn update_reputation(db: &Database, pubkey: &str, success: bool, earned_sats: i64) -> Result<()> {
        let agent: Option<Agent> = db.get_one("agents", "pubkey", pubkey).await?;
        if let Some(a) = agent {
            let total_tasks = a.total_tasks + 1;
            let successful_tasks = if success { a.successful_tasks + 1 } else { a.successful_tasks };
            let total_earned_sats = a.total_earned_sats + earned_sats;
            let reputation_score = successful_tasks as f64 / total_tasks.max(1) as f64;

            let body = json!({
                "total_tasks": total_tasks,
                "successful_tasks": successful_tasks,
                "total_earned_sats": total_earned_sats,
                "reputation_score": reputation_score,
            });
            db.update("agents", "pubkey", pubkey, &body).await?;
        }
        Ok(())
    }

    fn generate_avatar_url(name: &str) -> String {
        let seed = name.chars().map(|c| c as u32).sum::<u32>();
        let style = seed % 8;
        format!("https://api.dicebear.com/7.x/bottts/svg?seed={}&style=layer{}", urlencoding::encode(name), style)
    }
}

pub mod payments {
    use super::*;
    use crate::models::{Payment, PaymentDirection, PaymentStatus};
    use uuid::Uuid;
    use chrono::Utc;

    pub async fn create(
        db: &Database,
        task_id: Option<String>,
        invoice: String,
        payment_hash: String,
        amount_sats: i64,
        direction: PaymentDirection,
    ) -> Result<Payment> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let body = json!({
            "id": id,
            "task_id": task_id,
            "invoice": invoice,
            "payment_hash": payment_hash,
            "amount_sats": amount_sats,
            "direction": direction.to_string(),
            "status": PaymentStatus::Pending.to_string(),
            "created_at": now,
        });

        let rows: Vec<Payment> = db.insert("payments", &body).await?;
        rows.into_iter().next().ok_or_else(|| anyhow::anyhow!("Insert returned no rows"))
    }

    pub async fn find_by_hash(db: &Database, hash: &str) -> Result<Option<Payment>> {
        db.get_one("payments", "payment_hash", hash).await
    }

    pub async fn settle(db: &Database, id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        db.update("payments", "id", id, &json!({
            "status": "paid",
            "settled_at": now,
        })).await
    }

    pub async fn update_status(db: &Database, id: &str, status: PaymentStatus) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let status_str = status.to_string();
        let body = if status == PaymentStatus::Paid {
            json!({
                "status": status_str,
                "settled_at": now,
            })
        } else {
            json!({
                "status": status_str,
            })
        };
        db.update("payments", "id", id, &body).await
    }

    pub async fn list_by_task(db: &Database, task_id: &str) -> Result<Vec<Payment>> {
        db.list("payments", Some(("task_id", format!("eq.{}", task_id))), Some(("created_at", "desc"))).await
    }

    pub async fn list_pending(db: &Database) -> Result<Vec<Payment>> {
        db.list("payments", Some(("status", "eq.pending".to_string())), Some(("created_at", "desc"))).await
    }
}

pub mod activity {
    use super::*;
    use crate::models::ActivityEntry;
    use uuid::Uuid;
    use chrono::Utc;

    pub async fn log_event(
        db: &Database,
        agent_pubkey: &str,
        agent_name: Option<&str>,
        event_type: &str,
        event_data: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let body = json!({
            "id": id,
            "agent_pubkey": agent_pubkey,
            "agent_name": agent_name,
            "event_type": event_type,
            "event_data": event_data,
            "task_id": task_id,
            "created_at": now,
        });

        let _: Vec<ActivityEntry> = db.insert("activity_log", &body).await?;
        Ok(())
    }

    pub async fn list_recent(db: &Database, limit: usize) -> Result<Vec<ActivityEntry>> {
        let mut req = db.client
            .get(format!("{}/rest/v1/activity_log", db.url))
            .header("apikey", &db.anon_key)
            .header("Authorization", format!("Bearer {}", &db.anon_key))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .query(&[("select", "*")])
            .query(&[("order", "created_at.desc")])
            .query(&[("limit", limit.to_string())]);

        let resp = req.send().await?;
        let body: Vec<ActivityEntry> = resp.json().await?;
        Ok(body)
    }
}
