use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct QuickReply {
    pub id: Uuid,
    pub title: String,
    pub shortcut: Option<String>,
    pub content: String,
    pub category: Option<String>,
    pub is_global: bool,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateQuickReplyRequest {
    pub title: String,
    pub shortcut: Option<String>,
    pub content: String,
    pub category: Option<String>,
    pub is_global: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateQuickReplyRequest {
    pub title: Option<String>,
    pub shortcut: Option<String>,
    pub content: Option<String>,
    pub category: Option<String>,
    pub is_global: Option<bool>,
}
