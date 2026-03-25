use std::sync::Arc;

use dashmap::DashMap;
use sqlx::PgPool;
use tokio::sync::oneshot;
use tracing::{info, warn, error};
use uuid::Uuid;

use whatsapp_rust::bot::Bot;
use whatsapp_rust::client::Client;
use whatsapp_rust::TokioRuntime;
use whatsapp_rust::store::SqliteStore;
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;
use wacore_binary::jid::Jid;
use waproto::whatsapp as wa;

use crate::config::AppConfig;
use crate::webhook::WebhookClient;
use crate::websocket::hub::WebSocketHub;
use crate::whatsapp::events;

/// Info about a running WhatsApp session
pub struct SessionInfo {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub client: Option<Arc<Client>>,
    pub shutdown_tx: Option<oneshot::Sender<()>>,
}

/// Result from sending a media message, includes upload metadata for DB storage.
pub struct MediaSendResult {
    pub msg_id: String,
    pub direct_path: String,
    pub media_key: Vec<u8>,
    pub file_enc_sha256: Vec<u8>,
}

/// Manages multiple WhatsApp sessions using the whatsapp-rust library.
#[derive(Clone)]
pub struct WhatsAppManager {
    pub sessions: Arc<DashMap<Uuid, SessionInfo>>,
    pub db: PgPool,
    pub config: Arc<AppConfig>,
    pub ws_hub: Arc<WebSocketHub>,
    pub webhook_client: Arc<WebhookClient>,
}

impl WhatsAppManager {
    pub fn new(db: PgPool, config: Arc<AppConfig>, ws_hub: Arc<WebSocketHub>, webhook_client: Arc<WebhookClient>) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            db,
            config,
            ws_hub,
            webhook_client,
        }
    }

    /// Restore previously connected sessions on server start
    pub async fn restore_sessions(&self) {
        let rows = sqlx::query_as::<_, (Uuid, Uuid, String)>(
            "SELECT id, user_id, db_path FROM whatsapp_sessions WHERE status = 'connected'",
        )
        .fetch_all(&self.db)
        .await;

        match rows {
            Ok(sessions) => {
                for (session_id, user_id, _db_path) in sessions {
                    info!(%session_id, %user_id, "Restoring WhatsApp session");
                    if let Err(e) = self.start_session(session_id, user_id).await {
                        warn!(%session_id, "Failed to restore session: {}", e);
                        let _ = sqlx::query(
                            "UPDATE whatsapp_sessions SET status = 'disconnected' WHERE id = $1",
                        )
                        .bind(session_id)
                        .execute(&self.db)
                        .await;
                    }
                }
            }
            Err(e) => {
                error!("Failed to query sessions for restore: {}", e);
            }
        }
    }

    /// Start a new WhatsApp session with real whatsapp-rust Bot
    pub async fn start_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), String> {
        if self.sessions.contains_key(&session_id) {
            return Err("Session already running".into());
        }

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let info = SessionInfo {
            session_id,
            user_id,
            client: None,
            shutdown_tx: Some(shutdown_tx),
        };

        self.sessions.insert(session_id, info);

        let manager = self.clone();
        tokio::spawn(async move {
            if let Err(e) = run_whatsapp_session(manager, session_id, user_id, shutdown_rx).await {
                error!(%session_id, "WhatsApp session error: {}", e);
            }
        });

        Ok(())
    }

    /// Disconnect a running session
    pub async fn disconnect_session(&self, session_id: Uuid) -> Result<(), String> {
        if let Some((_, mut info)) = self.sessions.remove(&session_id) {
            // Disconnect the WhatsApp client
            if let Some(client) = info.client.take() {
                client.disconnect().await;
            }
            // Send shutdown signal to the bot run loop
            if let Some(tx) = info.shutdown_tx.take() {
                let _ = tx.send(());
            }
            let _ = sqlx::query(
                "UPDATE whatsapp_sessions SET status = 'disconnected' WHERE id = $1",
            )
            .bind(session_id)
            .execute(&self.db)
            .await;
            info!(%session_id, "Session disconnected");
            Ok(())
        } else {
            Err("Session not running".into())
        }
    }

    pub fn is_session_alive(&self, session_id: &Uuid) -> bool {
        self.sessions.contains_key(session_id)
    }

    /// Send a text message through a real WhatsApp session
    pub async fn send_text_message(
        &self,
        session_id: Uuid,
        jid: &str,
        text: &str,
    ) -> Result<String, String> {
        self.send_text_message_with_reply(session_id, jid, text, None, None).await
    }

    /// Send a text message with optional reply context through a real WhatsApp session
    pub async fn send_text_message_with_reply(
        &self,
        session_id: Uuid,
        jid: &str,
        text: &str,
        reply_to_message_id: Option<&str>,
        reply_to_sender: Option<&str>,
    ) -> Result<String, String> {
        let client = self.get_client(session_id)?;

        let parsed_jid: Jid = jid.parse()
            .map_err(|e| format!("Invalid JID '{}': {}", jid, e))?;

        let msg = if let Some(stanza_id) = reply_to_message_id {
            // Build message with context_info for reply
            wa::Message {
                extended_text_message: Some(Box::new(wa::message::ExtendedTextMessage {
                    text: Some(text.to_string()),
                    context_info: Some(Box::new(wa::ContextInfo {
                        stanza_id: Some(stanza_id.to_string()),
                        participant: reply_to_sender.map(|s| s.to_string()),
                        ..Default::default()
                    })),
                    ..Default::default()
                })),
                ..Default::default()
            }
        } else {
            wa::Message {
                conversation: Some(text.to_string()),
                ..Default::default()
            }
        };

        let msg_id = client.send_message(parsed_jid, msg)
            .await
            .map_err(|e| format!("Failed to send message: {}", e))?;

        info!(%session_id, jid, "Sent text message: {}", msg_id);
        Ok(msg_id)
    }

    /// Send an image message through a real WhatsApp session
    pub async fn send_image_message(
        &self,
        session_id: Uuid,
        jid: &str,
        data: Vec<u8>,
        mime_type: &str,
        caption: Option<&str>,
    ) -> Result<MediaSendResult, String> {
        let client = self.get_client(session_id)?;

        let parsed_jid: Jid = jid.parse()
            .map_err(|e| format!("Invalid JID: {}", e))?;

        let upload = client.upload(data, wacore::download::MediaType::Image)
            .await
            .map_err(|e| format!("Failed to upload image: {}", e))?;

        let direct_path = upload.direct_path.clone();
        let media_key = upload.media_key.clone();
        let file_enc_sha256 = upload.file_enc_sha256.clone();

        let msg = wa::Message {
            image_message: Some(Box::new(wa::message::ImageMessage {
                url: Some(upload.url),
                direct_path: Some(upload.direct_path),
                media_key: Some(upload.media_key),
                file_enc_sha256: Some(upload.file_enc_sha256),
                file_sha256: Some(upload.file_sha256),
                file_length: Some(upload.file_length),
                caption: caption.map(|s| s.to_string()),
                mimetype: Some(mime_type.to_string()),
                ..Default::default()
            })),
            ..Default::default()
        };

        let msg_id = client.send_message(parsed_jid, msg)
            .await
            .map_err(|e| format!("Failed to send image: {}", e))?;

        info!(%session_id, jid, "Sent image message: {}", msg_id);
        Ok(MediaSendResult { msg_id, direct_path, media_key, file_enc_sha256 })
    }

    /// Send a document message through a real WhatsApp session
    pub async fn send_document_message(
        &self,
        session_id: Uuid,
        jid: &str,
        data: Vec<u8>,
        mime_type: &str,
        filename: &str,
    ) -> Result<MediaSendResult, String> {
        let client = self.get_client(session_id)?;

        let parsed_jid: Jid = jid.parse()
            .map_err(|e| format!("Invalid JID: {}", e))?;

        let upload = client.upload(data, wacore::download::MediaType::Document)
            .await
            .map_err(|e| format!("Failed to upload document: {}", e))?;

        let direct_path = upload.direct_path.clone();
        let media_key = upload.media_key.clone();
        let file_enc_sha256 = upload.file_enc_sha256.clone();

        let msg = wa::Message {
            document_message: Some(Box::new(wa::message::DocumentMessage {
                url: Some(upload.url),
                direct_path: Some(upload.direct_path),
                media_key: Some(upload.media_key),
                file_enc_sha256: Some(upload.file_enc_sha256),
                file_sha256: Some(upload.file_sha256),
                file_length: Some(upload.file_length),
                file_name: Some(filename.to_string()),
                mimetype: Some(mime_type.to_string()),
                ..Default::default()
            })),
            ..Default::default()
        };

        let msg_id = client.send_message(parsed_jid, msg)
            .await
            .map_err(|e| format!("Failed to send document: {}", e))?;

        info!(%session_id, jid, "Sent document message: {}", msg_id);
        Ok(MediaSendResult { msg_id, direct_path, media_key, file_enc_sha256 })
    }

    /// Send an audio message through a real WhatsApp session
    pub async fn send_audio_message(
        &self,
        session_id: Uuid,
        jid: &str,
        data: Vec<u8>,
        ptt: bool,
    ) -> Result<MediaSendResult, String> {
        let client = self.get_client(session_id)?;

        let parsed_jid: Jid = jid.parse()
            .map_err(|e| format!("Invalid JID: {}", e))?;

        let upload = client.upload(data, wacore::download::MediaType::Audio)
            .await
            .map_err(|e| format!("Failed to upload audio: {}", e))?;

        let direct_path = upload.direct_path.clone();
        let media_key = upload.media_key.clone();
        let file_enc_sha256 = upload.file_enc_sha256.clone();

        let msg = wa::Message {
            audio_message: Some(Box::new(wa::message::AudioMessage {
                url: Some(upload.url),
                direct_path: Some(upload.direct_path),
                media_key: Some(upload.media_key),
                file_enc_sha256: Some(upload.file_enc_sha256),
                file_sha256: Some(upload.file_sha256),
                file_length: Some(upload.file_length),
                mimetype: Some("audio/ogg; codecs=opus".to_string()),
                ptt: Some(ptt),
                ..Default::default()
            })),
            ..Default::default()
        };

        let msg_id = client.send_message(parsed_jid, msg)
            .await
            .map_err(|e| format!("Failed to send audio: {}", e))?;

        info!(%session_id, jid, "Sent audio message: {}", msg_id);
        Ok(MediaSendResult { msg_id, direct_path, media_key, file_enc_sha256 })
    }

    /// Trigger a history sync re-request
    pub async fn trigger_sync(&self, session_id: Uuid) -> Result<(), String> {
        let _client = self.get_client(session_id)?;
        // History sync happens automatically on connection.
        info!(%session_id, "History sync triggered (automatic on connection)");
        Ok(())
    }

    /// Download and decrypt media from WhatsApp CDN
    pub async fn download_media(
        &self,
        session_id: Uuid,
        direct_path: &str,
        media_key: &[u8],
        file_enc_sha256: &[u8],
        media_type: wacore::download::MediaType,
    ) -> Result<Vec<u8>, String> {
        let client = self.get_client(session_id)?;

        // Construct the appropriate protobuf message type to satisfy the Downloadable trait.
        // Each message type (Image, Video, etc.) implements Downloadable with its own MediaType.
        let downloadable: Box<dyn wacore::download::Downloadable> = match media_type {
            wacore::download::MediaType::Image => {
                Box::new(waproto::whatsapp::message::ImageMessage {
                    direct_path: Some(direct_path.to_string()),
                    media_key: Some(media_key.to_vec()),
                    file_enc_sha256: Some(file_enc_sha256.to_vec()),
                    ..Default::default()
                })
            }
            wacore::download::MediaType::Sticker => {
                Box::new(waproto::whatsapp::message::StickerMessage {
                    direct_path: Some(direct_path.to_string()),
                    media_key: Some(media_key.to_vec()),
                    file_enc_sha256: Some(file_enc_sha256.to_vec()),
                    ..Default::default()
                })
            }
            wacore::download::MediaType::Video => {
                Box::new(waproto::whatsapp::message::VideoMessage {
                    direct_path: Some(direct_path.to_string()),
                    media_key: Some(media_key.to_vec()),
                    file_enc_sha256: Some(file_enc_sha256.to_vec()),
                    ..Default::default()
                })
            }
            wacore::download::MediaType::Audio => {
                Box::new(waproto::whatsapp::message::AudioMessage {
                    direct_path: Some(direct_path.to_string()),
                    media_key: Some(media_key.to_vec()),
                    file_enc_sha256: Some(file_enc_sha256.to_vec()),
                    ..Default::default()
                })
            }
            _ => {
                Box::new(waproto::whatsapp::message::DocumentMessage {
                    direct_path: Some(direct_path.to_string()),
                    media_key: Some(media_key.to_vec()),
                    file_enc_sha256: Some(file_enc_sha256.to_vec()),
                    ..Default::default()
                })
            }
        };

        let data = client.download(downloadable.as_ref())
            .await
            .map_err(|e| format!("Failed to download media: {}", e))?;

        info!(%session_id, bytes = data.len(), "Downloaded media");
        Ok(data)
    }

    /// Get the Arc<Client> for a running session
    fn get_client(&self, session_id: Uuid) -> Result<Arc<Client>, String> {
        let entry = self.sessions.get(&session_id)
            .ok_or_else(|| "Session not running".to_string())?;

        entry.client.clone()
            .ok_or_else(|| "Session is connecting but not ready yet".to_string())
    }
}

/// Run a real WhatsApp session using the whatsapp-rust Bot API
async fn run_whatsapp_session(
    manager: WhatsAppManager,
    session_id: Uuid,
    user_id: Uuid,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get the SQLite DB path for this session
    let db_path = {
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT db_path FROM whatsapp_sessions WHERE id = $1",
        )
        .bind(session_id)
        .fetch_one(&manager.db)
        .await?;
        row.0
    };

    // Ensure directory exists
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    info!(%session_id, %user_id, db_path = %db_path, "Starting WhatsApp session");

    // Update status to connecting
    sqlx::query("UPDATE whatsapp_sessions SET status = 'connecting' WHERE id = $1")
        .bind(session_id)
        .execute(&manager.db)
        .await?;

    // Create SQLite backend for this session
    let backend = Arc::new(SqliteStore::new(&db_path).await
        .map_err(|e| format!("Failed to create SQLite store: {}", e))?);

    // Set up shared state for event handling
    let db = manager.db.clone();
    let ws_hub = manager.ws_hub.clone();
    let webhook_client = manager.webhook_client.clone();
    let sessions = manager.sessions.clone();
    let sid = session_id;
    let uid = user_id;

    // Build the WhatsApp Bot
    let mut bot = Bot::builder()
        .with_backend(backend)
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_http_client(UreqHttpClient::new())
        .with_runtime(TokioRuntime)
        .on_event(move |event, client| {
            let db = db.clone();
            let ws_hub = ws_hub.clone();
            let webhook_client = webhook_client.clone();
            let sessions = sessions.clone();
            async move {
                // Store the client reference in SessionInfo on first event
                if let Some(mut entry) = sessions.get_mut(&sid) {
                    if entry.client.is_none() {
                        entry.client = Some(client.clone());
                    }
                }
                // Delegate to event handler
                events::handle_event(event, client, sid, uid, &db, &ws_hub, &webhook_client).await;
            }
        })
        .build()
        .await
        .map_err(|e| format!("Failed to build WhatsApp bot: {}", e))?;

    // Get client reference and store it
    let client = bot.client();
    if let Some(mut entry) = manager.sessions.get_mut(&session_id) {
        entry.client = Some(client.clone());
    }

    // Start the bot
    let bot_handle = bot.run()
        .await
        .map_err(|e| format!("Failed to start WhatsApp bot: {}", e))?;

    info!(%session_id, "WhatsApp bot started, waiting for QR scan...");

    // Wait for either bot completion or shutdown signal
    tokio::select! {
        result = bot_handle => {
            match result {
                Ok(_) => info!(%session_id, "WhatsApp session ended normally"),
                Err(e) => error!(%session_id, "WhatsApp session error: {:?}", e),
            }
        }
        _ = shutdown_rx => {
            info!(%session_id, "Shutdown signal received, disconnecting...");
            client.disconnect().await;
        }
    }

    // Cleanup
    if let Some((_, _info)) = manager.sessions.remove(&session_id) {
        let _ = sqlx::query(
            "UPDATE whatsapp_sessions SET status = 'disconnected' WHERE id = $1",
        )
        .bind(session_id)
        .execute(&manager.db)
        .await;
    }

    info!(%session_id, "WhatsApp session cleaned up");
    Ok(())
}
