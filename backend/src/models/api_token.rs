use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ApiToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub token_prefix: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_revoked: bool,
    pub rate_limit_per_minute: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Serializable response that includes the token prefix for identification
#[derive(Debug, Serialize)]
pub struct ApiTokenResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub token_prefix: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_revoked: bool,
    pub rate_limit_per_minute: Option<i32>,
    pub created_at: DateTime<Utc>,
}

impl From<ApiToken> for ApiTokenResponse {
    fn from(t: ApiToken) -> Self {
        Self {
            id: t.id,
            user_id: t.user_id,
            name: t.name,
            token_prefix: t.token_prefix,
            scopes: t.scopes,
            expires_at: t.expires_at,
            last_used_at: t.last_used_at,
            is_revoked: t.is_revoked,
            rate_limit_per_minute: t.rate_limit_per_minute,
            created_at: t.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateApiTokenRequest {
    /// Human-readable name for the token (e.g. "CI/CD Pipeline", "Monitoring Bot")
    pub name: String,
    /// Optional scopes to restrict the token (e.g. ["chats:read", "messages:read"])
    /// Empty = full access (inherits user's role permissions)
    pub scopes: Option<Vec<String>>,
    /// Optional expiry in days. None = never expires.
    pub expires_in_days: Option<i64>,
    /// Optional rate limit per minute. None = unlimited.
    pub rate_limit_per_minute: Option<i32>,
}
