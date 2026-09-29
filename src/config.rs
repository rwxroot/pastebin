use anyhow::{Context, Result};
use std::env;

/// Configuration loaded from environment variables.
#[derive(Clone)]
pub struct AppConfig {
    /// Server port (default: 2729)
    pub port: u16,
    /// Host to bind to (default: "0.0.0.0")
    pub host: String,
    /// Maximum paste size in bytes (default: 64KB = 65536)
    pub max_paste_size: usize,
    /// Enable per-client rate limiting (default: true). Set PASTEBIN_RATE_LIMIT=false to disable.
    pub rate_limit: bool,
}

impl AppConfig {
    /// Load configuration from environment variables.
    /// Panics if required environment variables are not set.
    pub fn load() -> Result<Self> {
        let port = env::var("PASTEBIN_PORT")
            .unwrap_or("2729".to_string())
            .parse::<u16>()
            .context("Invalid PASTEBIN_PORT")?;

        let host = env::var("PASTEBIN_HOST").unwrap_or("0.0.0.0".to_string());

        // Fail fast at startup if the DB URL is missing.
        env::var("DATABASE_URL").context("DATABASE_URL not set")?;

        let max_paste_size = env::var("PASTEBIN_MAX_PASTE_SIZE")
            .unwrap_or("65536".to_string())
            .parse::<usize>()
            .context("Invalid PASTEBIN_MAX_PASTE_SIZE")?;

        let rate_limit = env::var("PASTEBIN_RATE_LIMIT")
            .map(|v| v.parse::<bool>().unwrap_or(true))
            .unwrap_or(true);

        let config = AppConfig {
            port,
            host,
            max_paste_size,
            rate_limit,
        };

        Ok(config)
    }
}
