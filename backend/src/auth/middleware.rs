use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts, Query},
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::jwt::{validate_token, Claims};
use crate::models::user::UserRole;
use crate::AppState;

/// Extracts and validates the authenticated user from the request.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub username: String,
    pub role: UserRole,
}

#[derive(Debug, Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authentication token"),
            AuthError::InvalidToken(_) => (StatusCode::UNAUTHORIZED, "Invalid authentication token"),
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

        let claims: Claims = validate_token(&token, &app_state.config.jwt_secret)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        if claims.token_type != "access" {
            return Err(AuthError::InvalidToken("Not an access token".into()));
        }

        Ok(AuthUser {
            user_id: claims.sub,
            username: claims.username,
            role: claims.role,
        })
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
            return Err(AuthError::InvalidToken("Agent or admin access required".into()));
        }
        Ok(AgentUser(user))
    }
}
