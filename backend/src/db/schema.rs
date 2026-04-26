pub const CREATE_TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    prompt TEXT NOT NULL,
    bounty_sats INTEGER NOT NULL,
    stake_sats INTEGER DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending_payment',
    escrow_invoice TEXT,
    payment_hash TEXT,
    buyer_pubkey TEXT NOT NULL,
    worker_pubkey TEXT,
    worker_invoice TEXT,
    result TEXT,
    failure_reason TEXT,
    verified_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS agents (
    pubkey TEXT PRIMARY KEY,
    name TEXT,
    avatar_url TEXT,
    agent_type TEXT DEFAULT 'worker',
    lightning_address TEXT,
    reputation_score REAL DEFAULT 0.0,
    total_tasks INTEGER DEFAULT 0,
    successful_tasks INTEGER DEFAULT 0,
    total_earned_sats INTEGER DEFAULT 0,
    is_active BOOLEAN DEFAULT false,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS payments (
    id TEXT PRIMARY KEY,
    task_id TEXT,
    invoice TEXT NOT NULL,
    payment_hash TEXT,
    amount_sats INTEGER NOT NULL,
    direction TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL,
    settled_at TEXT
);

CREATE TABLE IF NOT EXISTS l402_tokens (
    id TEXT PRIMARY KEY,
    macaroon TEXT NOT NULL UNIQUE,
    payment_hash TEXT,
    amount_sats INTEGER NOT NULL,
    resource TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    used_at TEXT
);

CREATE TABLE IF NOT EXISTS activity_log (
    id TEXT PRIMARY KEY,
    agent_pubkey TEXT NOT NULL,
    agent_name TEXT,
    event_type TEXT NOT NULL,
    event_data TEXT,
    task_id TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_buyer ON tasks(buyer_pubkey);
CREATE INDEX IF NOT EXISTS idx_tasks_worker ON tasks(worker_pubkey);
CREATE INDEX IF NOT EXISTS idx_payments_hash ON payments(payment_hash);
CREATE INDEX IF NOT EXISTS idx_payments_task ON payments(task_id);
CREATE INDEX IF NOT EXISTS idx_activity_created ON activity_log(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_activity_agent ON activity_log(agent_pubkey);
"#;

pub const MIGRATE_AGENTS: &str = r#"
ALTER TABLE agents ADD COLUMN IF NOT EXISTS name TEXT;
ALTER TABLE agents ADD COLUMN IF NOT EXISTS avatar_url TEXT;
ALTER TABLE agents ADD COLUMN IF NOT EXISTS agent_type TEXT DEFAULT 'worker';
ALTER TABLE agents ADD COLUMN IF NOT EXISTS is_active BOOLEAN DEFAULT false;
"#;

pub const MIGRATE_ACTIVITY: &str = r#"
CREATE TABLE IF NOT EXISTS activity_log (
    id TEXT PRIMARY KEY,
    agent_pubkey TEXT NOT NULL,
    agent_name TEXT,
    event_type TEXT NOT NULL,
    event_data TEXT,
    task_id TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_activity_created ON activity_log(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_activity_agent ON activity_log(agent_pubkey);
"#;
