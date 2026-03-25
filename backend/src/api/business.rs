use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use uuid::Uuid;

use crate::auth::middleware::{AdminUser, AgentUser, require_scope};
use crate::models::assignment::*;
use crate::models::escalation::*;
use crate::models::quick_reply::*;
use crate::models::ticket::*;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/queue", get(get_queue))
        .route("/my-chats", get(my_chats))
        .route("/assign", post(assign_chat))
        .route("/take", post(take_chat))
        .route("/transfer", post(transfer_chat))
        .route("/tickets", get(list_tickets).post(create_ticket))
        .route("/tickets/:id", get(get_ticket).put(update_ticket))
        .route("/tickets/:id/notes", get(get_ticket_notes).post(add_ticket_note))
        .route("/escalations", post(create_escalation))
        .route("/escalations/:id/resolve", post(resolve_escalation))
        .route("/quick-replies", get(list_quick_replies).post(create_quick_reply))
        .route("/quick-replies/:id", put(update_quick_reply))
        .route("/analytics", get(get_analytics))
        .route("/agents", get(list_agents))
}

async fn get_queue(
    State(state): State<AppState>,
    auth: AgentUser,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "business:read")?;
    let queue = sqlx::query_as::<_, crate::models::chat::Chat>(
        "SELECT c.* FROM chats c
         WHERE NOT EXISTS (
             SELECT 1 FROM chat_assignments ca
             WHERE ca.chat_id = c.id AND ca.status = 'active'
         )
         AND c.unread_count > 0
         ORDER BY c.last_message_at DESC NULLS LAST",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?
    .into_iter()
    .map(crate::models::chat::Chat::sanitized)
    .collect::<Vec<_>>();

    Ok(Json(serde_json::json!({ "queue": queue })))
}

async fn my_chats(
    State(state): State<AppState>,
    auth: AgentUser,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "business:read")?;
    let chats = sqlx::query_as::<_, crate::models::chat::Chat>(
        "SELECT c.* FROM chats c
         INNER JOIN chat_assignments ca ON ca.chat_id = c.id
         WHERE ca.assigned_to = $1 AND ca.status = 'active'
         ORDER BY c.last_message_at DESC NULLS LAST",
    )
    .bind(auth.0.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?
    .into_iter()
    .map(crate::models::chat::Chat::sanitized)
    .collect::<Vec<_>>();

    Ok(Json(serde_json::json!({ "chats": chats })))
}

async fn assign_chat(
    State(state): State<AppState>,
    auth: AgentUser,
    Json(req): Json<AssignChatRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "business:write")?;
    let agent_id = req.agent_id.unwrap_or(auth.0.user_id);

    let assignment = sqlx::query_as::<_, ChatAssignment>(
        "INSERT INTO chat_assignments (id, chat_id, assigned_to, assigned_by, status)
         VALUES ($1, $2, $3, $4, 'active') RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(req.chat_id)
    .bind(agent_id)
    .bind(auth.0.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(serde_json::json!({ "assignment": assignment })))
}

async fn take_chat(
    State(state): State<AppState>,
    auth: AgentUser,
    Json(req): Json<TakeChatRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "business:write")?;
    // Check not already assigned
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM chat_assignments WHERE chat_id = $1 AND status = 'active')",
    )
    .bind(req.chat_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    if exists {
        return Err((
            axum::http::StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": "Chat already assigned" })),
        ));
    }

    let assignment = sqlx::query_as::<_, ChatAssignment>(
        "INSERT INTO chat_assignments (id, chat_id, assigned_to, assigned_by, status)
         VALUES ($1, $2, $3, $3, 'active') RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(req.chat_id)
    .bind(auth.0.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(serde_json::json!({ "assignment": assignment })))
}

async fn transfer_chat(
    State(state): State<AppState>,
    auth: AgentUser,
    Json(req): Json<TransferChatRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "business:write")?;
    // Mark existing assignment as transferred
    sqlx::query(
        "UPDATE chat_assignments SET status = 'transferred', completed_at = NOW()
         WHERE chat_id = $1 AND assigned_to = $2 AND status = 'active'",
    )
    .bind(req.chat_id)
    .bind(auth.0.user_id)
    .execute(&state.db)
    .await
    .ok();

    // Create new assignment
    let assignment = sqlx::query_as::<_, ChatAssignment>(
        "INSERT INTO chat_assignments (id, chat_id, assigned_to, assigned_by, status)
         VALUES ($1, $2, $3, $4, 'active') RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(req.chat_id)
    .bind(req.to_agent_id)
    .bind(auth.0.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(serde_json::json!({ "assignment": assignment })))
}

async fn list_tickets(
    State(state): State<AppState>,
    auth: AgentUser,
    Query(query): Query<TicketListQuery>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "tickets:read")?;
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;

    let tickets = sqlx::query_as::<_, Ticket>(
        "SELECT * FROM tickets ORDER BY
         CASE priority WHEN 'critical' THEN 1 WHEN 'high' THEN 2 WHEN 'medium' THEN 3 ELSE 4 END,
         created_at DESC
         LIMIT $1 OFFSET $2",
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(serde_json::json!({ "tickets": tickets })))
}

async fn create_ticket(
    State(state): State<AppState>,
    auth: AgentUser,
    Json(req): Json<CreateTicketRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "tickets:write")?;
    let priority = req.priority.unwrap_or(TicketPriority::Medium);

    let ticket = sqlx::query_as::<_, Ticket>(
        "INSERT INTO tickets (id, chat_id, title, description, priority, status, category, created_by)
         VALUES ($1, $2, $3, $4, $5, 'open', $6, $7) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(req.chat_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(&priority)
    .bind(&req.category)
    .bind(auth.0.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(serde_json::json!({ "ticket": ticket })))
}

async fn get_ticket(
    State(state): State<AppState>,
    auth: AgentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "tickets:read")?;
    let ticket = sqlx::query_as::<_, Ticket>("SELECT * FROM tickets WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Ticket not found" })),
            )
        })?;

    let notes = sqlx::query_as::<_, TicketNote>(
        "SELECT * FROM ticket_notes WHERE ticket_id = $1 ORDER BY created_at",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Ok(Json(serde_json::json!({ "ticket": ticket, "notes": notes })))
}

async fn update_ticket(
    State(state): State<AppState>,
    auth: AgentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTicketRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "tickets:write")?;
    let ticket = sqlx::query_as::<_, Ticket>(
        "UPDATE tickets SET
         status = COALESCE($2, status),
         priority = COALESCE($3, priority),
         assigned_to = COALESCE($4, assigned_to),
         category = COALESCE($5, category),
         resolved_at = CASE WHEN $2 = 'resolved' THEN NOW() ELSE resolved_at END,
         closed_at = CASE WHEN $2 = 'closed' THEN NOW() ELSE closed_at END
         WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(&req.status)
    .bind(&req.priority)
    .bind(req.assigned_to)
    .bind(&req.category)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?
    .ok_or_else(|| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Ticket not found" })),
        )
    })?;

    Ok(Json(serde_json::json!({ "ticket": ticket })))
}

async fn get_ticket_notes(
    State(state): State<AppState>,
    auth: AgentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "tickets:read")?;
    let notes = sqlx::query_as::<_, TicketNote>(
        "SELECT * FROM ticket_notes WHERE ticket_id = $1 ORDER BY created_at",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(serde_json::json!({ "notes": notes })))
}

async fn add_ticket_note(
    State(state): State<AppState>,
    auth: AgentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateTicketNoteRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "tickets:write")?;
    let note = sqlx::query_as::<_, TicketNote>(
        "INSERT INTO ticket_notes (id, ticket_id, note, created_by)
         VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(id)
    .bind(&req.note)
    .bind(auth.0.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(serde_json::json!({ "note": note })))
}

async fn create_escalation(
    State(state): State<AppState>,
    auth: AgentUser,
    Json(req): Json<CreateEscalationRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "tickets:write")?;
    let escalation = sqlx::query_as::<_, Escalation>(
        "INSERT INTO escalations (id, ticket_id, from_user_id, to_user_id, reason, status)
         VALUES ($1, $2, $3, $4, $5, 'pending') RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(req.ticket_id)
    .bind(auth.0.user_id)
    .bind(req.to_user_id)
    .bind(&req.reason)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(serde_json::json!({ "escalation": escalation })))
}

async fn resolve_escalation(
    State(state): State<AppState>,
    auth: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ResolveEscalationRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "tickets:write")?;
    let status: EscalationStatus = match req.action.as_str() {
        "accepted" => EscalationStatus::Accepted,
        "resolved" => EscalationStatus::Resolved,
        "rejected" => EscalationStatus::Rejected,
        _ => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid action. Must be accepted, resolved, or rejected" })),
            ));
        }
    };

    let escalation = sqlx::query_as::<_, Escalation>(
        "UPDATE escalations SET status = $2, resolution_note = $3, resolved_at = NOW()
         WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(&status)
    .bind(&req.resolution_note)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?
    .ok_or_else(|| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Escalation not found" })),
        )
    })?;

    Ok(Json(serde_json::json!({ "escalation": escalation })))
}

async fn list_quick_replies(
    State(state): State<AppState>,
    auth: AgentUser,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "quick_replies:read")?;
    let replies = sqlx::query_as::<_, QuickReply>(
        "SELECT * FROM quick_replies WHERE is_global = true OR created_by = $1 ORDER BY shortcut",
    )
    .bind(auth.0.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(serde_json::json!({ "quick_replies": replies })))
}

async fn create_quick_reply(
    State(state): State<AppState>,
    auth: AgentUser,
    Json(req): Json<CreateQuickReplyRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "quick_replies:write")?;
    let reply = sqlx::query_as::<_, QuickReply>(
        "INSERT INTO quick_replies (id, title, shortcut, content, category, is_global, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(&req.title)
    .bind(&req.shortcut)
    .bind(&req.content)
    .bind(&req.category)
    .bind(req.is_global.unwrap_or(false))
    .bind(auth.0.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(serde_json::json!({ "quick_reply": reply })))
}

async fn update_quick_reply(
    State(state): State<AppState>,
    auth: AgentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateQuickReplyRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "quick_replies:write")?;
    let reply = sqlx::query_as::<_, QuickReply>(
        "UPDATE quick_replies SET
         title = COALESCE($2, title),
         shortcut = COALESCE($3, shortcut),
         content = COALESCE($4, content),
         category = COALESCE($5, category),
         is_global = COALESCE($6, is_global)
         WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(&req.title)
    .bind(&req.shortcut)
    .bind(&req.content)
    .bind(&req.category)
    .bind(req.is_global)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?
    .ok_or_else(|| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Quick reply not found" })),
        )
    })?;

    Ok(Json(serde_json::json!({ "quick_reply": reply })))
}

async fn get_analytics(
    State(state): State<AppState>,
    auth: AgentUser,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "business:read")?;
    // Messages today
    let messages_today: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM messages WHERE timestamp >= CURRENT_DATE",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    // Unassigned count
    let unassigned_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM chats c WHERE NOT EXISTS (
            SELECT 1 FROM chat_assignments ca WHERE ca.chat_id = c.id AND ca.status = 'active'
        ) AND c.unread_count > 0",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    // Ticket breakdown
    let ticket_stats: Vec<(String, i64)> = sqlx::query_as(
        "SELECT status::text, COUNT(*) FROM tickets GROUP BY status",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut ticket_breakdown = serde_json::Map::new();
    for (status, count) in ticket_stats {
        ticket_breakdown.insert(status, serde_json::json!(count));
    }

    // Chats per agent
    let chats_per_agent: Vec<(String, i64)> = sqlx::query_as(
        "SELECT u.username, COUNT(ca.id)
         FROM chat_assignments ca
         JOIN users u ON u.id = ca.assigned_to
         WHERE ca.status = 'active'
         GROUP BY u.username
         ORDER BY COUNT(ca.id) DESC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let agents_data: Vec<serde_json::Value> = chats_per_agent
        .into_iter()
        .map(|(name, count)| serde_json::json!({ "agent_name": name, "count": count }))
        .collect();

    Ok(Json(serde_json::json!({
        "messages_today": messages_today,
        "unassigned_count": unassigned_count,
        "ticket_breakdown": ticket_breakdown,
        "chats_per_agent": agents_data,
    })))
}

async fn list_agents(
    State(state): State<AppState>,
    auth: AgentUser,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_scope(&auth.0, "business:read")?;
    let agents = sqlx::query_as::<_, crate::models::user::User>(
        "SELECT * FROM users WHERE role IN ('admin', 'agent') AND is_active = true ORDER BY username",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    let responses: Vec<crate::models::user::UserResponse> =
        agents.into_iter().map(|u| u.into()).collect();
    Ok(Json(serde_json::json!({ "agents": responses })))
}
