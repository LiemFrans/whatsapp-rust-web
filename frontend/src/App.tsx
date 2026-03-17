import { useState } from "react";
import "./App.css";

/* ── Icons (inline SVG) ────────────────────────────── */
const Icon = {
  Dashboard: () => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="3" width="7" height="7" rx="1" /><rect x="14" y="3" width="7" height="7" rx="1" />
      <rect x="3" y="14" width="7" height="7" rx="1" /><rect x="14" y="14" width="7" height="7" rx="1" />
    </svg>
  ),
  Inbox: () => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" />
      <polyline points="22,6 12,13 2,6" />
    </svg>
  ),
  Broadcast: () => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M5.636 18.364a9 9 0 010-12.728" /><path d="M18.364 5.636a9 9 0 010 12.728" />
      <path d="M8.464 15.536a5 5 0 010-7.072" /><path d="M15.536 8.464a5 5 0 010 7.072" />
      <circle cx="12" cy="12" r="1" />
    </svg>
  ),
  Settings: () => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </svg>
  ),
  Search: () => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
    </svg>
  ),
  Send: () => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <line x1="22" y1="2" x2="11" y2="13" /><polygon points="22 2 15 22 11 13 2 9 22 2" />
    </svg>
  ),
  Paperclip: () => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48" />
    </svg>
  ),
  Smile: () => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="10" /><path d="M8 14s1.5 2 4 2 4-2 4-2" /><line x1="9" y1="9" x2="9.01" y2="9" /><line x1="15" y1="9" x2="15.01" y2="9" />
    </svg>
  ),
  Bold: () => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M6 4h8a4 4 0 0 1 4 4 4 4 0 0 1-4 4H6z" /><path d="M6 12h9a4 4 0 0 1 4 4 4 4 0 0 1-4 4H6z" />
    </svg>
  ),
  Check: () => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="20 6 9 17 4 12" />
    </svg>
  ),
  MoreVert: () => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="5" r="1" /><circle cx="12" cy="12" r="1" /><circle cx="12" cy="19" r="1" />
    </svg>
  ),
  Tag: () => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z" /><line x1="7" y1="7" x2="7.01" y2="7" />
    </svg>
  ),
};

/* ── Mock data ─────────────────────────────────────── */
const AVATAR_COLORS = [
  "#6366f1", "#0ea5e9", "#f59e0b", "#ef4444", "#10b981",
  "#8b5cf6", "#ec4899", "#14b8a6", "#f97316", "#06b6d4",
];

interface Ticket {
  id: number;
  name: string;
  initials: string;
  preview: string;
  time: string;
  status: "open" | "resolved" | "pending";
  priority?: boolean;
  color: string;
  phone: string;
}

const tickets: Ticket[] = [
  { id: 1, name: "Emma Thompson", initials: "ET", preview: "I'm still experiencing issues with my account login…", time: "2m ago", status: "open", priority: true, color: AVATAR_COLORS[0], phone: "+1 555-0123" },
  { id: 2, name: "James Rodriguez", initials: "JR", preview: "Thank you for the quick response! The payment went…", time: "15m ago", status: "resolved", color: AVATAR_COLORS[1], phone: "+44 7700-900123" },
  { id: 3, name: "Sarah Chen", initials: "SC", preview: "Can I upgrade my plan to the Enterprise tier?", time: "1h ago", status: "open", color: AVATAR_COLORS[2], phone: "+61 400-123-456" },
  { id: 4, name: "Michael Osei", initials: "MO", preview: "The API documentation seems outdated for the v3 endpoint…", time: "2h ago", status: "pending", color: AVATAR_COLORS[3], phone: "+233 24-123-4567" },
  { id: 5, name: "Lisa Park", initials: "LP", preview: "I need to export all my data before the end of…", time: "3h ago", status: "open", color: AVATAR_COLORS[4], phone: "+82 10-1234-5678" },
  { id: 6, name: "David Müller", initials: "DM", preview: "Is there a way to integrate with our existing CRM?", time: "5h ago", status: "open", color: AVATAR_COLORS[5], phone: "+49 170-1234567" },
  { id: 7, name: "Aisha Patel", initials: "AP", preview: "The webhook notifications stopped working after…", time: "6h ago", status: "pending", priority: true, color: AVATAR_COLORS[6], phone: "+91 98765-43210" },
  { id: 8, name: "Carlos Vega", initials: "CV", preview: "Great product! Just wanted to share some feedback…", time: "1d ago", status: "resolved", color: AVATAR_COLORS[7], phone: "+34 612-345-678" },
];

interface Message {
  id: number;
  sender: "customer" | "agent";
  text: string;
  time: string;
}

const messagesByTicket: Record<number, Message[]> = {
  1: [
    { id: 1, sender: "customer", text: "Hi, I've been trying to log in for the past hour but keep getting an 'Invalid credentials' error. I'm sure my password is correct.", time: "10:23 AM" },
    { id: 2, sender: "agent", text: "Hello Emma! I'm sorry about the trouble. Let me look into your account right away. Could you confirm the email address you're using to sign in?", time: "10:25 AM" },
    { id: 3, sender: "customer", text: "Sure, it's emma.t@company.io. I also tried resetting my password but never received the email.", time: "10:26 AM" },
    { id: 4, sender: "agent", text: "I see the issue — it looks like your account has a temporary security hold due to multiple failed login attempts. I've lifted it now and sent a fresh password reset link. Please check your inbox (and spam folder).", time: "10:30 AM" },
    { id: 5, sender: "customer", text: "I'm still experiencing issues with my account login after resetting. The new password isn't working either.", time: "10:42 AM" },
  ],
  3: [
    { id: 1, sender: "customer", text: "Hi there! I'm currently on the Pro plan but we've grown quite a bit. Can I upgrade to Enterprise?", time: "9:15 AM" },
    { id: 2, sender: "agent", text: "Hi Sarah! Absolutely — congratulations on the growth! Enterprise includes dedicated support, custom integrations, and up to 500 seats. Would you like me to schedule a quick call with our sales team?", time: "9:18 AM" },
    { id: 3, sender: "customer", text: "Can I upgrade my plan to the Enterprise tier? That sounds great, yes please!", time: "9:20 AM" },
  ],
  4: [
    { id: 1, sender: "customer", text: "Hey, I noticed the v3 API docs still reference the old authentication flow. Has this been updated?", time: "8:30 AM" },
    { id: 2, sender: "agent", text: "Good catch, Michael. We deployed v3 last week and the docs are being updated. In the meantime, I can share the latest OpenAPI spec — would that help?", time: "8:45 AM" },
    { id: 3, sender: "customer", text: "The API documentation seems outdated for the v3 endpoint. The spec would be perfect, thanks!", time: "8:47 AM" },
  ],
};

// Default messages for tickets without specific conversation
const defaultMessages: Message[] = [
  { id: 1, sender: "customer", text: "Hi, I need some help with my account.", time: "10:00 AM" },
  { id: 2, sender: "agent", text: "Hello! I'd be happy to assist you. Could you provide more details about the issue?", time: "10:02 AM" },
];

/* ── Component ─────────────────────────────────────── */
function App() {
  const [activeNav, setActiveNav] = useState("inbox");
  const [activeTab, setActiveTab] = useState("all");
  const [selectedTicket, setSelectedTicket] = useState<Ticket>(tickets[0]);

  const filteredTickets =
    activeTab === "all"
      ? tickets
      : activeTab === "open"
        ? tickets.filter((t) => t.status === "open")
        : tickets.filter((t) => t.status === "resolved");

  const messages = messagesByTicket[selectedTicket.id] ?? defaultMessages;

  return (
    <div className="dashboard">
      {/* ── Sidebar ──────────────────────── */}
      <aside className="sidebar">
        <div className="sidebar-logo">HD</div>

        <nav className="sidebar-nav">
          {([
            ["dashboard", "Dashboard", <Icon.Dashboard />],
            ["inbox", "Inbox", <Icon.Inbox />],
            ["broadcasts", "Broadcasts", <Icon.Broadcast />],
            ["settings", "Settings", <Icon.Settings />],
          ] as [string, string, JSX.Element][]).map(([key, label, icon]) => (
            <button
              key={key}
              className={`sidebar-btn ${activeNav === key ? "active" : ""}`}
              onClick={() => setActiveNav(key)}
            >
              {icon}
              {label}
            </button>
          ))}
        </nav>

        <div className="sidebar-spacer" />
        <div className="sidebar-avatar">FD</div>
      </aside>

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
                  ? tickets.length
                  : tickets.filter((t) => t.status === tab).length}
              </span>
            </button>
          ))}
        </div>

        <div className="ticket-list">
          {filteredTickets.map((t) => (
            <div
              key={t.id}
              className={`ticket-item ${selectedTicket.id === t.id ? "active" : ""}`}
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
      <section className="conversation">
        <div className="conv-header">
          <div className="conv-header-left">
            <div
              className="conv-header-avatar"
              style={{ background: selectedTicket.color }}
            >
              {selectedTicket.initials}
            </div>
            <div className="conv-header-info">
              <h3>{selectedTicket.name}</h3>
              <span>{selectedTicket.phone} · WhatsApp</span>
            </div>
          </div>
          <div className="conv-header-actions">
            <button className="conv-action-btn">
              <Icon.Tag /> Assign
            </button>
            <button className="conv-action-btn primary">
              <Icon.Check /> Resolve
            </button>
            <button className="conv-action-btn">
              <Icon.MoreVert />
            </button>
          </div>
        </div>

        <div className="conv-messages">
          <div className="msg-divider">Today</div>
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
            <button className="composer-tool"><Icon.Bold /></button>
            <button className="composer-tool"><Icon.Paperclip /></button>
            <button className="composer-tool"><Icon.Smile /></button>
          </div>
          <div className="composer-input-wrap">
            <textarea
              className="composer-textarea"
              placeholder="Type your reply…"
              rows={1}
            />
            <button className="composer-send">
              <Icon.Send />
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}

export default App;
