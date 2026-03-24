use axum::{extract::State, routing::post, Json, Router};
use uuid::Uuid;

use crate::auth::jwt;
use crate::auth::middleware::AuthUser;
use crate::models::user::*;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .route("/refresh", post(refresh_token))
        .route("/me", axum::routing::get(me))
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1 AND is_active = true")
        .bind(&req.username)
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
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Invalid credentials" })),
            )
        })?;

    let valid = jwt::verify_password(&req.password, &user.password_hash).map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    if !valid {
        return Err((
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Invalid credentials" })),
        ));
    }

    let access_token = jwt::create_access_token(user.id, &user.username, &user.role, &state.config.jwt_secret)
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    let refresh_token = jwt::create_refresh_token(user.id, &user.username, &user.role, &state.config.jwt_secret)
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    // Store refresh token
    let token_hash = format!("{:x}", md5_hash(&refresh_token));
    sqlx::query("INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, NOW() + INTERVAL '7 days')")
        .bind(Uuid::new_v4())
        .bind(user.id)
        .bind(&token_hash)
        .execute(&state.db)
        .await
        .ok();

    // Update last login
    sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(&state.db)
        .await
        .ok();

    let user_resp: UserResponse = user.into();
    Ok(Json(serde_json::json!({
        "access_token": access_token,
        "refresh_token": refresh_token,
        "token_type": "Bearer",
        "expires_in": 86400,
        "user": user_resp,
    })))
}

async fn register(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Only admins can create agent/admin accounts
    let role = req.role.unwrap_or(UserRole::User);
    if role != UserRole::User && auth.role != UserRole::Admin {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Only admins can create agent/admin accounts" })),
        ));
    }

    // Check duplicates
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1 OR email = $2)")
        .bind(&req.username)
        .bind(&req.email)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    if exists {
        return Err((
            axum::http::StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": "Username or email already exists" })),
        ));
    }

    let password_hash = jwt::hash_password(&req.password).map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (id, username, email, password_hash, display_name, role)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(&req.username)
    .bind(&req.email)
    .bind(&password_hash)
    .bind(&req.display_name)
    .bind(&role)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    let user_resp: UserResponse = user.into();
    Ok(Json(serde_json::json!({ "user": user_resp })))
}

async fn refresh_token(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let claims = jwt::validate_token(&req.refresh_token, &state.config.jwt_secret).map_err(|_| {
        (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Invalid refresh token" })),
        )
    })?;

    if claims.token_type != "refresh" {
        return Err((
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Not a refresh token" })),
        ));
    }

    // Verify refresh token exists in DB
    let token_hash = format!("{:x}", md5_hash(&req.refresh_token));
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM refresh_tokens WHERE user_id = $1 AND token_hash = $2 AND expires_at > NOW())",
    )
    .bind(claims.sub)
    .bind(&token_hash)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    if !exists {
        return Err((
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Refresh token not found or expired" })),
        ));
    }

    // Delete old token
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1 AND token_hash = $2")
        .bind(claims.sub)
        .bind(&token_hash)
        .execute(&state.db)
        .await
        .ok();

    // Create new tokens
    let access_token = jwt::create_access_token(claims.sub, &claims.username, &claims.role, &state.config.jwt_secret)
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    let new_refresh = jwt::create_refresh_token(claims.sub, &claims.username, &claims.role, &state.config.jwt_secret)
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    // Store new refresh token
    let new_hash = format!("{:x}", md5_hash(&new_refresh));
    sqlx::query("INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, NOW() + INTERVAL '7 days')")
        .bind(Uuid::new_v4())
        .bind(claims.sub)
        .bind(&new_hash)
        .execute(&state.db)
        .await
        .ok();

    Ok(Json(serde_json::json!({
        "access_token": access_token,
        "refresh_token": new_refresh,
        "token_type": "Bearer",
        "expires_in": 86400,
    })))
}

async fn me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    let user_resp: UserResponse = user.into();
    Ok(Json(serde_json::json!({ "user": user_resp })))
}

fn md5_hash(input: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}
