use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use tracing::instrument;

use crate::{schema::fetch::FetchResponse, state::AppState};

#[instrument(name = "GET /api/{id}", skip(state))]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<FetchResponse>, StatusCode> {
    let paste = sqlx::query_as!(
        FetchResponse,
        r#"
        SELECT id, content, created_at, expires_at
        FROM pastes
        WHERE id = ?
        "#,
        id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|error| {
        tracing::error!(%error, "failed to fetch paste");
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(paste))
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode, header},
        routing::{get, post},
    };
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use crate::{api::paste, state::test_state};

    use super::fetch;

    fn router(state: crate::state::AppState) -> Router {
        Router::new()
            .route("/api/paste", post(paste::paste))
            .route("/api/fetch/{id}", get(fetch))
            .with_state(state)
    }

    async fn response_json(response: axum::response::Response) -> Value {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("failed to read response body");

        serde_json::from_slice(&body).expect("response body was not valid JSON")
    }

    /// Create a paste through the real `POST /api/paste` handler and return its
    /// response JSON (`id`, `created_at`, `expires_at`).
    async fn create_paste(router: &Router, content: &str, expires_in: Option<i64>) -> Value {
        let mut body = json!({ "content": content });
        if let Some(hours) = expires_in {
            body["expires_in"] = json!(hours);
        }
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/paste")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        response_json(response).await
    }

    #[tokio::test]
    async fn fetch_returns_500_when_database_is_unavailable() {
        let state = test_state().await;

        // Closing the pool makes the SELECT fail with PoolClosed.
        state.db.close().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/fetch/some-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn fetch_returns_404_when_paste_does_not_exist() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/fetch/does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn fetch_returns_200_when_paste_exists() {
        let state = test_state().await;
        let router = router(state);

        let created = create_paste(&router, "Hello, World!", Some(24)).await;
        let id = created["id"].as_str().expect("id").to_string();

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/fetch/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response_json(response).await;

        assert_eq!(body["id"], created["id"]);
        assert_eq!(body["content"], "Hello, World!");
        assert_eq!(body["created_at"], created["created_at"]);
        assert_eq!(body["expires_at"], created["expires_at"]);
    }

    #[tokio::test]
    async fn fetch_returns_paste_without_expiration() {
        let state = test_state().await;
        let router = router(state);

        let created = create_paste(&router, "This paste does not expire", None).await;
        let id = created["id"].as_str().expect("id").to_string();

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/fetch/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response_json(response).await;

        assert_eq!(body["id"], created["id"]);
        assert_eq!(body["content"], "This paste does not expire");
        assert_eq!(body["created_at"], created["created_at"]);
        assert!(body["expires_at"].is_null());
    }

    #[tokio::test]
    async fn fetch_returns_exact_content() {
        let state = test_state().await;
        let router = router(state);
        let content = "line one\nline two\nこんにちは";

        let created = create_paste(&router, content, None).await;
        let id = created["id"].as_str().expect("id").to_string();

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/fetch/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response_json(response).await;

        assert_eq!(body["id"], created["id"]);
        assert_eq!(body["content"], content);
        assert_eq!(body["created_at"], created["created_at"]);
        assert!(body["expires_at"].is_null());
    }
}
