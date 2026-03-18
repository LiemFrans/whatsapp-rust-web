//! WhatsApp event handling — processes incoming events from the client.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use whatsapp_rust::types::events::Event;
use whatsapp_rust::types::presence::{ChatPresence, ReceiptType};
use whatsapp_rust::waproto::whatsapp as wa;
use whatsapp_rust::{Client, Jid};

use crate::helpers::*;
use crate::models::*;
use crate::sync::*;

/// Central event dispatcher — called from the bot's `on_event` closure.
pub async fn handle_event(event: Event, client: Arc<Client>, state: AppState) {
    match event {
        Event::PairingQrCode { code, timeout } => {
            log::info!("QR code received (valid for {}s)", timeout.as_secs());
            state.is_connected.store(false, Ordering::SeqCst);
            state.is_syncing.store(false, Ordering::SeqCst);
            *state.qr_code.write().await = Some(code);
        }
        Event::Connected(_) => {
            log::info!("✅ WhatsApp connected!");
            state.is_connected.store(true, Ordering::SeqCst);
            *state.qr_code.write().await = None;
            tokio::spawn(wait_for_startup_sync(state.clone(), client.clone()));
        }
        Event::Disconnected(_) | Event::LoggedOut(_) | Event::ConnectFailure(_) => {
            state.is_connected.store(false, Ordering::SeqCst);
            state.is_syncing.store(false, Ordering::SeqCst);
        }
        Event::Message(msg, info) => {
            let preview = extract_text(&msg);
            log::info!("📩 Message from {}: {}", info.source.sender, preview);
            handle_incoming_message(state.clone(), client.clone(), msg, info).await;
        }
        Event::Receipt(receipt) => {
            let status = match receipt.r#type {
                ReceiptType::Sender => Some("sent"),
                ReceiptType::Delivered => Some("delivered"),
                ReceiptType::Read | ReceiptType::ReadSelf => Some("read"),
                ReceiptType::Played | ReceiptType::PlayedSelf => Some("played"),
                _ => None,
            };
            if let Some(status) = status {
                let mut store = state.store.write().await;
                store.update_message_receipt(
                    &receipt.source.chat.to_non_ad().to_string(),
                    &receipt.message_ids,
                    status,
                );
            }
        }
        Event::ChatPresence(update) => {
            let chat_jid = update.source.chat.to_non_ad().to_string();
            let typing = match update.state {
                ChatPresence::Composing => {
                    if update.source.is_group {
                        let sender_jid = update.source.sender.to_non_ad().to_string();
                        let sender_name = {
                            let store = state.store.read().await;
                            store.display_name_for_jid(&sender_jid)
                        };
                        Some(format!("{sender_name} is typing…"))
                    } else {
                        Some("typing…".into())
                    }
                }
                ChatPresence::Paused => None,
            };
            let mut store = state.store.write().await;
            store.set_typing(&chat_jid, typing);
        }
        Event::Presence(update) => {
            let mut store = state.store.write().await;
            store.set_presence(
                &update.from.to_non_ad().to_string(),
                !update.unavailable,
                update.last_seen.map(|value| value.timestamp_millis()),
            );
        }
        Event::JoinedGroup(conversation) => {
            if let Some(conv) = conversation.get() {
                let jid = conv.id.clone();
                let name = conv
                    .name
                    .clone()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| jid.clone());
                let mut store = state.store.write().await;
                store.ensure_chat(jid.clone(), Some(name), None, jid.ends_with("@g.us"));
            }
        }
        Event::PushNameUpdate(update) => {
            let mut store = state.store.write().await;
            store.rename_contact(&update.jid.to_non_ad().to_string(), &update.new_push_name);
        }
        Event::ContactUpdated(update) => {
            tokio::spawn(refresh_contact_profile(
                state.clone(),
                client.clone(),
                update.jid.to_non_ad(),
            ));
        }
        Event::ContactNumberChanged(change) => {
            let mut store = state.store.write().await;
            let old_jid = change.old_jid.to_non_ad().to_string();
            let new_jid = change.new_jid.to_non_ad().to_string();
            if let Some(contact) = store.contacts.remove(&old_jid) {
                store.upsert_contact(ContactSummary {
                    jid: new_jid.clone(),
                    phone: jid_phone(&change.new_jid.to_non_ad()),
                    ..contact
                });
            }
            if let Some(mut chat) = store.chats.remove(&old_jid) {
                chat.summary.jid = new_jid.clone();
                chat.summary.phone = jid_phone(&change.new_jid.to_non_ad());
                for message in &mut chat.messages {
                    message.chat_jid = new_jid.clone();
                }
                store.chats.insert(new_jid, chat);
            }
        }
        Event::ContactSyncRequested(_) => {
            tokio::spawn(refresh_all_known_contacts(state.clone(), client.clone()));
        }
        Event::PictureUpdate(update) => {
            let jid = update.jid.to_non_ad().to_string();
            if update.removed {
                let mut store = state.store.write().await;
                store.set_contact_avatar(&jid, None);
            } else {
                tokio::spawn(refresh_contact_profile(
                    state.clone(),
                    client.clone(),
                    update.jid.to_non_ad(),
                ));
            }
        }
        Event::UserAboutUpdate(update) => {
            let mut store = state.store.write().await;
            store.set_contact_status(&update.jid.to_non_ad().to_string(), Some(update.status));
        }
        Event::GroupUpdate(update) => {
            tokio::spawn(refresh_group_metadata(
                state.clone(),
                client.clone(),
                update.group_jid.to_non_ad(),
            ));
        }
        Event::ArchiveUpdate(update) => {
            let mut store = state.store.write().await;
            store.set_archived(&update.jid.to_non_ad().to_string(), true);
        }
        Event::MuteUpdate(update) => {
            let mut store = state.store.write().await;
            store.set_muted(&update.jid.to_non_ad().to_string(), true);
        }
        Event::MarkChatAsReadUpdate(update) => {
            let mut store = state.store.write().await;
            store.mark_read(&update.jid.to_non_ad().to_string());
        }
        Event::OfflineSyncCompleted(_) => {
            state.is_syncing.store(false, Ordering::SeqCst);
        }
        _ => {}
    }
}

/// Process a single incoming WhatsApp message into the data store.
pub async fn handle_incoming_message(
    state: AppState,
    client: Arc<Client>,
    msg: Box<wa::Message>,
    info: whatsapp_rust::types::message::MessageInfo,
) {
    let chat = info.source.chat.to_non_ad();
    let chat_jid = chat.to_string();
    let sender = info.source.sender.to_non_ad();
    let sender_jid = sender.to_string();
    let phone = resolve_phone_for_jid(&client, &chat).await;
    let sender_phone = resolve_phone_for_jid(&client, &sender).await;
    let display_name = if !info.push_name.trim().is_empty() {
        info.push_name.clone()
    } else {
        let store = state.store.read().await;
        store.display_name_for_jid(&sender_jid)
    };
    let text = extract_text(&msg);
    let mention_jids = extract_context_info(&msg)
        .map(|ctx| ctx.mentioned_jid.clone())
        .unwrap_or_default();
    let mention_summaries = resolve_mention_summaries(&state, &client, &mention_jids).await;
    let (mentions, media, media_blob) = {
        let media = extract_media(&msg, &chat_jid, &info.id);
        match media {
            Some((attachment, blob)) => (mention_summaries, Some(attachment), Some(blob)),
            None => (mention_summaries, None, None),
        }
    };

    {
        let mut store = state.store.write().await;
        if !jid_is_group(&chat) {
            store.upsert_contact(ContactSummary {
                jid: chat_jid.clone(),
                name: display_name.clone(),
                phone: phone.clone(),
                status: None,
                avatar_url: None,
                is_business: false,
                is_registered: true,
            });
        } else {
            store.upsert_contact(ContactSummary {
                jid: sender_jid.clone(),
                name: display_name.clone(),
                phone: sender_phone.clone(),
                status: None,
                avatar_url: None,
                is_business: false,
                is_registered: true,
            });
        }

        if !info.push_name.trim().is_empty() {
            store.rename_contact(&sender_jid, &info.push_name);
        }

        let chat_display_name = if jid_is_group(&chat) {
            store.display_name_for_jid(&chat_jid)
        } else {
            display_name.clone()
        };

        store.record_message(
            chat_jid.clone(),
            Some(chat_display_name),
            phone,
            jid_is_group(&chat),
            ChatMessage {
                id: info.id.clone(),
                chat_jid: chat_jid.clone(),
                sender_jid,
                sender_name: if jid_is_group(&chat) {
                    Some(display_name.clone())
                } else {
                    (!display_name.trim().is_empty()).then_some(display_name.clone())
                },
                text,
                timestamp_ms: message_timestamp_ms(&info),
                from_me: info.source.is_from_me,
                mentions,
                media,
                receipt_status: if info.source.is_from_me {
                    Some("sent".into())
                } else {
                    None
                },
            },
            media_blob,
        );
    }

    let state_for_mentions = state.clone();
    let client_for_mentions = client.clone();

    if jid_is_group(&chat) {
        tokio::spawn(refresh_group_metadata(state.clone(), client.clone(), chat));
        tokio::spawn(refresh_contact_profile(state, client, sender));
    } else {
        tokio::spawn(ensure_presence_subscription(client.clone(), chat.clone()));
        tokio::spawn(refresh_contact_profile(state, client, chat));
    }

    for mention_jid in mention_jids {
        if let Ok(parsed) = mention_jid.parse::<Jid>() {
            tokio::spawn(refresh_contact_profile(
                state_for_mentions.clone(),
                client_for_mentions.clone(),
                parsed.to_non_ad(),
            ));
        }
    }
}
