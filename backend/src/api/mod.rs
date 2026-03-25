pub mod api_tokens;
pub mod auth;
pub mod business;
pub mod chats;
pub mod users;
pub mod whatsapp_routes;

use axum::{routing::get, Json, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health_check))
        .nest("/api/auth", auth::routes())
        .nest("/api/tokens", api_tokens::routes())
        .nest("/api/users", users::routes())
        .nest("/api/whatsapp", whatsapp_routes::routes())
        .nest("/api/chats", chats::routes())
        .nest("/api/business", business::routes())
        .route("/ws", get(crate::websocket::hub::ws_handler))
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}
