//! WhatsApp Web Clone — Rust Backend
//!
//! Endpoints:
//!   GET  /api/auth/qr                  → current QR code string + connection status
//!   GET  /api/auth/status              → connection status + sync status
//!   POST /api/auth/logout              → disconnect current session
//!   GET  /api/bootstrap                → initial UI payload (chats + contacts)
//!   GET  /api/chats                    → synced chat summaries
//!   GET  /api/chats/:jid/messages      → messages for a synced chat
//!   GET  /api/contacts                 → known contacts
//!   POST /api/messages/send            → send a WhatsApp message

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use whatsapp_rust::bot::Bot;
use whatsapp_rust::store::SqliteStore;
use whatsapp_rust::transport::{TokioWebSocketTransportFactory, UreqHttpClient};
use whatsapp_rust::types::events::Event;
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
pub struct ChatMessage {
    pub id: String,
    pub chat_jid: String,
    pub sender_jid: String,
    pub sender_name: Option<String>,
    pub text: String,
    pub timestamp_ms: i64,
    pub from_me: bool,
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
}

impl DataStore {
    fn reset(&mut self) {
        self.chats.clear();
        self.contacts.clear();
    }

    fn sorted_chats(&self) -> Vec<ChatSummary> {
        let mut chats: Vec<_> = self.chats.values().map(|chat| chat.summary.clone()).collect();
        chats.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms).then_with(|| a.name.cmp(&b.name)));
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

    fn ensure_chat(
        &mut self,
        chat_jid: String,
        name: Option<String>,
        phone: Option<String>,
        is_group: bool,
    ) -> &mut ChatRecord {
        let fallback_name = name
            .clone()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| phone.clone().map(|value| format!("+{value}")))
            .unwrap_or_else(|| chat_jid.clone());

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

        if let Some(name) = name.filter(|value| !value.trim().is_empty()) {
            record.summary.name = name;
        }
        if let Some(phone) = phone {
            record.summary.phone = Some(phone);
        }
        record.summary.is_group = is_group;
        record
    }

    fn upsert_contact(&mut self, contact: ContactSummary) {
        let entry = self
            .contacts
            .entry(contact.jid.clone())
            .or_insert_with(|| contact.clone());

        if !contact.name.trim().is_empty() {
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
    ) {
        let preview = Some(message.text.clone());
        let timestamp_ms = Some(message.timestamp_ms);
        let from_me = message.from_me;

        let chat = self.ensure_chat(chat_jid, chat_name, phone, is_group);
        if chat.messages.iter().any(|existing| existing.id == message.id) {
            return;
        }

        chat.messages.push(message);
        if chat.messages.len() > MAX_MESSAGES_PER_CHAT {
            let overflow = chat.messages.len() - MAX_MESSAGES_PER_CHAT;
            chat.messages.drain(0..overflow);
        }

        chat.summary.preview = preview;
        chat.summary.timestamp_ms = timestamp_ms;
        if !from_me {
            chat.summary.unread_count = chat.summary.unread_count.saturating_add(1);
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
    pub phone: String,
    pub message: String,
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
    let phone = normalize_phone(&payload.phone);

    if phone.is_empty() || !phone.chars().all(|c| c.is_ascii_digit()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "\"phone\" is required and must contain only digits (E.164 format)".into(),
            }),
        ));
    }

    if payload.message.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "\"message\" must not be empty".into(),
            }),
        ));
    }

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

    let jid = Jid::pn(&phone);
    let wa_message = wa::Message {
        conversation: Some(payload.message.clone()),
        ..Default::default()
    };

    match client.send_message(jid.clone(), wa_message).await {
        Ok(msg_id) => {
            let mut store = state.store.write().await;
            let chat_jid = jid.to_string();
            let timestamp_ms = now_ms();

            store.upsert_contact(ContactSummary {
                jid: chat_jid.clone(),
                name: format!("+{phone}"),
                phone: Some(phone.clone()),
                status: None,
                avatar_url: None,
                is_business: false,
                is_registered: true,
            });

            store.record_message(
                chat_jid.clone(),
                Some(format!("+{phone}")),
                Some(phone),
                false,
                ChatMessage {
                    id: msg_id.clone(),
                    chat_jid,
                    sender_jid: "me".into(),
                    sender_name: Some("You".into()),
                    text: payload.message,
                    timestamp_ms,
                    from_me: true,
                },
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
        .route("/api/contacts", get(get_contacts))
        .route("/api/messages/send", post(send_message))
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
        return format!("📷 {caption}");
    }
    if let Some(caption) = message
        .video_message
        .as_ref()
        .and_then(|value| value.caption.clone())
        .filter(|value| !value.is_empty())
    {
        return format!("🎥 {caption}");
    }
    if message.audio_message.is_some() {
        return "🎵 Voice message".into();
    }
    if message.document_message.is_some() {
        return "📎 Document".into();
    }
    if message.sticker_message.is_some() {
        return "🪄 Sticker".into();
    }
    "<non-text>".into()
}

fn jid_phone(jid: &Jid) -> Option<String> {
    let jid_str = jid.to_string();
    if jid_str.contains("@s.whatsapp.net") {
        Some(jid_str.split('@').next().unwrap_or_default().to_string())
    } else {
        None
    }
}

fn jid_is_group(jid: &Jid) -> bool {
    jid.to_string().ends_with("@g.us")
}

async fn refresh_contact_profile(state: AppState, client: Arc<Client>, jid: Jid) {
    let lookup_jid = jid.to_non_ad();
    let phone = jid_phone(&lookup_jid);

    let mut updated_contact = ContactSummary {
        jid: lookup_jid.to_string(),
        name: phone
            .clone()
            .map(|value| format!("+{value}"))
            .unwrap_or_else(|| lookup_jid.to_string()),
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
            }
        }
    }

    if let Ok(info_map) = client.contacts().get_user_info(&[lookup_jid.clone()]).await {
        if let Some(info) = info_map.get(&lookup_jid) {
            updated_contact.jid = info.jid.to_string();
            updated_contact.status = info.status.clone().or(updated_contact.status.clone());
            updated_contact.is_business = info.is_business;
        }
    }

    if let Ok(Some(picture)) = client.contacts().get_profile_picture(&lookup_jid, true).await {
        updated_contact.avatar_url = Some(picture.url);
    }

    let contact_jid = updated_contact.jid.clone();
    let contact_name = updated_contact.name.clone();
    let contact_phone = updated_contact.phone.clone();
    let contact_status = updated_contact.status.clone();
    let contact_avatar = updated_contact.avatar_url.clone();

    let mut store = state.store.write().await;
    store.upsert_contact(updated_contact);
    if let Some(chat) = store.chats.get_mut(&contact_jid) {
        if !contact_name.trim().is_empty() {
            chat.summary.name = contact_name;
        }
        if contact_phone.is_some() {
            chat.summary.phone = contact_phone;
        }
        if contact_status.is_some() {
            chat.summary.status = contact_status;
        }
        if contact_avatar.is_some() {
            chat.summary.avatar_url = contact_avatar;
        }
    }
}

async fn refresh_group_metadata(state: AppState, client: Arc<Client>, jid: Jid) {
    if let Ok(group) = client.groups().get_metadata(&jid).await {
        let mut store = state.store.write().await;
        let chat = store.ensure_chat(jid.to_string(), Some(group.subject.clone()), None, true);
        chat.summary.name = group.subject;
        chat.summary.is_group = true;
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
    let text = extract_text(&msg);
    let chat = info.source.chat.to_non_ad();
    let chat_jid = chat.to_string();
    let sender_jid = info.source.sender.to_string();
    let phone = jid_phone(&chat);
    let display_name = if !info.push_name.trim().is_empty() {
        info.push_name.clone()
    } else if let Some(phone) = phone.clone() {
        format!("+{phone}")
    } else {
        chat_jid.clone()
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
        }

        store.record_message(
            chat_jid.clone(),
            Some(display_name),
            phone,
            jid_is_group(&chat),
            ChatMessage {
                id: info.id.clone(),
                chat_jid: chat_jid.clone(),
                sender_jid,
                sender_name: (!info.push_name.trim().is_empty()).then_some(info.push_name.clone()),
                text,
                timestamp_ms: message_timestamp_ms(&info),
                from_me: info.source.is_from_me,
            },
        );
    }

    if jid_is_group(&chat) {
        tokio::spawn(refresh_group_metadata(state, client, chat));
    } else {
        tokio::spawn(refresh_contact_profile(state, client, chat));
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
    async fn send_rejects_empty_phone() {
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
        assert!(json["error"].as_str().unwrap().contains("phone"));
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
        assert!(json["error"].as_str().unwrap().contains("digits"));
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
