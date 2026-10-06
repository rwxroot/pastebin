use serde::Serialize;
use sqlx::FromRow;

#[derive(FromRow, Serialize)]
pub struct PasteResponse {
    pub id: String,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
}
