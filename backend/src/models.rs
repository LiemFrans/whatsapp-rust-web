//! Data models, request/response types, and shared application state.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use whatsapp_rust::download::MediaType;
use whatsapp_rust::Client;

pub const MAX_MESSAGES_PER_CHAT: usize = 200;
pub const STARTUP_SYNC_TIMEOUT_SECS: u64 = 45;

// ---------------------------------------------------------------------------
// Core application state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AppState {
    pub qr_code: Arc<RwLock<Option<String>>>,
    pub is_connected: Arc<AtomicBool>,
    pub is_syncing: Arc<AtomicBool>,
    pub client: Arc<RwLock<Option<Arc<Client>>>>,
    pub store: Arc<RwLock<DataStore>>,
    pub db_path: Arc<String>,
}

impl AppState {
    pub fn new(db_path: impl Into<String>) -> Self {
        Self {
            qr_code: Arc::new(RwLock::new(None)),
            is_connected: Arc::new(AtomicBool::new(false)),
            is_syncing: Arc::new(AtomicBool::new(false)),
            client: Arc::new(RwLock::new(None)),
            store: Arc::new(RwLock::new(DataStore::default())),
            db_path: Arc::new(db_path.into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Domain models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MentionSummary {
    pub jid: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MediaAttachment {
    pub kind: String,
    pub mime_type: Option<String>,
    pub caption: Option<String>,
    pub title: Option<String>,
    pub file_name: Option<String>,
    pub file_length: Option<u64>,
    pub page_count: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_seconds: Option<u32>,
    pub is_voice_note: bool,
    pub is_gif: bool,
    pub is_sticker: bool,
    pub download_path: Option<String>,
    pub preview_image_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MediaBlob {
    pub mime_type: Option<String>,
    pub file_name: Option<String>,
    pub direct_path: String,
    pub media_key: Vec<u8>,
    pub file_sha256: Vec<u8>,
    pub file_enc_sha256: Vec<u8>,
    pub file_length: u64,
    pub media_type: MediaType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatMessage {
    pub id: String,
    pub chat_jid: String,
    pub sender_jid: String,
    pub sender_name: Option<String>,
    pub text: String,
    pub timestamp_ms: i64,
    pub from_me: bool,
    pub mentions: Vec<MentionSummary>,
    pub media: Option<MediaAttachment>,
    pub receipt_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContactSummary {
    pub jid: String,
    pub name: String,
    pub phone: Option<String>,
    pub status: Option<String>,
    pub avatar_url: Option<String>,
    pub is_business: bool,
    pub is_registered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatSummary {
    pub jid: String,
    pub name: String,
    pub phone: Option<String>,
    pub is_group: bool,
    pub preview: Option<String>,
    pub timestamp_ms: Option<i64>,
    pub unread_count: u32,
    pub archived: bool,
    pub muted: bool,
    pub avatar_url: Option<String>,
    pub status: Option<String>,
    pub typing: Option<String>,
    pub is_online: bool,
    pub last_seen_ms: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct ChatRecord {
    pub summary: ChatSummary,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Default)]
pub struct DataStore {
    pub chats: HashMap<String, ChatRecord>,
    pub contacts: HashMap<String, ContactSummary>,
    pub media: HashMap<String, MediaBlob>,
}

// ---------------------------------------------------------------------------
// API request / response DTOs
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct QrResponse {
    pub qr_code: Option<String>,
    pub is_connected: bool,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub is_connected: bool,
    pub is_syncing: bool,
}

#[derive(Serialize)]
pub struct BootstrapResponse {
    pub qr_code: Option<String>,
    pub is_connected: bool,
    pub is_syncing: bool,
    pub chats: Vec<ChatSummary>,
    pub contacts: Vec<ContactSummary>,
    pub logout_hint: Option<String>,
}

#[derive(Serialize)]
pub struct MessagesResponse {
    pub messages: Vec<ChatMessage>,
}

#[derive(Serialize)]
pub struct ContactsResponse {
    pub contacts: Vec<ContactSummary>,
}

#[derive(Serialize)]
pub struct ChatsResponse {
    pub chats: Vec<ChatSummary>,
}

#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub phone: Option<String>,
    pub jid: Option<String>,
    pub message: String,
    #[serde(default)]
    pub mentions: Vec<String>,
}

#[derive(Deserialize)]
pub struct TypingRequest {
    pub state: String,
}

#[derive(Serialize)]
pub struct SendMessageResponse {
    pub success: bool,
    pub message_id: Option<String>,
}

#[derive(Serialize)]
pub struct LogoutResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
