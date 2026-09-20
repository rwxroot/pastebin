use std::time::Duration;

use axum::{
    Router,
    extract::Request,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use tower_http::{
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use crate::{
    api::{fetch, health, paste},
    state::AppState,
};

pub fn get_router(state: AppState) -> Router {
    let trace_layer = TraceLayer::new_for_http().make_span_with(|req: &Request| {
        let uri = req.uri();
        let method = req.method();
        let request_id = req
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("unknown-x-request-id");
        let real_ip = req
            .headers()
            .get("x-real-ip")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("unknown-x-real-ip");

        tracing::debug_span!("Request", %request_id, %real_ip, %method, %uri)
    });

    let timeout_layer =
        TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(15));

    let request_id_layer = SetRequestIdLayer::x_request_id(MakeRequestUuid);

    // Application routes
    Router::new()
        .route("/health", get(health::health))
        .route("/fetch/{id}", get(fetch::fetch))
        .route(
            "/paste",
            post(paste::paste).layer(RequestBodyLimitLayer::new(state.config.max_paste_size)),
        )
        .layer(trace_layer)
        .layer(timeout_layer)
        .layer(request_id_layer)
        .fallback(handler_404)
        .with_state(state)
}

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::get_router;
    use crate::{config::AppConfig, state};

    async fn test_state() -> crate::state::AppState {
        dotenvy::dotenv().ok();
        let config = AppConfig::load().unwrap();

        state::get_shared_state(config)
            .await
            .expect("failed to create test state")
    }

    fn router(state: crate::state::AppState) -> axum::Router {
        get_router(state)
    }

    async fn response_body_bytes(response: axum::response::Response) -> Vec<u8> {
        to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("failed to read response body")
            .to_vec()
    }

    #[tokio::test]
    async fn health_route_returns_200() {
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
    async fn unknown_route_returns_404() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/does/not/exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = response_body_bytes(response).await;

        assert_eq!(&body[..], b"nothing to see here");
    }

    #[tokio::test]
    async fn paste_enforces_body_size_limit() {
        let state = test_state().await;
        let max_size = state.config.max_paste_size;

        // Create a body that exceeds the limit
        let oversized_body = "x".repeat(max_size + 1);

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/paste")
                    .header("content-type", "application/json")
                    .body(Body::from(oversized_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn paste_accepts_body_at_limit() {
        let state = test_state().await;
        let max_size = state.config.max_paste_size;

        // Create a body exactly at the limit
        let body_at_limit = "x".repeat(max_size);

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/paste")
                    .header("content-type", "application/json")
                    .body(Body::from(body_at_limit))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Body at limit should be processed (may succeed or fail validation, but not 413)
        assert_ne!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }
}
