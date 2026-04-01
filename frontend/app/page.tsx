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

import type {
  AliasListResponse,
  ApiChat,
  ApiMessage,
  BootstrapResponse,
  ContactAlias,
  SidebarView,
} from "./lib/types";
import { normalizeBootstrap, normalizeMessage } from "./lib/normalizers";
import {
  clsx,
  displayName,
  formatPresence,
  formatTime,
  getReceiptColor,
  getReceiptIcon,
  initials,
  makeManualChat,
  mentionTokenFromJid,
  phoneFromJid,
  renderTextWithMentions,
  shouldRenderMessageText,
} from "./lib/helpers";
import {
  LogoutIcon,
  NewChatIcon,
  PeopleIcon,
  SearchIcon,
  SendIcon,
  WhatsAppLogo,
} from "./components/Icons";
import { MessageMedia } from "./components/MessageMedia";
import { MediaPicker, type PickerTab } from "./components/MediaPicker";
import type { StickerInfo } from "./components/StickerPicker";
import { AliasEditor } from "./components/AliasEditor";

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
  const [picker, setPicker] = useState<PickerTab | null>(null);
  const [stickers, setStickers] = useState<StickerInfo[]>([]);
  const [aliases, setAliases] = useState<ContactAlias[]>([]);
  const [showAliasEditor, setShowAliasEditor] = useState(false);
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

  // Build a phone→alias lookup from the aliases list (highest priority)
  const aliasMap = useMemo(() => {
    const map = new Map<string, string>();
    for (const a of aliases) map.set(a.phone, a.name);
    return map;
  }, [aliases]);

  const mentionNameByJid = useMemo(() => {
    const map = new Map<string, string>();

    for (const contact of contacts) {
      // Check alias first
      const alias = contact.phone ? aliasMap.get(contact.phone) : undefined;
      const name = alias ?? displayName(contact.name, contact.jid, contact.phone);
      map.set(contact.jid, name);
      if (contact.phone) map.set(`${contact.phone}@s.whatsapp.net`, name);
    }

    for (const chat of chats) {
      const alias = chat.phone ? aliasMap.get(chat.phone) : undefined;
      const name = alias ?? displayName(chat.name, chat.jid, chat.phone);
      if (!map.has(chat.jid)) map.set(chat.jid, name);
      if (chat.phone && !map.has(`${chat.phone}@s.whatsapp.net`)) {
        map.set(`${chat.phone}@s.whatsapp.net`, name);
      }
    }

    return map;
  }, [chats, contacts, aliasMap]);

  const mentionNameByToken = useMemo(() => {
    const map = new Map<string, string>();

    const register = (jid: string, phone: string | null | undefined, rawName: string | null | undefined) => {
      const alias = phone ? aliasMap.get(phone) : undefined;
      const name = alias ?? displayName(rawName, jid, phone);
      const jidToken = mentionTokenFromJid(jid)?.toLowerCase();
      if (jidToken) map.set(jidToken, name);
      if (phone) map.set(phone.toLowerCase(), name);
    };

    for (const contact of contacts) {
      register(contact.jid, contact.phone, contact.name);
    }

    for (const chat of chats) {
      register(chat.jid, chat.phone, chat.name);
    }

    return map;
  }, [chats, contacts, aliasMap]);

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

  const fetchAliases = useCallback(async () => {
    try {
      const res = await fetch("/api/contact-aliases", { cache: "no-store" });
      if (res.ok) {
        const data = (await res.json()) as AliasListResponse;
        setAliases(data.aliases ?? []);
      }
    } catch {
      // ignore
    }
  }, []);

  const refresh = useCallback(
    async (preferredChatId?: string | null) => {
      try {
        const res = await fetch("/api/bootstrap", { cache: "no-store" });
        if (!res.ok) return;
        const data = normalizeBootstrap((await res.json()) as Partial<BootstrapResponse>);
        setBootstrap(data);

        // Fetch aliases in parallel with the rest
        void fetchAliases();

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
    [activeChatId, loadMessages, manualChats, fetchAliases],
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

  // ── Fetch stickers ────────────────────────────────────────────────────

  useEffect(() => {
    if (!isConnected) return;
    const fetchStickers = async () => {
      try {
        const res = await fetch("/api/stickers", { cache: "no-store" });
        if (res.ok) {
          const data = await res.json();
          setStickers(data.stickers?.map((s: Record<string, unknown>) => ({
            chatJid: s.chat_jid,
            messageId: s.message_id,
            downloadPath: s.download_path,
            isAnimated: s.is_animated,
          })) ?? []);
        }
      } catch { /* ignore */ }
    };
    void fetchStickers();
    const id = setInterval(fetchStickers, 15000);
    return () => clearInterval(id);
  }, [isConnected]);

  // ── Media send handlers ────────────────────────────────────────────────

  const handleEmojiSelect = useCallback(
    (emoji: string) => {
      setMessageText((prev) => prev + emoji);
    },
    [],
  );

  const handleGifSelect = useCallback(
    async (gifUrl: string, mp4Url: string | null, width: number, height: number) => {
      if (!activeChat) return;
      setPicker(null);
      setSendStatus("Sending GIF…");
      try {
        const res = await fetch("/api/messages/send-media", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            jid: activeChat.jid,
            phone: activeChat.phone ?? phoneFromJid(activeChat.jid),
            url: mp4Url ?? gifUrl,
            media_type: "gif",
            width,
            height,
          }),
        });
        const data = await res.json();
        if (data.success) {
          setSendStatus("GIF sent!");
          await refresh(activeChat.jid);
        } else {
          setSendStatus(data.error ?? "Failed to send GIF");
        }
      } catch {
        setSendStatus("Failed to send GIF");
      }
      window.setTimeout(() => setSendStatus(null), 3000);
    },
    [activeChat, refresh],
  );

  const handleStickerSelect = useCallback(
    async (sticker: StickerInfo) => {
      if (!activeChat) return;
      setPicker(null);
      setSendStatus("Sending sticker…");
      try {
        const stickerUrl = `/api/media/${encodeURIComponent(sticker.chatJid)}/${encodeURIComponent(sticker.messageId)}`;
        const res = await fetch("/api/messages/send-media", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            jid: activeChat.jid,
            phone: activeChat.phone ?? phoneFromJid(activeChat.jid),
            url: stickerUrl,
            media_type: "sticker",
          }),
        });
        const data = await res.json();
        if (data.success) {
          setSendStatus("Sticker sent!");
          await refresh(activeChat.jid);
        } else {
          setSendStatus(data.error ?? "Failed to send sticker");
        }
      } catch {
        setSendStatus("Failed to send sticker");
      }
      window.setTimeout(() => setSendStatus(null), 3000);
    },
    [activeChat, refresh],
  );

  // ── Loading state ──────────────────────────────────────────────────────

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

  // ── QR pairing screen ──────────────────────────────────────────────────

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

  // ── Main chat interface ────────────────────────────────────────────────

  return (
    <div className="flex h-screen bg-wa-bg">
      <div className="flex h-full w-full overflow-hidden shadow-2xl">
        {/* Navigation rail */}
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
              className={clsx(
                "flex h-12 w-12 items-center justify-center rounded-2xl transition-colors",
                showAliasEditor ? "bg-white/10 text-white" : "text-white/70 hover:bg-white/10",
              )}
              onClick={() => setShowAliasEditor((v) => !v)}
              aria-label="Contact Aliases"
              title="Contact Aliases"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2"><path strokeLinecap="round" strokeLinejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L10.582 16.07a4.5 4.5 0 01-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 011.13-1.897l8.932-8.931zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0115.75 21H5.25A2.25 2.25 0 013 18.75V8.25A2.25 2.25 0 015.25 6H10" /></svg>
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

        {/* Sidebar */}
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
                    const chatAlias = chat.phone ? aliasMap.get(chat.phone) : undefined;
                    const shownName = chatAlias ?? displayName(chat.name, chat.jid, chat.phone);
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
                  const contactAlias = contact.phone ? aliasMap.get(contact.phone) : undefined;
                  const shownName = contactAlias ?? displayName(contact.name, contact.jid, contact.phone);
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

        {/* Chat area */}
        <main className="flex flex-1 flex-col bg-[#efeae2]">
          {activeChat ? (
            <>
              {(() => {
                const activeChatAlias = activeChat.phone ? aliasMap.get(activeChat.phone) : undefined;
                const shownName = activeChatAlias ?? displayName(activeChat.name, activeChat.jid, activeChat.phone);
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
                            {(() => {
                              // Check alias for sender
                              const senderPhone = phoneFromJid(message.sender_jid);
                              const alias = senderPhone ? aliasMap.get(senderPhone) : undefined;
                              return alias ?? mentionNameByJid.get(message.sender_jid) ?? message.sender_name;
                            })()}
                          </p>
                        ) : null}
                        <MessageMedia message={message} />
                        {shouldRenderMessageText(message) ? (
                          <p className="whitespace-pre-wrap text-sm text-wa-text">{renderTextWithMentions(message, mentionNameByJid, mentionNameByToken, aliasMap)}</p>
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
                  <MediaPicker
                    activeTab={picker}
                    onTabChange={(tab) => setPicker(tab)}
                    onClose={() => setPicker(null)}
                    onEmojiSelect={handleEmojiSelect}
                    onGifSelect={handleGifSelect}
                    onStickerSelect={handleStickerSelect}
                    stickers={stickers}
                  />
                ) : null}
                <form onSubmit={handleSend} className="flex items-center gap-3">
                  <div className="flex items-center gap-1 text-xl text-wa-icon">
                    <button
                      type="button"
                      onClick={() => setPicker((current) => current === "emoji" ? null : "emoji")}
                      className={`rounded-full p-2 transition hover:bg-white ${picker === "emoji" ? "bg-white text-wa-teal" : ""}`}
                      title="Emoji"
                    >
                      😊
                    </button>
                    <button
                      type="button"
                      onClick={() => setPicker((current) => current === "gif" ? null : "gif")}
                      className={`rounded-full p-2 text-xs font-bold transition hover:bg-white ${picker === "gif" ? "bg-white text-wa-teal" : ""}`}
                      title="GIF"
                    >
                      GIF
                    </button>
                    <button
                      type="button"
                      onClick={() => setPicker((current) => current === "sticker" ? null : "sticker")}
                      className={`rounded-full p-2 transition hover:bg-white ${picker === "sticker" ? "bg-white text-wa-teal" : ""}`}
                      title="Stickers"
                    >
                      <svg viewBox="0 0 24 24" width="20" height="20" className="fill-current">
                        <path d="M21.8 10.3c-.1-.3-.2-.5-.4-.7l-.1-.1c-.1-.1-.1-.2-.2-.3l-7.3-7.3c-.1-.1-.2-.1-.3-.2l-.1-.1c-.2-.2-.5-.3-.7-.4-.3-.1-.6-.2-.9-.2H5C3.3 1 2 2.3 2 4v16c0 1.7 1.3 3 3 3h14c1.7 0 3-1.3 3-3v-8.8c0-.3-.1-.6-.2-.9zM14 3.4 20.6 10H15c-.6 0-1-.4-1-1V3.4zM20 20c0 .6-.4 1-1 1H5c-.6 0-1-.4-1-1V4c0-.6.4-1 1-1h7v6c0 1.7 1.3 3 3 3h6v8z" />
                      </svg>
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

      {/* Alias Editor Modal */}
      {showAliasEditor && (
        <AliasEditor
          aliases={aliases}
          contacts={contacts}
          chats={chats}
          onClose={() => setShowAliasEditor(false)}
          onAliasesChanged={() => void fetchAliases()}
        />
      )}
    </div>
  );
}
