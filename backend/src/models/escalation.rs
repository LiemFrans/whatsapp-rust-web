use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "escalation_status", rename_all = "snake_case")]
pub enum EscalationStatus {
    Pending,
    Accepted,
    Resolved,
    Rejected,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Escalation {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub from_user_id: Uuid,
    pub to_user_id: Uuid,
    pub reason: String,
    pub status: EscalationStatus,
    pub resolution_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEscalationRequest {
    pub ticket_id: Uuid,
    pub to_user_id: Uuid,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ResolveEscalationRequest {
    pub action: String,
    pub resolution_note: Option<String>,
}
