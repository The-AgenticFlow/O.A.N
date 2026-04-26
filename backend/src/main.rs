mod routes;
mod db;
mod lexe;
mod agents;
mod models;
mod wallet;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct AppState {
    pub db: db::Database,
    pub config: Config,
    pub wallet: wallet::MdkClient,
}

#[derive(Clone)]
pub struct Config {
    pub platform_fee_percent: f64,
    pub l402_task_board_cost_sats: u64,
    pub fireworks_api_key: Option<String>,
    pub fireworks_model: String,
    pub lexe_seed: Option<String>,
    pub mdk_wallet_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            platform_fee_percent: 1.0,
            l402_task_board_cost_sats: 1,
            fireworks_api_key: std::env::var("FIREWORKS_API_KEY").ok(),
            fireworks_model: std::env::var("FIREWORKS_MODEL")
                .unwrap_or_else(|_| "accounts/fireworks/models/llama-v3-70b-instruct".to_string()),
            lexe_seed: std::env::var("LEXE_SEED").ok(),
            mdk_wallet_port: std::env::var("MDK_WALLET_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3456),
        }
    }
}

pub type SharedState = Arc<AppState>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "oan_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();

    let config = Config::default();
    let supabase_url = std::env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_anon_key = std::env::var("SUPABASE_ANON_KEY").expect("SUPABASE_ANON_KEY must be set");
    let db = db::Database::new(&supabase_url, &supabase_anon_key);
    
    db.migrate().await?;

    let wallet = wallet::MdkClient::new(config.mdk_wallet_port);

    let state = Arc::new(AppState {
        db,
        config,
        wallet,
    });

    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/api/tasks", get(routes::tasks::list_tasks).post(routes::tasks::create_task))
        .route("/api/tasks/:id", get(routes::tasks::get_task))
        .route("/api/tasks/:id/claim", post(routes::tasks::claim_task))
        .route("/api/tasks/:id/submit", post(routes::tasks::submit_task))
        .route("/api/tasks/:id/status", get(routes::tasks::task_status))
        .route("/api/webhooks/payment", post(routes::webhooks::payment_webhook))
        .route("/api/agent/balance", get(routes::agent::get_balance))
        .route("/api/agent/withdraw", post(routes::agent::withdraw))
        .route("/api/l402/verify", post(routes::l402::verify_token))
        .route("/api/wallet/balance", get(routes::wallet::get_balance))
        .route("/api/wallet/receive", post(routes::wallet::receive))
        .route("/api/wallet/send", post(routes::wallet::send_payment))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("🚀 OAN Backend listening on http://localhost:3000");
    tracing::info!("💰 MDK Wallet on http://localhost:{}", state.config.mdk_wallet_port);
    
    axum::serve(listener, app).await?;

    Ok(())
}
