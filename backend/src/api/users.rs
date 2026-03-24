use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::models::user::*;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_users))
        .route("/:id", get(get_user).put(update_user))
        .route("/:id/toggle-active", put(toggle_active))
}

async fn list_users(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    if auth.role != UserRole::Admin {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Admin only" })),
        ));
    }

    let users = sqlx::query_as::<_, User>(
        "SELECT * FROM users ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    let responses: Vec<UserResponse> = users.into_iter().map(|u| u.into()).collect();
    Ok(Json(serde_json::json!({ "users": responses })))
}

async fn get_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    if auth.role != UserRole::Admin && auth.user_id != id {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Forbidden" })),
        ));
    }

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
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
                Json(serde_json::json!({ "error": "User not found" })),
            )
        })?;

    let resp: UserResponse = user.into();
    Ok(Json(serde_json::json!({ "user": resp })))
}

async fn update_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    if auth.role != UserRole::Admin && auth.user_id != id {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Forbidden" })),
        ));
    }

    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET
         display_name = COALESCE($2, display_name),
         email = COALESCE($3, email),
         role = COALESCE($4, role)
         WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(&req.display_name)
    .bind(&req.email)
    .bind(&req.role)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    let resp: UserResponse = user.into();
    Ok(Json(serde_json::json!({ "user": resp })))
}

async fn toggle_active(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    if auth.role != UserRole::Admin {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Admin only" })),
        ));
    }

    if auth.user_id == id {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Cannot deactivate yourself" })),
        ));
    }

    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET is_active = NOT is_active WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    let resp: UserResponse = user.into();
    Ok(Json(serde_json::json!({ "user": resp })))
}
