use askama::Template;
use axum::response::Html;

use crate::ui::PasteTemplate;

pub async fn paste() -> Html<String> {
    Html(PasteTemplate.render().unwrap())
}
