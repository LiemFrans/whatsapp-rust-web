/* ── Shared types ───────────────────────────────────── */

export const AVATAR_COLORS = [
  "#6366f1", "#0ea5e9", "#f59e0b", "#ef4444", "#10b981",
  "#8b5cf6", "#ec4899", "#14b8a6", "#f97316", "#06b6d4",
];

export interface Ticket {
  id: number;
  name: string;
  initials: string;
  preview: string;
  time: string;
  status: "open" | "resolved" | "pending";
  priority?: boolean;
  color: string;
  phone: string;
  /** Original chat JID from WhatsApp (e.g. "628xxx@s.whatsapp.net" or "id@lid") */
  jid: string;
}

export interface Message {
  id: number;
  sender: "customer" | "agent";
  text: string;
  time: string;
}
