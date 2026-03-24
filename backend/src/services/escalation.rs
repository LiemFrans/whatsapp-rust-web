use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::models::escalation::Escalation;

/// Create a new escalation
pub async fn create_escalation(
    db: &PgPool,
    ticket_id: Uuid,
    from_user_id: Uuid,
    to_user_id: Uuid,
    reason: &str,
) -> Result<Escalation, String> {
    // Update ticket status
    let _ = sqlx::query("UPDATE tickets SET status = 'pending' WHERE id = $1")
        .bind(ticket_id)
        .execute(db)
        .await;

    let escalation = sqlx::query_as::<_, Escalation>(
        "INSERT INTO escalations (id, ticket_id, from_user_id, to_user_id, reason, status)
         VALUES ($1, $2, $3, $4, $5, 'pending') RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(ticket_id)
    .bind(from_user_id)
    .bind(to_user_id)
    .bind(reason)
    .fetch_one(db)
    .await
    .map_err(|e| e.to_string())?;

    info!(%ticket_id, %from_user_id, %to_user_id, "Escalation created");
    Ok(escalation)
}

/// Resolve an escalation
pub async fn resolve_escalation(
    db: &PgPool,
    escalation_id: Uuid,
    action: &str,
    resolution_note: Option<&str>,
) -> Result<Escalation, String> {
    let escalation = sqlx::query_as::<_, Escalation>(
        "UPDATE escalations SET
         status = $2::escalation_status,
         resolution_note = $3,
         resolved_at = NOW()
         WHERE id = $1 RETURNING *",
    )
    .bind(escalation_id)
    .bind(action)
    .bind(resolution_note)
    .fetch_one(db)
    .await
    .map_err(|e| e.to_string())?;

    info!(%escalation_id, %action, "Escalation resolved");
    Ok(escalation)
}
