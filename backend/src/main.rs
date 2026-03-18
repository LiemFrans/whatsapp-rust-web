//! WhatsApp Web Clone — Rust Backend
//!
//! Endpoints:
//!   GET  /api/auth/qr                  → current QR code string + connection status
//!   GET  /api/auth/status              → connection status + sync status
//!   POST /api/auth/logout              → disconnect current session
//!   GET  /api/bootstrap                → initial UI payload (chats + contacts)
//!   GET  /api/chats                    → synced chat summaries
//!   GET  /api/chats/:jid/messages      → messages for a synced chat
//!   POST /api/chats/:jid/read          → locally mark a chat as read
//!   POST /api/chats/:jid/typing        → send chat state updates to WhatsApp
//!   GET  /api/contacts                 → known contacts
//!   POST /api/messages/send            → send a WhatsApp message
//!   GET  /api/media/:chat_jid/:msg_id  → download stored media for rendering

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use whatsapp_rust::bot::Bot;
use whatsapp_rust::download::MediaType;
use whatsapp_rust::store::SqliteStore;
use whatsapp_rust::transport::{TokioWebSocketTransportFactory, UreqHttpClient};
use whatsapp_rust::types::events::Event;
use whatsapp_rust::types::presence::{ChatPresence, ReceiptType};
use whatsapp_rust::waproto::whatsapp as wa;
use whatsapp_rust::{Client, Jid};

const MAX_MESSAGES_PER_CHAT: usize = 200;
const STARTUP_SYNC_TIMEOUT_SECS: u64 = 45;

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

impl DataStore {
    fn reset(&mut self) {
        self.chats.clear();
        self.contacts.clear();
        self.media.clear();
    }

    fn sorted_chats(&self) -> Vec<ChatSummary> {
        let mut chats: Vec<_> = self.chats.values().map(|chat| chat.summary.clone()).collect();
        chats.sort_by(|a, b| {
            b.timestamp_ms
                .cmp(&a.timestamp_ms)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        chats
    }

    fn sorted_contacts(&self) -> Vec<ContactSummary> {
        let mut contacts: Vec<_> = self.contacts.values().cloned().collect();
        contacts.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        contacts
    }

    fn messages_for(&self, chat_jid: &str) -> Option<Vec<ChatMessage>> {
        self.chats.get(chat_jid).map(|chat| chat.messages.clone())
    }

    fn media_for(&self, chat_jid: &str, message_id: &str) -> Option<MediaBlob> {
        self.media.get(&media_key(chat_jid, message_id)).cloned()
    }

    fn display_name_for_jid(&self, jid: &str) -> String {
        if let Some(contact) = self.contacts.get(jid) {
            return contact.name.clone();
        }
        if let Some(chat) = self.chats.get(jid) {
            return chat.summary.name.clone();
        }
        if let Some(phone) = jid_phone_str(jid) {
            return format!("+{phone}");
        }
        jid.to_string()
    }

    fn ensure_chat(
        &mut self,
        chat_jid: String,
        name: Option<String>,
        phone: Option<String>,
        is_group: bool,
    ) -> &mut ChatRecord {
        let fallback_name = preferred_display_name(name.as_deref(), phone.as_deref(), &chat_jid);

        let record = self
            .chats
            .entry(chat_jid.clone())
            .or_insert_with(|| ChatRecord {
                summary: ChatSummary {
                    jid: chat_jid.clone(),
                    name: fallback_name.clone(),
                    phone: phone.clone(),
                    is_group,
                    ..Default::default()
                },
                messages: Vec::new(),
            });

        if let Some(ref name_value) = name {
            if is_better_name(name_value, &record.summary.name, &chat_jid, phone.as_deref()) {
                record.summary.name = name_value.clone();
            }
        }
        if let Some(phone) = phone {
            record.summary.phone = Some(phone);
        }
        record.summary.is_group = is_group;
        record
    }

    fn upsert_contact(&mut self, contact: ContactSummary) {
        let contact_jid = contact.jid.clone();
        let contact_phone = contact.phone.clone();
        let entry = self
            .contacts
            .entry(contact.jid.clone())
            .or_insert_with(|| contact.clone());

        if is_better_name(&contact.name, &entry.name, &contact_jid, contact_phone.as_deref()) {
            entry.name = contact.name;
        }
        if contact.phone.is_some() {
            entry.phone = contact.phone;
        }
        if contact.status.is_some() {
            entry.status = contact.status;
        }
        if contact.avatar_url.is_some() {
            entry.avatar_url = contact.avatar_url;
        }
        entry.is_business = contact.is_business;
        entry.is_registered = contact.is_registered;
    }

    fn rename_contact(&mut self, jid: &str, new_name: &str) {
        if !new_name.trim().is_empty() {
            if let Some(contact) = self.contacts.get_mut(jid) {
                contact.name = new_name.to_string();
            }
            if let Some(chat) = self.chats.get_mut(jid) {
                chat.summary.name = new_name.to_string();
            }
        }
    }

    fn set_contact_status(&mut self, jid: &str, status: Option<String>) {
        if let Some(contact) = self.contacts.get_mut(jid) {
            contact.status = status.clone();
        }
        if let Some(chat) = self.chats.get_mut(jid) {
            chat.summary.status = status;
        }
    }

    fn set_contact_avatar(&mut self, jid: &str, avatar_url: Option<String>) {
        if let Some(contact) = self.contacts.get_mut(jid) {
            contact.avatar_url = avatar_url.clone();
        }
        if let Some(chat) = self.chats.get_mut(jid) {
            chat.summary.avatar_url = avatar_url;
        }
    }

    fn record_message(
        &mut self,
        chat_jid: String,
        chat_name: Option<String>,
        phone: Option<String>,
        is_group: bool,
        message: ChatMessage,
        media_blob: Option<MediaBlob>,
    ) {
        let preview = Some(preview_for_message(&message));
        let timestamp_ms = Some(message.timestamp_ms);
        let from_me = message.from_me;
        let message_id = message.id.clone();
        let download_key = media_key(&chat_jid, &message_id);
        let chat_key = chat_jid.clone();

        let mut removed_ids: Vec<String> = Vec::new();
        {
            let chat = self.ensure_chat(chat_jid, chat_name, phone, is_group);
            chat.summary.typing = None;
            if let Some(existing) = chat.messages.iter_mut().find(|existing| existing.id == message.id) {
                *existing = message;
                chat.summary.preview = preview;
                chat.summary.timestamp_ms = timestamp_ms;
                if let Some(blob) = media_blob {
                    self.media.insert(download_key, blob);
                }
                return;
            }

            chat.messages.push(message);
            if chat.messages.len() > MAX_MESSAGES_PER_CHAT {
                let overflow = chat.messages.len() - MAX_MESSAGES_PER_CHAT;
                let removed: Vec<ChatMessage> = chat.messages.drain(0..overflow).collect();
                removed_ids = removed.into_iter().map(|item| item.id).collect();
            }
            chat.summary.preview = preview;
            chat.summary.timestamp_ms = timestamp_ms;
            if !from_me {
                chat.summary.unread_count = chat.summary.unread_count.saturating_add(1);
            }
        }

        for removed_id in removed_ids {
            self.media.remove(&media_key(&chat_key, &removed_id));
        }
        if let Some(blob) = media_blob {
            self.media.insert(download_key, blob);
        }
    }

    fn mark_read(&mut self, chat_jid: &str) {
        if let Some(chat) = self.chats.get_mut(chat_jid) {
            chat.summary.unread_count = 0;
        }
    }

    fn set_archived(&mut self, chat_jid: &str, archived: bool) {
        if let Some(chat) = self.chats.get_mut(chat_jid) {
            chat.summary.archived = archived;
        }
    }

    fn set_muted(&mut self, chat_jid: &str, muted: bool) {
        if let Some(chat) = self.chats.get_mut(chat_jid) {
            chat.summary.muted = muted;
        }
    }

    fn set_typing(&mut self, chat_jid: &str, typing: Option<String>) {
        if let Some(chat) = self.chats.get_mut(chat_jid) {
            chat.summary.typing = typing;
        }
    }

    fn set_presence(&mut self, jid: &str, is_online: bool, last_seen_ms: Option<i64>) {
        if let Some(chat) = self.chats.get_mut(jid) {
            chat.summary.is_online = is_online;
            chat.summary.last_seen_ms = last_seen_ms;
        }
    }

    fn update_message_receipt(&mut self, chat_jid: &str, message_ids: &[String], receipt_status: &str) {
        if let Some(chat) = self.chats.get_mut(chat_jid) {
            for message in &mut chat.messages {
                if message.from_me && message_ids.iter().any(|id| id == &message.id) {
                    message.receipt_status = Some(promote_receipt_status(
                        message.receipt_status.as_deref(),
                        receipt_status,
                    ));
                }
            }
        }
    }

    fn refresh_mention_names(&mut self, alias_jids: &[String], resolved_name: &str, phone: Option<&str>) {
        if resolved_name.trim().is_empty() {
            return;
        }

        for chat in self.chats.values_mut() {
            for message in &mut chat.messages {
                for mention in &mut message.mentions {
                    if alias_jids.iter().any(|alias| alias == &mention.jid)
                        && is_better_name(&resolved_name, &mention.name, &mention.jid, phone)
                    {
                        mention.name = resolved_name.to_string();
                    }
                }
            }
        }
    }
}

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

pub async fn get_qr(State(state): State<AppState>) -> Json<QrResponse> {
    let qr = state.qr_code.read().await;
    Json(QrResponse {
        qr_code: qr.clone(),
        is_connected: state.is_connected.load(Ordering::SeqCst),
    })
}

pub async fn get_status(State(state): State<AppState>) -> Json<StatusResponse> {
    Json(StatusResponse {
        is_connected: state.is_connected.load(Ordering::SeqCst),
        is_syncing: state.is_syncing.load(Ordering::SeqCst),
    })
}

pub async fn get_bootstrap(State(state): State<AppState>) -> Json<BootstrapResponse> {
    let qr = state.qr_code.read().await.clone();
    let store = state.store.read().await;
    Json(BootstrapResponse {
        qr_code: qr,
        is_connected: state.is_connected.load(Ordering::SeqCst),
        is_syncing: state.is_syncing.load(Ordering::SeqCst),
        chats: store.sorted_chats(),
        contacts: store.sorted_contacts(),
        logout_hint: Some("Logout disconnects the current runtime session. Restart the app if WhatsApp asks for a fresh link.".into()),
    })
}

pub async fn get_chats(State(state): State<AppState>) -> Json<ChatsResponse> {
    let store = state.store.read().await;
    Json(ChatsResponse {
        chats: store.sorted_chats(),
    })
}

pub async fn get_contacts(State(state): State<AppState>) -> Json<ContactsResponse> {
    let store = state.store.read().await;
    Json(ContactsResponse {
        contacts: store.sorted_contacts(),
    })
}

pub async fn get_chat_messages(
    Path(chat_jid): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<MessagesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = state.store.read().await;
    let messages = store.messages_for(&chat_jid).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Chat not found".into(),
            }),
        )
    })?;
    Ok(Json(MessagesResponse { messages }))
}

pub async fn mark_chat_read(
    Path(chat_jid): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut store = state.store.write().await;
    if !store.chats.contains_key(&chat_jid) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Chat not found".into(),
            }),
        ));
    }
    store.mark_read(&chat_jid);
    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn update_typing(
    Path(chat_jid): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<TypingRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if !state.is_connected.load(Ordering::SeqCst) {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "WhatsApp is not connected.".into(),
            }),
        ));
    }

    let client_guard = state.client.read().await;
    let client = client_guard.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "WhatsApp client not yet initialised".into(),
            }),
        )
    })?;

    let jid: Jid = chat_jid.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid chat JID".into(),
            }),
        )
    })?;

    let result = match payload.state.as_str() {
        "composing" => client.chatstate().send_composing(&jid).await,
        "recording" => client.chatstate().send_recording(&jid).await,
        _ => client.chatstate().send_paused(&jid).await,
    };

    result.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to update typing state: {e}"),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn get_media(
    Path((chat_jid, message_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let blob = {
        let store = state.store.read().await;
        store.media_for(&chat_jid, &message_id)
    }
    .ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Media not found".into(),
            }),
        )
    })?;

    let client_guard = state.client.read().await;
    let client = client_guard.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "WhatsApp client not yet initialised".into(),
            }),
        )
    })?;

    let bytes = client
        .download_from_params(
            &blob.direct_path,
            &blob.media_key,
            &blob.file_sha256,
            &blob.file_enc_sha256,
            blob.file_length,
            blob.media_type,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: format!("Failed to download media: {e}"),
                }),
            )
        })?;

    let mut response = bytes.into_response();
    let mime_type = blob
        .mime_type
        .clone()
        .unwrap_or_else(|| default_mime_for_media_type(blob.media_type).to_string());
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&mime_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    if let Some(file_name) = blob.file_name {
        if let Ok(value) = HeaderValue::from_str(&format!("inline; filename=\"{file_name}\"")) {
            response.headers_mut().insert(header::CONTENT_DISPOSITION, value);
        }
    }

    Ok(response)
}

pub async fn logout(
    State(state): State<AppState>,
) -> Result<Json<LogoutResponse>, (StatusCode, Json<ErrorResponse>)> {
    let client = state.client.read().await.clone();

    if let Some(client) = client {
        client.enable_auto_reconnect.store(false, Ordering::SeqCst);
        client.disconnect().await;
    }

    let _ = tokio::fs::remove_file(state.db_path.as_str()).await;

    state.is_connected.store(false, Ordering::SeqCst);
    state.is_syncing.store(false, Ordering::SeqCst);
    *state.qr_code.write().await = None;
    *state.client.write().await = None;
    state.store.write().await.reset();

    Ok(Json(LogoutResponse {
        success: true,
        message: "Disconnected. Restart the application to pair again if needed.".into(),
    }))
}

pub async fn send_message(
    State(state): State<AppState>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<Json<SendMessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let trimmed_message = payload.message.trim();
    if trimmed_message.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "\"message\" must not be empty".into(),
            }),
        ));
    }

    let requested_jid = payload
        .jid
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value.parse::<Jid>().map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "\"jid\" is invalid".into(),
                    }),
                )
            })
        })
        .transpose()?;

    let explicit_phone = normalize_phone(payload.phone.as_deref().unwrap_or_default());
    let requested_jid_phone = requested_jid.as_ref().and_then(jid_phone);
    let fallback_store_phone = if explicit_phone.is_empty() {
        requested_jid.as_ref().and_then(|jid| {
            let jid_key = jid.to_non_ad().to_string();
            state
                .store
                .try_read()
                .ok()
                .and_then(|store| store.contacts.get(&jid_key).and_then(|contact| contact.phone.clone()))
        })
    } else {
        None
    };

    let resolved_phone = if explicit_phone.is_empty() {
        requested_jid_phone.or(fallback_store_phone)
    } else {
        Some(explicit_phone.clone())
    };

    let target_jid = match requested_jid.clone() {
        Some(jid) if jid_is_group(&jid) => jid,
        Some(_) => {
            let phone = resolved_phone.clone().filter(|value| value.chars().all(|c| c.is_ascii_digit())).ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "Direct chats require a valid mapped phone number before sending.".into(),
                    }),
                )
            })?;
            Jid::pn(&phone)
        }
        None => {
            let phone = resolved_phone.clone().filter(|value| value.chars().all(|c| c.is_ascii_digit())).ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "Provide a valid \"jid\" or a digit-only \"phone\" (E.164 format).".into(),
                    }),
                )
            })?;
            Jid::pn(&phone)
        }
    };

    let chat_store_jid = requested_jid
        .as_ref()
        .map(|jid| jid.to_non_ad().to_string())
        .unwrap_or_else(|| target_jid.to_non_ad().to_string());

    let mention_jids: Vec<String> = payload
        .mentions
        .iter()
        .filter_map(|jid| jid.parse::<Jid>().ok().map(|parsed| parsed.to_non_ad().to_string()))
        .collect();

    if !state.is_connected.load(Ordering::SeqCst) {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "WhatsApp is not connected. Please scan the QR code first.".into(),
            }),
        ));
    }

    let client_guard = state.client.read().await;
    let client = client_guard.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "WhatsApp client not yet initialised".into(),
            }),
        )
    })?;

    let wa_message = if mention_jids.is_empty() {
        wa::Message {
            conversation: Some(trimmed_message.to_string()),
            ..Default::default()
        }
    } else {
        wa::Message {
            extended_text_message: Some(Box::new(wa::message::ExtendedTextMessage {
                text: Some(trimmed_message.to_string()),
                context_info: Some(Box::new(wa::ContextInfo {
                    mentioned_jid: mention_jids.clone(),
                    ..Default::default()
                })),
                ..Default::default()
            })),
            ..Default::default()
        }
    };

    log::info!(
        "Sending outbound message to {} (store chat {})",
        target_jid,
        chat_store_jid
    );

    match client.send_message(target_jid.clone(), wa_message).await {
        Ok(msg_id) => {
            let chat_jid = chat_store_jid;
            let phone = if jid_is_group(&target_jid) {
                None
            } else {
                resolved_phone.clone().or_else(|| jid_phone(&target_jid.to_non_ad()))
            };
            let is_group = requested_jid
                .as_ref()
                .map(jid_is_group)
                .unwrap_or_else(|| jid_is_group(&target_jid));
            let mention_summaries = resolve_mention_summaries(&state, client, &mention_jids).await;

            let current_display_name = {
                let store = state.store.read().await;
                store.display_name_for_jid(&chat_jid)
            };
            let mut store = state.store.write().await;
            let timestamp_ms = now_ms();

            if !is_group {
                store.upsert_contact(ContactSummary {
                    jid: chat_jid.clone(),
                    name: preferred_display_name(Some(&current_display_name), phone.as_deref(), &chat_jid),
                    phone: phone.clone(),
                    status: None,
                    avatar_url: None,
                    is_business: false,
                    is_registered: true,
                });
            }

            store.record_message(
                chat_jid.clone(),
                Some(current_display_name),
                phone,
                is_group,
                ChatMessage {
                    id: msg_id.clone(),
                    chat_jid,
                    sender_jid: "me".into(),
                    sender_name: Some("You".into()),
                    text: trimmed_message.to_string(),
                    timestamp_ms,
                    from_me: true,
                    mentions: mention_summaries,
                    media: None,
                    receipt_status: Some("sent".into()),
                },
                None,
            );

            Ok(Json(SendMessageResponse {
                success: true,
                message_id: Some(msg_id),
            }))
        }
        Err(e) => {
            log::error!("Failed to send message: {e:?}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to send: {e}"),
                }),
            ))
        }
    }
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/qr", get(get_qr))
        .route("/api/auth/status", get(get_status))
        .route("/api/auth/logout", post(logout))
        .route("/api/bootstrap", get(get_bootstrap))
        .route("/api/chats", get(get_chats))
        .route("/api/chats/:jid/messages", get(get_chat_messages))
    .route("/api/chats/:jid/read", post(mark_chat_read))
    .route("/api/chats/:jid/typing", post(update_typing))
        .route("/api/contacts", get(get_contacts))
        .route("/api/messages/send", post(send_message))
    .route("/api/media/:chat_jid/:message_id", get(get_media))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

fn normalize_phone(input: &str) -> String {
    input.trim().replace(['+', '-', ' ', '(', ')'], "")
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn message_timestamp_ms(info: &whatsapp_rust::types::message::MessageInfo) -> i64 {
    info.timestamp.timestamp_millis()
}

fn jid_phone_str(jid: &str) -> Option<String> {
    if jid.contains("@s.whatsapp.net") {
        Some(jid.split('@').next().unwrap_or_default().to_string())
    } else {
        None
    }
}

fn preferred_display_name(name: Option<&str>, phone: Option<&str>, jid: &str) -> String {
    if let Some(value) = name.filter(|value| !value.trim().is_empty()) {
        return value.to_string();
    }
    if let Some(value) = phone.filter(|value| !value.is_empty()) {
        return format!("+{value}");
    }
    jid.to_string()
}

fn looks_like_fallback_name(name: &str, jid: &str, phone: Option<&str>) -> bool {
    if name.trim().is_empty() || name == jid {
        return true;
    }
    if let Some(phone) = phone {
        return name == phone || name == format!("+{phone}");
    }
    false
}

fn display_name_rank(name: &str, jid: &str, phone: Option<&str>) -> u8 {
    if name.trim().is_empty() {
        return 0;
    }
    if name == jid {
        return 1;
    }
    if let Some(phone) = phone {
        if name == phone || name == format!("+{phone}") {
            return 2;
        }
    }
    3
}

fn is_better_name(candidate: &str, current: &str, jid: &str, phone: Option<&str>) -> bool {
    if candidate.trim().is_empty() {
        return false;
    }
    if current.trim().is_empty() {
        return true;
    }
    display_name_rank(candidate, jid, phone) > display_name_rank(current, jid, phone)
}

fn media_key(chat_jid: &str, message_id: &str) -> String {
    format!("{chat_jid}:{message_id}")
}

fn receipt_rank(status: &str) -> usize {
    match status {
        "sent" => 1,
        "delivered" => 2,
        "read" => 3,
        "played" => 4,
        _ => 0,
    }
}

fn promote_receipt_status(current: Option<&str>, new_status: &str) -> String {
    match current {
        Some(existing) if receipt_rank(existing) >= receipt_rank(new_status) => existing.to_string(),
        _ => new_status.to_string(),
    }
}

fn render_text_with_mentions(text: &str, mentions: &[MentionSummary]) -> String {
    let mut output = text.to_string();
    for mention in mentions {
        let token = mention
            .jid
            .split('@')
            .next()
            .unwrap_or_default()
            .split(':')
            .next()
            .unwrap_or_default();
        if token.is_empty() {
            continue;
        }
        let label = format!("@{}", mention.name.trim_start_matches('+'));
        output = output.replace(&format!("@{token}"), &label);
    }
    output
}

fn preview_for_message(message: &ChatMessage) -> String {
    if let Some(media) = &message.media {
        match media.kind.as_str() {
            "image" => media.caption.clone().unwrap_or_else(|| "📷 Photo".into()),
            "video" => media.caption.clone().unwrap_or_else(|| {
                if media.is_gif {
                    "🎞️ GIF".into()
                } else {
                    "🎥 Video".into()
                }
            }),
            "document" => media
                .file_name
                .clone()
                .map(|name| format!("📎 {name}"))
                .unwrap_or_else(|| "📎 Document".into()),
            "audio" => {
                if media.is_voice_note {
                    "🎤 Voice note".into()
                } else {
                    "🎵 Audio".into()
                }
            }
            "sticker" => "🪄 Sticker".into(),
            _ => render_text_with_mentions(&message.text, &message.mentions),
        }
    } else {
        render_text_with_mentions(&message.text, &message.mentions)
    }
}

fn extract_context_info(message: &wa::Message) -> Option<&wa::ContextInfo> {
    if let Some(value) = message.extended_text_message.as_ref().and_then(|value| value.context_info.as_deref()) {
        return Some(value);
    }
    if let Some(value) = message.image_message.as_ref().and_then(|value| value.context_info.as_deref()) {
        return Some(value);
    }
    if let Some(value) = message.video_message.as_ref().and_then(|value| value.context_info.as_deref()) {
        return Some(value);
    }
    if let Some(value) = message.document_message.as_ref().and_then(|value| value.context_info.as_deref()) {
        return Some(value);
    }
    if let Some(value) = message.audio_message.as_ref().and_then(|value| value.context_info.as_deref()) {
        return Some(value);
    }
    if let Some(value) = message.sticker_message.as_ref().and_then(|value| value.context_info.as_deref()) {
        return Some(value);
    }
    None
}

fn extract_text(message: &wa::Message) -> String {
    if let Some(text) = message.conversation.as_ref().filter(|value| !value.is_empty()) {
        return text.clone();
    }
    if let Some(text) = message
        .extended_text_message
        .as_ref()
        .and_then(|value| value.text.clone())
        .filter(|value| !value.is_empty())
    {
        return text;
    }
    if let Some(caption) = message
        .image_message
        .as_ref()
        .and_then(|value| value.caption.clone())
        .filter(|value| !value.is_empty())
    {
        return caption;
    }
    if let Some(caption) = message
        .video_message
        .as_ref()
        .and_then(|value| value.caption.clone())
        .filter(|value| !value.is_empty())
    {
        return caption;
    }
    if let Some(caption) = message
        .document_message
        .as_ref()
        .and_then(|value| value.caption.clone())
        .filter(|value| !value.is_empty())
    {
        return caption;
    }
    if let Some(file_name) = message
        .document_message
        .as_ref()
        .and_then(|value| value.file_name.clone())
        .filter(|value| !value.is_empty())
    {
        return file_name;
    }
    if message.audio_message.is_some() {
        return "Voice message".into();
    }
    if message.sticker_message.is_some() {
        return "Sticker".into();
    }
    "<non-text>".into()
}

fn jid_phone(jid: &Jid) -> Option<String> {
    jid_phone_str(&jid.to_string())
}

fn jid_is_group(jid: &Jid) -> bool {
    jid.to_string().ends_with("@g.us")
}

fn build_media_blob(
    mime_type: Option<String>,
    file_name: Option<String>,
    direct_path: Option<String>,
    media_key: Option<Vec<u8>>,
    file_sha256: Option<Vec<u8>>,
    file_enc_sha256: Option<Vec<u8>>,
    file_length: Option<u64>,
    media_type: MediaType,
) -> Option<MediaBlob> {
    Some(MediaBlob {
        mime_type,
        file_name,
        direct_path: direct_path?,
        media_key: media_key?,
        file_sha256: file_sha256?,
        file_enc_sha256: file_enc_sha256?,
        file_length: file_length?,
        media_type,
    })
}

fn inline_jpeg_preview_url(thumbnail: Option<&Vec<u8>>) -> Option<String> {
    let bytes = thumbnail?;
    if bytes.is_empty() {
        return None;
    }

    Some(format!(
        "data:image/jpeg;base64,{}",
        BASE64_STANDARD.encode(bytes)
    ))
}

fn extract_media(message: &wa::Message, chat_jid: &str, message_id: &str) -> Option<(MediaAttachment, MediaBlob)> {
    if let Some(image) = message.image_message.as_ref() {
        let blob = build_media_blob(
            image.mimetype.clone(),
            None,
            image.direct_path.clone(),
            image.media_key.clone(),
            image.file_sha256.clone(),
            image.file_enc_sha256.clone(),
            image.file_length,
            MediaType::Image,
        )?;
        return Some((
            MediaAttachment {
                kind: "image".into(),
                mime_type: image.mimetype.clone(),
                caption: image.caption.clone(),
                title: None,
                file_name: None,
                file_length: image.file_length,
                page_count: None,
                width: image.width,
                height: image.height,
                duration_seconds: None,
                is_voice_note: false,
                is_gif: false,
                is_sticker: false,
                download_path: Some(format!("/api/media/{chat_jid}/{message_id}")),
                preview_image_url: None,
            },
            blob,
        ));
    }
    if let Some(video) = message.video_message.as_ref() {
        let blob = build_media_blob(
            video.mimetype.clone(),
            None,
            video.direct_path.clone(),
            video.media_key.clone(),
            video.file_sha256.clone(),
            video.file_enc_sha256.clone(),
            video.file_length,
            MediaType::Video,
        )?;
        return Some((
            MediaAttachment {
                kind: "video".into(),
                mime_type: video.mimetype.clone(),
                caption: video.caption.clone(),
                title: None,
                file_name: None,
                file_length: video.file_length,
                page_count: None,
                width: video.width,
                height: video.height,
                duration_seconds: video.seconds,
                is_voice_note: false,
                is_gif: video.gif_playback.unwrap_or(false),
                is_sticker: false,
                download_path: Some(format!("/api/media/{chat_jid}/{message_id}")),
                preview_image_url: None,
            },
            blob,
        ));
    }
    if let Some(document) = message.document_message.as_ref() {
        let blob = build_media_blob(
            document.mimetype.clone(),
            document.file_name.clone(),
            document.direct_path.clone(),
            document.media_key.clone(),
            document.file_sha256.clone(),
            document.file_enc_sha256.clone(),
            document.file_length,
            MediaType::Document,
        )?;
        return Some((
            MediaAttachment {
                kind: "document".into(),
                mime_type: document.mimetype.clone(),
                caption: document.caption.clone(),
                title: document.title.clone(),
                file_name: document.file_name.clone(),
                file_length: document.file_length,
                page_count: document.page_count,
                width: None,
                height: None,
                duration_seconds: None,
                is_voice_note: false,
                is_gif: false,
                is_sticker: false,
                download_path: Some(format!("/api/media/{chat_jid}/{message_id}")),
                preview_image_url: inline_jpeg_preview_url(document.jpeg_thumbnail.as_ref()),
            },
            blob,
        ));
    }
    if let Some(audio) = message.audio_message.as_ref() {
        let blob = build_media_blob(
            audio.mimetype.clone(),
            None,
            audio.direct_path.clone(),
            audio.media_key.clone(),
            audio.file_sha256.clone(),
            audio.file_enc_sha256.clone(),
            audio.file_length,
            MediaType::Audio,
        )?;
        return Some((
            MediaAttachment {
                kind: "audio".into(),
                mime_type: audio.mimetype.clone(),
                caption: None,
                title: None,
                file_name: None,
                file_length: audio.file_length,
                page_count: None,
                width: None,
                height: None,
                duration_seconds: audio.seconds,
                is_voice_note: audio.ptt.unwrap_or(false),
                is_gif: false,
                is_sticker: false,
                download_path: Some(format!("/api/media/{chat_jid}/{message_id}")),
                preview_image_url: None,
            },
            blob,
        ));
    }
    if let Some(sticker) = message.sticker_message.as_ref() {
        let mime = sticker.mimetype.clone().or_else(|| Some("image/webp".into()));
        let blob = build_media_blob(
            mime.clone(),
            None,
            sticker.direct_path.clone(),
            sticker.media_key.clone(),
            sticker.file_sha256.clone(),
            sticker.file_enc_sha256.clone(),
            sticker.file_length,
            MediaType::Sticker,
        )?;
        return Some((
            MediaAttachment {
                kind: "sticker".into(),
                mime_type: mime,
                caption: None,
                title: None,
                file_name: None,
                file_length: sticker.file_length,
                page_count: None,
                width: None,
                height: None,
                duration_seconds: None,
                is_voice_note: false,
                is_gif: false,
                is_sticker: true,
                download_path: Some(format!("/api/media/{chat_jid}/{message_id}")),
                preview_image_url: None,
            },
            blob,
        ));
    }
    None
}

fn default_mime_for_media_type(media_type: MediaType) -> &'static str {
    match media_type {
        MediaType::Image => "image/jpeg",
        MediaType::Video => "video/mp4",
        MediaType::Audio => "audio/mpeg",
        MediaType::Document => "application/octet-stream",
        MediaType::Sticker => "image/webp",
        _ => "application/octet-stream",
    }
}

async fn ensure_presence_subscription(client: Arc<Client>, jid: Jid) {
    if !jid_is_group(&jid) {
        let _ = client.presence().subscribe(&jid).await;
    }
}

async fn resolve_phone_for_jid(client: &Client, jid: &Jid) -> Option<String> {
    if jid.server == "lid" {
        client.get_phone_number_from_lid(&jid.to_string()).await
    } else {
        jid_phone(jid)
    }
}

fn resolved_display_name_for_store(store: &DataStore, jid: &str, phone: Option<&str>) -> String {
    let current = store.display_name_for_jid(jid);
    if looks_like_fallback_name(&current, jid, phone) {
        preferred_display_name(None, phone, jid)
    } else {
        current
    }
}

async fn resolve_mention_summaries(
    state: &AppState,
    client: &Arc<Client>,
    mention_jids: &[String],
) -> Vec<MentionSummary> {
    let mut summaries = Vec::with_capacity(mention_jids.len());

    for mention_jid in mention_jids {
        let phone = if let Ok(parsed) = mention_jid.parse::<Jid>() {
            resolve_phone_for_jid(client.as_ref(), &parsed.to_non_ad()).await
        } else {
            None
        };

        let name = {
            let store = state.store.read().await;
            resolved_display_name_for_store(&store, mention_jid, phone.as_deref())
        };

        summaries.push(MentionSummary {
            jid: mention_jid.clone(),
            name,
        });
    }

    summaries
}

async fn refresh_contact_profile(state: AppState, client: Arc<Client>, jid: Jid) {
    let lookup_jid = jid.to_non_ad();
    let lookup_jid_str = lookup_jid.to_string();
    let phone = resolve_phone_for_jid(&client, &lookup_jid).await;
    let existing_name = {
        let store = state.store.read().await;
        store
            .contacts
            .get(&lookup_jid_str)
            .map(|contact| contact.name.clone())
    };

    let mut alias_jids = vec![lookup_jid_str.clone()];

    let mut updated_contact = ContactSummary {
        jid: lookup_jid_str.clone(),
        name: existing_name
            .filter(|name| !looks_like_fallback_name(name, &lookup_jid_str, phone.as_deref()))
            .unwrap_or_else(|| preferred_display_name(None, phone.as_deref(), &lookup_jid_str)),
        phone: phone.clone(),
        status: None,
        avatar_url: None,
        is_business: false,
        is_registered: phone.is_some(),
    };

    if let Some(phone) = phone.as_deref() {
        if let Ok(info_list) = client.contacts().get_info(&[phone]).await {
            if let Some(info) = info_list.into_iter().next() {
                updated_contact.jid = info.jid.to_string();
                updated_contact.phone = Some(phone.to_string());
                updated_contact.status = info.status.clone();
                updated_contact.is_business = info.is_business;
                updated_contact.is_registered = info.is_registered;
                if let Some(lid) = info.lid {
                    alias_jids.push(lid.to_non_ad().to_string());
                }
            }
        }
    }

    if let Ok(info_map) = client.contacts().get_user_info(&[lookup_jid.clone()]).await {
        if let Some(info) = info_map.get(&lookup_jid) {
            updated_contact.jid = info.jid.to_string();
            updated_contact.status = info.status.clone().or(updated_contact.status.clone());
            updated_contact.is_business = info.is_business;
            if let Some(lid) = &info.lid {
                alias_jids.push(lid.to_non_ad().to_string());
            }
        }
    }

    alias_jids.push(updated_contact.jid.clone());
    alias_jids.sort();
    alias_jids.dedup();

    if let Ok(Some(picture)) = client.contacts().get_profile_picture(&lookup_jid, true).await {
        updated_contact.avatar_url = Some(picture.url);
    }

    let contact_jid = updated_contact.jid.clone();
    let contact_name = updated_contact.name.clone();
    let contact_phone = updated_contact.phone.clone();
    let contact_status = updated_contact.status.clone();
    let contact_avatar = updated_contact.avatar_url.clone();

    let mut store = state.store.write().await;
    store.upsert_contact(updated_contact.clone());
    for alias_jid in &alias_jids {
        store.upsert_contact(ContactSummary {
            jid: alias_jid.clone(),
            name: contact_name.clone(),
            phone: contact_phone.clone(),
            status: contact_status.clone(),
            avatar_url: contact_avatar.clone(),
            is_business: updated_contact.is_business,
            is_registered: updated_contact.is_registered,
        });

        if let Some(chat) = store.chats.get_mut(alias_jid.as_str()) {
            if is_better_name(&contact_name, &chat.summary.name, &contact_jid, contact_phone.as_deref()) {
                chat.summary.name = contact_name.clone();
            }
            if contact_phone.is_some() {
                chat.summary.phone = contact_phone.clone();
            }
            if contact_status.is_some() {
                chat.summary.status = contact_status.clone();
            }
            if contact_avatar.is_some() {
                chat.summary.avatar_url = contact_avatar.clone();
            }
        }
    }
    store.refresh_mention_names(&alias_jids, &contact_name, contact_phone.as_deref());
}

async fn refresh_group_metadata(state: AppState, client: Arc<Client>, jid: Jid) {
    if let Ok(group) = client.groups().get_metadata(&jid).await {
        let mut store = state.store.write().await;
        let chat = store.ensure_chat(jid.to_string(), Some(group.subject.clone()), None, true);
        chat.summary.name = group.subject;
        chat.summary.is_group = true;
    }

    if let Ok(group_info) = client.groups().query_info(&jid).await {
        let mut participant_jids = Vec::new();
        for participant in &group_info.participants {
            participant_jids.push(participant.to_non_ad());
        }
        for (lid_user, phone_jid) in group_info.lid_to_pn_map() {
            participant_jids.push(Jid::new(lid_user, "lid").to_non_ad());
            participant_jids.push(phone_jid.to_non_ad());
        }
        for participant in participant_jids {
            if !jid_is_group(&participant) {
                tokio::spawn(refresh_contact_profile(state.clone(), client.clone(), participant.clone()));
                tokio::spawn(ensure_presence_subscription(client.clone(), participant));
            }
        }
    }
}

async fn refresh_all_known_contacts(state: AppState, client: Arc<Client>) {
    let contact_jids: Vec<String> = {
        let store = state.store.read().await;
        store
            .contacts
            .keys()
            .chain(store.chats.keys())
            .cloned()
            .collect()
    };

    for jid in contact_jids {
        if let Ok(parsed) = jid.parse::<Jid>() {
            if jid_is_group(&parsed) {
                refresh_group_metadata(state.clone(), client.clone(), parsed).await;
            } else {
                ensure_presence_subscription(client.clone(), parsed.clone()).await;
                refresh_contact_profile(state.clone(), client.clone(), parsed).await;
            }
        }
    }
}

async fn wait_for_startup_sync(state: AppState, client: Arc<Client>) {
    state.is_syncing.store(true, Ordering::SeqCst);
    let _ = client
        .wait_for_startup_sync(Duration::from_secs(STARTUP_SYNC_TIMEOUT_SECS))
        .await;
    state.is_syncing.store(false, Ordering::SeqCst);
    refresh_all_known_contacts(state, client).await;
}

async fn handle_incoming_message(
    state: AppState,
    client: Arc<Client>,
    msg: Box<wa::Message>,
    info: whatsapp_rust::types::message::MessageInfo,
) {
    let chat = info.source.chat.to_non_ad();
    let chat_jid = chat.to_string();
    let sender = info.source.sender.to_non_ad();
    let sender_jid = sender.to_string();
    let phone = resolve_phone_for_jid(&client, &chat).await;
    let sender_phone = resolve_phone_for_jid(&client, &sender).await;
    let display_name = if !info.push_name.trim().is_empty() {
        info.push_name.clone()
    } else {
        let store = state.store.read().await;
        store.display_name_for_jid(&sender_jid)
    };
    let text = extract_text(&msg);
    let mention_jids = extract_context_info(&msg)
        .map(|ctx| ctx.mentioned_jid.clone())
        .unwrap_or_default();
    let mention_summaries = resolve_mention_summaries(&state, &client, &mention_jids).await;
    let (mentions, media, media_blob) = {
        let media = extract_media(&msg, &chat_jid, &info.id);
        match media {
            Some((attachment, blob)) => (mention_summaries, Some(attachment), Some(blob)),
            None => (mention_summaries, None, None),
        }
    };

    {
        let mut store = state.store.write().await;
        if !jid_is_group(&chat) {
            store.upsert_contact(ContactSummary {
                jid: chat_jid.clone(),
                name: display_name.clone(),
                phone: phone.clone(),
                status: None,
                avatar_url: None,
                is_business: false,
                is_registered: true,
            });
        } else {
            store.upsert_contact(ContactSummary {
                jid: sender_jid.clone(),
                name: display_name.clone(),
                phone: sender_phone.clone(),
                status: None,
                avatar_url: None,
                is_business: false,
                is_registered: true,
            });
        }

        if !info.push_name.trim().is_empty() {
            store.rename_contact(&sender_jid, &info.push_name);
        }

        let chat_display_name = if jid_is_group(&chat) {
            store.display_name_for_jid(&chat_jid)
        } else {
            display_name.clone()
        };

        store.record_message(
            chat_jid.clone(),
            Some(chat_display_name),
            phone,
            jid_is_group(&chat),
            ChatMessage {
                id: info.id.clone(),
                chat_jid: chat_jid.clone(),
                sender_jid,
                sender_name: if jid_is_group(&chat) {
                    Some(display_name.clone())
                } else {
                    (!display_name.trim().is_empty()).then_some(display_name.clone())
                },
                text,
                timestamp_ms: message_timestamp_ms(&info),
                from_me: info.source.is_from_me,
                mentions,
                media,
                receipt_status: if info.source.is_from_me {
                    Some("sent".into())
                } else {
                    None
                },
            },
            media_blob,
        );
    }

    let state_for_mentions = state.clone();
    let client_for_mentions = client.clone();

    if jid_is_group(&chat) {
        tokio::spawn(refresh_group_metadata(state.clone(), client.clone(), chat));
        tokio::spawn(refresh_contact_profile(state, client, sender));
    } else {
        tokio::spawn(ensure_presence_subscription(client.clone(), chat.clone()));
        tokio::spawn(refresh_contact_profile(state, client, chat));
    }

    for mention_jid in mention_jids {
        if let Ok(parsed) = mention_jid.parse::<Jid>() {
            tokio::spawn(refresh_contact_profile(
                state_for_mentions.clone(),
                client_for_mentions.clone(),
                parsed.to_non_ad(),
            ));
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let db_path = "whatsapp.db";
    let state = AppState::new(db_path);
    let handler_state = state.clone();

    log::info!("Initialising WhatsApp client …");
    let backend = Arc::new(SqliteStore::new(db_path).await?);

    let mut bot = Bot::builder()
        .with_backend(backend)
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_http_client(UreqHttpClient::new())
        .on_event(move |event, client| {
            let state = handler_state.clone();
            async move {
                match event {
                    Event::PairingQrCode { code, timeout } => {
                        log::info!("QR code received (valid for {}s)", timeout.as_secs());
                        state.is_connected.store(false, Ordering::SeqCst);
                        state.is_syncing.store(false, Ordering::SeqCst);
                        *state.qr_code.write().await = Some(code);
                    }
                    Event::Connected(_) => {
                        log::info!("✅ WhatsApp connected!");
                        state.is_connected.store(true, Ordering::SeqCst);
                        *state.qr_code.write().await = None;
                        tokio::spawn(wait_for_startup_sync(state.clone(), client.clone()));
                    }
                    Event::Disconnected(_) | Event::LoggedOut(_) | Event::ConnectFailure(_) => {
                        state.is_connected.store(false, Ordering::SeqCst);
                        state.is_syncing.store(false, Ordering::SeqCst);
                    }
                    Event::Message(msg, info) => {
                        let preview = extract_text(&msg);
                        log::info!("📩 Message from {}: {}", info.source.sender, preview);
                        handle_incoming_message(state.clone(), client.clone(), msg, info).await;
                    }
                    Event::Receipt(receipt) => {
                        let status = match receipt.r#type {
                            ReceiptType::Sender => Some("sent"),
                            ReceiptType::Delivered => Some("delivered"),
                            ReceiptType::Read | ReceiptType::ReadSelf => Some("read"),
                            ReceiptType::Played | ReceiptType::PlayedSelf => Some("played"),
                            _ => None,
                        };
                        if let Some(status) = status {
                            let mut store = state.store.write().await;
                            store.update_message_receipt(
                                &receipt.source.chat.to_non_ad().to_string(),
                                &receipt.message_ids,
                                status,
                            );
                        }
                    }
                    Event::ChatPresence(update) => {
                        let chat_jid = update.source.chat.to_non_ad().to_string();
                        let typing = match update.state {
                            ChatPresence::Composing => {
                                if update.source.is_group {
                                    let sender_jid = update.source.sender.to_non_ad().to_string();
                                    let sender_name = {
                                        let store = state.store.read().await;
                                        store.display_name_for_jid(&sender_jid)
                                    };
                                    Some(format!("{sender_name} is typing…"))
                                } else {
                                    Some("typing…".into())
                                }
                            }
                            ChatPresence::Paused => None,
                        };
                        let mut store = state.store.write().await;
                        store.set_typing(&chat_jid, typing);
                    }
                    Event::Presence(update) => {
                        let mut store = state.store.write().await;
                        store.set_presence(
                            &update.from.to_non_ad().to_string(),
                            !update.unavailable,
                            update.last_seen.map(|value| value.timestamp_millis()),
                        );
                    }
                    Event::JoinedGroup(conversation) => {
                        if let Some(conv) = conversation.get() {
                            let jid = conv.id.clone();
                            let name = conv
                                .name
                                .clone()
                                .filter(|value| !value.trim().is_empty())
                                .unwrap_or_else(|| jid.clone());
                            let mut store = state.store.write().await;
                            store.ensure_chat(jid.clone(), Some(name), None, jid.ends_with("@g.us"));
                        }
                    }
                    Event::PushNameUpdate(update) => {
                        let mut store = state.store.write().await;
                        store.rename_contact(&update.jid.to_non_ad().to_string(), &update.new_push_name);
                    }
                    Event::ContactUpdated(update) => {
                        tokio::spawn(refresh_contact_profile(
                            state.clone(),
                            client.clone(),
                            update.jid.to_non_ad(),
                        ));
                    }
                    Event::ContactNumberChanged(change) => {
                        let mut store = state.store.write().await;
                        let old_jid = change.old_jid.to_non_ad().to_string();
                        let new_jid = change.new_jid.to_non_ad().to_string();
                        if let Some(contact) = store.contacts.remove(&old_jid) {
                            store.upsert_contact(ContactSummary {
                                jid: new_jid.clone(),
                                phone: jid_phone(&change.new_jid.to_non_ad()),
                                ..contact
                            });
                        }
                        if let Some(mut chat) = store.chats.remove(&old_jid) {
                            chat.summary.jid = new_jid.clone();
                            chat.summary.phone = jid_phone(&change.new_jid.to_non_ad());
                            for message in &mut chat.messages {
                                message.chat_jid = new_jid.clone();
                            }
                            store.chats.insert(new_jid, chat);
                        }
                    }
                    Event::ContactSyncRequested(_) => {
                        tokio::spawn(refresh_all_known_contacts(state.clone(), client.clone()));
                    }
                    Event::PictureUpdate(update) => {
                        let jid = update.jid.to_non_ad().to_string();
                        if update.removed {
                            let mut store = state.store.write().await;
                            store.set_contact_avatar(&jid, None);
                        } else {
                            tokio::spawn(refresh_contact_profile(
                                state.clone(),
                                client.clone(),
                                update.jid.to_non_ad(),
                            ));
                        }
                    }
                    Event::UserAboutUpdate(update) => {
                        let mut store = state.store.write().await;
                        store.set_contact_status(&update.jid.to_non_ad().to_string(), Some(update.status));
                    }
                    Event::GroupUpdate(update) => {
                        tokio::spawn(refresh_group_metadata(
                            state.clone(),
                            client.clone(),
                            update.group_jid.to_non_ad(),
                        ));
                    }
                    Event::ArchiveUpdate(update) => {
                        let mut store = state.store.write().await;
                        store.set_archived(&update.jid.to_non_ad().to_string(), true);
                    }
                    Event::MuteUpdate(update) => {
                        let mut store = state.store.write().await;
                        store.set_muted(&update.jid.to_non_ad().to_string(), true);
                    }
                    Event::MarkChatAsReadUpdate(update) => {
                        let mut store = state.store.write().await;
                        store.mark_read(&update.jid.to_non_ad().to_string());
                    }
                    Event::OfflineSyncCompleted(_) => {
                        state.is_syncing.store(false, Ordering::SeqCst);
                    }
                    _ => {}
                }
            }
        })
        .build()
        .await?;

    let client = bot.client();
    *state.client.write().await = Some(client);

    let _run_handle = bot.run().await?;
    log::info!("WhatsApp bot running in background");

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .expect("PORT must be a valid number");

    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    log::info!("🚀 Backend listening on http://0.0.0.0:{port}");
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn pairing_state() -> AppState {
        let state = AppState::new("test.db");
        state.is_connected.store(false, Ordering::SeqCst);
        state.is_syncing.store(false, Ordering::SeqCst);
        state
    }

    fn connected_state_no_client() -> AppState {
        let state = AppState::new("test.db");
        state.is_connected.store(true, Ordering::SeqCst);
        state
    }

    async fn body_json(body: Body) -> serde_json::Value {
        let bytes = body.collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn json_post(uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap()
    }

    #[tokio::test]
    async fn qr_returns_code_when_pairing() {
        let state = pairing_state();
        *state.qr_code.write().await = Some("2@ABC123,ref,pk,cid".into());
        let app = create_router(state);
        let resp = app
            .oneshot(Request::get("/api/auth/qr").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["qr_code"], "2@ABC123,ref,pk,cid");
        assert_eq!(json["is_connected"], false);
    }

    #[tokio::test]
    async fn bootstrap_returns_empty_lists() {
        let app = create_router(pairing_state());
        let resp = app
            .oneshot(Request::get("/api/bootstrap").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["chats"].as_array().unwrap().len(), 0);
        assert_eq!(json["contacts"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn status_reports_disconnected() {
        let app = create_router(pairing_state());
        let resp = app
            .oneshot(Request::get("/api/auth/status").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let json = body_json(resp.into_body()).await;
        assert_eq!(json["is_connected"], false);
        assert_eq!(json["is_syncing"], false);
    }

    #[tokio::test]
    async fn send_rejects_empty_target() {
        let app = create_router(connected_state_no_client());
        let resp = app
            .oneshot(json_post(
                "/api/messages/send",
                r#"{"phone":"","message":"hi"}"#,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = body_json(resp.into_body()).await;
    assert!(json["error"].as_str().unwrap().contains("jid"));
    }

    #[tokio::test]
    async fn send_rejects_empty_message() {
        let app = create_router(connected_state_no_client());
        let resp = app
            .oneshot(json_post(
                "/api/messages/send",
                r#"{"phone":"15551234567","message":"  "}"#,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = body_json(resp.into_body()).await;
        assert!(json["error"].as_str().unwrap().contains("message"));
    }

    #[tokio::test]
    async fn send_rejects_malformed_json() {
        let app = create_router(connected_state_no_client());
        let resp = app
            .oneshot(json_post("/api/messages/send", r#"{ not json }"#))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn send_rejects_missing_fields() {
        let app = create_router(connected_state_no_client());
        let resp = app
            .oneshot(json_post(
                "/api/messages/send",
                r#"{"phone":"15551234567"}"#,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn send_returns_503_when_not_connected() {
        let app = create_router(pairing_state());
        let resp = app
            .oneshot(json_post(
                "/api/messages/send",
                r#"{"phone":"15551234567","message":"hello"}"#,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn send_returns_503_when_client_missing() {
        let app = create_router(connected_state_no_client());
        let resp = app
            .oneshot(json_post(
                "/api/messages/send",
                r#"{"phone":"15551234567","message":"hello"}"#,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn send_rejects_non_digit_phone() {
        let app = create_router(connected_state_no_client());
        let resp = app
            .oneshot(json_post(
                "/api/messages/send",
                r#"{"phone":"abc","message":"hello"}"#,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = body_json(resp.into_body()).await;
        assert!(json["error"].as_str().unwrap().contains("digit"));
    }

    #[tokio::test]
    async fn send_accepts_jid_shape() {
        let app = create_router(connected_state_no_client());
        let resp = app
            .oneshot(json_post(
                "/api/messages/send",
                r#"{"jid":"15551234567@s.whatsapp.net","message":"hello"}"#,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn messages_endpoint_returns_404_for_unknown_chat() {
        let app = create_router(pairing_state());
        let resp = app
            .oneshot(
                Request::get("/api/chats/unknown/messages")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
