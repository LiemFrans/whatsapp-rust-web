use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::models::ticket::{Ticket, TicketNote, TicketPriority, TicketStatus};

/// Create a new ticket
pub async fn create_ticket(
    db: &PgPool,
    chat_id: Option<Uuid>,
    title: &str,
    description: Option<&str>,
    priority: TicketPriority,
    category: Option<&str>,
    created_by: Uuid,
) -> Result<Ticket, String> {
    let ticket = sqlx::query_as::<_, Ticket>(
        "INSERT INTO tickets (id, chat_id, title, description, priority, status, category, created_by)
         VALUES ($1, $2, $3, $4, $5, 'open', $6, $7) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(chat_id)
    .bind(title)
    .bind(description)
    .bind(&priority)
    .bind(category)
    .bind(created_by)
    .fetch_one(db)
    .await
    .map_err(|e| e.to_string())?;

    info!(ticket_id = %ticket.id, %title, "Ticket created");
    Ok(ticket)
}

/// Update ticket status with validation
pub async fn update_ticket_status(
    db: &PgPool,
    ticket_id: Uuid,
    new_status: TicketStatus,
) -> Result<Ticket, String> {
    // Fetch current ticket
    let current = sqlx::query_as::<_, Ticket>("SELECT * FROM tickets WHERE id = $1")
        .bind(ticket_id)
        .fetch_optional(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Ticket not found")?;

    // Validate status transition
    let valid_transition = matches!(
        (&current.status, &new_status),
        (TicketStatus::Open, TicketStatus::InProgress)
            | (TicketStatus::Open, TicketStatus::Closed)
            | (TicketStatus::InProgress, TicketStatus::Pending)
            | (TicketStatus::InProgress, TicketStatus::Resolved)
            | (TicketStatus::Pending, TicketStatus::InProgress)
            | (TicketStatus::Pending, TicketStatus::Resolved)
            | (TicketStatus::Resolved, TicketStatus::Closed)
            | (TicketStatus::Resolved, TicketStatus::InProgress) // Reopen
    );

    if !valid_transition {
        return Err(format!(
            "Invalid status transition from {:?} to {:?}",
            current.status, new_status
        ));
    }

    sqlx::query(
        "UPDATE tickets SET
         status = $2,
         resolved_at = CASE WHEN $2 = 'resolved' THEN NOW() ELSE resolved_at END,
         closed_at = CASE WHEN $2 = 'closed' THEN NOW() ELSE closed_at END
         WHERE id = $1",
    )
    .bind(ticket_id)
    .bind(&new_status)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;

    // Re-fetch to get updated data
    let ticket = sqlx::query_as::<_, Ticket>("SELECT * FROM tickets WHERE id = $1")
        .bind(ticket_id)
        .fetch_one(db)
        .await
        .map_err(|e| e.to_string())?;

    info!(ticket_id = %ticket.id, new_status = ?new_status, "Ticket status updated");
    Ok(ticket)
}

/// Add a note to a ticket
pub async fn add_note(
    db: &PgPool,
    ticket_id: Uuid,
    created_by: Uuid,
    content: &str,
) -> Result<TicketNote, String> {
    let note = sqlx::query_as::<_, TicketNote>(
        "INSERT INTO ticket_notes (id, ticket_id, note, created_by)
         VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(ticket_id)
    .bind(content)
    .bind(created_by)
    .fetch_one(db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(note)
}
