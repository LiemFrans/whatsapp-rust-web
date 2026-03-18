//! HTTP handler functions for the Axum router.

use std::sync::atomic::Ordering;

use axum::extract::{Path, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use whatsapp_rust::waproto::whatsapp as wa;
use whatsapp_rust::Jid;

use crate::helpers::*;
use crate::models::*;
use crate::sync::resolve_mention_summaries;

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

pub async fn get_qr(State(state): State<AppState>) -> Json<QrResponse> {
    let qr = state.qr_code.read().await;
    Json(QrResponse {
        qr_code: qr.clone(),
        is_connected: state.is_connected.load(Ordering::SeqCst),
    })
}

pub async fn get_status(State(state): State<AppState>) -> Json<StatusResponse> {
    Json(StatusResponse {
        is_connected: state.is_connected.load(Ordering::SeqCst),
        is_syncing: state.is_syncing.load(Ordering::SeqCst),
    })
}

pub async fn logout(
    State(state): State<AppState>,
) -> Result<Json<LogoutResponse>, (StatusCode, Json<ErrorResponse>)> {
    let client = state.client.read().await.clone();

    if let Some(client) = client {
        client.enable_auto_reconnect.store(false, Ordering::SeqCst);
        client.disconnect().await;
    }

    let _ = tokio::fs::remove_file(state.db_path.as_str()).await;

    state.is_connected.store(false, Ordering::SeqCst);
    state.is_syncing.store(false, Ordering::SeqCst);
    *state.qr_code.write().await = None;
    *state.client.write().await = None;
    state.store.write().await.reset();

    Ok(Json(LogoutResponse {
        success: true,
        message: "Disconnected. Restart the application to pair again if needed.".into(),
    }))
}

// ---------------------------------------------------------------------------
// Bootstrap / listing
// ---------------------------------------------------------------------------

pub async fn get_bootstrap(State(state): State<AppState>) -> Json<BootstrapResponse> {
    let qr = state.qr_code.read().await.clone();
    let store = state.store.read().await;
    Json(BootstrapResponse {
        qr_code: qr,
        is_connected: state.is_connected.load(Ordering::SeqCst),
        is_syncing: state.is_syncing.load(Ordering::SeqCst),
        chats: store.sorted_chats(),
        contacts: store.sorted_contacts(),
        logout_hint: Some("Logout disconnects the current runtime session. Restart the app if WhatsApp asks for a fresh link.".into()),
    })
}

pub async fn get_chats(State(state): State<AppState>) -> Json<ChatsResponse> {
    let store = state.store.read().await;
    Json(ChatsResponse {
        chats: store.sorted_chats(),
    })
}

pub async fn get_contacts(State(state): State<AppState>) -> Json<ContactsResponse> {
    let store = state.store.read().await;
    Json(ContactsResponse {
        contacts: store.sorted_contacts(),
    })
}

pub async fn get_chat_messages(
    Path(chat_jid): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<MessagesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = state.store.read().await;
    let messages = store.messages_for(&chat_jid).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Chat not found".into(),
            }),
        )
    })?;
    Ok(Json(MessagesResponse { messages }))
}

pub async fn mark_chat_read(
    Path(chat_jid): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut store = state.store.write().await;
    if !store.chats.contains_key(&chat_jid) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Chat not found".into(),
            }),
        ));
    }
    store.mark_read(&chat_jid);
    Ok(Json(serde_json::json!({ "success": true })))
}

// ---------------------------------------------------------------------------
// Typing indicator
// ---------------------------------------------------------------------------

pub async fn update_typing(
    Path(chat_jid): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<TypingRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if !state.is_connected.load(Ordering::SeqCst) {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "WhatsApp is not connected.".into(),
            }),
        ));
    }

    let client_guard = state.client.read().await;
    let client = client_guard.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "WhatsApp client not yet initialised".into(),
            }),
        )
    })?;

    let jid: Jid = chat_jid.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid chat JID".into(),
            }),
        )
    })?;

    let result = match payload.state.as_str() {
        "composing" => client.chatstate().send_composing(&jid).await,
        "recording" => client.chatstate().send_recording(&jid).await,
        _ => client.chatstate().send_paused(&jid).await,
    };

    result.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to update typing state: {e}"),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({ "success": true })))
}

// ---------------------------------------------------------------------------
// Media download proxy
// ---------------------------------------------------------------------------

pub async fn get_media(
    Path((chat_jid, message_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let blob = {
        let store = state.store.read().await;
        store.media_for(&chat_jid, &message_id)
    }
    .ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Media not found".into(),
            }),
        )
    })?;

    let client_guard = state.client.read().await;
    let client = client_guard.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "WhatsApp client not yet initialised".into(),
            }),
        )
    })?;

    let bytes = client
        .download_from_params(
            &blob.direct_path,
            &blob.media_key,
            &blob.file_sha256,
            &blob.file_enc_sha256,
            blob.file_length,
            blob.media_type,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: format!("Failed to download media: {e}"),
                }),
            )
        })?;

    let mut response = bytes.into_response();
    let mime_type = blob
        .mime_type
        .clone()
        .unwrap_or_else(|| default_mime_for_media_type(blob.media_type).to_string());
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&mime_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    if let Some(file_name) = blob.file_name {
        if let Ok(value) = HeaderValue::from_str(&format!("inline; filename=\"{file_name}\"")) {
            response.headers_mut().insert(header::CONTENT_DISPOSITION, value);
        }
    }

    Ok(response)
}

// ---------------------------------------------------------------------------
// Send message
// ---------------------------------------------------------------------------

pub async fn send_message(
    State(state): State<AppState>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<Json<SendMessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let trimmed_message = payload.message.trim();
    if trimmed_message.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "\"message\" must not be empty".into(),
            }),
        ));
    }

    let requested_jid = payload
        .jid
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value.parse::<Jid>().map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "\"jid\" is invalid".into(),
                    }),
                )
            })
        })
        .transpose()?;

    let explicit_phone = normalize_phone(payload.phone.as_deref().unwrap_or_default());
    let requested_jid_phone = requested_jid.as_ref().and_then(jid_phone);
    let fallback_store_phone = if explicit_phone.is_empty() {
        requested_jid.as_ref().and_then(|jid| {
            let jid_key = jid.to_non_ad().to_string();
            state
                .store
                .try_read()
                .ok()
                .and_then(|store| store.contacts.get(&jid_key).and_then(|contact| contact.phone.clone()))
        })
    } else {
        None
    };

    let resolved_phone = if explicit_phone.is_empty() {
        requested_jid_phone.or(fallback_store_phone)
    } else {
        Some(explicit_phone.clone())
    };

    let target_jid = match requested_jid.clone() {
        Some(jid) if jid_is_group(&jid) => jid,
        Some(_) => {
            let phone = resolved_phone.clone().filter(|value| value.chars().all(|c| c.is_ascii_digit())).ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "Direct chats require a valid mapped phone number before sending.".into(),
                    }),
                )
            })?;
            Jid::pn(&phone)
        }
        None => {
            let phone = resolved_phone.clone().filter(|value| value.chars().all(|c| c.is_ascii_digit())).ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "Provide a valid \"jid\" or a digit-only \"phone\" (E.164 format).".into(),
                    }),
                )
            })?;
            Jid::pn(&phone)
        }
    };

    let chat_store_jid = requested_jid
        .as_ref()
        .map(|jid| jid.to_non_ad().to_string())
        .unwrap_or_else(|| target_jid.to_non_ad().to_string());

    let mention_jids: Vec<String> = payload
        .mentions
        .iter()
        .filter_map(|jid| jid.parse::<Jid>().ok().map(|parsed| parsed.to_non_ad().to_string()))
        .collect();

    if !state.is_connected.load(Ordering::SeqCst) {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "WhatsApp is not connected. Please scan the QR code first.".into(),
            }),
        ));
    }

    let client_guard = state.client.read().await;
    let client = client_guard.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "WhatsApp client not yet initialised".into(),
            }),
        )
    })?;

    let wa_message = if mention_jids.is_empty() {
        wa::Message {
            conversation: Some(trimmed_message.to_string()),
            ..Default::default()
        }
    } else {
        wa::Message {
            extended_text_message: Some(Box::new(wa::message::ExtendedTextMessage {
                text: Some(trimmed_message.to_string()),
                context_info: Some(Box::new(wa::ContextInfo {
                    mentioned_jid: mention_jids.clone(),
                    ..Default::default()
                })),
                ..Default::default()
            })),
            ..Default::default()
        }
    };

    log::info!(
        "Sending outbound message to {} (store chat {})",
        target_jid,
        chat_store_jid
    );

    match client.send_message(target_jid.clone(), wa_message).await {
        Ok(msg_id) => {
            let chat_jid = chat_store_jid;
            let phone = if jid_is_group(&target_jid) {
                None
            } else {
                resolved_phone.clone().or_else(|| jid_phone(&target_jid.to_non_ad()))
            };
            let is_group = requested_jid
                .as_ref()
                .map(jid_is_group)
                .unwrap_or_else(|| jid_is_group(&target_jid));
            let mention_summaries = resolve_mention_summaries(&state, client, &mention_jids).await;

            let current_display_name = {
                let store = state.store.read().await;
                store.display_name_for_jid(&chat_jid)
            };
            let mut store = state.store.write().await;
            let timestamp_ms = now_ms();

            if !is_group {
                store.upsert_contact(ContactSummary {
                    jid: chat_jid.clone(),
                    name: preferred_display_name(Some(&current_display_name), phone.as_deref(), &chat_jid),
                    phone: phone.clone(),
                    status: None,
                    avatar_url: None,
                    is_business: false,
                    is_registered: true,
                });
            }

            store.record_message(
                chat_jid.clone(),
                Some(current_display_name),
                phone,
                is_group,
                ChatMessage {
                    id: msg_id.clone(),
                    chat_jid,
                    sender_jid: "me".into(),
                    sender_name: Some("You".into()),
                    text: trimmed_message.to_string(),
                    timestamp_ms,
                    from_me: true,
                    mentions: mention_summaries,
                    media: None,
                    receipt_status: Some("sent".into()),
                },
                None,
            );

            Ok(Json(SendMessageResponse {
                success: true,
                message_id: Some(msg_id),
            }))
        }
        Err(e) => {
            log::error!("Failed to send message: {e:?}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to send: {e}"),
                }),
            ))
        }
    }
}
