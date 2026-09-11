use std::time::Duration;

use axum::{
    Router,
    extract::Request,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use tower_http::trace::TraceLayer;
use tower_http::{
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    timeout::TimeoutLayer,
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

        tracing::debug_span!("Request", %request_id, %method, %uri)
    });

    let timeout_layer =
        TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(10));

    let request_id_layer = SetRequestIdLayer::x_request_id(MakeRequestUuid);

    // Application routes
    Router::new()
        .route("/health", get(health::health))
        .route("/paste", post(paste::paste))
        .route("/fetch/{id}", get(fetch::fetch))
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
    use crate::state;

    async fn test_state() -> crate::state::AppState {
        dotenvy::dotenv().ok();

        state::get_shared_state()
            .await
            .expect("failed to create test state")
    }

    #[tokio::test]
    async fn health_route_returns_200() {
        let state = test_state().await;

        let response = get_router(state)
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

        let response = get_router(state)
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

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("failed to read response body");

        assert_eq!(&body[..], b"nothing to see here");
    }
}
