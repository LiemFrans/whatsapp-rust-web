mod api;
mod auth;
mod config;
mod db;
mod models;
mod services;
mod websocket;
mod whatsapp;

use std::sync::Arc;

use axum::Router;
use dashmap::DashMap;
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use config::AppConfig;
use websocket::hub::WebSocketHub;
use whatsapp::manager::WhatsAppManager;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<AppConfig>,
    pub wa_manager: Arc<WhatsAppManager>,
    pub ws_hub: Arc<WebSocketHub>,
    /// In-memory rate limiter for API tokens: token_id → (request_count, window_start)
    pub rate_limiter: Arc<DashMap<uuid::Uuid, (u32, std::time::Instant)>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Init tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Load config
    let config = AppConfig::from_env()?;
    let config = Arc::new(config);
    tracing::info!("Starting WhatsApp Web Backend on port {}", config.port);

    // Connect to database
    let db = db::connect(&config.database_url).await?;
    db::run_migrations(&db).await?;
    db::seed_admin(&db).await?;
    tracing::info!("Database connected and migrated");

    // Create shared state
    let ws_hub = Arc::new(WebSocketHub::new());
    let wa_manager = Arc::new(WhatsAppManager::new(
        db.clone(),
        config.clone(),
        ws_hub.clone(),
    ));

    // Restore existing sessions
    wa_manager.restore_sessions().await;

    let state = AppState {
        db,
        config: config.clone(),
        wa_manager,
        ws_hub,
        rate_limiter: Arc::new(DashMap::new()),
    };

    // CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        .merge(api::routes())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
