use std::env;

use anyhow::Result;
use sqlx::sqlite::SqlitePool;

pub type AppState = State;

#[derive(Clone)]
pub struct State {
    pub db: SqlitePool,
}

pub async fn get_shared_state() -> Result<AppState> {
    let db = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    let shared_state = State { db };

    Ok(shared_state)
}
