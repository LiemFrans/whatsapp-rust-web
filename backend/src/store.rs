//! In-memory data store — manages chats, contacts, and media blobs.

use crate::helpers::*;
use crate::models::*;

impl DataStore {
    pub fn reset(&mut self) {
        self.chats.clear();
        self.contacts.clear();
        self.media.clear();
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
            if let Some(contact) = self.contacts.get_mut(jid) {
                contact.name = new_name.to_string();
            }
            if let Some(chat) = self.chats.get_mut(jid) {
                chat.summary.name = new_name.to_string();
            }
        }
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
