use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::auth::middleware::AdminUser;
use crate::models::api_token::*;
use crate::AppState;

/// Available scopes for API tokens.
/// Empty scopes = full access (inherits user's role permissions).
pub const VALID_SCOPES: &[&str] = &[
    "chats:read",
    "chats:write",
    "business:read",
    "business:write",
    "tickets:read",
    "tickets:write",
    "users:read",
    "users:write",
    "whatsapp:manage",
    "quick_replies:read",
    "quick_replies:write",
];

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_tokens).post(create_token))
        .route("/:id", delete(revoke_token))
}

/// List all API tokens (admin only).
/// Token hashes are never exposed; only prefixes and metadata.
async fn list_tokens(
    State(state): State<AppState>,
    _auth: AdminUser,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let tokens = sqlx::query_as::<_, ApiToken>(
        "SELECT * FROM api_tokens ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    let responses: Vec<ApiTokenResponse> = tokens.into_iter().map(|t| t.into()).collect();
    Ok(Json(serde_json::json!({
        "tokens": responses,
        "available_scopes": VALID_SCOPES,
    })))
}

/// Generate a new API token (admin only).
/// The plaintext token is returned ONLY ONCE in this response — save it immediately.
async fn create_token(
    State(state): State<AppState>,
    auth: AdminUser,
    Json(req): Json<CreateApiTokenRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Validate scopes if provided
    if let Some(ref scopes) = req.scopes {
        for scope in scopes {
            if !VALID_SCOPES.contains(&scope.as_str()) {
                return Err((
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("Invalid scope: '{}'. Valid scopes: {:?}", scope, VALID_SCOPES)
                    })),
                ));
            }
        }
    }

    // Validate rate_limit_per_minute if provided
    if let Some(limit) = req.rate_limit_per_minute {
        if limit < 0 {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "rate_limit_per_minute must be non-negative" })),
            ));
        }
    }

    // Generate random token: wrt_ + 48 random bytes encoded as base64url
    let mut random_bytes = [0u8; 48];
    rand::rng().fill_bytes(&mut random_bytes);
    let token_raw = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes);
    let plaintext_token = format!("wrt_{}", token_raw);
    let token_prefix = &plaintext_token[..12]; // "wrt_" + 8 chars

    // Hash for storage (SHA-256)
    let token_hash = hex::encode(Sha256::digest(plaintext_token.as_bytes()));

    // Calculate expiry
    let expires_at = req
        .expires_in_days
        .map(|days| chrono::Utc::now() + chrono::Duration::days(days));

    let scopes = req.scopes.unwrap_or_default();
    let rate_limit = req.rate_limit_per_minute;

    let token = sqlx::query_as::<_, ApiToken>(
        "INSERT INTO api_tokens (id, user_id, name, token_hash, token_prefix, scopes, expires_at, rate_limit_per_minute)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(auth.0.user_id)
    .bind(&req.name)
    .bind(&token_hash)
    .bind(token_prefix)
    .bind(&scopes)
    .bind(expires_at)
    .bind(rate_limit)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    let response: ApiTokenResponse = token.into();
    Ok(Json(serde_json::json!({
        "token": plaintext_token,
        "details": response,
        "warning": "Save this token now. It will NOT be shown again."
    })))
}

/// Revoke an API token (admin only). Soft-delete by setting is_revoked = true.
async fn revoke_token(
    State(state): State<AppState>,
    _auth: AdminUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let token = sqlx::query_as::<_, ApiToken>(
        "UPDATE api_tokens SET is_revoked = true, updated_at = NOW() WHERE id = $1 RETURNING *",
    )
    .bind(id)
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
            Json(serde_json::json!({ "error": "Token not found" })),
        )
    })?;

    // Also clear the rate limiter entry
    state.rate_limiter.remove(&id);

    let response: ApiTokenResponse = token.into();
    Ok(Json(
        serde_json::json!({ "token": response, "message": "Token revoked successfully" }),
    ))
}
