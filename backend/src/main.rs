//! WhatsApp Web Clone — Rust Backend
//!
//! Endpoints:
//!   GET  /api/auth/qr                  → current QR code string + connection status
//!   GET  /api/auth/status              → connection status + sync status
//!   POST /api/auth/logout              → disconnect current session
//!   GET  /api/bootstrap                → initial UI payload (chats + contacts)
//!   GET  /api/chats                    → synced chat summaries
//!   GET  /api/chats/:jid/messages      → messages for a synced chat
//!   POST /api/chats/:jid/read          → locally mark a chat as read
//!   POST /api/chats/:jid/typing        → send chat state updates to WhatsApp
//!   GET  /api/contacts                 → known contacts
//!   POST /api/messages/send            → send a WhatsApp message
//!   GET  /api/media/:chat_jid/:msg_id  → download stored media for rendering

mod events;
mod handlers;
mod helpers;
mod models;
mod store;
mod sync;

// Re-export core types so tests and the router can reference them directly.
pub use models::*;

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use whatsapp_rust::bot::Bot;
use whatsapp_rust::store::SqliteStore;
use whatsapp_rust::transport::{TokioWebSocketTransportFactory, UreqHttpClient};

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/qr", get(handlers::get_qr))
        .route("/api/auth/status", get(handlers::get_status))
        .route("/api/auth/logout", post(handlers::logout))
        .route("/api/bootstrap", get(handlers::get_bootstrap))
        .route("/api/chats", get(handlers::get_chats))
        .route("/api/chats/:jid/messages", get(handlers::get_chat_messages))
        .route("/api/chats/:jid/read", post(handlers::mark_chat_read))
        .route("/api/chats/:jid/typing", post(handlers::update_typing))
        .route("/api/contacts", get(handlers::get_contacts))
        .route("/api/messages/send", post(handlers::send_message))
        .route("/api/media/:chat_jid/:message_id", get(handlers::get_media))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let db_path = "whatsapp.db";
    let state = AppState::new(db_path);
    let handler_state = state.clone();

    log::info!("Initialising WhatsApp client …");
    let backend = Arc::new(SqliteStore::new(db_path).await?);

    let mut bot = Bot::builder()
        .with_backend(backend)
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_http_client(UreqHttpClient::new())
        .on_event(move |event, client| {
            let state = handler_state.clone();
            async move {
                events::handle_event(event, client, state).await;
            }
        })
        .build()
        .await?;

    let client = bot.client();
    *state.client.write().await = Some(client);

    let _run_handle = bot.run().await?;
    log::info!("WhatsApp bot running in background");

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .expect("PORT must be a valid number");

    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    log::info!("🚀 Backend listening on http://0.0.0.0:{port}");
    axum::serve(listener, app).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use std::sync::atomic::Ordering;
    use tower::ServiceExt;

    fn pairing_state() -> AppState {
        let state = AppState::new("test.db");
        state.is_connected.store(false, Ordering::SeqCst);
        state.is_syncing.store(false, Ordering::SeqCst);
        state
    }

    fn connected_state_no_client() -> AppState {
        let state = AppState::new("test.db");
        state.is_connected.store(true, Ordering::SeqCst);
        state
    }

    async fn body_json(body: Body) -> serde_json::Value {
        let bytes = body.collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn json_post(uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap()
    }

    #[tokio::test]
    async fn qr_returns_code_when_pairing() {
        let state = pairing_state();
        *state.qr_code.write().await = Some("2@ABC123,ref,pk,cid".into());
        let app = create_router(state);
        let resp = app
            .oneshot(Request::get("/api/auth/qr").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["qr_code"], "2@ABC123,ref,pk,cid");
        assert_eq!(json["is_connected"], false);
    }

    #[tokio::test]
    async fn bootstrap_returns_empty_lists() {
        let app = create_router(pairing_state());
        let resp = app
            .oneshot(Request::get("/api/bootstrap").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["chats"].as_array().unwrap().len(), 0);
        assert_eq!(json["contacts"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn status_reports_disconnected() {
        let app = create_router(pairing_state());
        let resp = app
            .oneshot(Request::get("/api/auth/status").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let json = body_json(resp.into_body()).await;
        assert_eq!(json["is_connected"], false);
        assert_eq!(json["is_syncing"], false);
    }

    #[tokio::test]
    async fn send_rejects_empty_target() {
        let app = create_router(connected_state_no_client());
        let resp = app
            .oneshot(json_post(
                "/api/messages/send",
                r#"{"phone":"","message":"hi"}"#,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = body_json(resp.into_body()).await;
        assert!(json["error"].as_str().unwrap().contains("jid"));
    }

    #[tokio::test]
    async fn send_rejects_empty_message() {
        let app = create_router(connected_state_no_client());
        let resp = app
            .oneshot(json_post(
                "/api/messages/send",
                r#"{"phone":"15551234567","message":"  "}"#,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = body_json(resp.into_body()).await;
        assert!(json["error"].as_str().unwrap().contains("message"));
    }

    #[tokio::test]
    async fn send_rejects_malformed_json() {
        let app = create_router(connected_state_no_client());
        let resp = app
            .oneshot(json_post("/api/messages/send", r#"{ not json }"#))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn send_rejects_missing_fields() {
        let app = create_router(connected_state_no_client());
        let resp = app
            .oneshot(json_post(
                "/api/messages/send",
                r#"{"phone":"15551234567"}"#,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn send_returns_503_when_not_connected() {
        let app = create_router(pairing_state());
        let resp = app
            .oneshot(json_post(
                "/api/messages/send",
                r#"{"phone":"15551234567","message":"hello"}"#,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn send_returns_503_when_client_missing() {
        let app = create_router(connected_state_no_client());
        let resp = app
            .oneshot(json_post(
                "/api/messages/send",
                r#"{"phone":"15551234567","message":"hello"}"#,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn send_rejects_non_digit_phone() {
        let app = create_router(connected_state_no_client());
        let resp = app
            .oneshot(json_post(
                "/api/messages/send",
                r#"{"phone":"abc","message":"hello"}"#,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = body_json(resp.into_body()).await;
        assert!(json["error"].as_str().unwrap().contains("digit"));
    }

    #[tokio::test]
    async fn send_accepts_jid_shape() {
        let app = create_router(connected_state_no_client());
        let resp = app
            .oneshot(json_post(
                "/api/messages/send",
                r#"{"jid":"15551234567@s.whatsapp.net","message":"hello"}"#,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn messages_endpoint_returns_404_for_unknown_chat() {
        let app = create_router(pairing_state());
        let resp = app
            .oneshot(
                Request::get("/api/chats/unknown/messages")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
