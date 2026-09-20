use sqlx::migrate;

use crate::{config::AppConfig, router, state};

pub async fn init_server(config: AppConfig) -> anyhow::Result<()> {
    let listener =
        tokio::net::TcpListener::bind(&format!("{}:{}", config.host, config.port)).await?;
    tracing::info!("running on {}", listener.local_addr()?);

    let state = state::get_shared_state(config).await?;

    // Execute pending database migrations
    migrate!("./src/db/migrations").run(&state.db).await?;

    let router = router::get_router(state);
    axum::serve(listener, router).await?;

    Ok(())
}
