use std::sync::Arc;

use axum::{extract::State, routing::get, Router};
use wacore::types::events::Event;
use whatsapp::{bot::Bot, store::SqliteStore, Client};
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;

/// Shared application state accessible from all Axum route handlers.
struct AppState {
    wa_client: Arc<Client>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Allow the database path to be configured via an environment variable.
    let db_path = std::env::var("WHATSAPP_DB_PATH").unwrap_or_else(|_| "whatsapp.db".to_string());
    let backend = Arc::new(SqliteStore::new(&db_path).await?);

    let mut bot = Bot::builder()
        .with_backend(backend)
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_http_client(UreqHttpClient::new())
        .on_event(|event, _client| async move {
            match event {
                // Print the QR code to stdout so the operator can scan it.
                Event::PairingQrCode { code, .. } => {
                    println!("Scan the QR code below to log in:\n{code}");
                }
                Event::Message(msg, info) => {
                    println!("Message from {}: {:?}", info.source.sender, msg);
                }
                _ => {}
            }
        })
        .build()
        .await?;

    // Extract a clonable handle to the client before moving `bot` into the task.
    let wa_client: Arc<Client> = bot.client();

    // Run the WhatsApp connection loop in the background.
    tokio::spawn(async move {
        match bot.run().await {
            Ok(handle) => {
                if let Err(e) = handle.await {
                    eprintln!("WhatsApp connection task panicked: {e}");
                }
            }
            Err(e) => {
                eprintln!("WhatsApp bot error: {e}");
            }
        }
    });

    let state = Arc::new(AppState { wa_client });

    let app = Router::new()
        .route("/", get(root))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Backend listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn root(State(state): State<Arc<AppState>>) -> String {
    format!(
        "WhatsApp Rust Web – backend is running! Client connected: {}",
        state.wa_client.is_connected()
    )
}
