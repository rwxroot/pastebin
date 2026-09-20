use std::env;

use anyhow::Result;
use sqlx::sqlite::SqlitePool;

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
