use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::models::user::UserRole;

use crate::auth::jwt;
use crate::AppState;

const CHANNEL_CAPACITY: usize = 256;

/// Central WebSocket broadcast hub.
/// Maintains per-user and per-role broadcast channels.
pub struct WebSocketHub {
    /// Per-user channels: user_id -> broadcast sender
    user_channels: DashMap<Uuid, broadcast::Sender<String>>,
    /// Per-role channels: role string -> broadcast sender
    role_channels: DashMap<String, broadcast::Sender<String>>,
}

impl WebSocketHub {
    pub fn new() -> Self {
        Self {
            user_channels: DashMap::new(),
            role_channels: DashMap::new(),
        }
    }

    /// Get or create a broadcast channel for a user
    pub fn subscribe_user(&self, user_id: Uuid) -> broadcast::Receiver<String> {
        let entry = self
            .user_channels
            .entry(user_id)
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0);
        entry.subscribe()
    }

    /// Get or create a broadcast channel for a role
    pub fn subscribe_role(&self, role: &str) -> broadcast::Receiver<String> {
        let entry = self
            .role_channels
            .entry(role.to_string())
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0);
        entry.subscribe()
    }

    /// Send a message to a specific user
    pub fn send_to_user(&self, user_id: Uuid, msg: serde_json::Value) {
        if let Some(tx) = self.user_channels.get(&user_id) {
            let _ = tx.send(msg.to_string());
        }
    }

    /// Send a message to all users of a specific role
    pub fn send_to_role(&self, role: &str, msg: serde_json::Value) {
        if let Some(tx) = self.role_channels.get(role) {
            let _ = tx.send(msg.to_string());
        }
    }

    /// Broadcast to all connected users
    pub fn broadcast(&self, msg: serde_json::Value) {
        let text = msg.to_string();
        for entry in self.user_channels.iter() {
            let _ = entry.value().send(text.clone());
        }
    }

    /// Remove a user channel (cleanup)
    pub fn remove_user(&self, user_id: &Uuid) {
        self.user_channels.remove(user_id);
    }
}

#[derive(Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
}

/// WebSocket upgrade handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
) -> impl IntoResponse {
    // Authenticate via query token
    let token = match query.token {
        Some(t) => t,
        None => {
            return axum::response::Response::builder()
                .status(401)
                .body(axum::body::Body::from("Missing token"))
                .unwrap()
                .into_response();
        }
    };

    let claims = match jwt::validate_token(&token, &state.config.jwt_secret) {
        Ok(c) => c,
        Err(_) => {
            return axum::response::Response::builder()
                .status(401)
                .body(axum::body::Body::from("Invalid token"))
                .unwrap()
                .into_response();
        }
    };

    let user_id = claims.sub;
    let role = format!("{:?}", claims.role).to_lowercase();

    ws.on_upgrade(move |socket| handle_socket(socket, state, user_id, role))
        .into_response()
}

/// Handle an individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState, user_id: Uuid, role: String) {
    info!(%user_id, %role, "WebSocket connected");

    let ws_hub = &state.ws_hub;

    // Subscribe to user-specific and role-specific channels
    let mut user_rx = ws_hub.subscribe_user(user_id);
    let mut role_rx = ws_hub.subscribe_role(&role);

    let (mut ws_tx, mut ws_rx) = socket.split();

    // Spawn task to forward broadcast messages to the WebSocket client
    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = user_rx.recv() => {
                    match msg {
                        Ok(text) => {
                            if ws_tx.send(Message::Text(text.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!(%user_id, "WebSocket lagged {} messages", n);
                        }
                        Err(_) => break,
                    }
                }
                msg = role_rx.recv() => {
                    match msg {
                        Ok(text) => {
                            if ws_tx.send(Message::Text(text.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!(%user_id, "WebSocket role channel lagged {} messages", n);
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    });

    // Receive loop (handle incoming WS messages from client, e.g., pings)
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // Could handle client-sent commands here
                    let _ = text;
                }
                Ok(Message::Close(_)) => break,
                Err(e) => {
                    error!(%user_id, "WebSocket receive error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    info!(%user_id, "WebSocket disconnected");
}
