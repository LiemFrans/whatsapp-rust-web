"use client";

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type FormEvent,
} from "react";
import QRCode from "react-qr-code";

type SidebarView = "chats" | "contacts";

interface ApiMessage {
  id: string;
  chat_jid: string;
  sender_jid: string;
  sender_name: string | null;
  text: string;
  timestamp_ms: number;
  from_me: boolean;
  mentions: { jid: string; name: string }[];
  media: {
    kind: string;
    mime_type: string | null;
    caption: string | null;
    file_name: string | null;
    file_length: number | null;
    width: number | null;
    height: number | null;
    duration_seconds: number | null;
    is_voice_note: boolean;
    is_gif: boolean;
    is_sticker: boolean;
    download_path: string | null;
  } | null;
  receipt_status: string | null;
}

interface ApiChat {
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

interface ApiContact {
  jid: string;
  name: string;
  phone: string | null;
  status: string | null;
  avatar_url: string | null;
  is_business: boolean;
  is_registered: boolean;
}

interface BootstrapResponse {
  qr_code: string | null;
  is_connected: boolean;
  is_syncing: boolean;
  chats: ApiChat[];
  contacts: ApiContact[];
  logout_hint: string | null;
}

function normalizeMessage(input: Partial<ApiMessage> | null | undefined): ApiMessage {
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
          file_name: input.media.file_name ?? null,
          file_length: input.media.file_length ?? null,
          width: input.media.width ?? null,
          height: input.media.height ?? null,
          duration_seconds: input.media.duration_seconds ?? null,
          is_voice_note: Boolean(input.media.is_voice_note),
          is_gif: Boolean(input.media.is_gif),
          is_sticker: Boolean(input.media.is_sticker),
          download_path: input.media.download_path ?? null,
        }
      : null,
    receipt_status: input?.receipt_status ?? null,
  };
}

function normalizeChat(input: Partial<ApiChat> | null | undefined): ApiChat {
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

function normalizeContact(input: Partial<ApiContact> | null | undefined): ApiContact {
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

function normalizeBootstrap(input: Partial<BootstrapResponse> | null | undefined): BootstrapResponse {
  return {
    qr_code: input?.qr_code ?? null,
    is_connected: Boolean(input?.is_connected),
    is_syncing: Boolean(input?.is_syncing),
    chats: Array.isArray(input?.chats) ? input!.chats.map(normalizeChat) : [],
    contacts: Array.isArray(input?.contacts) ? input!.contacts.map(normalizeContact) : [],
    logout_hint: input?.logout_hint ?? null,
  };
}

function SendIcon() {
  return (
    <svg viewBox="0 0 24 24" width="24" height="24" className="fill-current">
      <path d="M1.101 21.757 23.8 12.028 1.101 2.3l.011 7.912 13.239 1.816-13.239 1.817-.011 7.912z" />
    </svg>
  );
}

function SearchIcon() {
  return (
    <svg viewBox="0 0 24 24" width="20" height="20" className="fill-current">
      <path d="M15.009 13.805h-.636l-.22-.219a5.184 5.184 0 0 0 1.256-3.386 5.207 5.207 0 1 0-5.207 5.208 5.183 5.183 0 0 0 3.385-1.255l.221.22v.635l4.004 3.999 1.194-1.195-3.997-4.007zm-4.808 0a3.6 3.6 0 1 1 0-7.2 3.6 3.6 0 0 1 0 7.2z" />
    </svg>
  );
}

function NewChatIcon() {
  return (
    <svg viewBox="0 0 24 24" width="24" height="24" className="fill-current">
      <path d="M19.005 3.175H4.674C3.642 3.175 3 3.789 3 4.821V21.02l3.544-3.514h12.461c1.033 0 2.064-1.06 2.064-2.093V4.821c-.001-1.032-1.032-1.646-2.064-1.646zm-4.989 9.869H7.041V11.1h6.975v1.944zm3-4H7.041V7.1h9.975v1.944z" />
    </svg>
  );
}

function PeopleIcon() {
  return (
    <svg viewBox="0 0 24 24" width="22" height="22" className="fill-current">
      <path d="M16 11c1.66 0 2.99-1.34 2.99-3S17.66 5 16 5s-3 1.34-3 3 1.34 3 3 3zm-8 0c1.66 0 2.99-1.34 2.99-3S9.66 5 8 5 5 6.34 5 8s1.34 3 3 3zm0 2c-2.33 0-7 1.17-7 3.5V19h14v-2.5C15 14.17 10.33 13 8 13zm8 0c-.29 0-.62.02-.97.05 1.16.84 1.97 1.98 1.97 3.45V19h6v-2.5c0-2.33-4.67-3.5-7-3.5z" />
    </svg>
  );
}

function LogoutIcon() {
  return (
    <svg viewBox="0 0 24 24" width="22" height="22" className="fill-current">
      <path d="M13 3v2h4v14h-4v2h6V3h-6zm-1 4-1.41 1.41L12.17 10H3v2h9.17l-1.58 1.59L12 15l4-4-4-4z" />
    </svg>
  );
}

function WhatsAppLogo() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width="39" height="39" viewBox="0 0 39 39">
      <path
        fill="#00E676"
        d="M10.7 32.8l.6.3c2.5 1.5 5.3 2.2 8.1 2.2 8.8 0 16-7.2 16-16 0-4.2-1.7-8.3-4.7-11.3s-7-4.7-11.3-4.7c-8.8 0-16 7.2-15.9 16.1 0 3 .9 5.9 2.4 8.4l.4.6-1.6 5.9 6-1.5z"
      />
      <path
        fill="#fff"
        d="M32.4 6.4C29 2.9 24.3 1 19.5 1 9.3 1 1.1 9.3 1.2 19.4c0 3.2.9 6.3 2.4 9.1L1 38l9.7-2.5c2.7 1.5 5.7 2.2 8.7 2.2 10.1 0 18.3-8.3 18.3-18.4 0-4.9-1.9-9.5-5.3-12.9zM19.5 34.6c-2.7 0-5.4-.7-7.7-2.1l-.6-.3-5.8 1.5L6.9 28l-.4-.6c-4.4-7.1-2.3-16.5 4.9-20.9s16.5-2.3 20.9 4.9 2.3 16.5-4.9 20.9c-2.3 1.5-5.1 2.3-7.9 2.3zm8.8-11.1l-1.1-.5s-1.6-.7-2.6-1.2c-.1 0-.2-.1-.3-.1-.3 0-.5.1-.7.2 0 0-.1.1-1.5 1.7-.1.2-.3.3-.5.3h-.1c-.1 0-.3-.1-.4-.2l-.5-.2c-1.1-.5-2.1-1.1-2.9-1.9-.2-.2-.5-.4-.7-.6-.7-.7-1.4-1.5-1.9-2.4l-.1-.2c-.1-.1-.1-.2-.2-.4 0-.2 0-.4.1-.5 0 0 .4-.5.7-.8.2-.2.3-.5.5-.7.2-.3.3-.7.2-1-.1-.5-1.3-3.2-1.6-3.8-.2-.3-.4-.4-.7-.5h-1.1c-.2 0-.4.1-.6.1l-.1.1c-.2.1-.4.3-.6.4-.2.2-.3.4-.5.6-.7.9-1.1 2-1.1 3.1 0 .8.2 1.6.5 2.3l.1.3c.9 1.9 2.1 3.6 3.7 5.1l.4.4c.3.3.6.5.8.8 2.1 1.8 4.5 3.1 7.2 3.8.3.1.7.1 1 .2h1c.5 0 1.1-.2 1.5-.4.3-.2.5-.2.7-.4l.2-.2c.2-.2.4-.3.6-.5s.3-.4.5-.6c.2-.4.3-.9.4-1.4v-.7s-.1-.1-.3-.2z"
      />
    </svg>
  );
}

function clsx(...values: Array<string | false | null | undefined>) {
  return values.filter(Boolean).join(" ");
}

function formatTime(timestampMs: number | null) {
  if (!timestampMs) return "";
  return new Date(timestampMs).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
}

function initials(name: string) {
  return (
    name
      .split(" ")
      .filter(Boolean)
      .slice(0, 2)
      .map((part) => part[0]?.toUpperCase() ?? "")
      .join("") || "WA"
  );
}

function phoneFromJid(jid: string | null) {
  if (!jid) return null;
  if (jid.endsWith("@s.whatsapp.net")) return jid.split("@")[0];
  return null;
}

function displayName(name: string | null | undefined, jid: string, phone?: string | null) {
  const trimmed = name?.trim();
  if (trimmed && trimmed !== jid) return trimmed;
  const fallbackPhone = phone ?? phoneFromJid(jid);
  if (fallbackPhone) return `+${fallbackPhone}`;
  return trimmed || jid;
}

function makeManualChat(phone: string): ApiChat {
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

function formatFileSize(bytes: number | null) {
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

function formatPresence(chat: ApiChat) {
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

function getReceiptIcon(status: string | null) {
  if (!status) return "";
  if (status === "played") return "▶▶";
  if (status === "read") return "✓✓";
  if (status === "delivered") return "✓✓";
  return "✓";
}

function getReceiptColor(status: string | null) {
  if (status === "read" || status === "played") return "text-sky-500";
  return "text-wa-text-secondary";
}

function mentionTokenFromJid(jid: string) {
  return jid.split("@")[0]?.split(":")[0] ?? jid;
}

function mentionLabel(name: string) {
  return `@${name.replace(/^\+/, "")}`;
}

function shouldRenderMessageText(message: ApiMessage) {
  const text = (message.text || message.media?.caption || "").trim();
  if (!text) return false;
  if (text === "Sticker" || text === "<non-text>") return false;
  return true;
}

function renderTextWithMentions(message: ApiMessage) {
  const text = message.text || message.media?.caption || "";
  const mentions = Array.isArray(message.mentions) ? message.mentions : [];
  if (!mentions.length) return text;

  let output = text;
  for (const mention of mentions) {
    const mentionToken = mentionTokenFromJid(mention.jid);
    const label = mentionLabel(mention.name);
    output = output.replaceAll(`@${mentionToken}`, label);

    const phone = phoneFromJid(mention.jid);
    if (phone) output = output.replaceAll(`@${phone}`, label);
  }
  return output;
}

function MessageMedia({ message }: { message: ApiMessage }) {
  if (!message.media?.download_path) return null;

  if (message.media.kind === "image" || message.media.kind === "sticker") {
    return (
      <img
        src={message.media.download_path}
        alt={message.media.caption ?? message.media.file_name ?? message.media.kind}
        className={clsx(
          "mb-2 max-h-72 rounded-2xl object-contain",
          message.media.kind === "sticker" && "max-h-40 bg-transparent",
        )}
      />
    );
  }

  if (message.media.kind === "video") {
    return (
      <video
        src={message.media.download_path}
        controls
        playsInline
        className="mb-2 max-h-80 rounded-2xl bg-black"
      />
    );
  }

  if (message.media.kind === "audio") {
    return <audio src={message.media.download_path} controls className="mb-2 w-full max-w-xs" />;
  }

  return (
    <a
      href={message.media.download_path}
      target="_blank"
      rel="noreferrer"
      className="mb-2 flex items-center gap-3 rounded-2xl bg-black/5 px-3 py-3 text-sm text-wa-text transition hover:bg-black/10"
    >
      <span className="text-lg">📎</span>
      <span className="min-w-0 flex-1">
        <span className="block truncate font-medium">{message.media.file_name ?? "Document"}</span>
        <span className="block text-xs text-wa-text-secondary">{formatFileSize(message.media.file_length)}</span>
      </span>
    </a>
  );
}

export default function Home() {
  const [bootstrap, setBootstrap] = useState<BootstrapResponse | null>(null);
  const [manualChats, setManualChats] = useState<ApiChat[]>([]);
  const [activeChatId, setActiveChatId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ApiMessage[]>([]);
  const [view, setView] = useState<SidebarView>("chats");
  const [search, setSearch] = useState("");
  const [messageText, setMessageText] = useState("");
  const [newPhone, setNewPhone] = useState("");
  const [sendStatus, setSendStatus] = useState<string | null>(null);
  const [showNewChat, setShowNewChat] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [isLoggingOut, setIsLoggingOut] = useState(false);
  const [picker, setPicker] = useState<"emoji" | "gif" | "sticker" | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const typingTimeoutRef = useRef<number | null>(null);

  const chats = useMemo(() => {
    const map = new Map<string, ApiChat>();
    for (const chat of bootstrap?.chats ?? []) map.set(chat.jid, chat);
    for (const chat of manualChats) {
      if (!map.has(chat.jid)) map.set(chat.jid, chat);
    }
    return Array.from(map.values()).sort(
      (a, b) => (b.timestamp_ms ?? 0) - (a.timestamp_ms ?? 0),
    );
  }, [bootstrap?.chats, manualChats]);

  const contacts = bootstrap?.contacts ?? [];
  const isConnected = bootstrap?.is_connected ?? false;
  const isSyncing = bootstrap?.is_syncing ?? false;
  const qrCode = bootstrap?.qr_code ?? null;

  const entityIndex = useMemo(() => {
    const map = new Map<string, { jid: string; name: string }>();
    for (const contact of contacts) {
      map.set(contact.jid, { jid: contact.jid, name: contact.name });
      if (contact.phone) map.set(contact.phone.toLowerCase(), { jid: contact.jid, name: contact.name });
      map.set(contact.name.toLowerCase(), { jid: contact.jid, name: contact.name });
    }
    for (const chat of chats) {
      map.set(chat.jid, { jid: chat.jid, name: chat.name });
      if (chat.phone) map.set(chat.phone.toLowerCase(), { jid: chat.jid, name: chat.name });
      map.set(chat.name.toLowerCase(), { jid: chat.jid, name: chat.name });
    }
    return map;
  }, [chats, contacts]);

  const extractMentions = useCallback(
    (text: string) => {
      const seen = new Set<string>();
      return text
        .split(/\s+/)
        .map((part) => part.trim())
        .filter((part) => part.startsWith("@") && part.length > 1)
        .map((part) => part.slice(1).replace(/[^a-zA-Z0-9._-]/g, "").toLowerCase())
        .map((token) => entityIndex.get(token) ?? null)
        .filter((value): value is { jid: string; name: string } => Boolean(value))
        .filter((value) => {
          if (seen.has(value.jid)) return false;
          seen.add(value.jid);
          return true;
        });
    },
    [entityIndex],
  );

  const activeChat = useMemo(
    () => chats.find((chat) => chat.jid === activeChatId) ?? null,
    [activeChatId, chats],
  );

  const filteredChats = useMemo(() => {
    const needle = search.trim().toLowerCase();
    if (!needle) return chats;
    return chats.filter((chat) =>
      [chat.name, chat.phone ?? "", chat.preview ?? ""]
        .join(" ")
        .toLowerCase()
        .includes(needle),
    );
  }, [chats, search]);

  const filteredContacts = useMemo(() => {
    const needle = search.trim().toLowerCase();
    if (!needle) return contacts;
    return contacts.filter((contact) =>
      [contact.name, contact.phone ?? "", contact.status ?? ""]
        .join(" ")
        .toLowerCase()
        .includes(needle),
    );
  }, [contacts, search]);

  const loadMessages = useCallback(async (chatId: string) => {
    try {
      const res = await fetch(`/api/chats/${encodeURIComponent(chatId)}/messages`, {
        cache: "no-store",
      });
      if (res.status === 404) {
        setMessages([]);
        return;
      }
      if (!res.ok) return;
      const data = (await res.json()) as { messages?: ApiMessage[] };
      setMessages(Array.isArray(data.messages) ? data.messages.map(normalizeMessage) : []);
      void fetch(`/api/chats/${encodeURIComponent(chatId)}/read`, { method: "POST" });
    } catch {
      setMessages([]);
    }
  }, []);

  const refresh = useCallback(
    async (preferredChatId?: string | null) => {
      try {
        const res = await fetch("/api/bootstrap", { cache: "no-store" });
        if (!res.ok) return;
        const data = normalizeBootstrap((await res.json()) as Partial<BootstrapResponse>);
        setBootstrap(data);

        const map = new Map<string, ApiChat>();
        for (const chat of data.chats) map.set(chat.jid, chat);
        for (const chat of manualChats) {
          if (!map.has(chat.jid)) map.set(chat.jid, chat);
        }
        const mergedChats = Array.from(map.values()).sort(
          (a, b) => (b.timestamp_ms ?? 0) - (a.timestamp_ms ?? 0),
        );

        const nextActive = preferredChatId ?? activeChatId ?? mergedChats[0]?.jid ?? null;
        if (nextActive !== activeChatId) setActiveChatId(nextActive);

        if (nextActive) {
          await loadMessages(nextActive);
        } else {
          setMessages([]);
        }
      } catch {
        // keep polling while the backend starts
      } finally {
        setIsLoading(false);
      }
    },
    [activeChatId, loadMessages, manualChats],
  );

  useEffect(() => {
    void refresh();
    const id = setInterval(() => {
      void refresh();
    }, 3000);
    return () => clearInterval(id);
  }, [refresh]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  useEffect(() => {
    if (!activeChat || !messageText.trim()) {
      if (typingTimeoutRef.current) {
        window.clearTimeout(typingTimeoutRef.current);
        typingTimeoutRef.current = null;
      }
      return;
    }

    void fetch(`/api/chats/${encodeURIComponent(activeChat.jid)}/typing`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ state: "composing" }),
    });

    if (typingTimeoutRef.current) {
      window.clearTimeout(typingTimeoutRef.current);
    }

    typingTimeoutRef.current = window.setTimeout(() => {
      void fetch(`/api/chats/${encodeURIComponent(activeChat.jid)}/typing`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ state: "paused" }),
      });
    }, 1200);

    return () => {
      if (typingTimeoutRef.current) {
        window.clearTimeout(typingTimeoutRef.current);
      }
    };
  }, [activeChat, messageText]);

  const handleSelectChat = useCallback(
    async (chatId: string) => {
      setView("chats");
      setActiveChatId(chatId);
      await loadMessages(chatId);
    },
    [loadMessages],
  );

  const startNewChat = useCallback(async () => {
    const phone = newPhone.replace(/\D/g, "");
    if (!phone) return;

    const manual = makeManualChat(phone);
    setManualChats((prev) =>
      prev.some((chat) => chat.jid === manual.jid) ? prev : [manual, ...prev],
    );
    setActiveChatId(manual.jid);
    setShowNewChat(false);
    setView("chats");
    setNewPhone("");
    setMessages([]);
  }, [newPhone]);

  const handleLogout = useCallback(async () => {
    setIsLoggingOut(true);
    try {
      const res = await fetch("/api/auth/logout", { method: "POST" });
      const data = await res.json();
      setSendStatus(data.message ?? "Disconnected");
      setBootstrap((prev) =>
        prev
          ? {
              ...prev,
              is_connected: false,
              is_syncing: false,
              chats: [],
              contacts: [],
            }
          : null,
      );
      setMessages([]);
      setManualChats([]);
      setActiveChatId(null);
      await refresh(null);
    } catch {
      setSendStatus("Failed to logout");
    } finally {
      setIsLoggingOut(false);
      window.setTimeout(() => setSendStatus(null), 4000);
    }
  }, [refresh]);

  const handleSend = useCallback(
    async (event: FormEvent) => {
      event.preventDefault();
      if (!activeChat || !messageText.trim()) return;

      const mentions = extractMentions(messageText);

      try {
        const res = await fetch("/api/messages/send", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            jid: activeChat.jid,
            phone: activeChat.phone ?? phoneFromJid(activeChat.jid),
            message: messageText,
            mentions: mentions.map((mention) => mention.jid),
          }),
        });
        const data = await res.json();

        if (data.success) {
          const optimistic: ApiMessage = {
            id: data.message_id || `${Date.now()}`,
            chat_jid: activeChat.jid,
            sender_jid: "me",
            sender_name: "You",
            text: messageText,
            timestamp_ms: Date.now(),
            from_me: true,
            mentions,
            media: null,
            receipt_status: "sent",
          };
          setMessages((prev) => [...prev, optimistic]);
          setMessageText("");
          setPicker(null);
          setSendStatus("Message sent!");
          await refresh(activeChat.jid);
        } else {
          setSendStatus(data.error ?? "Failed to send message");
        }
      } catch {
        setSendStatus("Failed to send message");
      }

      window.setTimeout(() => setSendStatus(null), 3000);
    },
    [activeChat, extractMentions, messageText, refresh],
  );

  if (isLoading) {
    return (
      <div className="flex h-screen items-center justify-center bg-wa-bg">
        <div className="flex flex-col items-center gap-4">
          <WhatsAppLogo />
          <div className="h-1 w-48 overflow-hidden rounded-full bg-gray-200">
            <div className="h-full w-1/2 animate-pulse rounded-full bg-wa-teal" />
          </div>
          <p className="text-sm text-wa-text-secondary">Connecting to WhatsApp…</p>
        </div>
      </div>
    );
  }

  if (!isConnected) {
    return (
      <div className="flex h-screen flex-col bg-wa-bg">
        <div className="h-[222px] w-full bg-wa-teal-dark" />
        <div className="-mt-[170px] flex flex-1 items-start justify-center">
          <div className="mx-8 flex w-full max-w-[1000px] overflow-hidden rounded-sm bg-white shadow-lg">
            <div className="flex-1 p-16">
              <h1 className="mb-6 text-2xl font-light text-wa-text">Use WhatsApp on your computer</h1>
              <ol className="space-y-4 text-wa-text-secondary">
                <li className="flex gap-2"><span className="font-medium text-wa-text">1.</span>Open WhatsApp on your phone</li>
                <li className="flex gap-2"><span className="font-medium text-wa-text">2.</span>Open Linked Devices</li>
                <li className="flex gap-2"><span className="font-medium text-wa-text">3.</span>Tap Link a Device</li>
                <li className="flex gap-2"><span className="font-medium text-wa-text">4.</span>Scan the QR code shown here</li>
              </ol>
            </div>
            <div className="flex items-center justify-center p-16">
              {qrCode ? (
                <div data-testid="qr-code" className="rounded-2xl border border-gray-100 p-4 shadow-sm">
                  <QRCode value={qrCode} size={264} bgColor="#ffffff" fgColor="#111b21" level="M" />
                </div>
              ) : (
                <div className="flex h-[264px] w-[264px] items-center justify-center">
                  <div className="flex flex-col items-center gap-3 text-wa-text-secondary">
                    <div className="h-8 w-8 animate-spin rounded-full border-4 border-gray-200 border-t-wa-teal" />
                    <p className="text-sm">Waiting for QR code…</p>
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-wa-bg">
      <div className="flex h-full w-full overflow-hidden shadow-2xl">
        <aside className="flex w-[72px] flex-col items-center justify-between border-r border-[#202c33] bg-[#202c33] py-4 text-white">
          <button
            className={clsx(
              "flex h-12 w-12 items-center justify-center rounded-2xl transition-colors",
              view === "chats" ? "bg-wa-teal text-white" : "text-white/70 hover:bg-white/10",
            )}
            onClick={() => setView("chats")}
            aria-label="Chats"
          >
            <WhatsAppLogo />
          </button>
          <div className="flex flex-col items-center gap-3">
            <button
              className={clsx(
                "flex h-12 w-12 items-center justify-center rounded-2xl transition-colors",
                view === "contacts" ? "bg-white/10 text-white" : "text-white/70 hover:bg-white/10",
              )}
              onClick={() => setView("contacts")}
              aria-label="Contacts"
            >
              <PeopleIcon />
            </button>
            <button
              data-testid="logout-button"
              className="flex h-12 w-12 items-center justify-center rounded-2xl text-white/70 transition-colors hover:bg-red-500/20 hover:text-red-200"
              onClick={() => void handleLogout()}
              disabled={isLoggingOut}
              aria-label="Logout"
            >
              <LogoutIcon />
            </button>
          </div>
        </aside>

        <section className="flex w-[390px] flex-col border-r border-wa-border bg-white">
          <div className="border-b border-wa-border bg-[#f0f2f5] px-4 py-3">
            <div className="flex items-center justify-between gap-3">
              <div>
                <h1 className="text-lg font-medium text-wa-text">WhatsApp Web</h1>
                <p className="text-xs text-wa-text-secondary">
                  {isSyncing ? "Syncing recent chats and contacts…" : "Connected to your phone"}
                </p>
              </div>
              <button
                onClick={() => setShowNewChat((current) => !current)}
                className="rounded-full p-2 text-wa-icon transition-colors hover:bg-gray-200"
                title="New chat"
                data-testid="toggle-new-chat"
              >
                <NewChatIcon />
              </button>
            </div>
            {bootstrap?.logout_hint ? (
              <p className="mt-2 text-[11px] leading-4 text-wa-text-secondary">{bootstrap.logout_hint}</p>
            ) : null}
          </div>

          {showNewChat ? (
            <div className="border-b border-wa-border bg-white p-3">
              <div className="flex gap-2">
                <input
                  type="text"
                  data-testid="phone-input"
                  value={newPhone}
                  onChange={(e) => setNewPhone(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && void startNewChat()}
                  placeholder="Phone number (e.g. 15551234567)"
                  className="flex-1 rounded-lg bg-wa-input-bg px-4 py-2 text-sm text-wa-text outline-none placeholder:text-wa-text-secondary"
                />
                <button
                  data-testid="new-chat-button"
                  onClick={() => void startNewChat()}
                  className="rounded-lg bg-wa-teal px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-wa-teal-dark"
                >
                  Start
                </button>
              </div>
            </div>
          ) : null}

          <div className="border-b border-wa-border p-2">
            <div className="flex items-center gap-3 rounded-lg bg-wa-input-bg px-3 py-2 text-wa-icon">
              <SearchIcon />
              <input
                type="text"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder={view === "chats" ? "Search chats" : "Search contacts"}
                className="flex-1 bg-transparent text-sm text-wa-text outline-none placeholder:text-wa-text-secondary"
              />
            </div>
            <div className="mt-3 flex rounded-xl bg-[#f0f2f5] p-1 text-sm">
              <button
                onClick={() => setView("chats")}
                className={clsx(
                  "flex-1 rounded-lg px-3 py-2 transition-colors",
                  view === "chats" ? "bg-white text-wa-text shadow-sm" : "text-wa-text-secondary",
                )}
              >
                Chats
              </button>
              <button
                onClick={() => setView("contacts")}
                className={clsx(
                  "flex-1 rounded-lg px-3 py-2 transition-colors",
                  view === "contacts" ? "bg-white text-wa-text shadow-sm" : "text-wa-text-secondary",
                )}
              >
                Contacts
              </button>
            </div>
          </div>

          <div className="flex-1 overflow-y-auto">
            {view === "chats" ? (
              filteredChats.length === 0 ? (
                <div className="flex h-full flex-col items-center justify-center px-8 text-center text-wa-text-secondary">
                  <NewChatIcon />
                  <p className="mt-3 text-sm">
                    {isSyncing
                      ? "Chats will appear here as sync completes or new messages arrive."
                      : "No chats yet. Start a new conversation or wait for incoming messages."}
                  </p>
                </div>
              ) : (
                filteredChats.map((chat) => (
                  (() => {
                    const shownName = displayName(chat.name, chat.jid, chat.phone);
                    return (
                  <button
                    key={chat.jid}
                    onClick={() => void handleSelectChat(chat.jid)}
                    className={clsx(
                      "flex w-full items-center gap-3 px-3 py-3 text-left transition-colors hover:bg-wa-sidebar-hover",
                      activeChatId === chat.jid && "bg-wa-input-bg",
                    )}
                  >
                    <div className="flex h-12 w-12 flex-shrink-0 items-center justify-center rounded-full bg-[#dfe5e7] text-sm font-semibold text-wa-text">
                      {chat.avatar_url ? (
                        <img src={chat.avatar_url} alt={shownName} className="h-full w-full rounded-full object-cover" />
                      ) : (
                        initials(shownName)
                      )}
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center justify-between gap-3">
                        <span className="truncate text-[17px] text-wa-text">{shownName}</span>
                        <span className="text-xs text-wa-text-secondary">{formatTime(chat.timestamp_ms)}</span>
                      </div>
                      <div className="mt-1 flex items-center justify-between gap-3">
                        <p className="truncate text-sm text-wa-text-secondary">
                          {chat.typing ?? chat.preview ?? (chat.is_group ? "Group synced" : chat.phone ? `+${chat.phone}` : chat.jid)}
                        </p>
                        {chat.unread_count > 0 ? (
                          <span className="rounded-full bg-wa-teal px-2 py-0.5 text-[11px] font-semibold text-white">
                            {chat.unread_count}
                          </span>
                        ) : null}
                      </div>
                    </div>
                  </button>
                    );
                  })()
                ))
              )
            ) : filteredContacts.length === 0 ? (
              <div className="flex h-full flex-col items-center justify-center px-8 text-center text-wa-text-secondary">
                <PeopleIcon />
                <p className="mt-3 text-sm">Known contacts will appear here as the session syncs.</p>
              </div>
            ) : (
              filteredContacts.map((contact) => (
                (() => {
                  const shownName = displayName(contact.name, contact.jid, contact.phone);
                  return (
                <button
                  key={contact.jid}
                  onClick={() => void handleSelectChat(contact.jid)}
                  className="flex w-full items-center gap-3 px-3 py-3 text-left transition-colors hover:bg-wa-sidebar-hover"
                >
                  <div className="flex h-12 w-12 flex-shrink-0 items-center justify-center rounded-full bg-[#dfe5e7] text-sm font-semibold text-wa-text">
                    {contact.avatar_url ? (
                      <img src={contact.avatar_url} alt={shownName} className="h-full w-full rounded-full object-cover" />
                    ) : (
                      initials(shownName)
                    )}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="truncate text-[15px] text-wa-text">{shownName}</span>
                      {contact.is_business ? (
                        <span className="rounded-full bg-[#e7fce3] px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-[#057a55]">
                          Business
                        </span>
                      ) : null}
                    </div>
                    <p className="truncate text-sm text-wa-text-secondary">
                      {contact.status ?? contact.phone ?? contact.jid}
                    </p>
                  </div>
                </button>
                  );
                })()
              ))
            )}
          </div>
        </section>

        <main className="flex flex-1 flex-col bg-[#efeae2]">
          {activeChat ? (
            <>
              {(() => {
                const shownName = displayName(activeChat.name, activeChat.jid, activeChat.phone);
                return (
              <header className="flex h-[60px] items-center justify-between border-b border-wa-border bg-[#f0f2f5] px-4">
                <div className="flex items-center gap-3">
                  <div className="flex h-10 w-10 items-center justify-center rounded-full bg-[#dfe5e7] text-sm font-semibold text-wa-text">
                    {activeChat.avatar_url ? (
                      <img src={activeChat.avatar_url} alt={shownName} className="h-full w-full rounded-full object-cover" />
                    ) : (
                      initials(shownName)
                    )}
                  </div>
                  <div>
                    <h2 className="text-sm font-medium text-wa-text">{shownName}</h2>
                    <p className="text-xs text-wa-text-secondary">
                      {formatPresence(activeChat)}
                    </p>
                  </div>
                </div>
                {isSyncing ? (
                  <span className="rounded-full bg-[#e7fce3] px-3 py-1 text-xs font-medium text-[#057a55]">
                    Syncing
                  </span>
                ) : null}
              </header>
                );
              })()}

              <div className="chat-bg flex-1 overflow-y-auto px-6 py-6">
                {messages.length === 0 ? (
                  <div className="flex h-full items-center justify-center">
                    <div className="max-w-md rounded-2xl bg-white/80 px-6 py-5 text-center text-sm text-wa-text-secondary shadow-sm backdrop-blur">
                      <p className="font-medium text-wa-text">No local messages yet for this chat.</p>
                      <p className="mt-2">
                        Live incoming messages and newly sent messages will appear here. Synced chat metadata is shown in the sidebar as it becomes available.
                      </p>
                    </div>
                  </div>
                ) : (
                  messages.map((message) => (
                    <div
                      key={message.id}
                      data-testid="message-bubble"
                      className={clsx("mb-2 flex", message.from_me ? "justify-end" : "justify-start")}
                    >
                      <div
                        className={clsx(
                          "max-w-[70%] rounded-2xl px-3 py-2 shadow-sm",
                          message.from_me ? "bg-[#d9fdd3]" : "bg-white",
                        )}
                      >
                        {!message.from_me && message.sender_name ? (
                          <p className="mb-1 text-[11px] font-semibold uppercase tracking-wide text-wa-teal-dark">
                            {message.sender_name}
                          </p>
                        ) : null}
                        <MessageMedia message={message} />
                        {shouldRenderMessageText(message) ? (
                          <p className="whitespace-pre-wrap text-sm text-wa-text">{renderTextWithMentions(message)}</p>
                        ) : null}
                        <div className="mt-1 flex items-center justify-end gap-1 text-right text-[11px] text-wa-text-secondary">
                          <span>{formatTime(message.timestamp_ms)}</span>
                          {message.from_me ? (
                            <span className={clsx("font-semibold", getReceiptColor(message.receipt_status))}>
                              {getReceiptIcon(message.receipt_status)}
                            </span>
                          ) : null}
                        </div>
                      </div>
                    </div>
                  ))
                )}
                <div ref={messagesEndRef} />
              </div>

              <footer className="border-t border-wa-border bg-[#f0f2f5] px-4 py-3">
                {sendStatus ? (
                  <div
                    data-testid="send-status"
                    className="mb-2 rounded-lg bg-white px-3 py-2 text-sm text-wa-text-secondary shadow-sm"
                  >
                    {sendStatus}
                  </div>
                ) : null}
                {picker ? (
                  <div className="mb-2 rounded-2xl bg-white px-3 py-3 shadow-sm">
                    <div className="mb-2 flex items-center justify-between text-xs font-medium uppercase tracking-wide text-wa-text-secondary">
                      <span>{picker === "emoji" ? "Emoji" : picker === "gif" ? "GIF shortcuts" : "Sticker shortcuts"}</span>
                      <button onClick={() => setPicker(null)} type="button" className="text-wa-teal">Close</button>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      {(picker === "emoji"
                        ? ["😀", "😂", "😍", "🙏", "🔥", "🎉", "👍", "❤️"]
                        : picker === "gif"
                          ? ["[GIF] thumbs up", "[GIF] applause", "[GIF] wow", "[GIF] hello"]
                          : ["[Sticker] 👍", "[Sticker] 😂", "[Sticker] ❤️", "[Sticker] 🎉"]
                      ).map((item) => (
                        <button
                          key={item}
                          type="button"
                          onClick={() => setMessageText((current) => `${current}${current ? " " : ""}${item}`)}
                          className="rounded-full bg-wa-input-bg px-3 py-2 text-sm text-wa-text transition hover:bg-gray-200"
                        >
                          {item}
                        </button>
                      ))}
                    </div>
                  </div>
                ) : null}
                <form onSubmit={handleSend} className="flex items-center gap-3">
                  <div className="flex items-center gap-2 text-xl text-wa-icon">
                    <button type="button" onClick={() => setPicker((current) => current === "emoji" ? null : "emoji")} className="rounded-full p-2 transition hover:bg-white">
                      😊
                    </button>
                    <button type="button" onClick={() => setPicker((current) => current === "gif" ? null : "gif")} className="rounded-full p-2 text-sm font-semibold transition hover:bg-white">
                      GIF
                    </button>
                    <button type="button" onClick={() => setPicker((current) => current === "sticker" ? null : "sticker")} className="rounded-full p-2 transition hover:bg-white">
                      🪄
                    </button>
                  </div>
                  <input
                    data-testid="message-input"
                    type="text"
                    value={messageText}
                    onChange={(e) => setMessageText(e.target.value)}
                    placeholder="Type a message or use @name / @phone for mentions"
                    className="flex-1 rounded-xl border border-transparent bg-white px-4 py-3 text-sm text-wa-text outline-none transition focus:border-wa-teal disabled:cursor-not-allowed disabled:bg-gray-100"
                  />
                  <button
                    data-testid="send-button"
                    type="submit"
                    disabled={!messageText.trim()}
                    className="rounded-full bg-wa-teal p-3 text-white transition hover:bg-wa-teal-dark disabled:cursor-not-allowed disabled:bg-gray-300"
                  >
                    <SendIcon />
                  </button>
                </form>
              </footer>
            </>
          ) : (
            <div className="flex flex-1 flex-col items-center justify-center bg-wa-bg px-8 text-center text-wa-text-secondary">
              <WhatsAppLogo />
              <h2 className="mt-6 text-3xl font-light text-wa-text">WhatsApp Web</h2>
              <p className="mt-2 text-sm">Choose a synced chat, a contact, or start a new direct conversation.</p>
            </div>
          )}
        </main>
      </div>
    </div>
  );
}
