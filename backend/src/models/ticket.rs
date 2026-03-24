use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "ticket_priority", rename_all = "snake_case")]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "ticket_status", rename_all = "snake_case")]
pub enum TicketStatus {
    Open,
    InProgress,
    Pending,
    Resolved,
    Closed,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Ticket {
    pub id: Uuid,
    pub chat_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub priority: TicketPriority,
    pub status: TicketStatus,
    pub category: Option<String>,
    pub assigned_to: Option<Uuid>,
    pub created_by: Uuid,
    pub due_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TicketNote {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub note: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTicketRequest {
    pub chat_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<TicketPriority>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTicketRequest {
    pub status: Option<TicketStatus>,
    pub priority: Option<TicketPriority>,
    pub assigned_to: Option<Uuid>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTicketNoteRequest {
    pub note: String,
}

#[derive(Debug, Deserialize)]
pub struct TicketListQuery {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assigned_to: Option<Uuid>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}
