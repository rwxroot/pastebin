use axum::{Json, extract::State, http::StatusCode};
use chrono::Utc;
use tracing::instrument;

use crate::{
    schema::{
        id::random_id,
        paste::{PasteRequest, PasteResponse},
    },
    state::AppState,
};

#[instrument(
    name = "POST /paste",
    skip(state),
    fields(length = req.content.len())
)]
pub async fn paste(
    State(state): State<AppState>,
    Json(req): Json<PasteRequest>,
) -> Result<(StatusCode, Json<PasteResponse>), StatusCode> {
    let id = random_id();
    let created_at = Utc::now().timestamp();
    let expires_at = req
        .expires_in
        .map(|hours| created_at + chrono::Duration::hours(hours).num_seconds());

    let paste = sqlx::query_as!(
        PasteResponse,
        r#"
        INSERT INTO pastes (id, content, created_at, expires_at)
        VALUES (?, ?, ?, ?)
        RETURNING id, created_at, expires_at
        "#,
        id,
        req.content,
        created_at,
        expires_at,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|error| {
        tracing::error!(%error, "failed to create paste");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((StatusCode::CREATED, Json(paste)))
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode, header},
        routing::post,
    };
    use chrono::Utc;
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::paste;
    use crate::{config::AppConfig, state};

    async fn test_state() -> crate::state::AppState {
        dotenvy::dotenv().ok();
        let config = AppConfig::load().unwrap();

        state::get_shared_state(config)
            .await
            .expect("failed to create test state")
    }

    fn router(state: crate::state::AppState) -> Router {
        Router::new().route("/paste", post(paste)).with_state(state)
    }

    fn json_request(body: Value) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/paste")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    async fn response_json(response: axum::response::Response) -> Value {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("failed to read response body");

        serde_json::from_slice(&body).expect("response body was not valid JSON")
    }

    #[tokio::test]
    async fn paste_returns_400_when_body_is_missing() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/paste")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn paste_returns_400_when_body_is_malformed() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/paste")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"content":"unterminated}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn paste_returns_422_when_content_is_missing() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(json_request(json!({})))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn paste_returns_422_when_content_has_wrong_type() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(json_request(json!({
                "content": 123
            })))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn paste_returns_201_with_valid_content() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(json_request(json!({
                "content": "Hello, World!"
            })))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response_json(response).await;

        assert!(body["id"].as_str().is_some());
        assert!(body["created_at"].as_i64().is_some());
        assert!(body["expires_in"].is_null());
    }

    #[tokio::test]
    async fn paste_returns_201_with_empty_content() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(json_request(json!({
                "content": ""
            })))
            .await
            .unwrap();

        // FIXME : This is not a desired behaviour.
        // The current handler allows empty content.
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn paste_returns_201_with_expiration() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(json_request(json!({
                "content": "Hello, World!",
                "expires_in": 24
            })))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response_json(response).await;

        assert!(body["id"].as_str().is_some());
        assert!(body["created_at"].as_i64().is_some());
        assert!(body["expires_at"].as_i64().is_some());
    }

    #[tokio::test]
    async fn paste_returns_201_without_expiration() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(json_request(json!({
                "content": "Never expires"
            })))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response_json(response).await;

        assert!(body["id"].as_str().is_some());
        assert!(body["created_at"].as_i64().is_some());
        assert!(body["expires_in"].is_null());
    }

    #[tokio::test]
    async fn paste_response_contains_id_and_timestamps() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(json_request(json!({
                "content": "Hello, World!",
                "expires_in": 24
            })))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response_json(response).await;

        let id = body["id"].as_str().expect("id should be a string");

        let created_at = body["created_at"]
            .as_i64()
            .expect("created_at should be an i64");

        let expires_at = body["expires_at"]
            .as_i64()
            .expect("expires_at should be an i64");

        assert_eq!(id.len(), 6);
        assert!(created_at > 0);
        assert!(expires_at > created_at);
    }

    #[tokio::test]
    async fn paste_response_has_valid_expiration() {
        let state = test_state().await;

        let before = Utc::now().timestamp();

        let response = router(state)
            .oneshot(json_request(json!({
                "content": "Expires later",
                "expires_in": 24
            })))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response_json(response).await;

        let created_at = body["created_at"]
            .as_i64()
            .expect("created_at should be an i64");

        let expires_at = body["expires_at"]
            .as_i64()
            .expect("expires_at should be an i64");

        let after = Utc::now().timestamp();

        assert!(created_at >= before);
        assert!(created_at <= after);
        assert_eq!(expires_at - created_at, 24 * 60 * 60);
    }

    #[tokio::test]
    async fn paste_generates_unique_ids() {
        let state = test_state().await;

        let response_one = router(state.clone())
            .oneshot(json_request(json!({
                "content": "First paste"
            })))
            .await
            .unwrap();

        let response_two = router(state)
            .oneshot(json_request(json!({
                "content": "Second paste"
            })))
            .await
            .unwrap();

        assert_eq!(response_one.status(), StatusCode::CREATED);
        assert_eq!(response_two.status(), StatusCode::CREATED);

        let body_one = response_json(response_one).await;
        let body_two = response_json(response_two).await;

        let id_one = body_one["id"]
            .as_str()
            .expect("first id should be a string");

        let id_two = body_two["id"]
            .as_str()
            .expect("second id should be a string");

        assert_eq!(id_one.len(), 6);
        assert_eq!(id_two.len(), 6);
        assert_ne!(id_one, id_two);
    }

    #[tokio::test]
    async fn paste_created_at_is_close_to_current_time() {
        let state = test_state().await;

        let before = Utc::now().timestamp();

        let response = router(state)
            .oneshot(json_request(json!({
                "content": "Timestamp test"
            })))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response_json(response).await;

        let created_at = body["created_at"]
            .as_i64()
            .expect("created_at should be an i64");

        let after = Utc::now().timestamp();

        assert!(created_at >= before);
        assert!(created_at <= after);
    }

    #[tokio::test]
    async fn paste_expiration_is_null_when_omitted() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(json_request(json!({
                "content": "No expiration"
            })))
            .await
            .unwrap();

        let body = response_json(response).await;

        assert!(body["expires_at"].is_null());
    }

    #[tokio::test]
    async fn paste_expiration_is_24_hours_after_creation() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(json_request(json!({
                "content": "24 hour expiration",
                "expires_in": 24
            })))
            .await
            .unwrap();

        let body = response_json(response).await;

        let created_at = body["created_at"]
            .as_i64()
            .expect("created_at should be an i64");

        let expires_at = body["expires_at"]
            .as_i64()
            .expect("expires_at should be an i64");

        assert_eq!(expires_at, created_at + 24 * 60 * 60);
    }

    #[tokio::test]
    async fn paste_accepts_zero_hour_expiration() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(json_request(json!({
                "content": "Immediately expires",
                "expires_in": 0
            })))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response_json(response).await;

        let created_at = body["created_at"]
            .as_i64()
            .expect("created_at should be an i64");

        let expires_at = body["expires_at"]
            .as_i64()
            .expect("expires_at should be an i64");

        assert_eq!(expires_at, created_at);
    }
}
