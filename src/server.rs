use std::net::SocketAddr;

use sqlx::migrate;
use tokio::net::TcpListener;

use crate::{api::internal::delete::delete_expired, config::AppConfig, router, state};

pub async fn init_server(config: AppConfig) -> anyhow::Result<()> {
    let listener = TcpListener::bind(&format!("{}:{}", config.host, config.port)).await?;
    tracing::info!("running on {}", listener.local_addr()?);

    let state = state::get_shared_state(config).await?;

    // Execute pending database migrations
    migrate!("./src/db/migrations").run(&state.db).await?;

    // One start up sweep of expired pastes
    let cleaned = delete_expired(&state.db).await?;
    tracing::info!("removed {cleaned} expired pastes");

    let router = router::get_router(state);
    // Provide the peer IP so the rate limiter has a fallback when no
    // X-Forwarded-For / X-Real-IP header is present (e.g. local requests).
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
