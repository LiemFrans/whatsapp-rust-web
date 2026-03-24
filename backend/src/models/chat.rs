use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Chat {
    pub id: Uuid,
    pub session_id: Uuid,
    pub chat_jid: String,
    pub name: Option<String>,
    pub phone_number: Option<String>,
    pub profile_pic_url: Option<String>,
    pub last_message: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub unread_count: i32,
    pub is_group: bool,
    pub is_archived: bool,
    pub is_pinned: bool,
    pub is_muted: bool,
    pub muted_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ChatListQuery {
    pub session_id: Option<Uuid>,
    pub search: Option<String>,
    pub filter: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct MessageListQuery {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    pub session_id: Option<Uuid>,
}
