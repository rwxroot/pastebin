use askama::Template;
use axum::response::Html;

use super::IndexTemplate;

pub async fn index() -> Html<String> {
    Html(IndexTemplate.render().unwrap())
}
