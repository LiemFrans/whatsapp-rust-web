use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::models::assignment::ChatAssignment;

/// Assign a chat to the next available agent using round-robin
pub async fn round_robin_assign(
    db: &PgPool,
    chat_id: Uuid,
    assigned_by: Uuid,
) -> Result<ChatAssignment, String> {
    // Get active agents ordered by fewest current assignments
    let agent_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT u.id FROM users u
         LEFT JOIN chat_assignments ca ON ca.assigned_to = u.id AND ca.status = 'active'
         WHERE u.role IN ('admin', 'agent') AND u.is_active = true
         GROUP BY u.id
         ORDER BY COUNT(ca.id) ASC
         LIMIT 1",
    )
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?;

    let agent_id = agent_id.ok_or("No agents available")?;

    let assignment = sqlx::query_as::<_, ChatAssignment>(
        "INSERT INTO chat_assignments (id, chat_id, assigned_to, assigned_by, status)
         VALUES ($1, $2, $3, $4, 'active') RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(chat_id)
    .bind(agent_id)
    .bind(assigned_by)
    .fetch_one(db)
    .await
    .map_err(|e| e.to_string())?;

    info!(%chat_id, %agent_id, "Chat assigned via round-robin");
    Ok(assignment)
}

/// Agent takes an unassigned chat
pub async fn take_chat(
    db: &PgPool,
    chat_id: Uuid,
    agent_id: Uuid,
) -> Result<ChatAssignment, String> {
    let already_assigned: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM chat_assignments WHERE chat_id = $1 AND status = 'active')",
    )
    .bind(chat_id)
    .fetch_one(db)
    .await
    .map_err(|e| e.to_string())?;

    if already_assigned {
        return Err("Chat is already assigned".into());
    }

    let assignment = sqlx::query_as::<_, ChatAssignment>(
        "INSERT INTO chat_assignments (id, chat_id, assigned_to, assigned_by, status)
         VALUES ($1, $2, $3, $3, 'active') RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(chat_id)
    .bind(agent_id)
    .fetch_one(db)
    .await
    .map_err(|e| e.to_string())?;

    info!(%chat_id, %agent_id, "Chat taken by agent");
    Ok(assignment)
}

/// Transfer a chat from one agent to another
pub async fn transfer_chat(
    db: &PgPool,
    chat_id: Uuid,
    from_agent_id: Uuid,
    to_agent_id: Uuid,
    _reason: Option<&str>,
) -> Result<ChatAssignment, String> {
    let rows = sqlx::query(
        "UPDATE chat_assignments SET status = 'transferred', completed_at = NOW()
         WHERE chat_id = $1 AND assigned_to = $2 AND status = 'active'",
    )
    .bind(chat_id)
    .bind(from_agent_id)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;

    if rows.rows_affected() == 0 {
        warn!(%chat_id, %from_agent_id, "No active assignment found to transfer");
    }

    let assignment = sqlx::query_as::<_, ChatAssignment>(
        "INSERT INTO chat_assignments (id, chat_id, assigned_to, assigned_by, status)
         VALUES ($1, $2, $3, $4, 'active') RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(chat_id)
    .bind(to_agent_id)
    .bind(from_agent_id)
    .fetch_one(db)
    .await
    .map_err(|e| e.to_string())?;

    info!(%chat_id, %from_agent_id, %to_agent_id, "Chat transferred");
    Ok(assignment)
}
