use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::display::{normalize_phone_number, preferred_display_name};

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

impl Chat {
    pub fn sanitized(mut self) -> Self {
        self.phone_number = normalize_phone_number(self.phone_number.as_deref());
        self.name = preferred_display_name(self.name.as_deref(), self.phone_number.as_deref());
        self
    }
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

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Contact {
    pub id: Uuid,
    pub session_id: Uuid,
    pub jid: String,
    pub push_name: Option<String>,
    pub phone_number: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl Contact {
    pub fn sanitized(mut self) -> Self {
        self.phone_number = normalize_phone_number(self.phone_number.as_deref());
        self.push_name = preferred_display_name(self.push_name.as_deref(), self.phone_number.as_deref());
        self
    }
}

#[derive(Debug, Deserialize)]
pub struct ContactsQuery {
    pub session_id: Option<Uuid>,
}
