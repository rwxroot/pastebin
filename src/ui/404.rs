use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};

use super::NotFoundTemplate;

/// Fallback for any unmatched route: renders the branded 404 page.
pub async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Html(NotFoundTemplate.render().unwrap()),
    )
}
