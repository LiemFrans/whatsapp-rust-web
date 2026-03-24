use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "assignment_status", rename_all = "snake_case")]
pub enum AssignmentStatus {
    Active,
    Transferred,
    Completed,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ChatAssignment {
    pub id: Uuid,
    pub chat_id: Uuid,
    pub assigned_to: Uuid,
    pub assigned_by: Uuid,
    pub status: AssignmentStatus,
    pub assigned_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct AssignChatRequest {
    pub chat_id: Uuid,
    pub agent_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct TransferChatRequest {
    pub chat_id: Uuid,
    pub to_agent_id: Uuid,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TakeChatRequest {
    pub chat_id: Uuid,
}
