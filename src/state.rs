use std::env;

use anyhow::Result;
use sqlx::sqlite::SqlitePool;

#[cfg(test)]
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: AppConfig,
}

pub async fn get_shared_state(config: AppConfig) -> Result<AppState> {
    let db = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;

    let shared_state = AppState { db, config };

    Ok(shared_state)
}

/// Test helper: returns an `AppState` backed by fresh in-memory SQLite
/// with migrations applied, so tests don't hit the real database.
#[cfg(test)]
pub async fn test_state() -> AppState {
    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new())
        .await
        .expect("failed to open in-memory sqlite");

    dotenvy::dotenv().ok();

    sqlx::migrate!("./src/db/migrations")
        .run(&db)
        .await
        .expect("failed to run migrations");

    let mut config = AppConfig::load().expect("failed to load config");
    // Tests send no proxy IP headers, so disable rate limiting to avoid 429s.
    config.rate_limit = false;

    AppState { db, config }
}
