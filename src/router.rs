use std::time::Duration;

use axum::{
    Router,
    extract::Request,
    http::StatusCode,
    routing::{get, post},
};
use tower_http::{
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use crate::{
    api::{fetch, health, paste},
    state::AppState,
    ui::{fetch as ui_fetch, not_found as ui_not_found, paste as ui_paste},
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

    let propagate_request_id_layer = PropagateRequestIdLayer::x_request_id();

    // UI routes
    let ui_router = Router::new()
        .route("/", get(ui_paste::index))
        .route("/fetch/{id}", get(ui_fetch::paste_view));

    // API routes
    let api_router = Router::new()
        .route("/api/health", get(health::health))
        .route(
            "/api/paste",
            post(paste::paste).layer(RequestBodyLimitLayer::new(state.config.max_paste_size)),
        )
        .route("/api/fetch/{id}", get(fetch::fetch));

    let router = ui_router
        .merge(api_router)
        .fallback(ui_not_found::not_found);

    router
        .layer(trace_layer)
        .layer(timeout_layer)
        .layer(propagate_request_id_layer)
        .layer(request_id_layer)
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::get_router;
    use crate::state::test_state;

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
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn responses_carry_request_id_header() {
        // The SetRequestIdLayer must stamp every response with an
        // x-request-id header, which the trace layer uses for correlation.
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .expect("response should carry an x-request-id header");

        // MakeRequestUuid generates v4 UUIDs.
        assert_eq!(request_id.len(), 36);
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
        let body_str = String::from_utf8(body).unwrap();

        assert!(body_str.contains("404"), "should render the 404 page");
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
                    .uri("/api/paste")
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
                    .uri("/api/paste")
                    .header("content-type", "application/json")
                    .body(Body::from(body_at_limit))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Body at limit should be processed (may succeed or fail validation, but not 413)
        assert_ne!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn index_page_returns_200() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response_body_bytes(response).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        // Check that the index page contains expected elements
        assert!(
            body_str.contains("<!DOCTYPE html>"),
            "Should contain DOCTYPE"
        );
        assert!(body_str.contains("textarea"), "Should contain textarea");
        assert!(
            body_str.contains("id=\"submit-btn\">Paste"),
            "Should contain Paste button"
        );
    }

    #[tokio::test]
    async fn view_paste_returns_404_when_not_found() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/fetch/nonexistent-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn view_paste_propagates_db_failure_as_500() {
        // UI route delegates to the API fetch handler, so a DB failure must
        // surface as a 500 (propagated), not be masked as a 404 page alone.
        let state = test_state().await;
        state.db.close().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/fetch/some-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn view_paste_returns_page_when_exists() {
        let state = test_state().await;

        // First, create a paste via the API
        let paste_body = serde_json::json!({
            "content": "Test paste content"
        });

        let create_response = router(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/paste")
                    .header("content-type", "application/json")
                    .body(Body::from(paste_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::CREATED);

        let body = response_body_bytes(create_response).await;
        let paste_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let paste_id = paste_response["id"].as_str().unwrap();

        // Now fetch the paste via the UI route
        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/fetch/{}", paste_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response_body_bytes(response).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        // Check that the paste view contains expected elements
        assert!(
            body_str.contains("<!DOCTYPE html>"),
            "Should contain DOCTYPE"
        );
        assert!(
            body_str.contains("Test paste content"),
            "Should contain paste content"
        );
        assert!(body_str.contains(paste_id), "Should contain paste ID");
    }

    #[tokio::test]
    async fn paste_view_escapes_html_in_content() {
        // User-supplied content must not be injected as raw HTML (XSS).
        let state = test_state().await;
        let payload = "<script>alert('xss')</script><b>bold</b>";

        let created = router(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/paste")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "content": payload }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::CREATED);

        let id = serde_json::from_slice::<serde_json::Value>(&response_body_bytes(created).await)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/fetch/{}", id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = String::from_utf8(response_body_bytes(response).await).unwrap();

        // The page has legit `<script>` blocks (copy button JS), so assert on
        // the content region itself: raw user markup must not survive.
        let content_line = body
            .lines()
            .find(|l| l.contains("paste-content"))
            .expect("paste content region");

        assert!(
            !content_line.contains("<script>") && !content_line.contains("<b>"),
            "raw user markup must not be rendered: {}",
            content_line
        );
        assert!(
            content_line.contains("&#60;script&#62;") && content_line.contains("&#60;b&#62;"),
            "user markup should be HTML-escaped: {}",
            content_line
        );
    }

    #[tokio::test]
    async fn api_fetch_returns_404_when_not_found() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/fetch/nonexistent-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn api_fetch_returns_json_when_exists() {
        let state = test_state().await;
        let id = format!("test-{}", nanoid::nanoid!(6));
        let content = "Test paste content";

        sqlx::query(
            r#"
            INSERT INTO pastes (id, content, created_at, expires_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(content)
        .bind(1_787_313_600_i64)
        .bind(Option::<i64>::None)
        .execute(&state.db)
        .await
        .expect("failed to insert test paste");

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/fetch/{}", id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response_body_bytes(response).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("\"id\""), "Should contain id field");
        assert!(body_str.contains(content), "Should contain paste content");
        assert!(body_str.contains(&id), "Should contain paste ID");
    }
}
