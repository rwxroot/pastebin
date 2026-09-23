use axum::{
    http::header::CONTENT_TYPE,
    response::IntoResponse,
};

pub async fn favicon() -> impl IntoResponse {
    (
        [(CONTENT_TYPE, "image/x-icon")],
        include_bytes!("icons/favicon.ico").as_slice(),
    )
}
