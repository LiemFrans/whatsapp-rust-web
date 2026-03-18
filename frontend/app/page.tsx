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
  return jid.endsWith("@s.whatsapp.net") ? jid.split("@")[0] : null;
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
  };
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
  const messagesEndRef = useRef<HTMLDivElement>(null);

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
      const data = (await res.json()) as { messages: ApiMessage[] };
      setMessages(data.messages);
    } catch {
      setMessages([]);
    }
  }, []);

  const refresh = useCallback(
    async (preferredChatId?: string | null) => {
      try {
        const res = await fetch("/api/bootstrap", { cache: "no-store" });
        if (!res.ok) return;
        const data = (await res.json()) as BootstrapResponse;
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

      const phone = activeChat.phone ?? phoneFromJid(activeChat.jid);
      if (!phone || activeChat.is_group) {
        setSendStatus("Sending is currently available for direct chats only.");
        window.setTimeout(() => setSendStatus(null), 3000);
        return;
      }

      try {
        const res = await fetch("/api/messages/send", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ phone, message: messageText }),
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
          };
          setMessages((prev) => [...prev, optimistic]);
          setMessageText("");
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
    [activeChat, messageText, refresh],
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
                        <img src={chat.avatar_url} alt={chat.name} className="h-full w-full rounded-full object-cover" />
                      ) : (
                        initials(chat.name)
                      )}
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center justify-between gap-3">
                        <span className="truncate text-[17px] text-wa-text">{chat.name}</span>
                        <span className="text-xs text-wa-text-secondary">{formatTime(chat.timestamp_ms)}</span>
                      </div>
                      <div className="mt-1 flex items-center justify-between gap-3">
                        <p className="truncate text-sm text-wa-text-secondary">
                          {chat.preview ?? (chat.is_group ? "Group synced" : chat.phone ? `+${chat.phone}` : chat.jid)}
                        </p>
                        {chat.unread_count > 0 ? (
                          <span className="rounded-full bg-wa-teal px-2 py-0.5 text-[11px] font-semibold text-white">
                            {chat.unread_count}
                          </span>
                        ) : null}
                      </div>
                    </div>
                  </button>
                ))
              )
            ) : filteredContacts.length === 0 ? (
              <div className="flex h-full flex-col items-center justify-center px-8 text-center text-wa-text-secondary">
                <PeopleIcon />
                <p className="mt-3 text-sm">Known contacts will appear here as the session syncs.</p>
              </div>
            ) : (
              filteredContacts.map((contact) => (
                <button
                  key={contact.jid}
                  onClick={() => void handleSelectChat(contact.jid)}
                  className="flex w-full items-center gap-3 px-3 py-3 text-left transition-colors hover:bg-wa-sidebar-hover"
                >
                  <div className="flex h-12 w-12 flex-shrink-0 items-center justify-center rounded-full bg-[#dfe5e7] text-sm font-semibold text-wa-text">
                    {contact.avatar_url ? (
                      <img src={contact.avatar_url} alt={contact.name} className="h-full w-full rounded-full object-cover" />
                    ) : (
                      initials(contact.name)
                    )}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="truncate text-[15px] text-wa-text">{contact.name}</span>
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
              ))
            )}
          </div>
        </section>

        <main className="flex flex-1 flex-col bg-[#efeae2]">
          {activeChat ? (
            <>
              <header className="flex h-[60px] items-center justify-between border-b border-wa-border bg-[#f0f2f5] px-4">
                <div className="flex items-center gap-3">
                  <div className="flex h-10 w-10 items-center justify-center rounded-full bg-[#dfe5e7] text-sm font-semibold text-wa-text">
                    {activeChat.avatar_url ? (
                      <img src={activeChat.avatar_url} alt={activeChat.name} className="h-full w-full rounded-full object-cover" />
                    ) : (
                      initials(activeChat.name)
                    )}
                  </div>
                  <div>
                    <h2 className="text-sm font-medium text-wa-text">{activeChat.name}</h2>
                    <p className="text-xs text-wa-text-secondary">
                      {activeChat.status ?? (activeChat.is_group ? "Synced group" : activeChat.phone ? `+${activeChat.phone}` : activeChat.jid)}
                    </p>
                  </div>
                </div>
                {isSyncing ? (
                  <span className="rounded-full bg-[#e7fce3] px-3 py-1 text-xs font-medium text-[#057a55]">
                    Syncing
                  </span>
                ) : null}
              </header>

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
                        <p className="whitespace-pre-wrap text-sm text-wa-text">{message.text}</p>
                        <div className="mt-1 text-right text-[11px] text-wa-text-secondary">
                          {formatTime(message.timestamp_ms)}
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
                <form onSubmit={handleSend} className="flex items-center gap-3">
                  <input
                    data-testid="message-input"
                    type="text"
                    value={messageText}
                    onChange={(e) => setMessageText(e.target.value)}
                    placeholder={activeChat.is_group ? "Sending for groups is not enabled in this MVP" : "Type a message"}
                    disabled={activeChat.is_group}
                    className="flex-1 rounded-xl border border-transparent bg-white px-4 py-3 text-sm text-wa-text outline-none transition focus:border-wa-teal disabled:cursor-not-allowed disabled:bg-gray-100"
                  />
                  <button
                    data-testid="send-button"
                    type="submit"
                    disabled={!messageText.trim() || activeChat.is_group}
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
