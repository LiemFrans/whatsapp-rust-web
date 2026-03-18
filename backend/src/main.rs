use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{HeaderValue, Method, header},
    response::IntoResponse,
    routing::{get, post},
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex};
use tower_http::cors::CorsLayer;
use wacore::types::events::Event;
use whatsapp_rust::{Jid, bot::Bot, store::SqliteStore, Client};
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;

// ── JSON types sent over WebSocket ─────────────────────────────────────────

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type")]
enum WsOutgoing {
    #[serde(rename = "qr")]
    Qr { code: String },
    #[serde(rename = "connected")]
    Connected,
    #[serde(rename = "disconnected")]
    Disconnected,
    #[serde(rename = "pair_success")]
    PairSuccess,
    #[serde(rename = "logged_out")]
    LoggedOut,
    #[serde(rename = "message")]
    Message {
        id: String,
        from: String,
        from_name: String,
        chat: String,
        text: String,
        timestamp: i64,
        is_from_me: bool,
    },
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum WsIncoming {
    #[serde(rename = "send_msg")]
    SendMsg { to: String, body: String },
}

// ── Application state ──────────────────────────────────────────────────────

struct AppState {
    wa_client: Arc<Client>,
    event_tx: broadcast::Sender<String>,
    qr_code: Arc<Mutex<Option<String>>>,
    connection_state: Arc<Mutex<String>>,
}

// ── Main ───────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let db_path = std::env::var("WHATSAPP_DB_PATH").unwrap_or_else(|_| "whatsapp.db".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8787".to_string());
    let backend = Arc::new(SqliteStore::new(&db_path).await?);

    // Create broadcast channel for WebSocket event fanout
    let (event_tx, _) = broadcast::channel::<String>(256);
    let qr_code: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let connection_state: Arc<Mutex<String>> = Arc::new(Mutex::new("disconnected".to_string()));

    // Clone shared state for the event handler closure
    let tx = event_tx.clone();
    let qr = qr_code.clone();
    let conn = connection_state.clone();

    let mut bot = Bot::builder()
        .with_backend(backend)
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_http_client(UreqHttpClient::new())
        .on_event(move |event, _client| {
            let tx = tx.clone();
            let qr = qr.clone();
            let conn = conn.clone();
            async move {
                match event {
                    Event::PairingQrCode { code, .. } => {
                        log::info!("QR code received");
                        *qr.lock().await = Some(code.clone());
                        *conn.lock().await = "scanning".to_string();
                        let evt = WsOutgoing::Qr { code };
                        let _ = tx.send(serde_json::to_string(&evt).unwrap());
                    }
                    Event::Connected(_) => {
                        log::info!("WhatsApp connected");
                        *conn.lock().await = "connected".to_string();
                        *qr.lock().await = None;
                        let _ = tx.send(serde_json::to_string(&WsOutgoing::Connected).unwrap());
                    }
                    Event::Disconnected(_) => {
                        log::info!("WhatsApp disconnected");
                        *conn.lock().await = "disconnected".to_string();
                        let _ =
                            tx.send(serde_json::to_string(&WsOutgoing::Disconnected).unwrap());
                    }
                    Event::PairSuccess(_) => {
                        log::info!("Pairing successful");
                        *conn.lock().await = "connected".to_string();
                        *qr.lock().await = None;
                        let _ =
                            tx.send(serde_json::to_string(&WsOutgoing::PairSuccess).unwrap());
                    }
                    Event::LoggedOut(_) => {
                        log::info!("Logged out");
                        *conn.lock().await = "disconnected".to_string();
                        *qr.lock().await = None;
                        let _ = tx.send(serde_json::to_string(&WsOutgoing::LoggedOut).unwrap());
                    }
                    Event::Message(msg, info) => {
                        let text = extract_text(&msg);
                        if text.is_empty() {
                            return;
                        }
                        let ws_msg = WsOutgoing::Message {
                            id: info.id.clone(),
                            from: info.source.sender.to_string(),
                            from_name: info.push_name.clone(),
                            chat: info.source.chat.to_string(),
                            text,
                            timestamp: info.timestamp.timestamp(),
                            is_from_me: info.source.is_from_me,
                        };
                        log::info!("Message from {}: {:?}", info.source.sender, ws_msg);
                        let _ = tx.send(serde_json::to_string(&ws_msg).unwrap());
                    }
                    _ => {}
                }
            }
        })
        .build()
        .await?;

    let wa_client = bot.client();

    // Run the WhatsApp bot in the background
    tokio::spawn(async move {
        match bot.run().await {
            Ok(handle) => {
                if let Err(e) = handle.await {
                    log::error!("WhatsApp task panicked: {e}");
                }
            }
            Err(e) => log::error!("WhatsApp bot error: {e}"),
        }
    });

    let state = Arc::new(AppState {
        wa_client,
        event_tx,
        qr_code,
        connection_state,
    });

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/", get(root))
        .route("/api/auth/qr", get(get_qr))
        .route("/api/auth/status", get(get_status))
        .route("/api/send", post(send_message))
        .route("/ws", get(ws_handler))
        .layer(cors)
        .with_state(state);

    let bind_addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    log::info!("Backend listening on http://{bind_addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn extract_text(msg: &whatsapp_rust::waproto::whatsapp::Message) -> String {
    if let Some(ref text) = msg.conversation {
        return text.clone();
    }
    if let Some(ref ext) = msg.extended_text_message {
        if let Some(ref text) = ext.text {
            return text.clone();
        }
    }
    String::new()
}

fn parse_jid(s: &str) -> Option<Jid> {
    if s.contains('@') {
        s.parse().ok()
    } else {
        format!("{s}@s.whatsapp.net").parse().ok()
    }
}

// ── REST handlers ──────────────────────────────────────────────────────────

async fn root(State(state): State<Arc<AppState>>) -> String {
    format!(
        "WhatsApp Rust Web – backend running! Connected: {}",
        state.wa_client.is_connected()
    )
}

#[derive(Serialize)]
struct QrResponse {
    qr_code: Option<String>,
    connection_state: String,
}

async fn get_qr(State(state): State<Arc<AppState>>) -> Json<QrResponse> {
    Json(QrResponse {
        qr_code: state.qr_code.lock().await.clone(),
        connection_state: state.connection_state.lock().await.clone(),
    })
}

#[derive(Serialize)]
struct StatusResponse {
    connected: bool,
    state: String,
}

async fn get_status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    Json(StatusResponse {
        connected: state.wa_client.is_connected(),
        state: state.connection_state.lock().await.clone(),
    })
}

#[derive(Deserialize)]
struct SendRequest {
    to: String,
    body: String,
}

#[derive(Serialize)]
struct SendResponse {
    ok: bool,
    error: Option<String>,
}

async fn send_message(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendRequest>,
) -> Json<SendResponse> {
    let jid = match parse_jid(&req.to) {
        Some(j) => j,
        None => {
            return Json(SendResponse {
                ok: false,
                error: Some("Invalid JID format".into()),
            });
        }
    };

    let msg = whatsapp_rust::waproto::whatsapp::Message {
        conversation: Some(req.body),
        ..Default::default()
    };

    match state.wa_client.send_message(jid, msg).await {
        Ok(_) => Json(SendResponse {
            ok: true,
            error: None,
        }),
        Err(e) => Json(SendResponse {
            ok: false,
            error: Some(e.to_string()),
        }),
    }
}

// ── WebSocket handler ──────────────────────────────────────────────────────

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Send current state immediately so the client knows where we stand
    let current_state = state.connection_state.lock().await.clone();
    let init_event = match current_state.as_str() {
        "connected" => WsOutgoing::Connected,
        "scanning" => {
            if let Some(code) = state.qr_code.lock().await.clone() {
                WsOutgoing::Qr { code }
            } else {
                WsOutgoing::Disconnected
            }
        }
        _ => WsOutgoing::Disconnected,
    };
    let _ = sender
        .send(WsMessage::Text(
            serde_json::to_string(&init_event).unwrap().into(),
        ))
        .await;

    // Subscribe to event broadcast
    let mut rx = state.event_tx.subscribe();

    // Forward events to WebSocket client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(WsMessage::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages from WebSocket client
    let state_clone = state.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                WsMessage::Text(text) => {
                    if let Ok(incoming) = serde_json::from_str::<WsIncoming>(&text) {
                        handle_incoming_ws(&incoming, &state_clone).await;
                    }
                }
                WsMessage::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}

async fn handle_incoming_ws(incoming: &WsIncoming, state: &Arc<AppState>) {
    match incoming {
        WsIncoming::SendMsg { to, body } => {
            if let Some(jid) = parse_jid(to) {
                let msg = whatsapp_rust::waproto::whatsapp::Message {
                    conversation: Some(body.clone()),
                    ..Default::default()
                };
                if let Err(e) = state.wa_client.send_message(jid, msg).await {
                    log::error!("Failed to send message: {e}");
                }
            } else {
                log::warn!("Invalid JID in send_msg: {to}");
            }
        }
    }
}
