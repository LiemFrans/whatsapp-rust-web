// Pure helper / utility functions used across the frontend.

import type { ApiChat, ApiMessage } from "./types";

export function clsx(...values: Array<string | false | null | undefined>) {
  return values.filter(Boolean).join(" ");
}

export function formatTime(timestampMs: number | null) {
  if (!timestampMs) return "";
  return new Date(timestampMs).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function initials(name: string) {
  return (
    name
      .split(" ")
      .filter(Boolean)
      .slice(0, 2)
      .map((part) => part[0]?.toUpperCase() ?? "")
      .join("") || "WA"
  );
}

export function phoneFromJid(jid: string | null) {
  if (!jid) return null;
  if (jid.endsWith("@s.whatsapp.net")) return jid.split("@")[0];
  return null;
}

export function displayName(name: string | null | undefined, jid: string, phone?: string | null) {
  const trimmed = name?.trim();
  if (trimmed && trimmed !== jid) return trimmed;
  const fallbackPhone = phone ?? phoneFromJid(jid);
  if (fallbackPhone) return `+${fallbackPhone}`;
  return trimmed || jid;
}

export function makeManualChat(phone: string): ApiChat {
  return {
    jid: `${phone}@s.whatsapp.net`,
    name: `+${phone}`,
    phone,
    is_group: false,
    preview: null,
    timestamp_ms: null,
    unread_count: 0,
    archived: false,
    muted: false,
    avatar_url: null,
    status: null,
    typing: null,
    is_online: false,
    last_seen_ms: null,
  };
}

export function formatFileSize(bytes: number | null) {
  if (!bytes) return null;
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value.toFixed(value >= 10 || unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`;
}

export function fileExtension(fileName: string | null | undefined, mimeType: string | null | undefined) {
  const fromName = fileName?.split(".").pop()?.trim();
  if (fromName) return fromName.slice(0, 6).toUpperCase();

  const subtype = mimeType?.split("/")[1]?.split(";")[0]?.trim();
  if (!subtype) return "FILE";
  if (subtype === "pdf") return "PDF";
  return subtype.replace(/[^a-z0-9]/gi, "").slice(0, 6).toUpperCase() || "FILE";
}

export function documentMeta(media: NonNullable<ApiMessage["media"]>) {
  const items = [formatFileSize(media.file_length)];
  if (media.page_count) items.push(`${media.page_count} page${media.page_count > 1 ? "s" : ""}`);
  if (!media.page_count && media.mime_type) {
    const subtype = media.mime_type.split("/")[1]?.split(";")[0]?.replace(/[.+_-]/g, " ");
    if (subtype) items.push(subtype.toUpperCase());
  }
  return items.filter(Boolean).join(" • ");
}

export function formatPresence(chat: ApiChat) {
  const phoneLabel = !chat.is_group && chat.phone ? `+${chat.phone}` : null;

  if (chat.typing) {
    return phoneLabel ? `${chat.typing} · ${phoneLabel}` : chat.typing;
  }
  if (chat.is_online) {
    return phoneLabel ? `online · ${phoneLabel}` : "online";
  }
  if (chat.last_seen_ms) {
    const lastSeen = `last seen ${new Date(chat.last_seen_ms).toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
    })}`;
    return phoneLabel ? `${lastSeen} · ${phoneLabel}` : lastSeen;
  }

  if (chat.status && phoneLabel) {
    return `${chat.status} · ${phoneLabel}`;
  }

  return chat.status ?? (chat.is_group ? "Synced group" : phoneLabel ?? displayName(chat.name, chat.jid, chat.phone));
}

export function getReceiptIcon(status: string | null) {
  if (!status) return "";
  if (status === "played") return "▶▶";
  if (status === "read") return "✓✓";
  if (status === "delivered") return "✓✓";
  return "✓";
}

export function getReceiptColor(status: string | null) {
  if (status === "read" || status === "played") return "text-sky-500";
  return "text-wa-text-secondary";
}

export function mentionTokenFromJid(jid: string) {
  return jid.split("@")[0]?.split(":")[0] ?? jid;
}

export function mentionLabel(name: string) {
  return `@${name.replace(/^\+/, "")}`;
}

export function resolveMentionName(
  mention: ApiMessage["mentions"][number],
  mentionNameByJid?: ReadonlyMap<string, string>,
  mentionNameByPhone?: ReadonlyMap<string, string>,
  aliasMap?: ReadonlyMap<string, string>,
) {
  // 0. Check alias by phone (highest priority)
  if (mention.phone && aliasMap?.size) {
    const alias = aliasMap.get(mention.phone);
    if (alias) return alias;
  }
  // Also try phone from JID
  const jidPhone = phoneFromJid(mention.jid);
  if (jidPhone && aliasMap?.size) {
    const alias = aliasMap.get(jidPhone);
    if (alias) return alias;
  }
  // 1. Try by JID
  const byJid = mentionNameByJid?.get(mention.jid)?.trim();
  if (byJid) {
    const fallbackPhone = jidPhone ?? mention.phone ?? null;
    const resolved = displayName(byJid, mention.jid, fallbackPhone);
    if (resolved !== mention.jid && !resolved.startsWith("+")) return resolved;
  }
  // 2. Try by phone
  if (mention.phone) {
    const byPhone = mentionNameByPhone?.get(mention.phone)?.trim();
    if (byPhone && !byPhone.startsWith("+")) return byPhone;
  }
  // 3. Try phone-based JID in the JID map
  if (mention.phone) {
    const phoneJid = `${mention.phone}@s.whatsapp.net`;
    const byPhoneJid = mentionNameByJid?.get(phoneJid)?.trim();
    if (byPhoneJid && !byPhoneJid.startsWith("+")) return byPhoneJid;
  }
  // 4. Fallback
  const fallbackPhone = jidPhone ?? mention.phone ?? null;
  return displayName(mention.name, mention.jid, fallbackPhone);
}

export function shouldRenderMessageText(message: ApiMessage) {
  const text = (message.text || message.media?.caption || "").trim();
  if (!text) return false;
  if (text === "Sticker" || text === "<non-text>") return false;
  if (message.media?.kind === "document") {
    const fileName = message.media.file_name?.trim();
    const title = message.media.title?.trim();
    if (text === fileName || text === title) return false;
  }
  return true;
}

export function renderTextWithMentions(
  message: ApiMessage,
  mentionNameByJid?: ReadonlyMap<string, string>,
  mentionNameByToken?: ReadonlyMap<string, string>,
  aliasMap?: ReadonlyMap<string, string>,
) {
  const text = message.text || message.media?.caption || "";
  const mentions = Array.isArray(message.mentions) ? message.mentions : [];
  let output = text;

  for (const mention of mentions) {
    const token = mentionTokenFromJid(mention.jid);
    const label = mentionLabel(resolveMentionName(mention, mentionNameByJid, mentionNameByToken, aliasMap));
    // Replace by JID token (e.g. LID number)
    output = output.replaceAll(`@${token}`, label);
    // Replace by phone token
    if (mention.phone) output = output.replaceAll(`@${mention.phone}`, label);
    // Replace by phone extracted from JID (if phone-based JID)
    const phone = phoneFromJid(mention.jid);
    if (phone) output = output.replaceAll(`@${phone}`, label);
  }

  // Fallback: replace any remaining @<token> using contacts/chats data
  if (mentionNameByToken?.size) {
    output = output.replace(/@([0-9A-Za-z._-]{5,})/g, (full, token: string) => {
      const resolved = mentionNameByToken.get(token.toLowerCase());
      return resolved ? mentionLabel(resolved) : full;
    });
  }

  return output;
}
