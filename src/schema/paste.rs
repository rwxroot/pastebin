use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PasteRequest {
    pub content: String,
    #[serde(default)]
    pub expires_in: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PasteResponse {
    pub id: String,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
}
