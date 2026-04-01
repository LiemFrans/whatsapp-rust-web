// Defensive normalizers that ensure API responses always have safe defaults.

import type {
  ApiChat,
  ApiContact,
  ApiMessage,
  BootstrapResponse,
} from "./types";
import { phoneFromJid } from "./helpers";

export function normalizeMessage(input: Partial<ApiMessage> | null | undefined): ApiMessage {
  return {
    id: input?.id ?? `${Date.now()}`,
    chat_jid: input?.chat_jid ?? "",
    sender_jid: input?.sender_jid ?? "",
    sender_name: input?.sender_name ?? null,
    text: input?.text ?? "",
    timestamp_ms: input?.timestamp_ms ?? Date.now(),
    from_me: Boolean(input?.from_me),
    mentions: Array.isArray(input?.mentions) ? input!.mentions : [],
    media: input?.media
      ? {
          kind: input.media.kind ?? "unknown",
          mime_type: input.media.mime_type ?? null,
          caption: input.media.caption ?? null,
          title: input.media.title ?? null,
          file_name: input.media.file_name ?? null,
          file_length: input.media.file_length ?? null,
          page_count: input.media.page_count ?? null,
          width: input.media.width ?? null,
          height: input.media.height ?? null,
          duration_seconds: input.media.duration_seconds ?? null,
          is_voice_note: Boolean(input.media.is_voice_note),
          is_gif: Boolean(input.media.is_gif),
          is_sticker: Boolean(input.media.is_sticker),
          download_path: input.media.download_path ?? null,
          preview_image_url: input.media.preview_image_url ?? null,
        }
      : null,
    receipt_status: input?.receipt_status ?? null,
  };
}

export function normalizeChat(input: Partial<ApiChat> | null | undefined): ApiChat {
  const derivedPhone = input?.phone ?? phoneFromJid(input?.jid ?? null);
  return {
    jid: input?.jid ?? "",
    name: input?.name ?? derivedPhone ?? input?.jid ?? "Unknown",
    phone: derivedPhone,
    is_group: Boolean(input?.is_group),
    preview: input?.preview ?? null,
    timestamp_ms: input?.timestamp_ms ?? null,
    unread_count: input?.unread_count ?? 0,
    archived: Boolean(input?.archived),
    muted: Boolean(input?.muted),
    avatar_url: input?.avatar_url ?? null,
    status: input?.status ?? null,
    typing: input?.typing ?? null,
    is_online: Boolean(input?.is_online),
    last_seen_ms: input?.last_seen_ms ?? null,
  };
}

export function normalizeContact(input: Partial<ApiContact> | null | undefined): ApiContact {
  const derivedPhone = input?.phone ?? phoneFromJid(input?.jid ?? null);
  return {
    jid: input?.jid ?? "",
    name: input?.name ?? derivedPhone ?? input?.jid ?? "Unknown",
    phone: derivedPhone,
    status: input?.status ?? null,
    avatar_url: input?.avatar_url ?? null,
    is_business: Boolean(input?.is_business),
    is_registered: input?.is_registered ?? true,
  };
}

export function normalizeBootstrap(input: Partial<BootstrapResponse> | null | undefined): BootstrapResponse {
  return {
    qr_code: input?.qr_code ?? null,
    is_connected: Boolean(input?.is_connected),
    is_syncing: Boolean(input?.is_syncing),
    chats: Array.isArray(input?.chats) ? input!.chats.map(normalizeChat) : [],
    contacts: Array.isArray(input?.contacts) ? input!.contacts.map(normalizeContact) : [],
    logout_hint: input?.logout_hint ?? null,
  };
}
