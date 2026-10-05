use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
};

use crate::{
    api::fetch,
    state::AppState,
    ui::{FetchTemplate, NotFoundTemplate},
};

/// Render a paste as HTML by consuming the API loader directly.
/// Returns the paste view, or an empty body with the given status when
/// the paste is missing or the DB fails.
pub async fn paste(
    Path(id): Path<String>,
    state: State<AppState>,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    match fetch::get_paste(state, Path(id.clone())).await {
        Ok(paste) => Ok(Html(
            FetchTemplate {
                id: &paste.id,
                content: &paste.content,
            }
            .render()
            .unwrap(),
        )),
        Err(status) => Err((status, Html(NotFoundTemplate.render().unwrap()))),
    }
}
