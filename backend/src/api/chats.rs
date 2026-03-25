use axum::{
    extract::{Path, Query, State, Multipart},
    routing::{get, post},
    Json, Router,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::auth::middleware::{AuthUser, require_scope};
use crate::models::chat::*;
use crate::models::message::*;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_chats))
        .route("/contacts", get(list_contacts))
        .route("/:chat_id/messages", get(list_messages))
        .route("/:chat_id/messages/:message_id/media", get(get_media))
        .route("/:chat_id/send", post(send_message))
        .route("/:chat_id/send-media", post(send_media_message))
        .route("/:chat_id/read", post(mark_as_read))
        .route("/sync", post(trigger_sync))
}

async fn list_chats(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ChatListQuery>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth, "chats:read")?;
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).min(100);
    let offset = (page - 1) * per_page;

    let chats = if let Some(search) = &query.search {
        let search_pattern = format!("%{}%", search);
        sqlx::query_as::<_, Chat>(
            "SELECT * FROM chats
             WHERE ($1::uuid IS NULL OR session_id = $1)
             AND (name ILIKE $2 OR phone_number ILIKE $2 OR chat_jid ILIKE $2)
             ORDER BY COALESCE(last_message_at, created_at) DESC
             LIMIT $3 OFFSET $4",
        )
        .bind(query.session_id)
        .bind(&search_pattern)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, Chat>(
            "SELECT * FROM chats
             WHERE ($1::uuid IS NULL OR session_id = $1)
             ORDER BY COALESCE(last_message_at, created_at) DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(query.session_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.db)
        .await
    };

    let chats = chats.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?
    .into_iter()
    .map(Chat::sanitized)
    .collect::<Vec<_>>();

    Ok(Json(serde_json::json!({ "chats": chats })))
}

async fn list_contacts(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ContactsQuery>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth, "chats:read")?;
    let contacts = sqlx::query_as::<_, Contact>(
        "SELECT * FROM contacts WHERE ($1::uuid IS NULL OR session_id = $1) ORDER BY push_name ASC NULLS LAST",
    )
    .bind(query.session_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?
    .into_iter()
    .map(Contact::sanitized)
    .collect::<Vec<_>>();

    Ok(Json(serde_json::json!({ "contacts": contacts })))
}

async fn list_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(chat_id): Path<Uuid>,
    Query(query): Query<MessageListQuery>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth, "chats:read")?;
    let limit = query.limit.unwrap_or(50).min(100);

    let messages = if let Some(cursor) = &query.cursor {
        let cursor_time: chrono::DateTime<chrono::Utc> = cursor
            .parse()
            .map_err(|_| {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": "Invalid cursor" })),
                )
            })?;
        sqlx::query_as::<_, Message>(
                        "SELECT m.*, contact_match.phone_number AS sender_phone_number
                         FROM messages m
                         JOIN chats ch ON ch.id = m.chat_id
                         LEFT JOIN LATERAL (
                             SELECT c.phone_number
                             FROM contacts c
                             WHERE c.session_id = ch.session_id
                                 AND c.phone_number IS NOT NULL
                                 AND (
                                     c.jid = m.sender
                                     OR c.jid = CONCAT(split_part(split_part(m.sender, '@', 1), ':', 1), '@', split_part(m.sender, '@', 2))
                                     OR c.jid = CONCAT(split_part(split_part(m.sender, '@', 1), ':', 1), '@lid')
                                     OR split_part(c.jid, '@', 1) = split_part(m.sender, '@', 1)
                                     OR split_part(split_part(c.jid, '@', 1), ':', 1) = split_part(split_part(m.sender, '@', 1), ':', 1)
                                 )
                             ORDER BY CASE
                                 WHEN c.jid = m.sender THEN 0
                                 WHEN c.jid = CONCAT(split_part(split_part(m.sender, '@', 1), ':', 1), '@', split_part(m.sender, '@', 2)) THEN 1
                                 WHEN c.jid = CONCAT(split_part(split_part(m.sender, '@', 1), ':', 1), '@lid') THEN 2
                                 WHEN split_part(c.jid, '@', 1) = split_part(m.sender, '@', 1) THEN 3
                                 WHEN split_part(split_part(c.jid, '@', 1), ':', 1) = split_part(split_part(m.sender, '@', 1), ':', 1) THEN 4
                                 ELSE 5
                             END
                             LIMIT 1
                         ) AS contact_match ON TRUE
             WHERE m.chat_id = $1 AND m.timestamp < $2
             ORDER BY m.timestamp DESC LIMIT $3",
        )
        .bind(chat_id)
        .bind(cursor_time)
        .bind(limit)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, Message>(
                        "SELECT m.*, contact_match.phone_number AS sender_phone_number
                         FROM messages m
                         JOIN chats ch ON ch.id = m.chat_id
                         LEFT JOIN LATERAL (
                             SELECT c.phone_number
                             FROM contacts c
                             WHERE c.session_id = ch.session_id
                                 AND c.phone_number IS NOT NULL
                                 AND (
                                     c.jid = m.sender
                                     OR c.jid = CONCAT(split_part(split_part(m.sender, '@', 1), ':', 1), '@', split_part(m.sender, '@', 2))
                                     OR c.jid = CONCAT(split_part(split_part(m.sender, '@', 1), ':', 1), '@lid')
                                     OR split_part(c.jid, '@', 1) = split_part(m.sender, '@', 1)
                                     OR split_part(split_part(c.jid, '@', 1), ':', 1) = split_part(split_part(m.sender, '@', 1), ':', 1)
                                 )
                             ORDER BY CASE
                                 WHEN c.jid = m.sender THEN 0
                                 WHEN c.jid = CONCAT(split_part(split_part(m.sender, '@', 1), ':', 1), '@', split_part(m.sender, '@', 2)) THEN 1
                                 WHEN c.jid = CONCAT(split_part(split_part(m.sender, '@', 1), ':', 1), '@lid') THEN 2
                                 WHEN split_part(c.jid, '@', 1) = split_part(m.sender, '@', 1) THEN 3
                                 WHEN split_part(split_part(c.jid, '@', 1), ':', 1) = split_part(split_part(m.sender, '@', 1), ':', 1) THEN 4
                                 ELSE 5
                             END
                             LIMIT 1
                         ) AS contact_match ON TRUE
             WHERE m.chat_id = $1
             ORDER BY m.timestamp DESC LIMIT $2",
        )
        .bind(chat_id)
        .bind(limit)
        .fetch_all(&state.db)
        .await
    };

    let mut messages = messages.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?
    .into_iter()
    .map(Message::sanitized)
    .collect::<Vec<_>>();

    let has_more = messages.len() as i64 == limit;
    messages.reverse(); // Return in chronological order

    Ok(Json(serde_json::json!({
        "messages": messages,
        "has_more": has_more,
    })))
}

/// Proxy endpoint to download and serve WhatsApp media
/// The media is encrypted on WhatsApp CDN and needs decryption keys from DB
async fn get_media(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((chat_id, message_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth, "chats:read")?;
    // Get message with media info
    let msg = sqlx::query_as::<_, Message>("SELECT * FROM messages WHERE id = $1 AND chat_id = $2")
        .bind(message_id)
        .bind(chat_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Message not found" })),
            )
        })?;

    // Get the chat to find session_id
    let chat = sqlx::query_as::<_, crate::models::chat::Chat>("SELECT * FROM chats WHERE id = $1")
        .bind(chat_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Chat not found" })),
            )
        })?;

    // We need direct_path, media_key, file_enc_sha256 to download
    let direct_path = msg.direct_path.ok_or_else(|| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "No media download info available" })),
        )
    })?;
    let media_key = msg.media_key.ok_or_else(|| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "No media key available" })),
        )
    })?;
    let file_enc_sha256 = msg.file_enc_sha256.unwrap_or_default();

    // Determine media type for decryption
    let media_type = match msg.message_type {
        MessageType::Image => wacore::download::MediaType::Image,
        MessageType::Video => wacore::download::MediaType::Video,
        MessageType::Audio => wacore::download::MediaType::Audio,
        MessageType::Document => wacore::download::MediaType::Document,
        MessageType::Sticker => wacore::download::MediaType::Sticker,
        _ => wacore::download::MediaType::Document,
    };

    // Download media via WhatsApp manager
    let data = state
        .wa_manager
        .download_media(chat.session_id, &direct_path, &media_key, &file_enc_sha256, media_type)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    let content_type = msg.media_mime_type.unwrap_or_else(|| "application/octet-stream".to_string());

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        content_type.parse().unwrap_or_else(|_| "application/octet-stream".parse().unwrap()),
    );
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        "public, max-age=86400".parse().unwrap(),
    );
    if let Some(filename) = &msg.media_filename {
        headers.insert(
            axum::http::header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", filename).parse().unwrap_or_else(|_| "inline".parse().unwrap()),
        );
    }

    Ok((headers, data))
}

async fn send_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(chat_id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth, "chats:write")?;
    // Get chat info
    let chat = sqlx::query_as::<_, Chat>("SELECT * FROM chats WHERE id = $1")
        .bind(chat_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Chat not found" })),
            )
        })?;

    // Send via WhatsApp (with reply context if provided)
    let reply_to = req.reply_to.as_deref();
    let msg_id = if reply_to.is_some() {
        // Look up the original message's sender for reply context
        let reply_sender = if let Some(reply_msg_id) = reply_to {
            sqlx::query_as::<_, (String,)>(
                "SELECT sender FROM messages WHERE message_id = $1 AND chat_id = $2 LIMIT 1"
            )
            .bind(reply_msg_id)
            .bind(chat_id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .map(|(s,)| s)
        } else {
            None
        };
        state
            .wa_manager
            .send_text_message_with_reply(
                chat.session_id,
                &chat.chat_jid,
                &req.content,
                reply_to,
                reply_sender.as_deref(),
            )
            .await
    } else {
        state
            .wa_manager
            .send_text_message(chat.session_id, &chat.chat_jid, &req.content)
            .await
    }.map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    // Save to DB
    let message = sqlx::query_as::<_, Message>(
        "INSERT INTO messages (id, chat_id, message_id, sender, sender_name, content, message_type, status, is_from_me, reply_to_message_id, timestamp)
         VALUES ($1, $2, $3, $4, $5, $6, 'text', 'sent', true, $7, NOW()) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(chat_id)
    .bind(&msg_id)
    .bind(&auth.username)
    .bind(&auth.username)
    .bind(&req.content)
    .bind(reply_to)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?
    .sanitized();

    // Update chat last_message
    sqlx::query("UPDATE chats SET last_message = $1, last_message_at = NOW() WHERE id = $2")
        .bind(&req.content)
        .bind(chat_id)
        .execute(&state.db)
        .await
        .ok();

    // Broadcast via WebSocket
    state
        .ws_hub
        .send_to_user(
            auth.user_id,
            serde_json::json!({
                "type": "message_sent",
                "data": { "message": &message, "chat_id": chat_id }
            }),
        );

    Ok(Json(serde_json::json!({ "message": message })))
}

async fn send_media_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(chat_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth, "chats:write")?;
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut mime_type: Option<String> = None;
    let mut caption: Option<String> = None;
    let mut media_type: Option<String> = None; // "image" or "document"

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                mime_type = field.content_type().map(|s| s.to_string());
                file_name = field.file_name().map(|s| s.to_string());
                file_data = Some(field.bytes().await.map_err(|e| {
                    (
                        axum::http::StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({ "error": format!("Failed to read file: {}", e) })),
                    )
                })?.to_vec());
            }
            "caption" => {
                caption = Some(field.text().await.unwrap_or_default());
            }
            "type" => {
                media_type = Some(field.text().await.unwrap_or_default());
            }
            _ => {}
        }
    }

    let file_data = file_data.ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "No file provided" })),
        )
    })?;

    let mime = mime_type.unwrap_or_else(|| "application/octet-stream".to_string());

    // Get chat info
    let chat = sqlx::query_as::<_, Chat>("SELECT * FROM chats WHERE id = $1")
        .bind(chat_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Chat not found" })),
            )
        })?;

    // Determine message type and send
    let is_image = media_type.as_deref() == Some("image")
        || mime.starts_with("image/");
    let result = if is_image {
        state
            .wa_manager
            .send_image_message(chat.session_id, &chat.chat_jid, file_data, &mime, caption.as_deref())
            .await
            .map_err(|e| {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
            })?
    } else {
        let fname = file_name.clone().unwrap_or_else(|| "file".to_string());
        state
            .wa_manager
            .send_document_message(chat.session_id, &chat.chat_jid, file_data, &mime, &fname)
            .await
            .map_err(|e| {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
            })?
    };

    let msg_type_str = if is_image { "image" } else { "document" };

    // Save to DB
    let display_content = caption.clone().unwrap_or_else(|| {
        if is_image { "📷 Photo".to_string() } else { format!("📄 {}", file_name.as_deref().unwrap_or("Document")) }
    });

    let message = sqlx::query_as::<_, Message>(
        "INSERT INTO messages (id, chat_id, message_id, sender, sender_name, content, message_type, media_mime_type, media_filename, direct_path, media_key, file_enc_sha256, status, is_from_me, timestamp)
         VALUES ($1, $2, $3, $4, $5, $6, $7::message_type, $8, $9, $10, $11, $12, 'sent', true, NOW()) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(chat_id)
    .bind(&result.msg_id)
    .bind(&auth.username)
    .bind(&auth.username)
    .bind(&display_content)
    .bind(msg_type_str)
    .bind(&mime)
    .bind(&file_name)
    .bind(&result.direct_path)
    .bind(&result.media_key)
    .bind(&result.file_enc_sha256)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?
    .sanitized();

    // Update chat last_message
    sqlx::query("UPDATE chats SET last_message = $1, last_message_at = NOW() WHERE id = $2")
        .bind(&display_content)
        .bind(chat_id)
        .execute(&state.db)
        .await
        .ok();

    // Broadcast via WebSocket
    state.ws_hub.send_to_user(
        auth.user_id,
        serde_json::json!({
            "type": "message_sent",
            "data": { "message": &message, "chat_id": chat_id }
        }),
    );

    Ok(Json(serde_json::json!({ "message": message })))
}

async fn mark_as_read(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(chat_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth, "chats:write")?;
    sqlx::query("UPDATE chats SET unread_count = 0 WHERE id = $1")
        .bind(chat_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn trigger_sync(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SyncRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth, "chats:write")?;
    let session_id = req.session_id.ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "session_id is required" })),
        )
    })?;

    state
        .wa_manager
        .trigger_sync(session_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    state
        .ws_hub
        .send_to_user(
            auth.user_id,
            serde_json::json!({ "type": "sync_started", "data": { "session_id": session_id } }),
        );

    Ok(Json(serde_json::json!({ "ok": true })))
}
