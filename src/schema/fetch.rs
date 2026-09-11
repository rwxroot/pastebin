use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct FetchResponse {
    pub id: String,
    pub content: String,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
}
