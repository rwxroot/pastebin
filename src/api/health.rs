use axum::{extract::State, http::StatusCode};

use crate::state::AppState;

pub async fn health(State(state): State<AppState>) -> Result<(), StatusCode> {
    sqlx::query("SELECT 1")
        .execute(&state.db)
        .await
        .map_err(|error| {
            tracing::error!(%error, "database health check failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use tower::ServiceExt;

    use super::health;
    use crate::state;

    async fn test_state() -> crate::state::AppState {
        dotenvy::dotenv().ok();

        state::get_shared_state()
            .await
            .expect("failed to create test state")
    }

    fn router(state: crate::state::AppState) -> Router {
        Router::new()
            .route("/health", get(health))
            .with_state(state)
    }

    #[tokio::test]
    async fn health_returns_200_when_database_is_available() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn health_returns_an_empty_body() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("failed to read response body");

        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn health_returns_500_when_database_is_unavailable() {
        let state = test_state().await;

        // Closing the pool makes the SELECT 1 query fail with PoolClosed.
        state.db.close().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
