import { useState, useRef, useEffect, useMemo } from "react";
import EmojiPicker, { Theme, type EmojiClickData } from "emoji-picker-react";
import { Icon } from "../components/Icons";
import { Modal } from "../components/Modal";
import { useToast } from "../components/Toast";
import { useWhatsApp } from "../hooks/useWhatsApp";
import { AVATAR_COLORS, type Ticket, type Message } from "../data";

const AGENTS = [
  { name: "Frans D.", initials: "FD", color: "#7c3aed" },
  { name: "Alice W.", initials: "AW", color: "#0ea5e9" },
  { name: "Bob K.", initials: "BK", color: "#10b981" },
];

const LS_TICKETS = "wa_tickets";
const LS_MESSAGES = "wa_messages";

function loadTickets(): Ticket[] {
  try {
    const raw = localStorage.getItem(LS_TICKETS);
    return raw ? (JSON.parse(raw) as Ticket[]) : [];
  } catch { return []; }
}

function loadMessages(): Record<string, Message[]> {
  try {
    const raw = localStorage.getItem(LS_MESSAGES);
    return raw ? (JSON.parse(raw) as Record<string, Message[]>) : {};
  } catch { return {}; }
}

function saveTickets(tickets: Ticket[]) {
  try { localStorage.setItem(LS_TICKETS, JSON.stringify(tickets)); } catch {}
}

function saveMessages(messages: Record<string, Message[]>) {
  try { localStorage.setItem(LS_MESSAGES, JSON.stringify(messages)); } catch {}
}

/** Turn a JID like "628123456789@s.whatsapp.net" or "LID@lid" into a display string */
function jidToPhone(jid: string): string {
  const user = jid.split("@")[0] ?? jid;
  if (jid.includes("@lid")) return user;
  return "+" + user;
}

/** Generate initials from a name or phone */
function makeInitials(name: string): string {
  const parts = name.trim().split(/\s+/);
  if (parts.length >= 2) return (parts[0][0] + parts[1][0]).toUpperCase();
  return name.slice(0, 2).toUpperCase();
}

export default function InboxPage() {
  const { showToast } = useToast();
  const { messages: waMessages, sendMessage: waSend } = useWhatsApp();

  const [activeTab, setActiveTab] = useState("all");
  const [ticketList, setTicketList] = useState<Ticket[]>(loadTickets);
  const [selectedTicket, setSelectedTicket] = useState<Ticket | null>(null);
  const [localMessages, setLocalMessages] = useState<Record<string, Message[]>>(loadMessages);
  const [composerText, setComposerText] = useState("");
  const [assignModal, setAssignModal] = useState(false);
  const [profileModal, setProfileModal] = useState(false);
  const [optionsOpen, setOptionsOpen] = useState(false);
  const [emojiOpen, setEmojiOpen] = useState(false);
  const [attachedFiles, setAttachedFiles] = useState<File[]>([]);

  const fileInputRef = useRef<HTMLInputElement>(null);
  const emojiRef = useRef<HTMLDivElement>(null);
  const processedWaMsgIds = useRef<Set<string>>(new Set());

  // ── Persist state to localStorage ────────────────────────────────────
  useEffect(() => { saveTickets(ticketList); }, [ticketList]);
  useEffect(() => { saveMessages(localMessages); }, [localMessages]);

  // Auto-select first ticket on load if we have persisted tickets
  useEffect(() => {
    if (!selectedTicket && ticketList.length > 0) {
      setSelectedTicket(ticketList[0]);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ── Process incoming WhatsApp messages into tickets ───────────────────

  // Seed processedIds with messages we already have in localStorage
  // so we don't re-process them after a refresh
  const hasSeededIds = useRef(false);
  useEffect(() => {
    if (hasSeededIds.current) return;
    hasSeededIds.current = true;
    // Mark all messages the hook already has as processed
    for (const waMsg of waMessages) {
      processedWaMsgIds.current.add(waMsg.id);
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    let ticketsChanged = false;
    let msgsChanged = false;

    for (const waMsg of waMessages) {
      if (processedWaMsgIds.current.has(waMsg.id)) continue;
      processedWaMsgIds.current.add(waMsg.id);

      // Skip messages we sent from this UI to avoid duplicates
      if (waMsg.isFromMe) continue;

      const chatJid = waMsg.chat;
      const phone = jidToPhone(chatJid);
      const displayName = waMsg.fromName || phone;
      const initials = makeInitials(displayName);

      const msg: Message = {
        id: Date.now() + Math.random(),
        sender: "customer",
        text: waMsg.text,
        time: new Date(waMsg.timestamp * 1000).toLocaleTimeString([], {
          hour: "2-digit",
          minute: "2-digit",
        }),
      };

      // Append message keyed by JID (the unique conversation identifier)
      setLocalMessages((prev) => {
        const updated = {
          ...prev,
          [chatJid]: [...(prev[chatJid] ?? []), msg],
        };
        return updated;
      });
      msgsChanged = true;

      setTicketList((prev) => {
        const existingIdx = prev.findIndex((t) => t.jid === chatJid);
        if (existingIdx >= 0) {
          const updated = [...prev];
          const ticket = { ...updated[existingIdx], preview: waMsg.text, time: "just now" };
          updated.splice(existingIdx, 1);
          return [ticket, ...updated];
        }

        const newTicket: Ticket = {
          id: Date.now() + Math.random(),
          name: displayName,
          initials,
          preview: waMsg.text,
          time: "just now",
          status: "open",
          color: AVATAR_COLORS[prev.length % AVATAR_COLORS.length],
          phone,
          jid: chatJid,
        };
        return [newTicket, ...prev];
      });
      ticketsChanged = true;
    }

    // Auto-select if nothing was selected and we just got new tickets
    if (ticketsChanged || msgsChanged) {
      setSelectedTicket((prev) => {
        if (prev) return prev;
        // Will be picked up by the state after ticketList updates
        return null;
      });
    }
  }, [waMessages]);

  // Auto-select first ticket when list grows from empty
  useEffect(() => {
    if (!selectedTicket && ticketList.length > 0) {
      setSelectedTicket(ticketList[0]);
    }
  }, [ticketList, selectedTicket]);

  /* Close emoji picker on outside click */
  useEffect(() => {
    if (!emojiOpen) return;
    const handler = (e: MouseEvent) => {
      if (emojiRef.current && !emojiRef.current.contains(e.target as Node)) {
        setEmojiOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [emojiOpen]);

  const filteredTickets = useMemo(() => {
    if (activeTab === "all") return ticketList;
    if (activeTab === "open") return ticketList.filter((t) => t.status === "open");
    return ticketList.filter((t) => t.status === "resolved");
  }, [activeTab, ticketList]);

  // Messages for the currently selected ticket — keyed by JID
  const messages = selectedTicket ? (localMessages[selectedTicket.jid] ?? []) : [];

  const handleSend = () => {
    const text = composerText.trim();
    if (!text && attachedFiles.length === 0) return;
    if (!selectedTicket) return;

    const parts: string[] = [];
    if (text) parts.push(text);
    if (attachedFiles.length > 0) {
      parts.push("📎 " + attachedFiles.map((f) => f.name).join(", "));
    }

    const msgText = parts.join("\n");

    const newMsg: Message = {
      id: Date.now(),
      sender: "agent",
      text: msgText,
      time: new Date().toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
      }),
    };
    setLocalMessages((prev) => ({
      ...prev,
      [selectedTicket.jid]: [...(prev[selectedTicket.jid] ?? []), newMsg],
    }));

    // Update ticket preview
    setTicketList((prev) =>
      prev.map((t) =>
        t.jid === selectedTicket.jid
          ? { ...t, preview: msgText, time: "just now" }
          : t
      )
    );

    // Send via WhatsApp backend using the original JID directly.
    // This ensures @lid JIDs are preserved as-is and @s.whatsapp.net ones
    // are also sent correctly without any phone-number stripping.
    if (text) {
      waSend(selectedTicket.jid, text);
    }

    setComposerText("");
    setAttachedFiles([]);
    showToast("Message sent");
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleEmojiClick = (emojiData: EmojiClickData) => {
    setComposerText((prev) => prev + emojiData.emoji);
    setEmojiOpen(false);
  };

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files) {
      setAttachedFiles((prev) => [...prev, ...Array.from(e.target.files!)]);
    }
    e.target.value = "";
  };

  const removeFile = (index: number) => {
    setAttachedFiles((prev) => prev.filter((_, i) => i !== index));
  };

  const removeTicket = (ticketId: number, action: string) => {
    setTicketList((prev) => {
      const next = prev.filter((t) => t.id !== ticketId);
      if (selectedTicket?.id === ticketId) {
        setSelectedTicket(next.length > 0 ? next[0] : null);
      }
      return next;
    });
    showToast(`Ticket ${action}`);
    setOptionsOpen(false);
  };

  return (
    <>
      {/* ── Ticket list ──────────────────── */}
      <section className="ticket-panel">
        <div className="ticket-header">
          <h2>Conversations</h2>
          <div className="ticket-search">
            <Icon.Search />
            <input type="text" placeholder="Search conversations…" />
          </div>
        </div>

        <div className="ticket-tabs">
          {(["all", "open", "resolved"] as const).map((tab) => (
            <button
              key={tab}
              className={`ticket-tab ${activeTab === tab ? "active" : ""}`}
              onClick={() => setActiveTab(tab)}
            >
              {tab.charAt(0).toUpperCase() + tab.slice(1)}
              <span className="count">
                {tab === "all"
                  ? ticketList.length
                  : ticketList.filter((t) => t.status === tab).length}
              </span>
            </button>
          ))}
        </div>

        <div className="ticket-list">
          {filteredTickets.length === 0 && (
            <div className="ticket-list-empty" data-testid="ticket-list-empty">
              <Icon.Inbox />
              <span>No conversations yet</span>
            </div>
          )}
          {filteredTickets.map((t) => (
            <div
              key={t.id}
              className={`ticket-item ${selectedTicket?.id === t.id ? "active" : ""}`}
              onClick={() => setSelectedTicket(t)}
            >
              <div className="ticket-avatar" style={{ background: t.color }}>
                {t.initials}
              </div>
              <div className="ticket-body">
                <div className="ticket-top">
                  <span className="ticket-name">{t.name}</span>
                  <span className="ticket-time">{t.time}</span>
                </div>
                <div className="ticket-preview">{t.preview}</div>
                <div className="ticket-meta">
                  <span className={`badge badge-${t.status}`}>{t.status}</span>
                  {t.priority && (
                    <span className="badge badge-priority">priority</span>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* ── Conversation ─────────────────── */}
      {selectedTicket ? (
        <section className="conversation">
          <div className="conv-header">
            <div className="conv-header-left">
              <div
                className="conv-header-avatar"
                style={{ background: selectedTicket.color, cursor: "pointer" }}
                onClick={() => setProfileModal(true)}
              >
                {selectedTicket.initials}
              </div>
              <div className="conv-header-info">
                <h3
                  style={{ cursor: "pointer" }}
                  onClick={() => setProfileModal(true)}
                >
                  {selectedTicket.name}
                </h3>
                <span>{selectedTicket.phone} · WhatsApp</span>
              </div>
            </div>
            <div className="conv-header-actions" style={{ position: "relative" }}>
              <button
                className="conv-action-btn"
                onClick={() => setAssignModal(true)}
              >
                <Icon.Tag /> Assign
              </button>
              <button
                className="conv-action-btn primary"
                onClick={() =>
                  showToast(`Ticket #${selectedTicket.id} marked as resolved`)
                }
              >
                <Icon.Check /> Resolve
              </button>
              <button
                className="conv-action-btn"
                onClick={() => setOptionsOpen((o) => !o)}
              >
                <Icon.MoreVert />
              </button>
              {optionsOpen && (
                <div className="options-dropdown">
                  <button
                    onClick={() => removeTicket(selectedTicket.id, "archived")}
                    data-testid="archive-btn"
                  >
                    <Icon.Archive /> Archive
                  </button>
                  <button
                    onClick={() => {
                      showToast("Ticket muted");
                      setOptionsOpen(false);
                    }}
                  >
                    Mute
                  </button>
                  <button
                    onClick={() => removeTicket(selectedTicket.id, "deleted")}
                    data-testid="delete-btn"
                    className="dropdown-danger"
                  >
                    <Icon.Trash /> Delete
                  </button>
                </div>
              )}
            </div>
          </div>

          <div className="conv-messages">
            {messages.length > 0 && <div className="msg-divider">Today</div>}
            {messages.length === 0 && (
              <div className="conv-empty" data-testid="conv-empty">
                <p>No messages yet — start the conversation!</p>
              </div>
            )}
            {messages.map((m) => (
              <div
                key={m.id}
                className={`msg ${m.sender === "customer" ? "incoming" : "outgoing"}`}
              >
                <div
                  className="msg-avatar"
                  style={{
                    background:
                      m.sender === "customer" ? selectedTicket.color : "#7c3aed",
                  }}
                >
                  {m.sender === "customer" ? selectedTicket.initials : "FD"}
                </div>
                <div className="msg-content">
                  <span className="msg-sender">
                    {m.sender === "customer" ? selectedTicket.name : "You"}
                  </span>
                  <div className="msg-bubble">{m.text}</div>
                  <span className="msg-time">{m.time}</span>
                </div>
              </div>
            ))}
          </div>

          <div className="composer">
            <div className="composer-toolbar">
              <button
                className="composer-tool"
                onClick={() => showToast("Bold formatting toggled")}
              >
                <Icon.Bold />
              </button>
              <button
                className="composer-tool"
                onClick={() => fileInputRef.current?.click()}
                data-testid="attach-btn"
              >
                <Icon.Paperclip />
              </button>
              <div className="emoji-picker-container" ref={emojiRef}>
                <button
                  className="composer-tool"
                  onClick={() => setEmojiOpen((o) => !o)}
                  data-testid="emoji-btn"
                >
                  <Icon.Smile />
                </button>
                {emojiOpen && (
                  <div className="emoji-picker-popover" data-testid="emoji-picker">
                    <EmojiPicker
                      onEmojiClick={handleEmojiClick}
                      theme={Theme.DARK}
                      width={320}
                      height={400}
                      searchDisabled={false}
                      skinTonesDisabled
                      previewConfig={{ showPreview: false }}
                    />
                  </div>
                )}
              </div>
              <input
                type="file"
                multiple
                ref={fileInputRef}
                style={{ display: "none" }}
                onChange={handleFileSelect}
                data-testid="file-input"
              />
            </div>

            {/* Attached files badges */}
            {attachedFiles.length > 0 && (
              <div className="attached-files" data-testid="attached-files">
                {attachedFiles.map((f, i) => (
                  <span key={i} className="file-badge">
                    <Icon.FileText />
                    {f.name}
                    <button
                      className="file-badge-remove"
                      onClick={() => removeFile(i)}
                      aria-label={`Remove ${f.name}`}
                    >
                      <Icon.Close />
                    </button>
                  </span>
                ))}
              </div>
            )}

            <div className="composer-input-wrap">
              <textarea
                className="composer-textarea"
                placeholder="Type your reply…"
                rows={1}
                value={composerText}
                onChange={(e) => setComposerText(e.target.value)}
                onKeyDown={handleKeyDown}
              />
              <button className="composer-send" onClick={handleSend}>
                <Icon.Send />
              </button>
            </div>
          </div>
        </section>
      ) : (
        <section className="conversation">
          <div className="inbox-empty-state" data-testid="inbox-empty">
            <div className="inbox-empty-icon">
              <Icon.Inbox />
            </div>
            <h3>No conversations yet</h3>
            <p>Incoming WhatsApp messages will appear here automatically once the backend is connected.</p>
          </div>
        </section>
      )}

      {/* ── Assign Modal ─────────────────── */}
      {selectedTicket && (
        <Modal
          title="Assign Ticket"
          open={assignModal}
          onClose={() => setAssignModal(false)}
        >
          <p>Select an agent to assign this ticket to:</p>
          <ul className="modal-agent-list">
            {AGENTS.map((a) => (
              <li key={a.initials}>
                <button
                  className="modal-agent-item"
                  onClick={() => {
                    showToast(`Ticket assigned to ${a.name}`);
                    setAssignModal(false);
                  }}
                >
                  <div
                    className="modal-agent-avatar"
                    style={{ background: a.color }}
                  >
                    {a.initials}
                  </div>
                  {a.name}
                </button>
              </li>
            ))}
          </ul>
        </Modal>
      )}

      {/* ── Profile Modal ────────────────── */}
      {selectedTicket && (
        <Modal
          title="Contact Profile"
          open={profileModal}
          onClose={() => setProfileModal(false)}
        >
          <div className="modal-profile-card">
            <div
              className="modal-profile-avatar"
              style={{ background: selectedTicket.color }}
            >
              {selectedTicket.initials}
            </div>
            <div className="modal-profile-name">{selectedTicket.name}</div>
            <div className="modal-profile-detail">
              {selectedTicket.phone} · WhatsApp
            </div>
            <div className="modal-profile-detail">
              Status: {selectedTicket.status}
            </div>
          </div>
        </Modal>
      )}
    </>
  );
}
