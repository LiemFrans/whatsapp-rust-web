//! Async helper functions for contact/group resolution and presence.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use whatsapp_rust::Client;
use whatsapp_rust::Jid;

use crate::helpers::*;
use crate::models::*;

pub async fn ensure_presence_subscription(client: Arc<Client>, jid: Jid) {
    if !jid_is_group(&jid) {
        let _ = client.presence().subscribe(&jid).await;
    }
}

pub async fn resolve_phone_for_jid(client: &Client, jid: &Jid) -> Option<String> {
    if jid.server == "lid" {
        client.get_phone_number_from_lid(&jid.to_string()).await
    } else {
        jid_phone(jid)
    }
}

pub fn resolved_display_name_for_store(store: &DataStore, jid: &str, phone: Option<&str>) -> String {
    let current = store.display_name_for_jid(jid);
    if looks_like_fallback_name(&current, jid, phone) {
        preferred_display_name(None, phone, jid)
    } else {
        current
    }
}

pub async fn resolve_mention_summaries(
    state: &AppState,
    client: &Arc<Client>,
    mention_jids: &[String],
) -> Vec<MentionSummary> {
    let mut summaries = Vec::with_capacity(mention_jids.len());

    for mention_jid in mention_jids {
        let phone = if let Ok(parsed) = mention_jid.parse::<Jid>() {
            resolve_phone_for_jid(client.as_ref(), &parsed.to_non_ad()).await
        } else {
            None
        };

        let name = {
            let store = state.store.read().await;

            // 0. Check alias (highest priority)
            if let Some(alias) = store.alias_for_jid(mention_jid, phone.as_deref()) {
                log::info!("  mention resolve: alias found for {mention_jid} → {alias}");
                alias
            } else {
            // 1. Try direct JID lookup
            let direct_name = resolved_display_name_for_store(&store, mention_jid, phone.as_deref());
            log::info!("  mention resolve step 1 (direct JID {mention_jid}): {direct_name:?}");

            if !looks_like_fallback_name(&direct_name, mention_jid, phone.as_deref()) {
                log::info!("  → using direct name");
                direct_name
            } else if let Some(ref p) = phone {
                // 2. Search ALL contacts/chats by phone number
                let by_phone = store.find_display_name_by_phone(p);
                log::info!("  mention resolve step 2 (find_by_phone {p}): {by_phone:?}");

                if let Some(found) = by_phone {
                    log::info!("  → using phone-found name");
                    found
                } else {
                    // 3. Try phone-based JID lookup
                    let phone_jid = format!("{p}@s.whatsapp.net");
                    let phone_name = resolved_display_name_for_store(&store, &phone_jid, Some(p));
                    log::info!("  mention resolve step 3 (phone JID {phone_jid}): {phone_name:?}");
                    if !looks_like_fallback_name(&phone_name, mention_jid, Some(p)) {
                        log::info!("  → using phone-jid name");
                        phone_name
                    } else {
                        log::info!("  → fallback to direct name");
                        direct_name
                    }
                }
            } else {
                log::info!("  → no phone, using direct name");
                direct_name
            }
            }
        };

        log::info!(
            "Mention: raw_jid={mention_jid} phone={phone:?} name={name}"
        );

        summaries.push(MentionSummary {
            jid: mention_jid.clone(),
            name,
            phone: phone.clone(),
        });
    }

    summaries
}

pub async fn refresh_contact_profile(state: AppState, client: Arc<Client>, jid: Jid) {
    let lookup_jid = jid.to_non_ad();
    let lookup_jid_str = lookup_jid.to_string();
    let phone = resolve_phone_for_jid(&client, &lookup_jid).await;
    let existing_name = {
        let store = state.store.read().await;
        store
            .contacts
            .get(&lookup_jid_str)
            .map(|contact| contact.name.clone())
    };

    let mut alias_jids = vec![lookup_jid_str.clone()];

    let mut updated_contact = ContactSummary {
        jid: lookup_jid_str.clone(),
        name: existing_name
            .filter(|name| !looks_like_fallback_name(name, &lookup_jid_str, phone.as_deref()))
            .unwrap_or_else(|| preferred_display_name(None, phone.as_deref(), &lookup_jid_str)),
        phone: phone.clone(),
        status: None,
        avatar_url: None,
        is_business: false,
        is_registered: phone.is_some(),
    };

    if let Some(phone) = phone.as_deref() {
        if let Ok(info_list) = client.contacts().get_info(&[phone]).await {
            if let Some(info) = info_list.into_iter().next() {
                updated_contact.jid = info.jid.to_string();
                updated_contact.phone = Some(phone.to_string());
                updated_contact.status = info.status.clone();
                updated_contact.is_business = info.is_business;
                updated_contact.is_registered = info.is_registered;
                if let Some(lid) = info.lid {
                    alias_jids.push(lid.to_non_ad().to_string());
                }
            }
        }
    }

    if let Ok(info_map) = client.contacts().get_user_info(&[lookup_jid.clone()]).await {
        if let Some(info) = info_map.get(&lookup_jid) {
            updated_contact.jid = info.jid.to_string();
            updated_contact.status = info.status.clone().or(updated_contact.status.clone());
            updated_contact.is_business = info.is_business;
            if let Some(lid) = &info.lid {
                alias_jids.push(lid.to_non_ad().to_string());
            }
        }
    }

    alias_jids.push(updated_contact.jid.clone());
    alias_jids.sort();
    alias_jids.dedup();

    if let Ok(Some(picture)) = client.contacts().get_profile_picture(&lookup_jid, true).await {
        updated_contact.avatar_url = Some(picture.url);
    }

    let contact_jid = updated_contact.jid.clone();
    let contact_name = updated_contact.name.clone();
    let contact_phone = updated_contact.phone.clone();
    let contact_status = updated_contact.status.clone();
    let contact_avatar = updated_contact.avatar_url.clone();

    let mut store = state.store.write().await;
    store.upsert_contact(updated_contact.clone());
    for alias_jid in &alias_jids {
        store.upsert_contact(ContactSummary {
            jid: alias_jid.clone(),
            name: contact_name.clone(),
            phone: contact_phone.clone(),
            status: contact_status.clone(),
            avatar_url: contact_avatar.clone(),
            is_business: updated_contact.is_business,
            is_registered: updated_contact.is_registered,
        });

        if let Some(chat) = store.chats.get_mut(alias_jid.as_str()) {
            if is_better_name(&contact_name, &chat.summary.name, &contact_jid, contact_phone.as_deref()) {
                chat.summary.name = contact_name.clone();
            }
            if contact_phone.is_some() {
                chat.summary.phone = contact_phone.clone();
            }
            if contact_status.is_some() {
                chat.summary.status = contact_status.clone();
            }
            if contact_avatar.is_some() {
                chat.summary.avatar_url = contact_avatar.clone();
            }
        }
    }
    store.refresh_mention_names(&alias_jids, &contact_name, contact_phone.as_deref());
}

pub async fn refresh_group_metadata(state: AppState, client: Arc<Client>, jid: Jid) {
    if let Ok(group) = client.groups().get_metadata(&jid).await {
        let mut store = state.store.write().await;
        let chat = store.ensure_chat(jid.to_string(), Some(group.subject.clone()), None, true);
        chat.summary.name = group.subject;
        chat.summary.is_group = true;
    }

    if let Ok(group_info) = client.groups().query_info(&jid).await {
        let mut participant_jids = Vec::new();
        for participant in &group_info.participants {
            participant_jids.push(participant.to_non_ad());
        }
        for (lid_user, phone_jid) in group_info.lid_to_pn_map() {
            participant_jids.push(Jid::new(lid_user, "lid").to_non_ad());
            participant_jids.push(phone_jid.to_non_ad());
        }
        for participant in participant_jids {
            if !jid_is_group(&participant) {
                tokio::spawn(refresh_contact_profile(state.clone(), client.clone(), participant.clone()));
                tokio::spawn(ensure_presence_subscription(client.clone(), participant));
            }
        }
    }
}

pub async fn refresh_all_known_contacts(state: AppState, client: Arc<Client>) {
    let contact_jids: Vec<String> = {
        let store = state.store.read().await;
        store
            .contacts
            .keys()
            .chain(store.chats.keys())
            .cloned()
            .collect()
    };

    for jid in contact_jids {
        if let Ok(parsed) = jid.parse::<Jid>() {
            if jid_is_group(&parsed) {
                refresh_group_metadata(state.clone(), client.clone(), parsed).await;
            } else {
                ensure_presence_subscription(client.clone(), parsed.clone()).await;
                refresh_contact_profile(state.clone(), client.clone(), parsed).await;
            }
        }
    }
}

pub async fn wait_for_startup_sync(state: AppState, client: Arc<Client>) {
    state.is_syncing.store(true, Ordering::SeqCst);
    let _ = client
        .wait_for_startup_sync(Duration::from_secs(STARTUP_SYNC_TIMEOUT_SECS))
        .await;
    state.is_syncing.store(false, Ordering::SeqCst);
    refresh_all_known_contacts(state, client).await;
}
