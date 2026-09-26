use anyhow::{Context, Result};
use std::env;
use validator::Validate;

/// Configuration loaded from environment variables.
#[derive(Validate, Clone)]
pub struct AppConfig {
    /// Server port (default: 2729)
    #[validate(range(min = 1, max = u16::MAX, message = "port must be a valid port number"))]
    pub port: u16,
    /// Host to bind to (default: "0.0.0.0")
    #[validate(length(min = 1, max = 255, message = "host must be 1-255 characters"))]
    pub host: String,
    /// Database connection URL (required)
    #[validate(length(min = 1, message = "DATABASE_URL is required"))]
    pub database_url: String,
    /// Maximum paste size in bytes (default: 64KB = 65536)
    #[validate(range(min = 1, max = usize::MAX, message = "max_paste_size must be at least 1"))]
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

        let database_url = env::var("DATABASE_URL").context("DATABASE_URL not set")?;

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
            database_url,
            max_paste_size,
            rate_limit,
        };

        config.validate()?;

        Ok(config)
    }
}
