use askama::Template;
use axum::{extract::State, response::Html};

use crate::{state::AppState, ui::PasteTemplate};

pub async fn paste(State(state): State<AppState>) -> Html<String> {
    Html(
        (PasteTemplate {
            max_size: state.config.max_paste_size,
        })
        .render()
        .unwrap(),
    )
}
