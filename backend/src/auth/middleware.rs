use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts, Query},
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::auth::jwt::validate_token;
use crate::models::user::UserRole;
use crate::AppState;

/// Extracts and validates the authenticated user from the request.
/// Supports both JWT tokens and long-lived API tokens (prefixed with `wrt_`).
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub username: String,
    pub role: UserRole,
    /// None = JWT (full role access), Some(vec) = API token with specific scopes.
    /// Empty vec = full access (inherits user's role permissions).
    pub scopes: Option<Vec<String>>,
}

impl AuthUser {
    /// Check if this user has the given scope.
    /// JWT users (scopes = None) always return true.
    /// API tokens with empty scopes also return true (full access).
    pub fn has_scope(&self, scope: &str) -> bool {
        match &self.scopes {
            None => true,
            Some(scopes) => scopes.is_empty() || scopes.iter().any(|s| s == scope),
        }
    }
}

/// Check if the authenticated user has the required scope.
/// JWT users (scopes = None) always pass. API tokens with empty scopes also pass (full access).
/// Returns a handler-compatible error if scope is missing.
pub fn require_scope(
    auth: &AuthUser,
    scope: &str,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if auth.has_scope(scope) {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": format!("Insufficient scope. Required: {}", scope)
            })),
        ))
    }
}

#[derive(Debug, Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken(String),
    RateLimited,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AuthError::MissingToken => {
                (StatusCode::UNAUTHORIZED, "Missing authentication token")
            }
            AuthError::InvalidToken(_) => {
                (StatusCode::UNAUTHORIZED, "Invalid authentication token")
            }
            AuthError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded for this API token",
            ),
        };
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        // Try Authorization header first
        let token = if let Some(auth_header) = parts.headers.get(AUTHORIZATION) {
            let header_str = auth_header
                .to_str()
                .map_err(|_| AuthError::InvalidToken("Invalid header".into()))?;
            if let Some(token) = header_str.strip_prefix("Bearer ") {
                token.to_string()
            } else {
                return Err(AuthError::MissingToken);
            }
        } else {
            // Try query parameter (for WebSocket connections)
            let Query(query) = Query::<TokenQuery>::from_request_parts(parts, state)
                .await
                .map_err(|_| AuthError::MissingToken)?;
            query.token.ok_or(AuthError::MissingToken)?
        };

        // 1) Try JWT validation first
        if let Ok(claims) = validate_token(&token, &app_state.config.jwt_secret) {
            if claims.token_type == "access" {
                return Ok(AuthUser {
                    user_id: claims.sub,
                    username: claims.username,
                    role: claims.role,
                    scopes: None, // JWT = full role access
                });
            }
        }

        // 2) If JWT failed, try API token lookup (tokens start with "wrt_")
        if token.starts_with("wrt_") {
            let token_hash = hex::encode(Sha256::digest(token.as_bytes()));

            let api_token = sqlx::query_as::<_, crate::models::api_token::ApiToken>(
                "SELECT * FROM api_tokens
                 WHERE token_hash = $1
                   AND NOT is_revoked
                   AND (expires_at IS NULL OR expires_at > NOW())",
            )
            .bind(&token_hash)
            .fetch_optional(&app_state.db)
            .await
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?
            .ok_or_else(|| AuthError::InvalidToken("Invalid or expired API token".into()))?;

            // Check rate limit (if configured)
            if let Some(limit) = api_token.rate_limit_per_minute {
                if limit > 0 {
                    let now = std::time::Instant::now();
                    let mut entry = app_state
                        .rate_limiter
                        .entry(api_token.id)
                        .or_insert((0u32, now));
                    let (count, window_start) = entry.value_mut();

                    if now.duration_since(*window_start).as_secs() >= 60 {
                        // New window — reset
                        *count = 1;
                        *window_start = now;
                    } else if *count < limit as u32 {
                        *count += 1;
                    } else {
                        return Err(AuthError::RateLimited);
                    }
                }
            }

            // Resolve the token owner
            let user = sqlx::query_as::<_, crate::models::user::User>(
                "SELECT * FROM users WHERE id = $1 AND is_active = true",
            )
            .bind(api_token.user_id)
            .fetch_optional(&app_state.db)
            .await
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?
            .ok_or_else(|| {
                AuthError::InvalidToken("Token owner not found or inactive".into())
            })?;

            // Update last_used_at asynchronously (fire-and-forget)
            let db = app_state.db.clone();
            let token_id = api_token.id;
            tokio::spawn(async move {
                let _ = sqlx::query("UPDATE api_tokens SET last_used_at = NOW() WHERE id = $1")
                    .bind(token_id)
                    .execute(&db)
                    .await;
            });

            return Ok(AuthUser {
                user_id: user.id,
                username: user.username,
                role: user.role,
                scopes: Some(api_token.scopes),
            });
        }

        Err(AuthError::InvalidToken("Invalid token".into()))
    }
}

/// Guard that requires admin role
pub struct AdminUser(pub AuthUser);

#[async_trait]
impl<S> FromRequestParts<S> for AdminUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.role != UserRole::Admin {
            return Err(AuthError::InvalidToken("Admin access required".into()));
        }
        Ok(AdminUser(user))
    }
}

/// Guard that requires agent or admin role
pub struct AgentUser(pub AuthUser);

#[async_trait]
impl<S> FromRequestParts<S> for AgentUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.role == UserRole::User {
            return Err(AuthError::InvalidToken(
                "Agent or admin access required".into(),
            ));
        }
        Ok(AgentUser(user))
    }
}
