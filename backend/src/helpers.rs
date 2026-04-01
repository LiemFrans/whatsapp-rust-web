//! Pure helper functions — no async, no application state.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use whatsapp_rust::download::MediaType;
use whatsapp_rust::waproto::whatsapp as wa;
use whatsapp_rust::Jid;

use crate::models::*;

// ---------------------------------------------------------------------------
// String / phone helpers
// ---------------------------------------------------------------------------

pub fn normalize_phone(input: &str) -> String {
    input.trim().replace(['+', '-', ' ', '(', ')'], "")
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

pub fn message_timestamp_ms(info: &whatsapp_rust::types::message::MessageInfo) -> i64 {
    info.timestamp.timestamp_millis()
}

pub fn jid_phone_str(jid: &str) -> Option<String> {
    if jid.contains("@s.whatsapp.net") {
        Some(jid.split('@').next().unwrap_or_default().to_string())
    } else {
        None
    }
}

pub fn jid_phone(jid: &Jid) -> Option<String> {
    jid_phone_str(&jid.to_string())
}

pub fn jid_is_group(jid: &Jid) -> bool {
    jid.to_string().ends_with("@g.us")
}

// ---------------------------------------------------------------------------
// Display‑name ranking helpers
// ---------------------------------------------------------------------------

pub fn preferred_display_name(name: Option<&str>, phone: Option<&str>, jid: &str) -> String {
    if let Some(value) = name.filter(|value| !value.trim().is_empty()) {
        return value.to_string();
    }
    if let Some(value) = phone.filter(|value| !value.is_empty()) {
        return format!("+{value}");
    }
    jid.to_string()
}

pub fn looks_like_fallback_name(name: &str, jid: &str, phone: Option<&str>) -> bool {
    if name.trim().is_empty() || name == jid {
        return true;
    }
    if let Some(phone) = phone {
        return name == phone || name == format!("+{phone}");
    }
    false
}

pub fn display_name_rank(name: &str, jid: &str, phone: Option<&str>) -> u8 {
    if name.trim().is_empty() {
        return 0;
    }
    if name == jid {
        return 1;
    }
    if let Some(phone) = phone {
        if name == phone || name == format!("+{phone}") {
            return 2;
        }
    }
    3
}

pub fn is_better_name(candidate: &str, current: &str, jid: &str, phone: Option<&str>) -> bool {
    if candidate.trim().is_empty() {
        return false;
    }
    if current.trim().is_empty() {
        return true;
    }
    display_name_rank(candidate, jid, phone) > display_name_rank(current, jid, phone)
}

// ---------------------------------------------------------------------------
// Media / receipt helpers
// ---------------------------------------------------------------------------

pub fn media_key(chat_jid: &str, message_id: &str) -> String {
    format!("{chat_jid}:{message_id}")
}

pub fn receipt_rank(status: &str) -> usize {
    match status {
        "sent" => 1,
        "delivered" => 2,
        "read" => 3,
        "played" => 4,
        _ => 0,
    }
}

pub fn promote_receipt_status(current: Option<&str>, new_status: &str) -> String {
    match current {
        Some(existing) if receipt_rank(existing) >= receipt_rank(new_status) => existing.to_string(),
        _ => new_status.to_string(),
    }
}

pub fn default_mime_for_media_type(media_type: MediaType) -> &'static str {
    match media_type {
        MediaType::Image => "image/jpeg",
        MediaType::Video => "video/mp4",
        MediaType::Audio => "audio/mpeg",
        MediaType::Document => "application/octet-stream",
        MediaType::Sticker => "image/webp",
        _ => "application/octet-stream",
    }
}

// ---------------------------------------------------------------------------
// Message text rendering
// ---------------------------------------------------------------------------

pub fn render_text_with_mentions(text: &str, mentions: &[MentionSummary]) -> String {
    let mut output = text.to_string();
    for mention in mentions {
        let token = mention
            .jid
            .split('@')
            .next()
            .unwrap_or_default()
            .split(':')
            .next()
            .unwrap_or_default();
        if token.is_empty() {
            continue;
        }
        let label = format!("@{}", mention.name.trim_start_matches('+'));
        output = output.replace(&format!("@{token}"), &label);

        // Also replace by phone token (handles LID mentions where the text
        // contains the LID number but we also want @phone → @Name)
        if let Some(ref phone) = mention.phone {
            output = output.replace(&format!("@{phone}"), &label);
        }
        if let Some(phone) = jid_phone_str(&mention.jid) {
            output = output.replace(&format!("@{phone}"), &label);
        }
    }
    output
}

pub fn preview_for_message(message: &ChatMessage) -> String {
    if let Some(media) = &message.media {
        match media.kind.as_str() {
            "image" => media.caption.clone().unwrap_or_else(|| "📷 Photo".into()),
            "video" => media.caption.clone().unwrap_or_else(|| {
                if media.is_gif {
                    "🎞️ GIF".into()
                } else {
                    "🎥 Video".into()
                }
            }),
            "document" => media
                .file_name
                .clone()
                .map(|name| format!("📎 {name}"))
                .unwrap_or_else(|| "📎 Document".into()),
            "audio" => {
                if media.is_voice_note {
                    "🎤 Voice note".into()
                } else {
                    "🎵 Audio".into()
                }
            }
            "sticker" => "🪄 Sticker".into(),
            _ => render_text_with_mentions(&message.text, &message.mentions),
        }
    } else {
        render_text_with_mentions(&message.text, &message.mentions)
    }
}

// ---------------------------------------------------------------------------
// WhatsApp proto extraction helpers
// ---------------------------------------------------------------------------

pub fn extract_context_info(message: &wa::Message) -> Option<&wa::ContextInfo> {
    if let Some(value) = message.extended_text_message.as_ref().and_then(|value| value.context_info.as_deref()) {
        return Some(value);
    }
    if let Some(value) = message.image_message.as_ref().and_then(|value| value.context_info.as_deref()) {
        return Some(value);
    }
    if let Some(value) = message.video_message.as_ref().and_then(|value| value.context_info.as_deref()) {
        return Some(value);
    }
    if let Some(value) = message.document_message.as_ref().and_then(|value| value.context_info.as_deref()) {
        return Some(value);
    }
    if let Some(value) = message.audio_message.as_ref().and_then(|value| value.context_info.as_deref()) {
        return Some(value);
    }
    if let Some(value) = message.sticker_message.as_ref().and_then(|value| value.context_info.as_deref()) {
        return Some(value);
    }
    None
}

pub fn extract_text(message: &wa::Message) -> String {
    if let Some(text) = message.conversation.as_ref().filter(|value| !value.is_empty()) {
        return text.clone();
    }
    if let Some(text) = message
        .extended_text_message
        .as_ref()
        .and_then(|value| value.text.clone())
        .filter(|value| !value.is_empty())
    {
        return text;
    }
    if let Some(caption) = message
        .image_message
        .as_ref()
        .and_then(|value| value.caption.clone())
        .filter(|value| !value.is_empty())
    {
        return caption;
    }
    if let Some(caption) = message
        .video_message
        .as_ref()
        .and_then(|value| value.caption.clone())
        .filter(|value| !value.is_empty())
    {
        return caption;
    }
    if let Some(caption) = message
        .document_message
        .as_ref()
        .and_then(|value| value.caption.clone())
        .filter(|value| !value.is_empty())
    {
        return caption;
    }
    if let Some(file_name) = message
        .document_message
        .as_ref()
        .and_then(|value| value.file_name.clone())
        .filter(|value| !value.is_empty())
    {
        return file_name;
    }
    if message.audio_message.is_some() {
        return "Voice message".into();
    }
    if message.sticker_message.is_some() {
        return "Sticker".into();
    }
    "<non-text>".into()
}

// ---------------------------------------------------------------------------
// Media blob building
// ---------------------------------------------------------------------------

pub fn build_media_blob(
    mime_type: Option<String>,
    file_name: Option<String>,
    direct_path: Option<String>,
    media_key: Option<Vec<u8>>,
    file_sha256: Option<Vec<u8>>,
    file_enc_sha256: Option<Vec<u8>>,
    file_length: Option<u64>,
    media_type: MediaType,
) -> Option<MediaBlob> {
    Some(MediaBlob {
        mime_type,
        file_name,
        direct_path: direct_path?,
        media_key: media_key?,
        file_sha256: file_sha256?,
        file_enc_sha256: file_enc_sha256?,
        file_length: file_length?,
        media_type,
    })
}

pub fn inline_jpeg_preview_url(thumbnail: Option<&Vec<u8>>) -> Option<String> {
    let bytes = thumbnail?;
    if bytes.is_empty() {
        return None;
    }

    Some(format!(
        "data:image/jpeg;base64,{}",
        BASE64_STANDARD.encode(bytes)
    ))
}

pub fn extract_media(message: &wa::Message, chat_jid: &str, message_id: &str) -> Option<(MediaAttachment, MediaBlob)> {
    if let Some(image) = message.image_message.as_ref() {
        let blob = build_media_blob(
            image.mimetype.clone(),
            None,
            image.direct_path.clone(),
            image.media_key.clone(),
            image.file_sha256.clone(),
            image.file_enc_sha256.clone(),
            image.file_length,
            MediaType::Image,
        )?;
        return Some((
            MediaAttachment {
                kind: "image".into(),
                mime_type: image.mimetype.clone(),
                caption: image.caption.clone(),
                title: None,
                file_name: None,
                file_length: image.file_length,
                page_count: None,
                width: image.width,
                height: image.height,
                duration_seconds: None,
                is_voice_note: false,
                is_gif: false,
                is_sticker: false,
                download_path: Some(format!("/api/media/{chat_jid}/{message_id}")),
                preview_image_url: None,
            },
            blob,
        ));
    }
    if let Some(video) = message.video_message.as_ref() {
        let blob = build_media_blob(
            video.mimetype.clone(),
            None,
            video.direct_path.clone(),
            video.media_key.clone(),
            video.file_sha256.clone(),
            video.file_enc_sha256.clone(),
            video.file_length,
            MediaType::Video,
        )?;
        return Some((
            MediaAttachment {
                kind: "video".into(),
                mime_type: video.mimetype.clone(),
                caption: video.caption.clone(),
                title: None,
                file_name: None,
                file_length: video.file_length,
                page_count: None,
                width: video.width,
                height: video.height,
                duration_seconds: video.seconds,
                is_voice_note: false,
                is_gif: video.gif_playback.unwrap_or(false),
                is_sticker: false,
                download_path: Some(format!("/api/media/{chat_jid}/{message_id}")),
                preview_image_url: None,
            },
            blob,
        ));
    }
    if let Some(document) = message.document_message.as_ref() {
        let blob = build_media_blob(
            document.mimetype.clone(),
            document.file_name.clone(),
            document.direct_path.clone(),
            document.media_key.clone(),
            document.file_sha256.clone(),
            document.file_enc_sha256.clone(),
            document.file_length,
            MediaType::Document,
        )?;
        return Some((
            MediaAttachment {
                kind: "document".into(),
                mime_type: document.mimetype.clone(),
                caption: document.caption.clone(),
                title: document.title.clone(),
                file_name: document.file_name.clone(),
                file_length: document.file_length,
                page_count: document.page_count,
                width: None,
                height: None,
                duration_seconds: None,
                is_voice_note: false,
                is_gif: false,
                is_sticker: false,
                download_path: Some(format!("/api/media/{chat_jid}/{message_id}")),
                preview_image_url: inline_jpeg_preview_url(document.jpeg_thumbnail.as_ref()),
            },
            blob,
        ));
    }
    if let Some(audio) = message.audio_message.as_ref() {
        let blob = build_media_blob(
            audio.mimetype.clone(),
            None,
            audio.direct_path.clone(),
            audio.media_key.clone(),
            audio.file_sha256.clone(),
            audio.file_enc_sha256.clone(),
            audio.file_length,
            MediaType::Audio,
        )?;
        return Some((
            MediaAttachment {
                kind: "audio".into(),
                mime_type: audio.mimetype.clone(),
                caption: None,
                title: None,
                file_name: None,
                file_length: audio.file_length,
                page_count: None,
                width: None,
                height: None,
                duration_seconds: audio.seconds,
                is_voice_note: audio.ptt.unwrap_or(false),
                is_gif: false,
                is_sticker: false,
                download_path: Some(format!("/api/media/{chat_jid}/{message_id}")),
                preview_image_url: None,
            },
            blob,
        ));
    }
    if let Some(sticker) = message.sticker_message.as_ref() {
        let mime = sticker.mimetype.clone().or_else(|| Some("image/webp".into()));
        let blob = build_media_blob(
            mime.clone(),
            None,
            sticker.direct_path.clone(),
            sticker.media_key.clone(),
            sticker.file_sha256.clone(),
            sticker.file_enc_sha256.clone(),
            sticker.file_length,
            MediaType::Sticker,
        )?;
        return Some((
            MediaAttachment {
                kind: "sticker".into(),
                mime_type: mime,
                caption: None,
                title: None,
                file_name: None,
                file_length: sticker.file_length,
                page_count: None,
                width: None,
                height: None,
                duration_seconds: None,
                is_voice_note: false,
                is_gif: false,
                is_sticker: true,
                download_path: Some(format!("/api/media/{chat_jid}/{message_id}")),
                preview_image_url: None,
            },
            blob,
        ));
    }
    None
}
