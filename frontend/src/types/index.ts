// ── User & Auth ───────────────────────────────────────────────

export type UserRole = 'admin' | 'agent' | 'user';
export type AppMode = 'personal' | 'business';

export interface User {
  id: string;
  username: string;
  email: string;
  display_name: string | null;
  role: UserRole;
  is_active: boolean;
  last_login_at: string | null;
  created_at: string;
}

export interface AuthResponse {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
  user: User;
}

// ── WhatsApp Session ──────────────────────────────────────────

export type SessionStatus = 'disconnected' | 'connecting' | 'connected' | 'qr_code';

export interface WhatsAppSession {
  id: string;
  session_name: string;
  phone_number: string | null;
  status: SessionStatus;
  created_at: string;
  last_active_at: string | null;
}

// ── Chat ──────────────────────────────────────────────────────

export interface Contact {
  id: string;
  session_id: string;
  jid: string;
  push_name: string | null;
  phone_number: string | null;
  updated_at: string;
}

export interface Chat {
  id: string;
  session_id: string;
  chat_jid: string;
  name: string | null;
  phone_number: string | null;
  profile_pic_url: string | null;
  last_message: string | null;
  last_message_at: string | null;
  unread_count: number;
  is_group: boolean;
  is_archived: boolean;
  is_pinned: boolean;
  is_muted: boolean;
  last_message_type?: string | null;
  last_message_status?: string | null;
  is_last_message_from_me?: boolean;
  // Business mode
  assigned_to?: string;
  assigned_agent_name?: string;
  ticket_id?: string;
  labels?: Label[];
}

export interface Label {
  id: string;
  name: string;
  color: string;
}

// ── Message ───────────────────────────────────────────────────

export type MessageType =
  | 'text' | 'image' | 'video' | 'audio' | 'voice' | 'document'
  | 'sticker' | 'contact' | 'location' | 'poll' | 'reaction'
  | 'system' | 'revoked' | 'unknown';

export type MessageStatus = 'pending' | 'sent' | 'delivered' | 'read' | 'failed';

export interface Message {
  id: string;
  message_id: string;
  sender: string;
  sender_name: string | null;
  content: string | null;
  message_type: MessageType;
  media_url: string | null;
  media_mime_type: string | null;
  media_filename: string | null;
  thumbnail_base64: string | null;
  status: MessageStatus;
  is_from_me: boolean;
  is_forwarded: boolean;
  is_starred: boolean;
  reply_to_message_id: string | null;
  reply_to: string | null;
  quote_content: string | null;
  quote_sender: string | null;
  quote_sender_name: string | null;
  is_deleted: boolean;
  timestamp: string;
  edited_at: string | null;
}

// ── Ticket ────────────────────────────────────────────────────

export type TicketPriority = 'low' | 'medium' | 'high' | 'critical';
export type TicketStatus = 'open' | 'in_progress' | 'pending' | 'resolved' | 'closed';

export interface Ticket {
  id: string;
  chat_id: string;
  title: string;
  description: string | null;
  priority: TicketPriority;
  status: TicketStatus;
  category: string | null;
  assigned_to: string | null;
  assigned_to_name: string | null;
  created_by: string;
  created_by_name: string | null;
  due_at: string | null;
  resolved_at: string | null;
  notes_count: number;
  created_at: string;
  updated_at: string;
}

export interface TicketNote {
  id: string;
  ticket_id: string;
  note: string;
  created_by: string;
  created_by_name: string | null;
  created_at: string;
}

// ── Assignment ────────────────────────────────────────────────

export type AssignmentStatus = 'active' | 'transferred' | 'completed';

export interface ChatAssignment {
  id: string;
  chat_id: string;
  assigned_to: string;
  assigned_to_name: string | null;
  assigned_by: string;
  status: AssignmentStatus;
  assigned_at: string;
}

// ── Escalation ────────────────────────────────────────────────

export type EscalationStatus = 'pending' | 'accepted' | 'resolved' | 'rejected';

export interface Escalation {
  id: string;
  ticket_id: string;
  from_user_id: string;
  from_user_name: string | null;
  to_user_id: string;
  to_user_name: string | null;
  reason: string;
  status: EscalationStatus;
  resolution_note: string | null;
  created_at: string;
  resolved_at: string | null;
}

// ── Quick Reply ───────────────────────────────────────────────

export interface QuickReply {
  id: string;
  title: string;
  shortcut: string | null;
  content: string;
  category: string | null;
  is_global: boolean;
}

// ── Analytics ─────────────────────────────────────────────────

export interface AgentAnalytics {
  agent_id: string;
  agent_name: string;
  active_chats: number;
}

export interface Analytics {
  agents: AgentAnalytics[];
  tickets: Record<string, number>;
  messages_today: number;
  unassigned_chats: number;
}

// ── WebSocket Events ──────────────────────────────────────────

export interface WsEvent {
  type: string;
  data: Record<string, unknown>;
}

// ── Pagination ────────────────────────────────────────────────

export interface Pagination {
  page: number;
  per_page: number;
  total: number;
  total_pages: number;
}
