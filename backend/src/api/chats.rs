use axum::{
    extract::{Path, Query, State, Multipart},
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::models::chat::*;
use crate::models::message::*;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_chats))
        .route("/:chat_id/messages", get(list_messages))
        .route("/:chat_id/send", post(send_message))
        .route("/:chat_id/send-media", post(send_media_message))
        .route("/:chat_id/read", post(mark_as_read))
        .route("/sync", post(trigger_sync))
}

async fn list_chats(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(query): Query<ChatListQuery>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
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
    })?;

    Ok(Json(serde_json::json!({ "chats": chats })))
}

async fn list_messages(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(chat_id): Path<Uuid>,
    Query(query): Query<MessageListQuery>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
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
            "SELECT * FROM messages
             WHERE chat_id = $1 AND timestamp < $2
             ORDER BY timestamp DESC LIMIT $3",
        )
        .bind(chat_id)
        .bind(cursor_time)
        .bind(limit)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, Message>(
            "SELECT * FROM messages
             WHERE chat_id = $1
             ORDER BY timestamp DESC LIMIT $2",
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
    })?;

    let has_more = messages.len() as i64 == limit;
    messages.reverse(); // Return in chronological order

    Ok(Json(serde_json::json!({
        "messages": messages,
        "has_more": has_more,
    })))
}

async fn send_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(chat_id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
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

    // Send via WhatsApp
    let msg_id = state
        .wa_manager
        .send_text_message(chat.session_id, &chat.chat_jid, &req.content)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    // Save to DB
    let message = sqlx::query_as::<_, Message>(
        "INSERT INTO messages (id, chat_id, message_id, sender, sender_name, content, message_type, status, is_from_me, timestamp)
         VALUES ($1, $2, $3, $4, $5, $6, 'text', 'sent', true, NOW()) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(chat_id)
    .bind(&msg_id)
    .bind(&auth.username)
    .bind(&auth.username)
    .bind(&req.content)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

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
    let (msg_type_str, msg_id) = if is_image {
        let msg_id = state
            .wa_manager
            .send_image_message(chat.session_id, &chat.chat_jid, file_data, &mime, caption.as_deref())
            .await
            .map_err(|e| {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
            })?;
        ("image", msg_id)
    } else {
        let fname = file_name.clone().unwrap_or_else(|| "file".to_string());
        let msg_id = state
            .wa_manager
            .send_document_message(chat.session_id, &chat.chat_jid, file_data, &mime, &fname)
            .await
            .map_err(|e| {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
            })?;
        ("document", msg_id)
    };

    // Save to DB
    let display_content = caption.clone().unwrap_or_else(|| {
        if is_image { "📷 Photo".to_string() } else { format!("📄 {}", file_name.as_deref().unwrap_or("Document")) }
    });

    let message = sqlx::query_as::<_, Message>(
        "INSERT INTO messages (id, chat_id, message_id, sender, sender_name, content, message_type, media_mime_type, media_filename, status, is_from_me, timestamp)
         VALUES ($1, $2, $3, $4, $5, $6, $7::message_type, $8, $9, 'sent', true, NOW()) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(chat_id)
    .bind(&msg_id)
    .bind(&auth.username)
    .bind(&auth.username)
    .bind(&display_content)
    .bind(msg_type_str)
    .bind(&mime)
    .bind(&file_name)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

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
    _auth: AuthUser,
    Path(chat_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
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
