use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use tracing::instrument;

use crate::{schema::fetch::FetchResponse, state::AppState};

#[instrument(name = "GET /{id}", skip(state))]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<FetchResponse>), StatusCode> {
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

    Ok((StatusCode::OK, Json(paste)))
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use serde_json::Value;
    use tower::ServiceExt;

    use super::fetch;
    use crate::state;

    const CREATED_AT: i64 = 1_787_313_600;
    const EXPIRES_AT: i64 = 1_787_400_000;

    async fn test_state() -> crate::state::AppState {
        dotenvy::dotenv().ok();

        state::get_shared_state()
            .await
            .expect("failed to create test state")
    }

    fn router(state: crate::state::AppState) -> Router {
        Router::new()
            .route("/fetch/{id}", get(fetch))
            .with_state(state)
    }

    async fn response_json(response: axum::response::Response) -> Value {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("failed to read response body");

        serde_json::from_slice(&body).expect("response body was not valid JSON")
    }

    async fn insert_paste(
        state: &crate::state::AppState,
        id: &str,
        content: &str,
        created_at: i64,
        expires_at: Option<i64>,
    ) {
        sqlx::query(
            r#"
            INSERT INTO pastes (id, content, created_at, expires_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(content)
        .bind(created_at)
        .bind(expires_at)
        .execute(&state.db)
        .await
        .expect("failed to insert test paste");
    }

    #[tokio::test]
    async fn fetch_returns_404_when_paste_does_not_exist() {
        let state = test_state().await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/fetch/does-not-exist")
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
        let id = format!("test-{}", nanoid::nanoid!(6));

        insert_paste(&state, &id, "Hello, World!", CREATED_AT, Some(EXPIRES_AT)).await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/fetch/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response_json(response).await;

        assert_eq!(body["id"], id);
        assert_eq!(body["content"], "Hello, World!");
        assert_eq!(body["created_at"], CREATED_AT);
        assert_eq!(body["expires_at"], EXPIRES_AT);
    }

    #[tokio::test]
    async fn fetch_returns_paste_without_expiration() {
        let state = test_state().await;
        let id = format!("test-{}", nanoid::nanoid!(6));

        insert_paste(&state, &id, "This paste does not expire", CREATED_AT, None).await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/fetch/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response_json(response).await;

        assert_eq!(body["id"], id);
        assert_eq!(body["content"], "This paste does not expire");
        assert_eq!(body["created_at"], CREATED_AT);
        assert!(body["expires_at"].is_null());
    }

    #[tokio::test]
    async fn fetch_returns_exact_content() {
        let state = test_state().await;
        let id = format!("test-{}", nanoid::nanoid!(6));
        let content = "line one\nline two\nこんにちは";

        insert_paste(&state, &id, content, CREATED_AT, None).await;

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/fetch/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response_json(response).await;

        assert_eq!(body["id"], id);
        assert_eq!(body["content"], content);
        assert_eq!(body["created_at"], CREATED_AT);
        assert!(body["expires_at"].is_null());
    }
}
