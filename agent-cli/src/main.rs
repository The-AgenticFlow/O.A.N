mod commands;

use clap::{Parser, Subcommand};
use anyhow::Result;
use std::path::Path;

#[derive(Parser)]
#[command(name = "oan-agent")]
#[command(about = "OAN Agent CLI - Autonomous task worker/buyer")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, env = "OAN_API_URL", default_value = "http://localhost:3000")]
    api_url: String,
    
    #[arg(long, env = "FIREWORKS_API_KEY")]
    fireworks_key: Option<String>,
    
    #[arg(long, env = "FIREWORKS_MODEL", default_value = "accounts/fireworks/models/llama-v3-70b-instruct")]
    fireworks_model: String,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[arg(short, long, default_value = "worker")]
        mode: String,
        
        #[arg(short, long)]
        pubkey: Option<String>,
        
        #[arg(short, long)]
        lightning_address: Option<String>,
    },
    
    Create {
        #[arg(short, long)]
        prompt: String,
        
        #[arg(short, long, default_value = "100")]
        bounty: i64,
        
        #[arg(short, long, default_value = "0")]
        stake: i64,
    },
    
    List,
    
    Claim {
        #[arg(short, long)]
        task_id: String,
        
        #[arg(short, long)]
        invoice: String,
    },
    
    Submit {
        #[arg(short, long)]
        task_id: String,
        
        #[arg(short, long)]
        result: String,
    },
    
    Balance,
}

fn load_env() {
    let paths = [".env", "backend/.env", "../backend/.env"];
    for path in &paths {
        if Path::new(path).exists() {
            let _ = dotenv::from_path(path);
            return;
        }
    }
    let _ = dotenv::dotenv();
}

#[tokio::main]
async fn main() -> Result<()> {
    load_env();
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    let config = commands::Config {
        api_url: cli.api_url,
        fireworks_key: cli.fireworks_key,
        fireworks_model: cli.fireworks_model,
    };
    
    match cli.command {
        Commands::Run { mode, pubkey, lightning_address } => {
            commands::run_agent(mode, pubkey, lightning_address, config).await?;
        }
        Commands::Create { prompt, bounty, stake } => {
            commands::create_task(prompt, bounty, stake, config).await?;
        }
        Commands::List => {
            commands::list_tasks(config).await?;
        }
        Commands::Claim { task_id, invoice } => {
            commands::claim_task(task_id, invoice, config).await?;
        }
        Commands::Submit { task_id, result } => {
            commands::submit_task(task_id, result, config).await?;
        }
        Commands::Balance => {
            commands::get_balance(config).await?;
        }
    }
    
    Ok(())
}
