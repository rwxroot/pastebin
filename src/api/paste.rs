use axum::{Json, extract::State, http::StatusCode};
use chrono::Utc;
use tracing::instrument;
use validator::Validate;

use crate::{
    schema::{
        id::random_id,
        paste::{PasteRequest, PasteResponse},
    },
    state::AppState,
};

#[instrument(
    name = "POST /api/paste",
    skip(state),
    fields(length = req.content.len())
)]
pub async fn paste(
    State(state): State<AppState>,
    Json(req): Json<PasteRequest>,
) -> Result<(StatusCode, Json<PasteResponse>), StatusCode> {
    req.validate().map_err(|error| {
        tracing::warn!(%error, "validation failed");
        StatusCode::UNPROCESSABLE_ENTITY
    })?;

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

    use crate::state::test_state;

    use super::paste;

    fn router(state: crate::state::AppState) -> Router {
        Router::new()
            .route("/api/paste", post(paste))
            .with_state(state)
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
                    .uri("/api/paste")
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
                    .uri("/api/paste")
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
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/paste")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn paste_returns_422_when_content_has_wrong_type() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/paste")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "content": 123
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn paste_returns_201_with_valid_content() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/paste")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "content": "Hello, World!"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response_json(response).await;

        assert!(body["id"].as_str().is_some());
        assert!(body["created_at"].as_i64().is_some());
        assert!(body["expires_at"].is_null());
    }

    #[tokio::test]
    async fn paste_rejects_missing_content_type() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/paste")
                    .body(Body::from(json!({ "content": "x" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn paste_rejects_non_integer_expires_in() {
        let state = test_state().await;

        for bad in [
            json!({ "content": "x", "expires_in": "24" }),
            json!({ "content": "x", "expires_in": 1.5 }),
            json!({ "content": "x", "expires_in": true }),
        ] {
            let response = router(state.clone())
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/paste")
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(bad.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                StatusCode::UNPROCESSABLE_ENTITY,
                "expires_in must be an integer, got {}",
                bad
            );
        }
    }

    #[tokio::test]
    async fn paste_returns_422_with_empty_content() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/paste")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "content": ""
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn paste_response_contains_id_and_timestamps() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/paste")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "content": "Hello, World!",
                            "expires_in": 24
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
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
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/paste")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "content": "Expires later",
                            "expires_in": 24
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
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
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/paste")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "content": "First paste"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let response_two = router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/paste")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "content": "Second paste"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
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
    async fn paste_accepts_zero_hour_expiration() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/paste")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "content": "Immediately expires",
                            "expires_in": 0
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
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
