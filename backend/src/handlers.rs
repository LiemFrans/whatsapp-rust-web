//! HTTP handler functions for the Axum router.

use std::sync::atomic::Ordering;
use std::io::Read;

use axum::extract::{Path, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use whatsapp_rust::download::MediaType;
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
            // Mark group as warmed (SKDM was distributed with this message)
            if jid_is_group(&target_jid) {
                state.warmed_groups.write().await.insert(target_jid.to_non_ad().to_string());
            }
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

// ---------------------------------------------------------------------------
// Stickers listing
// ---------------------------------------------------------------------------

pub async fn get_stickers(
    State(state): State<AppState>,
) -> Json<StickersResponse> {
    let store = state.store.read().await;
    Json(StickersResponse {
        stickers: store.recent_stickers(),
    })
}

// ---------------------------------------------------------------------------
// Send media (sticker / GIF / image)
// ---------------------------------------------------------------------------

pub async fn send_media(
    State(state): State<AppState>,
    Json(payload): Json<SendMediaRequest>,
) -> Result<Json<SendMessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    if payload.url.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "\"url\" must not be empty".into(),
            }),
        ));
    }

    let requested_jid = payload
        .jid
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| {
            v.parse::<Jid>().map_err(|_| {
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
                .and_then(|store| store.contacts.get(&jid_key).and_then(|c| c.phone.clone()))
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
            let phone = resolved_phone
                .clone()
                .filter(|v| v.chars().all(|c| c.is_ascii_digit()))
                .ok_or_else(|| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: "Direct chats require a valid phone number.".into(),
                        }),
                    )
                })?;
            Jid::pn(&phone)
        }
        None => {
            let phone = resolved_phone
                .clone()
                .filter(|v| v.chars().all(|c| c.is_ascii_digit()))
                .ok_or_else(|| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: "Provide a valid \"jid\" or \"phone\".".into(),
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

    // Fetch the media bytes — either from local store or external URL
    let media_bytes = if payload.url.starts_with("/api/media/") {
        // Local media: download from our own store via the WA client
        let parts: Vec<&str> = payload.url.trim_start_matches("/api/media/").splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Invalid local media path".into(),
                }),
            ));
        }
        let (media_chat_jid, media_msg_id) = (parts[0], parts[1]);
        let blob = {
            let store = state.store.read().await;
            store.media_for(media_chat_jid, media_msg_id)
        }
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Local media not found in store".into(),
                }),
            )
        })?;
        client
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
                        error: format!("Failed to download local media: {e}"),
                    }),
                )
            })?
    } else {
        // External URL: fetch via ureq (blocking — must run on spawn_blocking)
        let url = payload.url.clone();
        log::info!("Fetching external media: {url}");
        tokio::task::spawn_blocking(move || -> Result<Vec<u8>, (StatusCode, Json<ErrorResponse>)> {
            let resp = ureq::get(&url).call().map_err(|e| {
                log::error!("Failed to fetch media URL: {e}");
                (
                    StatusCode::BAD_GATEWAY,
                    Json(ErrorResponse {
                        error: format!("Failed to download media from URL: {e}"),
                    }),
                )
            })?;
            let len = resp
                .header("Content-Length")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(2_000_000);
            let mut buf = Vec::with_capacity(len.min(20_000_000));
            resp.into_reader()
                .read_to_end(&mut buf)
                .map_err(|e| {
                    (
                        StatusCode::BAD_GATEWAY,
                        Json(ErrorResponse {
                            error: format!("Failed to read media: {e}"),
                        }),
                    )
                })?;
            log::info!("Downloaded {} bytes from external URL", buf.len());
            Ok(buf)
        })
        .await
        .map_err(|e| {
            log::error!("spawn_blocking join error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Internal error: {e}"),
                }),
            )
        })??
    };

    let (media_type, mime_str) = match payload.media_type.as_str() {
        "sticker" => (MediaType::Sticker, "image/webp"),
        "gif" => (MediaType::Video, "video/mp4"),
        "image" => (MediaType::Image, "image/jpeg"),
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Unsupported media_type: {other}"),
                }),
            ));
        }
    };

    // ── Group SKDM pre-warming ─────────────────────────────────────────
    // When sending media to a group for the first time in this session,
    // send a lightweight text message first so the sender-key distribution
    // message (SKDM) gets delivered to all members. Then wait a few
    // seconds for retries to settle before sending the heavy media. This
    // prevents the media message itself from being caught in a retry
    // storm that could trigger a temporary ban.
    if jid_is_group(&target_jid) {
        let group_key = target_jid.to_non_ad().to_string();
        let needs_warmup = !state.warmed_groups.read().await.contains(&group_key);
        if needs_warmup {
            log::info!(
                "Group {} not yet warmed — sending SKDM warm-up text first",
                group_key
            );
            let warmup = wa::Message {
                conversation: Some("\u{200B}".to_string()), // zero-width space
                ..Default::default()
            };
            match client.send_message(target_jid.clone(), warmup).await {
                Ok(warmup_id) => {
                    log::info!(
                        "Warm-up sent ({warmup_id}), waiting 4 s for SKDM distribution…"
                    );
                    state.warmed_groups.write().await.insert(group_key);
                    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                }
                Err(e) => {
                    log::warn!("Warm-up send failed ({e}), proceeding with media anyway");
                }
            }
        }
    }

    // Upload media to WhatsApp servers
    let data_len = media_bytes.len();
    log::info!("Uploading {} bytes as {:?} to WhatsApp", data_len, media_type);
    let upload = client
        .upload(media_bytes, media_type)
        .await
        .map_err(|e| {
            log::error!("Failed to upload media: {e}");
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: format!("Upload failed: {e}"),
                }),
            )
        })?;

    log::info!(
        "Upload OK — url={}, direct_path={}, file_length={}, key_len={}, sha256_len={}",
        upload.url,
        upload.direct_path,
        upload.file_length,
        upload.media_key.len(),
        upload.file_sha256.len(),
    );

    // Build the appropriate message (matches whatsapp-rust e2e test patterns)
    let wa_message = match payload.media_type.as_str() {
        "sticker" => wa::Message {
            sticker_message: Some(Box::new(wa::message::StickerMessage {
                url: Some(upload.url),
                direct_path: Some(upload.direct_path),
                media_key: Some(upload.media_key),
                file_sha256: Some(upload.file_sha256),
                file_enc_sha256: Some(upload.file_enc_sha256),
                file_length: Some(upload.file_length),
                mimetype: Some(mime_str.to_string()),
                is_animated: Some(false),
                width: payload.width,
                height: payload.height,
                ..Default::default()
            })),
            ..Default::default()
        },
        "gif" => wa::Message {
            video_message: Some(Box::new(wa::message::VideoMessage {
                url: Some(upload.url),
                direct_path: Some(upload.direct_path),
                media_key: Some(upload.media_key),
                file_sha256: Some(upload.file_sha256),
                file_enc_sha256: Some(upload.file_enc_sha256),
                file_length: Some(upload.file_length),
                mimetype: Some("video/mp4".to_string()),
                gif_playback: Some(true),
                gif_attribution: Some(2), // TENOR = 2
                caption: payload.caption.clone(),
                width: payload.width,
                height: payload.height,
                ..Default::default()
            })),
            ..Default::default()
        },
        "image" | _ => wa::Message {
            image_message: Some(Box::new(wa::message::ImageMessage {
                url: Some(upload.url),
                direct_path: Some(upload.direct_path),
                media_key: Some(upload.media_key),
                file_sha256: Some(upload.file_sha256),
                file_enc_sha256: Some(upload.file_enc_sha256),
                file_length: Some(upload.file_length),
                mimetype: Some(mime_str.to_string()),
                caption: payload.caption.clone(),
                width: payload.width,
                height: payload.height,
                ..Default::default()
            })),
            ..Default::default()
        },
    };

    let media_label = payload.media_type.clone();

    log::info!(
        "Sending {media_label} message ({data_len} bytes) to {target_jid} (chat {chat_store_jid}) — \
         Note: retry receipts are normal for groups on first message from a new session"
    );
    match client.send_message(target_jid.clone(), wa_message).await {
        Ok(msg_id) => {
            // Ensure group is marked warmed after media send too
            if jid_is_group(&target_jid) {
                state.warmed_groups.write().await.insert(target_jid.to_non_ad().to_string());
            }
            let is_group = requested_jid.as_ref().map(jid_is_group).unwrap_or(false);
            let phone = if is_group {
                None
            } else {
                resolved_phone.clone().or_else(|| jid_phone(&target_jid.to_non_ad()))
            };

            let current_display_name = {
                let store = state.store.read().await;
                store.display_name_for_jid(&chat_store_jid)
            };
            let mut store = state.store.write().await;
            let timestamp_ms = now_ms();

            let preview_text = format!("[{media_label}]");
            store.record_message(
                chat_store_jid.clone(),
                Some(current_display_name),
                phone,
                is_group,
                ChatMessage {
                    id: msg_id.clone(),
                    chat_jid: chat_store_jid,
                    sender_jid: "me".into(),
                    sender_name: Some("You".into()),
                    text: preview_text,
                    timestamp_ms,
                    from_me: true,
                    mentions: vec![],
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
            log::error!("Failed to send media: {e:?}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to send media: {e}"),
                }),
            ))
        }
    }
}
