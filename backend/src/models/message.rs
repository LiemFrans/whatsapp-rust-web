use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "message_type", rename_all = "snake_case")]
pub enum MessageType {
    Text,
    Image,
    Video,
    Audio,
    Document,
    Sticker,
    Contact,
    Location,
    Poll,
    Reaction,
    System,
    Revoked,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "message_status", rename_all = "snake_case")]
pub enum MessageStatus {
    Pending,
    Sent,
    Delivered,
    Read,
    Failed,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Message {
    pub id: Uuid,
    pub chat_id: Uuid,
    pub message_id: String,
    pub sender: String,
    pub sender_name: Option<String>,
    pub content: Option<String>,
    pub message_type: MessageType,
    pub media_url: Option<String>,
    pub media_mime_type: Option<String>,
    pub media_size: Option<i64>,
    pub media_filename: Option<String>,
    pub thumbnail_base64: Option<String>,
    pub status: MessageStatus,
    pub is_from_me: bool,
    pub is_forwarded: bool,
    pub is_starred: bool,
    pub reply_to_message_id: Option<String>,
    pub quote_content: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
    pub reply_to: Option<String>,
}
