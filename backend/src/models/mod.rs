use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    PendingPayment,
    Funded,
    Claimed,
    Verifying,
    Completed,
    Failed,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::PendingPayment => write!(f, "pending_payment"),
            TaskStatus::Funded => write!(f, "funded"),
            TaskStatus::Claimed => write!(f, "claimed"),
            TaskStatus::Verifying => write!(f, "verifying"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending_payment" => Ok(TaskStatus::PendingPayment),
            "funded" => Ok(TaskStatus::Funded),
            "claimed" => Ok(TaskStatus::Claimed),
            "verifying" => Ok(TaskStatus::Verifying),
            "completed" => Ok(TaskStatus::Completed),
            "failed" => Ok(TaskStatus::Failed),
            _ => Err(format!("Invalid task status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub prompt: String,
    pub bounty_sats: i64,
    pub stake_sats: i64,
    pub status: String,
    pub escrow_invoice: Option<String>,
    pub payment_hash: Option<String>,
    pub buyer_pubkey: String,
    pub worker_pubkey: Option<String>,
    pub worker_invoice: Option<String>,
    pub result: Option<String>,
    pub verified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Task {
    pub fn status_enum(&self) -> TaskStatus {
        self.status.parse().unwrap_or(TaskStatus::PendingPayment)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub prompt: String,
    pub bounty_sats: i64,
    pub stake_sats: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskResponse {
    pub task_id: String,
    pub escrow_invoice: String,
    pub payment_hash: String,
    pub amount_sats: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimTaskRequest {
    pub worker_pubkey: String,
    pub worker_invoice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimTaskResponse {
    pub claimed: bool,
    pub stake_invoice: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitTaskRequest {
    pub result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatusResponse {
    pub task_id: String,
    pub status: String,
    pub result: Option<String>,
    pub payout_tx: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub pubkey: String,
    pub lightning_address: Option<String>,
    pub reputation_score: f64,
    pub total_tasks: i64,
    pub successful_tasks: i64,
    pub total_earned_sats: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentDirection {
    In,
    Out,
}

impl std::fmt::Display for PaymentDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentDirection::In => write!(f, "in"),
            PaymentDirection::Out => write!(f, "out"),
        }
    }
}

impl std::str::FromStr for PaymentDirection {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "in" => Ok(PaymentDirection::In),
            "out" => Ok(PaymentDirection::Out),
            _ => Err(format!("Invalid payment direction: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentStatus {
    Pending,
    Paid,
    Failed,
}

impl std::fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentStatus::Pending => write!(f, "pending"),
            PaymentStatus::Paid => write!(f, "paid"),
            PaymentStatus::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: String,
    pub task_id: Option<String>,
    pub invoice: String,
    pub payment_hash: Option<String>,
    pub amount_sats: i64,
    pub direction: String,
    pub status: String,
    pub created_at: String,
    pub settled_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L402Challenge {
    pub macaroon: String,
    pub invoice: String,
    pub payment_hash: String,
    pub amount_sats: u64,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawRequest {
    pub lightning_address: String,
    pub amount_sats: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawResponse {
    pub payment_hash: String,
    pub amount_sats: i64,
}
