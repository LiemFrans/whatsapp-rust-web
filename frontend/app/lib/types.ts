// Shared TypeScript interfaces for the WhatsApp Web frontend.

export type SidebarView = "chats" | "contacts";

export interface ApiMessage {
  id: string;
  chat_jid: string;
  sender_jid: string;
  sender_name: string | null;
  text: string;
  timestamp_ms: number;
  from_me: boolean;
  mentions: { jid: string; name: string; phone?: string | null }[];
  media: {
    kind: string;
    mime_type: string | null;
    caption: string | null;
    title: string | null;
    file_name: string | null;
    file_length: number | null;
    page_count: number | null;
    width: number | null;
    height: number | null;
    duration_seconds: number | null;
    is_voice_note: boolean;
    is_gif: boolean;
    is_sticker: boolean;
    download_path: string | null;
    preview_image_url: string | null;
  } | null;
  receipt_status: string | null;
}

export interface ApiChat {
  jid: string;
  name: string;
  phone: string | null;
  is_group: boolean;
  preview: string | null;
  timestamp_ms: number | null;
  unread_count: number;
  archived: boolean;
  muted: boolean;
  avatar_url: string | null;
  status: string | null;
  typing: string | null;
  is_online: boolean;
  last_seen_ms: number | null;
}

export interface ApiContact {
  jid: string;
  name: string;
  phone: string | null;
  status: string | null;
  avatar_url: string | null;
  is_business: boolean;
  is_registered: boolean;
}

export interface BootstrapResponse {
  qr_code: string | null;
  is_connected: boolean;
  is_syncing: boolean;
  chats: ApiChat[];
  contacts: ApiContact[];
  logout_hint: string | null;
}

// ---------------------------------------------------------------------------
// Contact aliases
// ---------------------------------------------------------------------------

export interface ContactAlias {
  phone: string;
  name: string;
}

export interface AliasListResponse {
  aliases: ContactAlias[];
}
