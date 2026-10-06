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

/// Test helper
///
/// Returns an `AppState` backed by fresh in-memory SQLite
/// with migrations applied, so tests don't hit the real database.
#[cfg(test)]
pub async fn test_state() -> AppState {
    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new())
        .await
        .expect("failed to open in-memory sqlite");

    sqlx::migrate!("./src/db/migrations")
        .run(&db)
        .await
        .expect("failed to run migrations");

    AppState {
        db,
        config: AppConfig {
            port: 2729,
            host: "0.0.0.0".into(),
            max_paste_size: 65536,
            rate_limit: false,
        },
    }
}
