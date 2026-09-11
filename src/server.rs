use sqlx::migrate;

use crate::{config, router, state};

pub async fn init_server(config: config::Config) -> anyhow::Result<()> {
    let state = state::get_shared_state().await?;

    // Execute pending database migrations
    migrate!("./src/db/migrations").run(&state.db).await?;

    let router = router::get_router(state);

    let listener =
        tokio::net::TcpListener::bind(&format!("{}:{}", config.host, config.port)).await?;
    tracing::info!("running on {}", listener.local_addr()?);

    axum::serve(listener, router).await?;

    Ok(())
}
