mod api;
mod config;
mod router;
mod schema;
mod server;
mod state;

use anyhow::Result;
use config::AppConfig;
use dotenvy::dotenv;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Load env variables
    dotenv().ok();

    // Load configuration from environment variables
    let config = AppConfig::load()?;

    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize server
    server::init_server(config).await?;

    Ok(())
}
