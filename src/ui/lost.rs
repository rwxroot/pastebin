use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};

use crate::ui::NotFoundTemplate;

/// Fallback for any unmatched route: renders the branded 404 page.
pub async fn lost() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Html(NotFoundTemplate.render().unwrap()),
    )
}
