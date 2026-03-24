use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::display::{normalize_phone_number, preferred_display_name};

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
    #[sqlx(default)]
    pub sender_phone_number: Option<String>,
    pub content: Option<String>,
    pub message_type: MessageType,
    pub media_url: Option<String>,
    pub media_mime_type: Option<String>,
    pub media_size: Option<i64>,
    pub media_filename: Option<String>,
    pub media_key: Option<Vec<u8>>,
    pub direct_path: Option<String>,
    pub file_enc_sha256: Option<Vec<u8>>,
    pub thumbnail_base64: Option<String>,
    pub status: MessageStatus,
    pub is_from_me: bool,
    pub is_forwarded: bool,
    pub is_starred: bool,
    pub reply_to_message_id: Option<String>,
    pub quote_content: Option<String>,
    pub quote_sender: Option<String>,
    pub quote_sender_name: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Message {
    pub fn sanitized(mut self) -> Self {
        self.sender_phone_number = normalize_phone_number(self.sender_phone_number.as_deref());
        self.sender_name = preferred_display_name(self.sender_name.as_deref(), self.sender_phone_number.as_deref());
        self.quote_sender_name = preferred_display_name(self.quote_sender_name.as_deref(), None);
        self
    }
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
    pub reply_to: Option<String>,
}
