use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct PasteRequest {
    #[validate(length(min = 1, message = "content must not be empty"))]
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
