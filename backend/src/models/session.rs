use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "session_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Disconnected,
    Connecting,
    Connected,
    QrCode,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct WhatsAppSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub session_name: String,
    pub phone_number: Option<String>,
    pub db_path: String,
    pub status: SessionStatus,
    pub qr_code_data: Option<String>,
    pub webhook_url: Option<String>,
    pub webhook_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_active_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub session_name: String,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: Uuid,
    pub session_name: String,
    pub phone_number: Option<String>,
    pub status: SessionStatus,
    pub webhook_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_active_at: Option<DateTime<Utc>>,
}

impl From<WhatsAppSession> for SessionResponse {
    fn from(s: WhatsAppSession) -> Self {
        Self {
            id: s.id,
            session_name: s.session_name,
            phone_number: s.phone_number,
            status: s.status,
            webhook_url: s.webhook_url,
            created_at: s.created_at,
            last_active_at: s.last_active_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateWebhookRequest {
    pub webhook_url: Option<String>,
    pub webhook_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QrCodeResponse {
    pub session_id: Uuid,
    pub qr_code: String,
}
