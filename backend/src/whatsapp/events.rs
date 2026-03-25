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

use crate::models::display::preferred_display_name;
use crate::webhook::{self, WebhookClient};
use crate::websocket::hub::WebSocketHub;

fn normalize_phone_jid(jid: &impl ToString) -> String {
    let jid = jid.to_string();
    let user = jid.split('@').next().unwrap_or(&jid);
    let user = user.split(':').next().unwrap_or(user);
    if user.starts_with('+') {
        user.to_string()
    } else {
        format!("+{}", user)
    }
}

/// Main event dispatcher — called for every WhatsApp event
pub async fn handle_event(
    event: Event,
    client: Arc<Client>,
    session_id: Uuid,
    user_id: Uuid,
    db: &PgPool,
    ws_hub: &WebSocketHub,
    webhook_client: &WebhookClient,
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

            let phone = normalize_phone_jid(&pair_info.id);
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

            let phone_number = client
                .get_pn()
                .await
                .map(|jid| normalize_phone_jid(&jid));

            let _ = sqlx::query(
                "UPDATE whatsapp_sessions
                 SET status = 'connected',
                     phone_number = COALESCE($2, phone_number),
                     qr_code_data = NULL,
                     last_active_at = NOW()
                 WHERE id = $1",
            )
            .bind(session_id)
            .bind(&phone_number)
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
            let info = extract_message_info(&msg);

            debug!(
                %session_id, %chat_jid, %sender_jid, %message_id,
                msg_type = %info.msg_type, is_from_me, "Incoming message"
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
            let display_msg = info.content.clone().or_else(|| {
                Some(match info.msg_type.as_str() {
                    "image" => "📷 Photo".to_string(),
                    "video" => "🎥 Video".to_string(),
                    "audio" => "🎵 Audio".to_string(),
                    "document" => format!("📄 {}", info.media_filename.as_deref().unwrap_or("Document")),
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

                // Upsert contacts table for push name resolution
                let _ = sqlx::query(
                    "INSERT INTO contacts (id, session_id, jid, push_name, updated_at)
                     VALUES ($1, $2, $3, $4, NOW())
                     ON CONFLICT (session_id, jid) DO UPDATE SET push_name = EXCLUDED.push_name, updated_at = NOW()"
                )
                .bind(Uuid::new_v4())
                .bind(session_id)
                .bind(&sender_jid)
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
                "INSERT INTO messages (id, chat_id, message_id, sender, sender_name, content, message_type, media_url, media_mime_type, media_filename, media_key, direct_path, file_enc_sha256, status, is_from_me, is_forwarded, reply_to_message_id, quote_content, quote_sender, timestamp)
                 VALUES ($1, $2, $3, $4, $5, $6, $7::message_type, $8, $9, $10, $11, $12, $13, 'delivered', $14, $15, $16, $17, $18, $19)
                 ON CONFLICT (chat_id, message_id) DO NOTHING"
            )
            .bind(msg_uuid)
            .bind(chat_id)
            .bind(&message_id)
            .bind(&sender_jid)
            .bind(&sender_name)
            .bind(&info.content)
            .bind(&info.msg_type)
            .bind(&info.media_url)
            .bind(&info.media_mime)
            .bind(&info.media_filename)
            .bind(&info.media_key)
            .bind(&info.direct_path)
            .bind(&info.file_enc_sha256)
            .bind(is_from_me)
            .bind(info.is_forwarded)
            .bind(&info.reply_to_message_id)
            .bind(&info.quote_content)
            .bind(&info.quote_sender)
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

            // Look up sender's phone number from contacts.
            // Important: incoming senders can arrive as device-scoped LID JIDs
            // like "4372444528669:71@lid" while contacts are often stored as
            // "4372444528669@lid". Normalize before matching.
            let sender_phone_number = lookup_contact_phone_number(db, session_id, &sender_jid).await;

            let sender_name = preferred_display_name(
                sender_name.as_deref(),
                sender_phone_number.as_deref(),
            );

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
                        "sender_phone_number": sender_phone_number,
                        "content": info.content,
                        "message_type": info.msg_type,
                        "media_url": info.media_url,
                        "media_mime_type": info.media_mime,
                        "media_filename": info.media_filename,
                        "status": "delivered",
                        "is_from_me": is_from_me,
                        "is_forwarded": info.is_forwarded,
                        "reply_to_message_id": info.reply_to_message_id,
                        "quote_content": info.quote_content,
                        "quote_sender": info.quote_sender,
                        "timestamp": timestamp.to_rfc3339(),
                    }
                }
            }));

            // ── Webhook: dispatch incoming message to external system ──
            if !is_from_me {
                let session_phone = get_session_phone_number(db, session_id).await;
                if let Some(ref phone) = session_phone {
                    let webhook_data = webhook::build_message_payload(
                        &message_id,
                        sender_phone_number.as_deref().unwrap_or(&sender_jid),
                        sender_name.as_deref(),
                        phone,
                        &timestamp,
                        &info.msg_type,
                        info.content.as_deref(),
                        info.media_url.as_deref(),
                        info.media_mime.as_deref(),
                        info.media_filename.as_deref(),
                    );
                    webhook_client.dispatch(session_id, phone, "message", webhook_data);
                }
            }
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

                // ── Webhook: dispatch status update to external system ──
                let session_phone = get_session_phone_number(db, session_id).await;
                if let Some(ref phone) = session_phone {
                    let webhook_data = webhook::build_status_payload(msg_id, status, None, None);
                    webhook_client.dispatch(session_id, phone, "status", webhook_data);
                }
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
                let info =
                    if let Some(msg) = &web_msg_info.message {
                        extract_message_info(msg)
                    } else {
                        MessageInfo {
                            msg_type: "text".to_string(), content: None, media_mime: None,
                            media_url: None, media_filename: None, media_key: None,
                            direct_path: None, file_enc_sha256: None, is_forwarded: false,
                            reply_to_message_id: None, quote_content: None, quote_sender: None,
                        }
                    };

                let insert_result = sqlx::query(
                    "INSERT INTO messages (id, chat_id, message_id, sender, sender_name, content, message_type, media_url, media_mime_type, media_filename, media_key, direct_path, file_enc_sha256, status, is_from_me, is_forwarded, reply_to_message_id, quote_content, quote_sender, timestamp)
                     VALUES ($1, $2, $3, $4, $5, $6, $7::message_type, $8, $9, $10, $11, $12, $13, 'delivered', $14, $15, $16, $17, $18, $19)
                     ON CONFLICT (chat_id, message_id) DO NOTHING"
                )
                .bind(Uuid::new_v4())
                .bind(chat_id)
                .bind(&wa_msg_id)
                .bind(&sender)
                .bind(&push_name)
                .bind(&info.content)
                .bind(&info.msg_type)
                .bind(&info.media_url)
                .bind(&info.media_mime)
                .bind(&info.media_filename)
                .bind(&info.media_key)
                .bind(&info.direct_path)
                .bind(&info.file_enc_sha256)
                .bind(is_from_me)
                .bind(info.is_forwarded)
                .bind(&info.reply_to_message_id)
                .bind(&info.quote_content)
                .bind(&info.quote_sender)
                .bind(timestamp)
                .execute(db)
                .await;

                if let Err(e) = insert_result {
                    debug!(%session_id, "Failed to insert history message: {}", e);
                } else {
                    msg_count += 1;
                }

                // Update chat last_message if this is the latest
                let display_msg = info.content.clone().unwrap_or_else(|| {
                    match info.msg_type.as_str() {
                        "image" => "📷 Photo".to_string(),
                        "video" => "🎥 Video".to_string(),
                        "audio" => "🎵 Audio".to_string(),
                        "document" => format!("📄 {}", info.media_filename.as_deref().unwrap_or("Document")),
                        "sticker" => "🏷️ Sticker".to_string(),
                        "location" => "📍 Location".to_string(),
                        "contact" => "👤 Contact".to_string(),
                        _ => info.msg_type.clone(),
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

            // Upsert contacts table
            let phone = if jid.contains("@s.whatsapp.net") {
                jid.split('@').next().map(|p| format!("+{}", p))
            } else {
                None
            };
            let _ = sqlx::query(
                "INSERT INTO contacts (id, session_id, jid, push_name, phone_number, updated_at)
                 VALUES ($1, $2, $3, $4, $5, NOW())
                 ON CONFLICT (session_id, jid) DO UPDATE SET push_name = EXCLUDED.push_name, phone_number = COALESCE(EXCLUDED.phone_number, contacts.phone_number), updated_at = NOW()"
            )
            .bind(Uuid::new_v4())
            .bind(session_id)
            .bind(&jid)
            .bind(new_name)
            .bind(&phone)
            .execute(db)
            .await;

            let sanitized_name = preferred_display_name(Some(new_name), phone.as_deref());

            // Notify frontend about the contact update
            ws_hub.send_to_user(user_id, serde_json::json!({
                "type": "contacts_updated",
                "data": {
                    "session_id": session_id,
                    "jid": jid,
                    "push_name": sanitized_name,
                    "phone_number": phone,
                }
            }));

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

/// Extracted info from a WhatsApp message
struct MessageInfo {
    msg_type: String,
    content: Option<String>,
    media_mime: Option<String>,
    media_url: Option<String>,
    media_filename: Option<String>,
    media_key: Option<Vec<u8>>,
    direct_path: Option<String>,
    file_enc_sha256: Option<Vec<u8>>,
    is_forwarded: bool,
    reply_to_message_id: Option<String>,
    quote_content: Option<String>,
    quote_sender: Option<String>,
}

/// Helper to extract ContextInfo from various message types
fn extract_context_info(msg: &waproto::whatsapp::Message) -> Option<&waproto::whatsapp::ContextInfo> {
    if let Some(ext) = &msg.extended_text_message {
        return ext.context_info.as_deref();
    }
    if let Some(img) = &msg.image_message {
        return img.context_info.as_deref();
    }
    if let Some(vid) = &msg.video_message {
        return vid.context_info.as_deref();
    }
    if let Some(audio) = &msg.audio_message {
        return audio.context_info.as_deref();
    }
    if let Some(doc) = &msg.document_message {
        return doc.context_info.as_deref();
    }
    if let Some(sticker) = &msg.sticker_message {
        return sticker.context_info.as_deref();
    }
    if let Some(contact) = &msg.contact_message {
        return contact.context_info.as_deref();
    }
    if let Some(loc) = &msg.location_message {
        return loc.context_info.as_deref();
    }
    None
}

/// Extract message type and content from a WhatsApp protobuf message
fn extract_message_info(
    msg: &waproto::whatsapp::Message,
) -> MessageInfo {
    // Extract context info (reply/forward) from all message types
    let ctx = extract_context_info(msg);
    let is_forwarded = ctx.and_then(|c| c.is_forwarded).unwrap_or(false);
    let reply_to_message_id = ctx.and_then(|c| c.stanza_id.clone());
    let quote_sender = ctx.and_then(|c| c.participant.clone());
    let quote_content = ctx.and_then(|c| {
        c.quoted_message.as_ref().and_then(|qm| {
            // Try to get text from quoted message
            qm.conversation.clone()
                .or_else(|| qm.extended_text_message.as_ref().and_then(|e| e.text.clone()))
                .or_else(|| qm.image_message.as_ref().and_then(|i| i.caption.clone()).or(Some("📷 Photo".to_string())))
                .or_else(|| qm.video_message.as_ref().and_then(|v| v.caption.clone()).or(Some("🎥 Video".to_string())))
                .or_else(|| Some("📎 Media".to_string()))
        })
    });

    let base = MessageInfo {
        msg_type: String::new(),
        content: None,
        media_mime: None,
        media_url: None,
        media_filename: None,
        media_key: None,
        direct_path: None,
        file_enc_sha256: None,
        is_forwarded,
        reply_to_message_id,
        quote_content,
        quote_sender,
    };

    // Text message
    if let Some(text) = &msg.conversation {
        return MessageInfo { msg_type: "text".to_string(), content: Some(text.clone()), ..base };
    }

    // Extended text message (links, quoted replies)
    if let Some(ext) = &msg.extended_text_message {
        let text = ext.text.clone();
        return MessageInfo { msg_type: "text".to_string(), content: text, ..base };
    }

    // Image message
    if let Some(img) = &msg.image_message {
        return MessageInfo {
            msg_type: "image".to_string(),
            content: img.caption.clone(),
            media_mime: img.mimetype.clone(),
            media_url: img.url.clone(),
            media_filename: None,
            media_key: img.media_key.clone(),
            direct_path: img.direct_path.clone(),
            file_enc_sha256: img.file_enc_sha256.clone(),
            ..base
        };
    }

    // Video message
    if let Some(vid) = &msg.video_message {
        return MessageInfo {
            msg_type: "video".to_string(),
            content: vid.caption.clone(),
            media_mime: vid.mimetype.clone(),
            media_url: vid.url.clone(),
            media_filename: None,
            media_key: vid.media_key.clone(),
            direct_path: vid.direct_path.clone(),
            file_enc_sha256: vid.file_enc_sha256.clone(),
            ..base
        };
    }

    // Audio message
    if let Some(audio) = &msg.audio_message {
        return MessageInfo {
            msg_type: "audio".to_string(),
            content: None,
            media_mime: audio.mimetype.clone(),
            media_url: audio.url.clone(),
            media_filename: None,
            media_key: audio.media_key.clone(),
            direct_path: audio.direct_path.clone(),
            file_enc_sha256: audio.file_enc_sha256.clone(),
            ..base
        };
    }

    // Document message
    if let Some(doc) = &msg.document_message {
        return MessageInfo {
            msg_type: "document".to_string(),
            content: doc.caption.clone(),
            media_mime: doc.mimetype.clone(),
            media_url: doc.url.clone(),
            media_filename: doc.file_name.clone(),
            media_key: doc.media_key.clone(),
            direct_path: doc.direct_path.clone(),
            file_enc_sha256: doc.file_enc_sha256.clone(),
            ..base
        };
    }

    // Sticker message
    if let Some(sticker) = &msg.sticker_message {
        return MessageInfo {
            msg_type: "sticker".to_string(),
            content: None,
            media_mime: Some("image/webp".to_string()),
            media_url: sticker.url.clone(),
            media_filename: None,
            media_key: sticker.media_key.clone(),
            direct_path: sticker.direct_path.clone(),
            file_enc_sha256: sticker.file_enc_sha256.clone(),
            ..base
        };
    }

    // Contact message
    if let Some(contact) = &msg.contact_message {
        let name = contact.display_name.clone();
        return MessageInfo { msg_type: "contact".to_string(), content: name, ..base };
    }

    // Location message
    if let Some(loc) = &msg.location_message {
        let content = Some(format!(
            "Location: {:.6}, {:.6}",
            loc.degrees_latitude.unwrap_or(0.0),
            loc.degrees_longitude.unwrap_or(0.0),
        ));
        return MessageInfo { msg_type: "location".to_string(), content, ..base };
    }

    // Try text_content() fallback from MessageExt
    if let Some(text) = msg.text_content() {
        return MessageInfo { msg_type: "text".to_string(), content: Some(text.to_string()), ..base };
    }

    // Unknown message type
    MessageInfo { msg_type: "unknown".to_string(), ..base }
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

fn jid_lookup_parts(jid: &str) -> (String, String, String, String) {
    let mut split = jid.splitn(2, '@');
    let user_part = split.next().unwrap_or_default().to_string();
    let domain = split.next().unwrap_or_default().to_string();
    let clean_user_part = user_part.split(':').next().unwrap_or_default().to_string();
    let canonical_jid = if domain.is_empty() {
        clean_user_part.clone()
    } else {
        format!("{}@{}", clean_user_part, domain)
    };

    (user_part, clean_user_part, domain, canonical_jid)
}

async fn lookup_contact_phone_number(
    db: &PgPool,
    session_id: Uuid,
    sender_jid: &str,
) -> Option<String> {
    let (user_part, clean_user_part, _domain, canonical_jid) = jid_lookup_parts(sender_jid);
    let canonical_lid_jid = format!("{}@lid", clean_user_part);

    sqlx::query_scalar(
        "SELECT phone_number
         FROM contacts
         WHERE session_id = $1
           AND phone_number IS NOT NULL
           AND (
               jid = $2
               OR jid = $3
               OR jid = $4
               OR split_part(jid, '@', 1) = $5
               OR split_part(split_part(jid, '@', 1), ':', 1) = $6
           )
         ORDER BY CASE
             WHEN jid = $2 THEN 0
             WHEN jid = $3 THEN 1
             WHEN jid = $4 THEN 2
             WHEN split_part(jid, '@', 1) = $5 THEN 3
             WHEN split_part(split_part(jid, '@', 1), ':', 1) = $6 THEN 4
             ELSE 5
         END
         LIMIT 1",
    )
    .bind(session_id)
    .bind(sender_jid)
    .bind(&canonical_jid)
    .bind(&canonical_lid_jid)
    .bind(&user_part)
    .bind(&clean_user_part)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
}

/// Look up the phone_number (e.g. "6281234567890") for a given session id.
/// Returns `None` when the session doesn't exist or the query fails.
async fn get_session_phone_number(db: &sqlx::PgPool, session_id: uuid::Uuid) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT phone_number FROM whatsapp_sessions WHERE id = $1")
        .bind(session_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── extract_message_info tests ─────────────────────────────

    #[test]
    fn test_extract_text_message() {
        let msg = waproto::whatsapp::Message {
            conversation: Some("Hello world".to_string()),
            ..Default::default()
        };
        let info = extract_message_info(&msg);
        assert_eq!(info.msg_type, "text");
        assert_eq!(info.content.as_deref(), Some("Hello world"));
        assert!(!info.is_forwarded);
        assert!(info.reply_to_message_id.is_none());
    }

    #[test]
    fn test_extract_extended_text_message() {
        let msg = waproto::whatsapp::Message {
            extended_text_message: Some(Box::new(waproto::whatsapp::message::ExtendedTextMessage {
                text: Some("Link message".to_string()),
                ..Default::default()
            })),
            ..Default::default()
        };
        let info = extract_message_info(&msg);
        assert_eq!(info.msg_type, "text");
        assert_eq!(info.content.as_deref(), Some("Link message"));
    }

    #[test]
    fn test_extract_image_message() {
        let msg = waproto::whatsapp::Message {
            image_message: Some(Box::new(waproto::whatsapp::message::ImageMessage {
                caption: Some("A photo".to_string()),
                mimetype: Some("image/jpeg".to_string()),
                url: Some("https://mmg.whatsapp.net/image".to_string()),
                media_key: Some(vec![1, 2, 3]),
                direct_path: Some("/some/path".to_string()),
                file_enc_sha256: Some(vec![4, 5, 6]),
                ..Default::default()
            })),
            ..Default::default()
        };
        let info = extract_message_info(&msg);
        assert_eq!(info.msg_type, "image");
        assert_eq!(info.content.as_deref(), Some("A photo"));
        assert_eq!(info.media_mime.as_deref(), Some("image/jpeg"));
        assert_eq!(info.media_key, Some(vec![1, 2, 3]));
        assert_eq!(info.direct_path.as_deref(), Some("/some/path"));
        assert_eq!(info.file_enc_sha256, Some(vec![4, 5, 6]));
    }

    #[test]
    fn test_extract_video_message() {
        let msg = waproto::whatsapp::Message {
            video_message: Some(Box::new(waproto::whatsapp::message::VideoMessage {
                caption: Some("A video".to_string()),
                mimetype: Some("video/mp4".to_string()),
                media_key: Some(vec![7, 8, 9]),
                direct_path: Some("/video/path".to_string()),
                ..Default::default()
            })),
            ..Default::default()
        };
        let info = extract_message_info(&msg);
        assert_eq!(info.msg_type, "video");
        assert_eq!(info.content.as_deref(), Some("A video"));
        assert_eq!(info.media_mime.as_deref(), Some("video/mp4"));
        assert_eq!(info.media_key, Some(vec![7, 8, 9]));
    }

    #[test]
    fn test_extract_document_message() {
        let msg = waproto::whatsapp::Message {
            document_message: Some(Box::new(waproto::whatsapp::message::DocumentMessage {
                file_name: Some("report.pdf".to_string()),
                mimetype: Some("application/pdf".to_string()),
                ..Default::default()
            })),
            ..Default::default()
        };
        let info = extract_message_info(&msg);
        assert_eq!(info.msg_type, "document");
        assert_eq!(info.media_filename.as_deref(), Some("report.pdf"));
    }

    #[test]
    fn test_extract_sticker_message() {
        let msg = waproto::whatsapp::Message {
            sticker_message: Some(Box::new(waproto::whatsapp::message::StickerMessage {
                media_key: Some(vec![10, 11]),
                direct_path: Some("/sticker/path".to_string()),
                ..Default::default()
            })),
            ..Default::default()
        };
        let info = extract_message_info(&msg);
        assert_eq!(info.msg_type, "sticker");
        assert_eq!(info.media_mime.as_deref(), Some("image/webp"));
    }

    #[test]
    fn test_extract_location_message() {
        let msg = waproto::whatsapp::Message {
            location_message: Some(Box::new(waproto::whatsapp::message::LocationMessage {
                degrees_latitude: Some(-6.2088),
                degrees_longitude: Some(106.8456),
                ..Default::default()
            })),
            ..Default::default()
        };
        let info = extract_message_info(&msg);
        assert_eq!(info.msg_type, "location");
        assert!(info.content.as_ref().unwrap().contains("-6.208800"));
        assert!(info.content.as_ref().unwrap().contains("106.845600"));
    }

    #[test]
    fn test_extract_unknown_message() {
        let msg = waproto::whatsapp::Message::default();
        let info = extract_message_info(&msg);
        assert_eq!(info.msg_type, "unknown");
        assert!(info.content.is_none());
    }

    // ─── Context info (reply/forward) tests ─────────────────────

    #[test]
    fn test_extract_forwarded_message() {
        let msg = waproto::whatsapp::Message {
            extended_text_message: Some(Box::new(waproto::whatsapp::message::ExtendedTextMessage {
                text: Some("Forwarded text".to_string()),
                context_info: Some(Box::new(waproto::whatsapp::ContextInfo {
                    is_forwarded: Some(true),
                    forwarding_score: Some(1),
                    ..Default::default()
                })),
                ..Default::default()
            })),
            ..Default::default()
        };
        let info = extract_message_info(&msg);
        assert!(info.is_forwarded);
        assert_eq!(info.content.as_deref(), Some("Forwarded text"));
    }

    #[test]
    fn test_extract_reply_context() {
        let msg = waproto::whatsapp::Message {
            extended_text_message: Some(Box::new(waproto::whatsapp::message::ExtendedTextMessage {
                text: Some("Reply text".to_string()),
                context_info: Some(Box::new(waproto::whatsapp::ContextInfo {
                    stanza_id: Some("original-msg-123".to_string()),
                    participant: Some("sender@s.whatsapp.net".to_string()),
                    quoted_message: Some(Box::new(waproto::whatsapp::Message {
                        conversation: Some("Original message".to_string()),
                        ..Default::default()
                    })),
                    ..Default::default()
                })),
                ..Default::default()
            })),
            ..Default::default()
        };
        let info = extract_message_info(&msg);
        assert_eq!(info.reply_to_message_id.as_deref(), Some("original-msg-123"));
        assert_eq!(info.quote_sender.as_deref(), Some("sender@s.whatsapp.net"));
        assert_eq!(info.quote_content.as_deref(), Some("Original message"));
    }

    #[test]
    fn test_extract_reply_to_image_shows_photo_label() {
        let msg = waproto::whatsapp::Message {
            extended_text_message: Some(Box::new(waproto::whatsapp::message::ExtendedTextMessage {
                text: Some("My reply".to_string()),
                context_info: Some(Box::new(waproto::whatsapp::ContextInfo {
                    stanza_id: Some("img-msg-123".to_string()),
                    quoted_message: Some(Box::new(waproto::whatsapp::Message {
                        image_message: Some(Box::new(waproto::whatsapp::message::ImageMessage {
                            caption: None,
                            ..Default::default()
                        })),
                        ..Default::default()
                    })),
                    ..Default::default()
                })),
                ..Default::default()
            })),
            ..Default::default()
        };
        let info = extract_message_info(&msg);
        assert_eq!(info.quote_content.as_deref(), Some("📷 Photo"));
    }

    // ─── is_meaningful_name tests ───────────────────────────────

    #[test]
    fn test_meaningful_name_real_name() {
        assert!(is_meaningful_name("John Doe", "6281380888035@s.whatsapp.net"));
    }

    #[test]
    fn test_meaningful_name_same_as_jid() {
        assert!(!is_meaningful_name("6281380888035@s.whatsapp.net", "6281380888035@s.whatsapp.net"));
    }

    #[test]
    fn test_meaningful_name_pure_digits_short() {
        // Phone-number-length digits (10-15) are still considered "meaningful"
        // because they could be a push name that happens to be digits
        assert!(is_meaningful_name("6281380888035", "6281380888035@s.whatsapp.net"));
    }

    #[test]
    fn test_meaningful_name_lid_pattern_long() {
        // >15 digit pure numeric strings are NOT meaningful (LID numbers)
        assert!(!is_meaningful_name("1496153703385451234", "1496153703385451234@lid"));
    }

    #[test]
    fn test_meaningful_name_lid_jid() {
        // LID JIDs in name are not meaningful
        assert!(!is_meaningful_name("149615370338545@lid", "149615370338545@lid"));
    }
}
