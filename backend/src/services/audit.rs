use sqlx::PgPool;
use uuid::Uuid;

/// Log an audit event. Fire-and-forget - errors are silently ignored.
pub fn log_audit(
    db: PgPool,
    user_id: Option<Uuid>,
    action: String,
    entity_type: String,
    entity_id: Option<Uuid>,
    details: Option<serde_json::Value>,
    ip_address: Option<String>,
) {
    tokio::spawn(async move {
        let _ = sqlx::query(
            "INSERT INTO audit_logs (id, user_id, action, entity_type, entity_id, details, ip_address)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(&action)
        .bind(&entity_type)
        .bind(entity_id)
        .bind(&details)
        .bind(ip_address.as_deref())
        .execute(&db)
        .await;
    });
}
