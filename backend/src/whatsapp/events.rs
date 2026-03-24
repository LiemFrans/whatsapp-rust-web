//! Event handling for real WhatsApp events from whatsapp-rust.
//!
//! Processes QR codes, connection events, incoming messages, read receipts,
//! and history sync. Persists data to PostgreSQL and broadcasts updates
//! via the WebSocket hub.

use std::sync::Arc;

use sqlx::PgPool;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use whatsapp_rust::client::Client;
use whatsapp_rust::proto_helpers::MessageExt;
use wacore::types::events::Event;

use crate::websocket::hub::WebSocketHub;

/// Main event dispatcher — called for every WhatsApp event
pub async fn handle_event(
    event: Event,
    _client: Arc<Client>,
    session_id: Uuid,
    user_id: Uuid,
    db: &PgPool,
    ws_hub: &WebSocketHub,
) {
    match event {
        // ── QR Code Pairing ─────────────────────────────────────────
        Event::PairingQrCode { code, timeout } => {
            info!(%session_id, timeout_secs = timeout.as_secs(), "QR code received");

            // Update session status to qr_code
            let _ = sqlx::query(
                "UPDATE whatsapp_sessions SET status = 'qr_code', qr_code_data = $2 WHERE id = $1",
            )
            .bind(session_id)
            .bind(&code)
            .execute(db)
            .await;

            // Send QR code to user via WebSocket
            ws_hub.send_to_user(user_id, serde_json::json!({
                "type": "qr_code",
                "data": {
                    "session_id": session_id,
                    "qr_code": code,
                    "timeout_secs": timeout.as_secs(),
                }
            }));
        }

        // ── Pair Code (phone number pairing) ────────────────────────
        Event::PairingCode { code, timeout } => {
            info!(%session_id, "Pair code received: {}", code);

            ws_hub.send_to_user(user_id, serde_json::json!({
                "type": "pair_code",
                "data": {
                    "session_id": session_id,
                    "code": code,
                    "timeout_secs": timeout.as_secs(),
                }
            }));
        }

        // ── Pair Success ────────────────────────────────────────────
        Event::PairSuccess(pair_info) => {
            info!(%session_id, "Pairing successful: {:?}", pair_info.id);

            let phone = pair_info.id.to_string();
            let _ = sqlx::query(
                "UPDATE whatsapp_sessions SET status = 'connected', phone_number = $2, qr_code_data = NULL, last_active_at = NOW() WHERE id = $1",
            )
            .bind(session_id)
            .bind(&phone)
            .execute(db)
            .await;
        }

        // ── Connected ───────────────────────────────────────────────
        Event::Connected(_) => {
            info!(%session_id, "WhatsApp session connected");

            let _ = sqlx::query(
                "UPDATE whatsapp_sessions SET status = 'connected', qr_code_data = NULL, last_active_at = NOW() WHERE id = $1",
            )
            .bind(session_id)
            .execute(db)
            .await;

            ws_hub.send_to_user(user_id, serde_json::json!({
                "type": "session_connected",
                "data": {
                    "session_id": session_id,
                }
            }));
        }

        // ── Disconnected ────────────────────────────────────────────
        Event::Disconnected(_) => {
            warn!(%session_id, "WhatsApp session disconnected");

            ws_hub.send_to_user(user_id, serde_json::json!({
                "type": "session_disconnected",
                "data": {
                    "session_id": session_id,
                }
            }));
        }

        // ── Logged Out ──────────────────────────────────────────────
        Event::LoggedOut(_) => {
            error!(%session_id, "WhatsApp session logged out");

            let _ = sqlx::query(
                "UPDATE whatsapp_sessions SET status = 'disconnected' WHERE id = $1",
            )
            .bind(session_id)
            .execute(db)
            .await;

            ws_hub.send_to_user(user_id, serde_json::json!({
                "type": "session_logged_out",
                "data": {
                    "session_id": session_id,
                }
            }));
        }

        // ── Incoming Message ────────────────────────────────────────
        Event::Message(msg, msg_info) => {
            let chat_jid = msg_info.source.chat.to_string();
            let sender_jid = msg_info.source.sender.to_string();
            let message_id = msg_info.id.clone();
            let is_from_me = msg_info.source.is_from_me;
            let is_group = msg_info.source.is_group;
            let timestamp = msg_info.timestamp;

            // Determine message type and content
            let (msg_type, content, media_mime, media_url, media_filename) = extract_message_info(&msg);

            debug!(
                %session_id, %chat_jid, %sender_jid, %message_id,
                msg_type = %msg_type, is_from_me, "Incoming message"
            );

            // Upsert the chat
            let chat_name = if is_group {
                // For groups, use chat JID (group name will come from other events)
                chat_jid.clone()
            } else if is_from_me {
                // For outgoing messages, use chat JID as name (receiver)
                chat_jid.clone()
            } else {
                // For incoming messages, use push name if available
                let pn = &msg_info.push_name;
                if pn.is_empty() { chat_jid.clone() } else { pn.clone() }
            };

            // Build display text for chat's last_message (include type label for media)
            let display_msg = content.clone().or_else(|| {
                Some(match msg_type.as_str() {
                    "image" => "📷 Photo".to_string(),
                    "video" => "🎥 Video".to_string(),
                    "audio" => "🎵 Audio".to_string(),
                    "document" => format!("📄 {}", media_filename.as_deref().unwrap_or("Document")),
                    "sticker" => "🏷️ Sticker".to_string(),
                    "location" => "📍 Location".to_string(),
                    "contact" => "👤 Contact".to_string(),
                    _ => return None,
                })
            });

            let chat_id = upsert_chat(
                db,
                session_id,
                &chat_jid,
                &chat_name,
                is_group,
                display_msg.as_deref(),
                timestamp,
            ).await;

            let Some(chat_id) = chat_id else {
                error!(%session_id, %chat_jid, "Failed to upsert chat");
                return;
            };

            // If this is a non-group incoming message with a push name,
            // also explicitly update the chat name if it's currently a raw LID/JID
            if !is_group && !is_from_me && !msg_info.push_name.is_empty() {
                let _ = sqlx::query(
                    "UPDATE chats SET name = $2 WHERE id = $1 AND (name IS NULL OR name = chat_jid OR name LIKE '%@lid' OR name LIKE '%@s.whatsapp.net' OR name LIKE '+%∙%')",
                )
                .bind(chat_id)
                .bind(&msg_info.push_name)
                .execute(db)
                .await;
            }

            // Insert the message
            let sender_name: Option<String> = if msg_info.push_name.is_empty() {
                None
            } else {
                Some(msg_info.push_name.clone())
            };
            let msg_uuid = Uuid::new_v4();

            let insert_result = sqlx::query(
                "INSERT INTO messages (id, chat_id, message_id, sender, sender_name, content, message_type, media_url, media_mime_type, media_filename, status, is_from_me, timestamp)
                 VALUES ($1, $2, $3, $4, $5, $6, $7::message_type, $8, $9, $10, 'delivered', $11, $12)
                 ON CONFLICT (chat_id, message_id) DO NOTHING"
            )
            .bind(msg_uuid)
            .bind(chat_id)
            .bind(&message_id)
            .bind(&sender_jid)
            .bind(&sender_name)
            .bind(&content)
            .bind(&msg_type)
            .bind(&media_url)
            .bind(&media_mime)
            .bind(&media_filename)
            .bind(is_from_me)
            .bind(timestamp)
            .execute(db)
            .await;

            if let Err(e) = insert_result {
                error!(%session_id, "Failed to insert message: {}", e);
                return;
            }

            // Update unread count if not from me
            if !is_from_me {
                let _ = sqlx::query(
                    "UPDATE chats SET unread_count = unread_count + 1 WHERE id = $1",
                )
                .bind(chat_id)
                .execute(db)
                .await;
            }

            // Broadcast new message via WebSocket
            ws_hub.send_to_user(user_id, serde_json::json!({
                "type": "new_message",
                "data": {
                    "chat_id": chat_id,
                    "message": {
                        "id": msg_uuid,
                        "chat_id": chat_id,
                        "message_id": message_id,
                        "sender": sender_jid,
                        "sender_name": sender_name,
                        "content": content,
                        "message_type": msg_type,
                        "media_url": media_url,
                        "media_mime_type": media_mime,
                        "media_filename": media_filename,
                        "status": "delivered",
                        "is_from_me": is_from_me,
                        "timestamp": timestamp.to_rfc3339(),
                    }
                }
            }));
        }

        // ── Read Receipts ───────────────────────────────────────────
        Event::Receipt(receipt) => {
            debug!(%session_id, "Receipt event: {:?}", receipt);

            // Update message status based on receipt type
            for msg_id in &receipt.message_ids {
                let status = match receipt.r#type {
                    wacore::types::presence::ReceiptType::Read
                    | wacore::types::presence::ReceiptType::ReadSelf => "read",
                    wacore::types::presence::ReceiptType::Delivered => "delivered",
                    _ => continue,
                };

                let _ = sqlx::query(
                    "UPDATE messages SET status = $2::message_status WHERE message_id = $1",
                )
                .bind(msg_id)
                .bind(status)
                .execute(db)
                .await;

                ws_hub.send_to_user(user_id, serde_json::json!({
                    "type": "message_status",
                    "data": {
                        "message_id": msg_id,
                        "status": status,
                    }
                }));
            }
        }

        // ── History Sync: per-conversation via JoinedGroup ─────────
        // The whatsapp-rust library dispatches Event::JoinedGroup for
        // each conversation during history sync (NOT Event::HistorySync).
        Event::JoinedGroup(lazy_conv) => {
            // Parse the conversation WITH messages included
            let Some(conv) = lazy_conv.get_with_messages() else {
                debug!(%session_id, "JoinedGroup: failed to parse conversation");
                return;
            };

            let conv_jid = conv.id.clone();
            if conv_jid.is_empty() {
                return;
            }

            let conv_name = conv.display_name.clone()
                .or_else(|| conv.name.clone())
                .unwrap_or_else(|| conv_jid.clone());

            let is_group = conv_jid.ends_with("@g.us");

            // Upsert chat
            let chat_id = upsert_chat(
                db,
                session_id,
                &conv_jid,
                &conv_name,
                is_group,
                None,
                chrono::Utc::now(),
            ).await;

            let Some(chat_id) = chat_id else {
                error!(%session_id, %conv_jid, "Failed to upsert chat from history sync");
                return;
            };

            let mut msg_count = 0u32;

            // Process messages in conversation
            for hist_msg in &conv.messages {
                let Some(web_msg_info) = &hist_msg.message else {
                    continue;
                };

                let wa_msg_id = web_msg_info.key.id.clone()
                    .unwrap_or_default();
                if wa_msg_id.is_empty() {
                    continue;
                }

                let is_from_me = web_msg_info.key.from_me
                    .unwrap_or(false);

                let sender = if is_from_me {
                    "me".to_string()
                } else if is_group {
                    // For group messages, use participant (the actual sender)
                    // Fallback to remote_jid only if participant is missing
                    let participant = web_msg_info.key.participant.clone().unwrap_or_default();
                    if participant.is_empty() {
                        web_msg_info.key.remote_jid.clone().unwrap_or_default()
                    } else {
                        participant
                    }
                } else {
                    web_msg_info.key.remote_jid.clone()
                        .or_else(|| web_msg_info.key.participant.clone())
                        .unwrap_or_default()
                };

                let push_name = web_msg_info.push_name.clone();

                let ts_secs = web_msg_info.message_timestamp.unwrap_or(0) as i64;
                let timestamp = chrono::DateTime::from_timestamp(ts_secs, 0)
                    .unwrap_or_else(chrono::Utc::now);

                // Extract message content
                let (msg_type, content, media_mime, media_url, media_filename) =
                    if let Some(msg) = &web_msg_info.message {
                        extract_message_info(msg)
                    } else {
                        ("text".to_string(), None, None, None, None)
                    };

                let insert_result = sqlx::query(
                    "INSERT INTO messages (id, chat_id, message_id, sender, sender_name, content, message_type, media_url, media_mime_type, media_filename, status, is_from_me, timestamp)
                     VALUES ($1, $2, $3, $4, $5, $6, $7::message_type, $8, $9, $10, 'delivered', $11, $12)
                     ON CONFLICT (chat_id, message_id) DO NOTHING"
                )
                .bind(Uuid::new_v4())
                .bind(chat_id)
                .bind(&wa_msg_id)
                .bind(&sender)
                .bind(&push_name)
                .bind(&content)
                .bind(&msg_type)
                .bind(&media_url)
                .bind(&media_mime)
                .bind(&media_filename)
                .bind(is_from_me)
                .bind(timestamp)
                .execute(db)
                .await;

                if let Err(e) = insert_result {
                    debug!(%session_id, "Failed to insert history message: {}", e);
                } else {
                    msg_count += 1;
                }

                // Update chat last_message if this is the latest
                let display_msg = content.clone().unwrap_or_else(|| {
                    match msg_type.as_str() {
                        "image" => "📷 Photo".to_string(),
                        "video" => "🎥 Video".to_string(),
                        "audio" => "🎵 Audio".to_string(),
                        "document" => format!("📄 {}", media_filename.as_deref().unwrap_or("Document")),
                        "sticker" => "🏷️ Sticker".to_string(),
                        "location" => "📍 Location".to_string(),
                        "contact" => "👤 Contact".to_string(),
                        _ => msg_type.clone(),
                    }
                });
                let _ = sqlx::query(
                    "UPDATE chats SET last_message = $2, last_message_at = $3
                     WHERE id = $1 AND (last_message_at IS NULL OR last_message_at < $3)",
                )
                .bind(chat_id)
                .bind(&display_msg)
                .bind(timestamp)
                .execute(db)
                .await;
            }

            info!(
                %session_id, %conv_jid, name = %conv_name,
                messages = msg_count, "History sync: conversation saved"
            );

            // Notify frontend about the new/updated chat
            ws_hub.send_to_user(user_id, serde_json::json!({
                "type": "chats_updated",
                "data": {
                    "session_id": session_id,
                    "chat_jid": conv_jid,
                    "chat_name": conv_name,
                    "messages": msg_count,
                }
            }));
        }

        // ── History Sync (raw protobuf — rarely fired directly) ─────
        Event::HistorySync(_history) => {
            debug!(%session_id, "HistorySync event received (raw protobuf)");
        }

        // ── Offline Sync Completed ──────────────────────────────────
        Event::OfflineSyncCompleted(_) => {
            info!(%session_id, "Offline sync completed");

            ws_hub.send_to_user(user_id, serde_json::json!({
                "type": "sync_progress",
                "data": {
                    "session_id": session_id,
                    "status": "completed",
                }
            }));

            // Also trigger a full chat refresh notification
            ws_hub.send_to_user(user_id, serde_json::json!({
                "type": "chats_updated",
                "data": {
                    "session_id": session_id,
                    "reason": "sync_completed",
                }
            }));
        }

        // ── Offline Sync Preview ────────────────────────────────────
        Event::OfflineSyncPreview(_) => {
            debug!(%session_id, "Offline sync preview received");
        }

        // ── Push Name Updates ───────────────────────────────────────
        Event::PushNameUpdate(update) => {
            let jid = update.jid.to_string();
            let new_name = &update.new_push_name;
            info!(%session_id, %jid, name = %new_name, "Push name update");

            // Update chat name for exact JID match
            let result = sqlx::query(
                "UPDATE chats SET name = $2 WHERE session_id = $1 AND chat_jid = $3 AND (name IS NULL OR name = chat_jid OR name LIKE '%@lid' OR name LIKE '+%∙%')",
            )
            .bind(session_id)
            .bind(new_name)
            .bind(&jid)
            .execute(db)
            .await;

            if let Ok(r) = &result {
                if r.rows_affected() > 0 {
                    info!(%session_id, %jid, name = %new_name, "Chat name updated from push name");
                    ws_hub.send_to_user(user_id, serde_json::json!({
                        "type": "chats_updated",
                        "data": { "session_id": session_id, "reason": "push_name_update" }
                    }));
                }
            }

            // Also try matching by user part only (strip @server)
            if let Some(user_part) = jid.split('@').next() {
                let lid_jid = format!("{}@lid", user_part);
                if lid_jid != jid {
                    let _ = sqlx::query(
                        "UPDATE chats SET name = $2 WHERE session_id = $1 AND chat_jid = $3 AND (name IS NULL OR name = chat_jid OR name LIKE '%@lid' OR name LIKE '+%∙%')",
                    )
                    .bind(session_id)
                    .bind(new_name)
                    .bind(&lid_jid)
                    .execute(db)
                    .await;
                }
            }
        }

        // ── Temporary Ban ───────────────────────────────────────────
        Event::TemporaryBan(ban) => {
            error!(%session_id, "Temporary ban: {:?}", ban);
            ws_hub.send_to_user(user_id, serde_json::json!({
                "type": "session_error",
                "data": {
                    "session_id": session_id,
                    "error": "Account temporarily banned by WhatsApp",
                }
            }));
        }

        // ── Stream Error ────────────────────────────────────────────
        Event::StreamError(err) => {
            error!(%session_id, "Stream error: {:?}", err);
        }

        // ── Other events (logged at debug level) ────────────────────
        _ => {
            debug!(%session_id, "Unhandled WhatsApp event");
        }
    }
}

/// Extract message type and content from a WhatsApp protobuf message
fn extract_message_info(
    msg: &waproto::whatsapp::Message,
) -> (String, Option<String>, Option<String>, Option<String>, Option<String>) {
    // Text message
    if let Some(text) = &msg.conversation {
        return ("text".to_string(), Some(text.clone()), None, None, None);
    }

    // Extended text message (links, quoted replies)
    if let Some(ext) = &msg.extended_text_message {
        let text = ext.text.clone();
        return ("text".to_string(), text, None, None, None);
    }

    // Image message
    if let Some(img) = &msg.image_message {
        let caption = img.caption.clone();
        let mime = img.mimetype.clone();
        let url = img.url.clone();
        return ("image".to_string(), caption, mime, url, None);
    }

    // Video message
    if let Some(vid) = &msg.video_message {
        let caption = vid.caption.clone();
        let mime = vid.mimetype.clone();
        let url = vid.url.clone();
        return ("video".to_string(), caption, mime, url, None);
    }

    // Audio message
    if let Some(audio) = &msg.audio_message {
        let mime = audio.mimetype.clone();
        let url = audio.url.clone();
        return ("audio".to_string(), None, mime, url, None);
    }

    // Document message
    if let Some(doc) = &msg.document_message {
        let filename = doc.file_name.clone();
        let mime = doc.mimetype.clone();
        let url = doc.url.clone();
        let caption = doc.caption.clone();
        return ("document".to_string(), caption, mime, url, filename);
    }

    // Sticker message
    if let Some(_sticker) = &msg.sticker_message {
        return ("sticker".to_string(), None, Some("image/webp".to_string()), None, None);
    }

    // Contact message
    if let Some(contact) = &msg.contact_message {
        let name = contact.display_name.clone();
        return ("contact".to_string(), name, None, None, None);
    }

    // Location message
    if let Some(loc) = &msg.location_message {
        let content = Some(format!(
            "Location: {:.6}, {:.6}",
            loc.degrees_latitude.unwrap_or(0.0),
            loc.degrees_longitude.unwrap_or(0.0),
        ));
        return ("location".to_string(), content, None, None, None);
    }

    // Try text_content() fallback from MessageExt
    if let Some(text) = msg.text_content() {
        return ("text".to_string(), Some(text.to_string()), None::<String>, None, None);
    }

    // Unknown message type
    ("unknown".to_string(), None, None, None, None)
}

/// Check if a name is a "real" display name (not a raw JID/LID identifier)
fn is_meaningful_name(name: &str, chat_jid: &str) -> bool {
    // If name equals the JID itself, it's not meaningful
    if name == chat_jid {
        return false;
    }
    // If name looks like a raw LID/JID (contains @lid or @g.us or @s.whatsapp.net)
    if name.contains("@lid") || name.contains("@g.us") || name.contains("@s.whatsapp.net") {
        return false;
    }
    // Pure numeric strings longer than 15 chars are likely LID numbers
    if name.chars().all(|c| c.is_ascii_digit()) && name.len() > 15 {
        return false;
    }
    !name.is_empty()
}

/// Upsert a chat in the database, returning the chat UUID
async fn upsert_chat(
    db: &PgPool,
    session_id: Uuid,
    chat_jid: &str,
    name: &str,
    is_group: bool,
    last_message: Option<&str>,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> Option<Uuid> {
    // Extract phone number from JID
    // Only for @s.whatsapp.net addresses (real phone numbers)
    // NOT for @lid addresses (internal WhatsApp Local IDs)
    let phone_number = if !is_group && chat_jid.contains("@s.whatsapp.net") {
        chat_jid.split('@').next().map(|p| format!("+{}", p))
    } else {
        None
    };

    // Only pass the name to SQL if it's meaningful
    let effective_name: Option<&str> = if is_meaningful_name(name, chat_jid) {
        Some(name)
    } else {
        None
    };

    let result = sqlx::query_as::<_, (Uuid,)>(
        "INSERT INTO chats (id, session_id, chat_jid, name, phone_number, is_group, last_message, last_message_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (session_id, chat_jid) DO UPDATE SET
             name = COALESCE(EXCLUDED.name, chats.name),
             phone_number = COALESCE(EXCLUDED.phone_number, chats.phone_number),
             last_message = COALESCE(EXCLUDED.last_message, chats.last_message),
             last_message_at = GREATEST(EXCLUDED.last_message_at, chats.last_message_at)
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(session_id)
    .bind(chat_jid)
    .bind(effective_name)
    .bind(&phone_number)
    .bind(is_group)
    .bind(last_message)
    .bind(timestamp)
    .fetch_one(db)
    .await;

    match result {
        Ok((id,)) => Some(id),
        Err(e) => {
            error!("Failed to upsert chat {}: {}", chat_jid, e);
            None
        }
    }
}
