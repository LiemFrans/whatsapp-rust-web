//! Webhook client for dispatching events to external systems (e.g., Cakap/Chatwoot).
//!
//! When a WhatsApp session has a `webhook_url` configured (either per-session or
//! from the global `WEBHOOK_URL` env var), incoming messages and status updates
//! are POSTed to that URL in a format compatible with the Cakap
//! `Webhooks::WhatsappRustController`.
//!
//! Payload format:
//! ```json
//! {
//!   "phone_number": "+6281234567890",
//!   "event": "message" | "status",
//!   "data": { ... }
//! }
//! ```

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use sqlx::PgPool;
use tracing::{error, warn, debug};
use uuid::Uuid;

use crate::config::AppConfig;

/// Shared webhook dispatcher, cheaply cloneable.
#[derive(Clone)]
pub struct WebhookClient {
    http: reqwest::Client,
    config: Arc<AppConfig>,
    db: PgPool,
}

impl WebhookClient {
    pub fn new(config: Arc<AppConfig>, db: PgPool) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { http, config, db }
    }

    /// Dispatch a webhook event for the given session. Looks up the session's
    /// webhook_url (falls back to global config). Returns immediately —
    /// the actual HTTP POST runs in a background task.
    pub fn dispatch(&self, session_id: Uuid, phone_number: &str, event: &str, data: Value) {
        let client = self.clone();
        let phone = phone_number.to_string();
        let evt = event.to_string();

        tokio::spawn(async move {
            if let Err(e) = client.send_webhook(session_id, &phone, &evt, data).await {
                error!(%session_id, "Webhook delivery failed: {}", e);
            }
        });
    }

    async fn send_webhook(
        &self,
        session_id: Uuid,
        phone_number: &str,
        event: &str,
        data: Value,
    ) -> Result<(), String> {
        // Look up per-session webhook config
        let (webhook_url, webhook_token) = self.resolve_webhook_config(session_id).await;

        let webhook_url = match webhook_url {
            Some(url) if !url.is_empty() => url,
            _ => {
                debug!(%session_id, "No webhook URL configured, skipping");
                return Ok(());
            }
        };

        let webhook_url = webhook_url.replace("{phone_number}", phone_number);

        let payload = serde_json::json!({
            "phone_number": phone_number,
            "event": event,
            "data": data,
        });

        debug!(%session_id, %webhook_url, %event, "Sending webhook");

        // Retry up to 3 times with exponential backoff
        let mut last_err = String::new();
        for attempt in 0..3u32 {
            if attempt > 0 {
                let delay = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                tokio::time::sleep(delay).await;
            }

            let mut req = self
                .http
                .post(&webhook_url)
                .json(&payload);

            if let Some(ref token) = webhook_token {
                req = req.header("Authorization", format!("Bearer {}", token));
            }

            match req.send().await {
                Ok(resp) if resp.status().is_success() => {
                    debug!(%session_id, %webhook_url, "Webhook delivered successfully");
                    return Ok(());
                }
                Ok(resp) => {
                    last_err = format!("HTTP {}", resp.status());
                    warn!(
                        %session_id, %webhook_url, attempt,
                        "Webhook returned {}", resp.status()
                    );
                }
                Err(e) => {
                    last_err = e.to_string();
                    warn!(
                        %session_id, %webhook_url, attempt,
                        "Webhook request failed: {}", e
                    );
                }
            }
        }

        Err(format!("All 3 attempts failed: {}", last_err))
    }

    /// Resolve webhook URL and token: per-session overrides global config.
    async fn resolve_webhook_config(&self, session_id: Uuid) -> (Option<String>, Option<String>) {
        // Try per-session config first
        let row = sqlx::query_as::<_, (Option<String>, Option<String>)>(
            "SELECT webhook_url, webhook_token FROM whatsapp_sessions WHERE id = $1",
        )
        .bind(session_id)
        .fetch_optional(&self.db)
        .await
        .ok()
        .flatten();

        if let Some((Some(url), token)) = row {
            if !url.is_empty() {
                return (Some(url), token);
            }
        }

        // Fall back to global config
        (
            self.config.webhook_url.clone(),
            self.config.webhook_token.clone(),
        )
    }
}

/// Build the webhook payload for an incoming message event.
/// This format matches what Cakap's `Webhooks::WhatsappRustEventsJob` expects.
pub fn build_message_payload(
    message_id: &str,
    from: &str,
    from_name: Option<&str>,
    to: &str,
    timestamp: &chrono::DateTime<chrono::Utc>,
    msg_type: &str,
    content: Option<&str>,
    media_url: Option<&str>,
    media_mime: Option<&str>,
    media_filename: Option<&str>,
) -> Value {
    let mut data = serde_json::json!({
        "message_id": message_id,
        "from": from,
        "from_name": from_name.unwrap_or(from),
        "to": to,
        "timestamp": timestamp.to_rfc3339(),
        "type": msg_type,
    });

    // Add type-specific data
    match msg_type {
        "text" => {
            data["text"] = serde_json::json!({ "body": content.unwrap_or("") });
        }
        "image" | "video" => {
            data[msg_type] = serde_json::json!({
                "url": media_url,
                "caption": content,
                "mime_type": media_mime,
            });
        }
        "audio" => {
            data["audio"] = serde_json::json!({
                "url": media_url,
                "mime_type": media_mime,
            });
        }
        "document" => {
            data["document"] = serde_json::json!({
                "url": media_url,
                "filename": media_filename,
                "mime_type": media_mime,
            });
        }
        "location" => {
            // content is in format "Location: lat, lng"
            if let Some(text) = content {
                let parts: Vec<&str> = text.trim_start_matches("Location: ").split(", ").collect();
                if parts.len() == 2 {
                    data["location"] = serde_json::json!({
                        "latitude": parts[0].parse::<f64>().unwrap_or(0.0),
                        "longitude": parts[1].parse::<f64>().unwrap_or(0.0),
                    });
                }
            }
        }
        "contact" => {
            data["text"] = serde_json::json!({ "body": content.unwrap_or("Contact") });
        }
        _ => {
            // For unknown types, send as text fallback
            if let Some(text) = content {
                data["text"] = serde_json::json!({ "body": text });
            }
        }
    }

    data
}

/// Build the webhook payload for a message status update.
pub fn build_status_payload(
    message_id: &str,
    status: &str,
    error_code: Option<i32>,
    error_title: Option<&str>,
) -> Value {
    let mut data = serde_json::json!({
        "message_id": message_id,
        "status": status,
    });

    if let Some(code) = error_code {
        data["error"] = serde_json::json!({
            "code": code,
            "title": error_title.unwrap_or("Unknown error"),
        });
    }

    data
}
