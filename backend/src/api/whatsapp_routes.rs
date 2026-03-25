use axum::{
    extract::{Path, State},
    routing::{get, post, put, delete},
    Json, Router,
};
use uuid::Uuid;

use crate::auth::middleware::{AuthUser, require_scope};
use crate::models::session::*;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/sessions", get(list_sessions))
        .route("/sessions/:session_id", delete(delete_session))
        .route("/sessions/:session_id/webhook", put(update_webhook))
        .route("/connect", post(connect_session))
        .route("/disconnect/:session_id", post(disconnect_session))
        .route("/status/:session_id", get(session_status))
}

async fn list_sessions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth, "whatsapp:manage")?;
    let sessions = sqlx::query_as::<_, WhatsAppSession>(
        "SELECT * FROM whatsapp_sessions WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    let responses: Vec<SessionResponse> = sessions.into_iter().map(SessionResponse::from).collect();
    Ok(Json(serde_json::json!({ "sessions": responses })))
}

async fn connect_session(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth, "whatsapp:manage")?;
    let session_id = Uuid::new_v4();
    let db_path = format!(
        "{}/{}.db",
        state.config.session_db_dir,
        session_id
    );

    // Create directory
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    // Insert session record
    let session = sqlx::query_as::<_, WhatsAppSession>(
        "INSERT INTO whatsapp_sessions (id, user_id, session_name, db_path, status)
         VALUES ($1, $2, $3, $4, 'connecting') RETURNING *",
    )
    .bind(session_id)
    .bind(auth.user_id)
    .bind(&req.session_name)
    .bind(&db_path)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    // Start WhatsApp session in background
    let manager = state.wa_manager.clone();
    let user_id = auth.user_id;
    tokio::spawn(async move {
        if let Err(e) = manager.start_session(session_id, user_id).await {
            tracing::error!("Failed to start session {}: {}", session_id, e);
        }
    });

    let resp: SessionResponse = session.into();
    Ok(Json(serde_json::json!({ "session": resp })))
}

async fn disconnect_session(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth, "whatsapp:manage")?;
    state
        .wa_manager
        .disconnect_session(session_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    sqlx::query("UPDATE whatsapp_sessions SET status = 'disconnected' WHERE id = $1")
        .bind(session_id)
        .execute(&state.db)
        .await
        .ok();

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn delete_session(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth, "whatsapp:manage")?;
    // Verify ownership
    let session = sqlx::query_as::<_, WhatsAppSession>(
        "SELECT * FROM whatsapp_sessions WHERE id = $1 AND user_id = $2",
    )
    .bind(session_id)
    .bind(auth.user_id)
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
            Json(serde_json::json!({ "error": "Session not found" })),
        )
    })?;

    // Disconnect if still active
    let _ = state.wa_manager.disconnect_session(session_id).await;

    // Delete from DB (cascades to chats → messages)
    sqlx::query("DELETE FROM whatsapp_sessions WHERE id = $1")
        .bind(session_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    // Delete SQLite session file from disk
    let _ = std::fs::remove_file(&session.db_path);

    tracing::info!(%session_id, "Session deleted");
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn session_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth, "whatsapp:manage")?;
    let session = sqlx::query_as::<_, WhatsAppSession>(
        "SELECT * FROM whatsapp_sessions WHERE id = $1",
    )
    .bind(session_id)
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
            Json(serde_json::json!({ "error": "Session not found" })),
        )
    })?;

    let alive = state.wa_manager.is_session_alive(&session_id);
    let resp: SessionResponse = session.into();
    Ok(Json(serde_json::json!({ "session": resp, "alive": alive })))
}

/// PUT /api/whatsapp/sessions/:session_id/webhook
/// Configure the webhook URL and optional auth token for a session.
async fn update_webhook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(session_id): Path<Uuid>,
    Json(req): Json<UpdateWebhookRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth, "whatsapp:manage")?;

    // Verify session exists and belongs to the authenticated user
    let session = sqlx::query_as::<_, WhatsAppSession>(
        "SELECT * FROM whatsapp_sessions WHERE id = $1",
    )
    .bind(session_id)
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
            Json(serde_json::json!({ "error": "Session not found" })),
        )
    })?;

    if session.user_id != auth.user_id {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Not your session" })),
        ));
    }

    sqlx::query(
        "UPDATE whatsapp_sessions SET webhook_url = $1, webhook_token = $2 WHERE id = $3",
    )
    .bind(&req.webhook_url)
    .bind(&req.webhook_token)
    .bind(session_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(serde_json::json!({
        "message": "Webhook configuration updated",
        "webhook_url": req.webhook_url,
    })))
}
