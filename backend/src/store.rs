//! In-memory data store — manages chats, contacts, media blobs, and aliases.

use crate::helpers::*;
use crate::models::*;

use std::collections::HashMap;
use std::path::Path;

const MAX_STICKERS: usize = 100;
const ALIAS_FILE: &str = "contact_aliases.json";

/// Load aliases from disk. Returns empty map on any error.
pub fn load_aliases_from_disk() -> HashMap<String, String> {
    let path = Path::new(ALIAS_FILE);
    if !path.exists() {
        return HashMap::new();
    }
    match std::fs::read_to_string(path) {
        Ok(data) => serde_json::from_str::<HashMap<String, String>>(&data).unwrap_or_default(),
        Err(e) => {
            log::warn!("Failed to read {ALIAS_FILE}: {e}");
            HashMap::new()
        }
    }
}

/// Persist current aliases to disk.
pub fn save_aliases_to_disk(aliases: &HashMap<String, String>) {
    match serde_json::to_string_pretty(aliases) {
        Ok(json) => {
            if let Err(e) = std::fs::write(ALIAS_FILE, json) {
                log::error!("Failed to write {ALIAS_FILE}: {e}");
            }
        }
        Err(e) => log::error!("Failed to serialize aliases: {e}"),
    }
}

impl DataStore {
    pub fn reset(&mut self) {
        self.chats.clear();
        self.contacts.clear();
        self.media.clear();
        self.stickers.clear();
        self.push_names.clear();
        // NOTE: aliases are NOT cleared on reset — they persist across sessions.
    }

    pub fn record_sticker(&mut self, record: StickerRecord) {
        // Deduplicate by message_id
        if self.stickers.iter().any(|s| s.message_id == record.message_id) {
            return;
        }
        self.stickers.push(record);
        // Keep only the most recent stickers
        if self.stickers.len() > MAX_STICKERS {
            self.stickers.drain(0..self.stickers.len() - MAX_STICKERS);
        }
    }

    pub fn recent_stickers(&self) -> Vec<StickerRecord> {
        let mut stickers = self.stickers.clone();
        stickers.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
        stickers
    }

    pub fn sorted_chats(&self) -> Vec<ChatSummary> {
        let mut chats: Vec<_> = self.chats.values().map(|chat| chat.summary.clone()).collect();
        chats.sort_by(|a, b| {
            b.timestamp_ms
                .cmp(&a.timestamp_ms)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        chats
    }

    pub fn sorted_contacts(&self) -> Vec<ContactSummary> {
        let mut contacts: Vec<_> = self.contacts.values().cloned().collect();
        contacts.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        contacts
    }

    pub fn messages_for(&self, chat_jid: &str) -> Option<Vec<ChatMessage>> {
        self.chats.get(chat_jid).map(|chat| chat.messages.clone())
    }

    pub fn media_for(&self, chat_jid: &str, message_id: &str) -> Option<MediaBlob> {
        self.media.get(&media_key(chat_jid, message_id)).cloned()
    }

    pub fn display_name_for_jid(&self, jid: &str) -> String {
        // 0. Check aliases by phone extracted from JID
        if let Some(phone) = jid_phone_str(jid) {
            if let Some(alias) = self.aliases.get(&phone) {
                return alias.clone();
            }
        }
        // Also check aliases by phone stored in contacts/chats
        if let Some(contact) = self.contacts.get(jid) {
            if let Some(ref phone) = contact.phone {
                if let Some(alias) = self.aliases.get(phone) {
                    return alias.clone();
                }
            }
        }
        if let Some(chat) = self.chats.get(jid) {
            if let Some(ref phone) = chat.summary.phone {
                if let Some(alias) = self.aliases.get(phone) {
                    return alias.clone();
                }
            }
        }

        if let Some(contact) = self.contacts.get(jid) {
            return contact.name.clone();
        }
        if let Some(chat) = self.chats.get(jid) {
            return chat.summary.name.clone();
        }
        if let Some(phone) = jid_phone_str(jid) {
            return format!("+{phone}");
        }
        jid.to_string()
    }

    /// Search all aliases, contacts, chats, and push-name cache for a real
    /// display name matching the given phone number.  Returns `None` if no
    /// match is found or if every match is just a fallback (phone/JID-based)
    /// name.  Aliases always take highest priority.
    pub fn find_display_name_by_phone(&self, phone: &str) -> Option<String> {
        // 0. Check aliases (highest priority)
        if let Some(alias) = self.aliases.get(phone) {
            return Some(alias.clone());
        }

        // Search contacts first
        for contact in self.contacts.values() {
            if contact.phone.as_deref() == Some(phone) {
                if !looks_like_fallback_name(&contact.name, &contact.jid, Some(phone)) {
                    return Some(contact.name.clone());
                }
            }
        }
        // Then search chat summaries
        for chat in self.chats.values() {
            if chat.summary.phone.as_deref() == Some(phone) {
                if !looks_like_fallback_name(&chat.summary.name, &chat.summary.jid, Some(phone)) {
                    return Some(chat.summary.name.clone());
                }
            }
        }
        // Then search push-names cache (keyed by JID, but we match phone-based JID)
        let phone_jid = format!("{phone}@s.whatsapp.net");
        if let Some(push) = self.push_names.get(&phone_jid) {
            if !looks_like_fallback_name(push, &phone_jid, Some(phone)) {
                return Some(push.clone());
            }
        }
        // Also search ALL push_names for any JID whose phone matches
        for (jid, push) in &self.push_names {
            if jid_phone_str(jid).as_deref() == Some(phone) {
                if !looks_like_fallback_name(push, jid, Some(phone)) {
                    return Some(push.clone());
                }
            }
        }
        None
    }

    pub fn ensure_chat(
        &mut self,
        chat_jid: String,
        name: Option<String>,
        phone: Option<String>,
        is_group: bool,
    ) -> &mut ChatRecord {
        let fallback_name = preferred_display_name(name.as_deref(), phone.as_deref(), &chat_jid);

        let record = self
            .chats
            .entry(chat_jid.clone())
            .or_insert_with(|| ChatRecord {
                summary: ChatSummary {
                    jid: chat_jid.clone(),
                    name: fallback_name.clone(),
                    phone: phone.clone(),
                    is_group,
                    ..Default::default()
                },
                messages: Vec::new(),
            });

        if let Some(ref name_value) = name {
            if is_better_name(name_value, &record.summary.name, &chat_jid, phone.as_deref()) {
                record.summary.name = name_value.clone();
            }
        }
        if let Some(phone) = phone {
            record.summary.phone = Some(phone);
        }
        record.summary.is_group = is_group;
        record
    }

    pub fn upsert_contact(&mut self, contact: ContactSummary) {
        let contact_jid = contact.jid.clone();
        let contact_phone = contact.phone.clone();
        let entry = self
            .contacts
            .entry(contact.jid.clone())
            .or_insert_with(|| contact.clone());

        if is_better_name(&contact.name, &entry.name, &contact_jid, contact_phone.as_deref()) {
            entry.name = contact.name;
        }
        if contact.phone.is_some() {
            entry.phone = contact.phone;
        }
        if contact.status.is_some() {
            entry.status = contact.status;
        }
        if contact.avatar_url.is_some() {
            entry.avatar_url = contact.avatar_url;
        }
        entry.is_business = contact.is_business;
        entry.is_registered = contact.is_registered;
    }

    pub fn rename_contact(&mut self, jid: &str, new_name: &str) {
        if !new_name.trim().is_empty() {
            self.push_names.insert(jid.to_string(), new_name.to_string());
            if let Some(contact) = self.contacts.get_mut(jid) {
                contact.name = new_name.to_string();
            }
            if let Some(chat) = self.chats.get_mut(jid) {
                chat.summary.name = new_name.to_string();
            }
        }
    }

    // -----------------------------------------------------------------------
    // Alias management
    // -----------------------------------------------------------------------

    /// Look up alias for a JID by resolving its phone number from contacts,
    /// chats, the JID itself, or a supplied phone number.
    pub fn alias_for_jid(&self, jid: &str, phone_hint: Option<&str>) -> Option<String> {
        // Try phone hint first
        if let Some(phone) = phone_hint {
            if let Some(alias) = self.aliases.get(phone) {
                return Some(alias.clone());
            }
        }
        // Try phone from contact
        if let Some(contact) = self.contacts.get(jid) {
            if let Some(ref phone) = contact.phone {
                if let Some(alias) = self.aliases.get(phone) {
                    return Some(alias.clone());
                }
            }
        }
        // Try phone from chat
        if let Some(chat) = self.chats.get(jid) {
            if let Some(ref phone) = chat.summary.phone {
                if let Some(alias) = self.aliases.get(phone) {
                    return Some(alias.clone());
                }
            }
        }
        // Try phone from JID itself
        if let Some(phone) = jid_phone_str(jid) {
            if let Some(alias) = self.aliases.get(&phone) {
                return Some(alias.clone());
            }
        }
        None
    }

    pub fn set_alias(&mut self, phone: String, name: String) {
        self.aliases.insert(phone, name);
        save_aliases_to_disk(&self.aliases);
    }

    pub fn remove_alias(&mut self, phone: &str) -> bool {
        let removed = self.aliases.remove(phone).is_some();
        if removed {
            save_aliases_to_disk(&self.aliases);
        }
        removed
    }

    pub fn list_aliases(&self) -> Vec<ContactAlias> {
        self.aliases
            .iter()
            .map(|(phone, name)| ContactAlias {
                phone: phone.clone(),
                name: name.clone(),
            })
            .collect()
    }

    pub fn set_contact_status(&mut self, jid: &str, status: Option<String>) {
        if let Some(contact) = self.contacts.get_mut(jid) {
            contact.status = status.clone();
        }
        if let Some(chat) = self.chats.get_mut(jid) {
            chat.summary.status = status;
        }
    }

    pub fn set_contact_avatar(&mut self, jid: &str, avatar_url: Option<String>) {
        if let Some(contact) = self.contacts.get_mut(jid) {
            contact.avatar_url = avatar_url.clone();
        }
        if let Some(chat) = self.chats.get_mut(jid) {
            chat.summary.avatar_url = avatar_url;
        }
    }

    pub fn record_message(
        &mut self,
        chat_jid: String,
        chat_name: Option<String>,
        phone: Option<String>,
        is_group: bool,
        message: ChatMessage,
        media_blob: Option<MediaBlob>,
    ) {
        let preview = Some(preview_for_message(&message));
        let timestamp_ms = Some(message.timestamp_ms);
        let from_me = message.from_me;
        let message_id = message.id.clone();
        let download_key = media_key(&chat_jid, &message_id);
        let chat_key = chat_jid.clone();

        let mut removed_ids: Vec<String> = Vec::new();
        {
            let chat = self.ensure_chat(chat_jid, chat_name, phone, is_group);
            chat.summary.typing = None;
            if let Some(existing) = chat.messages.iter_mut().find(|existing| existing.id == message.id) {
                *existing = message;
                chat.summary.preview = preview;
                chat.summary.timestamp_ms = timestamp_ms;
                if let Some(blob) = media_blob {
                    self.media.insert(download_key, blob);
                }
                return;
            }

            chat.messages.push(message);
            if chat.messages.len() > MAX_MESSAGES_PER_CHAT {
                let overflow = chat.messages.len() - MAX_MESSAGES_PER_CHAT;
                let removed: Vec<ChatMessage> = chat.messages.drain(0..overflow).collect();
                removed_ids = removed.into_iter().map(|item| item.id).collect();
            }
            chat.summary.preview = preview;
            chat.summary.timestamp_ms = timestamp_ms;
            if !from_me {
                chat.summary.unread_count = chat.summary.unread_count.saturating_add(1);
            }
        }

        for removed_id in removed_ids {
            self.media.remove(&media_key(&chat_key, &removed_id));
        }
        if let Some(blob) = media_blob {
            self.media.insert(download_key, blob);
        }
    }

    pub fn mark_read(&mut self, chat_jid: &str) {
        if let Some(chat) = self.chats.get_mut(chat_jid) {
            chat.summary.unread_count = 0;
        }
    }

    pub fn set_archived(&mut self, chat_jid: &str, archived: bool) {
        if let Some(chat) = self.chats.get_mut(chat_jid) {
            chat.summary.archived = archived;
        }
    }

    pub fn set_muted(&mut self, chat_jid: &str, muted: bool) {
        if let Some(chat) = self.chats.get_mut(chat_jid) {
            chat.summary.muted = muted;
        }
    }

    pub fn set_typing(&mut self, chat_jid: &str, typing: Option<String>) {
        if let Some(chat) = self.chats.get_mut(chat_jid) {
            chat.summary.typing = typing;
        }
    }

    pub fn set_presence(&mut self, jid: &str, is_online: bool, last_seen_ms: Option<i64>) {
        if let Some(chat) = self.chats.get_mut(jid) {
            chat.summary.is_online = is_online;
            chat.summary.last_seen_ms = last_seen_ms;
        }
    }

    pub fn update_message_receipt(&mut self, chat_jid: &str, message_ids: &[String], receipt_status: &str) {
        if let Some(chat) = self.chats.get_mut(chat_jid) {
            for message in &mut chat.messages {
                if message.from_me && message_ids.iter().any(|id| id == &message.id) {
                    message.receipt_status = Some(promote_receipt_status(
                        message.receipt_status.as_deref(),
                        receipt_status,
                    ));
                }
            }
        }
    }

    pub fn refresh_mention_names(&mut self, alias_jids: &[String], resolved_name: &str, phone: Option<&str>) {
        if resolved_name.trim().is_empty() {
            return;
        }

        for chat in self.chats.values_mut() {
            for message in &mut chat.messages {
                for mention in &mut message.mentions {
                    if alias_jids.iter().any(|alias| alias == &mention.jid)
                        && is_better_name(resolved_name, &mention.name, &mention.jid, phone)
                    {
                        mention.name = resolved_name.to_string();
                    }
                }
            }
        }
    }
}
