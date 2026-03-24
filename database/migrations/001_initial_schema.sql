-- WhatsApp Web UI - Initial Database Schema
-- PostgreSQL Migration

-- ============================================================
-- ENUM TYPES
-- ============================================================

CREATE TYPE user_role AS ENUM ('admin', 'agent', 'user');
CREATE TYPE session_status AS ENUM ('disconnected', 'connecting', 'connected', 'qr_code');
CREATE TYPE message_type AS ENUM (
    'text', 'image', 'video', 'audio', 'document',
    'sticker', 'contact', 'location', 'poll', 'reaction',
    'system', 'revoked', 'unknown'
);
CREATE TYPE message_status AS ENUM ('pending', 'sent', 'delivered', 'read', 'failed');
CREATE TYPE ticket_priority AS ENUM ('low', 'medium', 'high', 'critical');
CREATE TYPE ticket_status AS ENUM ('open', 'in_progress', 'pending', 'resolved', 'closed');
CREATE TYPE assignment_status AS ENUM ('active', 'transferred', 'completed');
CREATE TYPE escalation_status AS ENUM ('pending', 'accepted', 'resolved', 'rejected');

-- ============================================================
-- USERS (Web Login)
-- ============================================================

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(100),
    role user_role NOT NULL DEFAULT 'user',
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_login_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_role ON users(role);

-- ============================================================
-- REFRESH TOKENS
-- ============================================================

CREATE TABLE refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_refresh_tokens_user ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_hash ON refresh_tokens(token_hash);

-- ============================================================
-- WHATSAPP SESSIONS
-- ============================================================

CREATE TABLE whatsapp_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_name VARCHAR(100) NOT NULL,
    phone_number VARCHAR(20),
    db_path VARCHAR(500) NOT NULL,
    status session_status NOT NULL DEFAULT 'disconnected',
    qr_code_data TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_active_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_wa_sessions_user ON whatsapp_sessions(user_id);
CREATE INDEX idx_wa_sessions_status ON whatsapp_sessions(status);

-- ============================================================
-- CHATS
-- ============================================================

CREATE TABLE chats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES whatsapp_sessions(id) ON DELETE CASCADE,
    chat_jid VARCHAR(100) NOT NULL,
    name VARCHAR(255),
    phone_number VARCHAR(20),
    profile_pic_url TEXT,
    last_message TEXT,
    last_message_at TIMESTAMPTZ,
    unread_count INTEGER NOT NULL DEFAULT 0,
    is_group BOOLEAN NOT NULL DEFAULT false,
    is_archived BOOLEAN NOT NULL DEFAULT false,
    is_pinned BOOLEAN NOT NULL DEFAULT false,
    is_muted BOOLEAN NOT NULL DEFAULT false,
    muted_until TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(session_id, chat_jid)
);

CREATE INDEX idx_chats_session ON chats(session_id);
CREATE INDEX idx_chats_jid ON chats(chat_jid);
CREATE INDEX idx_chats_last_message ON chats(last_message_at DESC);
CREATE INDEX idx_chats_unread ON chats(session_id, unread_count) WHERE unread_count > 0;
CREATE INDEX idx_chats_archived ON chats(session_id, is_archived);

-- ============================================================
-- MESSAGES
-- ============================================================

CREATE TABLE messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chat_id UUID NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    message_id VARCHAR(200) NOT NULL,
    sender VARCHAR(100) NOT NULL,
    sender_name VARCHAR(255),
    content TEXT,
    message_type message_type NOT NULL DEFAULT 'text',
    media_url TEXT,
    media_mime_type VARCHAR(100),
    media_size BIGINT,
    media_filename VARCHAR(500),
    thumbnail_base64 TEXT,
    status message_status NOT NULL DEFAULT 'sent',
    is_from_me BOOLEAN NOT NULL DEFAULT false,
    is_forwarded BOOLEAN NOT NULL DEFAULT false,
    is_starred BOOLEAN NOT NULL DEFAULT false,
    reply_to_message_id VARCHAR(200),
    quote_content TEXT,
    timestamp TIMESTAMPTZ NOT NULL,
    edited_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(chat_id, message_id)
);

CREATE INDEX idx_messages_chat ON messages(chat_id);
CREATE INDEX idx_messages_chat_ts ON messages(chat_id, timestamp DESC);
CREATE INDEX idx_messages_message_id ON messages(message_id);
CREATE INDEX idx_messages_sender ON messages(sender);
CREATE INDEX idx_messages_type ON messages(message_type);
CREATE INDEX idx_messages_starred ON messages(chat_id, is_starred) WHERE is_starred = true;

-- ============================================================
-- LABELS / TAGS
-- ============================================================

CREATE TABLE labels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(50) NOT NULL,
    color VARCHAR(7) NOT NULL DEFAULT '#25D366',
    description TEXT,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE chat_labels (
    chat_id UUID NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    label_id UUID NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    assigned_by UUID REFERENCES users(id),
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (chat_id, label_id)
);

-- ============================================================
-- TICKETS
-- ============================================================

CREATE TABLE tickets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chat_id UUID NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    priority ticket_priority NOT NULL DEFAULT 'medium',
    status ticket_status NOT NULL DEFAULT 'open',
    category VARCHAR(100),
    assigned_to UUID REFERENCES users(id),
    created_by UUID NOT NULL REFERENCES users(id),
    due_at TIMESTAMPTZ,
    resolved_at TIMESTAMPTZ,
    closed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tickets_chat ON tickets(chat_id);
CREATE INDEX idx_tickets_assigned ON tickets(assigned_to);
CREATE INDEX idx_tickets_status ON tickets(status);
CREATE INDEX idx_tickets_priority ON tickets(priority);
CREATE INDEX idx_tickets_created_by ON tickets(created_by);
CREATE INDEX idx_tickets_created ON tickets(created_at DESC);

-- ============================================================
-- TICKET NOTES (Internal)
-- ============================================================

CREATE TABLE ticket_notes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ticket_id UUID NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    note TEXT NOT NULL,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ticket_notes_ticket ON ticket_notes(ticket_id);

-- ============================================================
-- CHAT ASSIGNMENTS
-- ============================================================

CREATE TABLE chat_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chat_id UUID NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    assigned_to UUID NOT NULL REFERENCES users(id),
    assigned_by UUID NOT NULL REFERENCES users(id),
    status assignment_status NOT NULL DEFAULT 'active',
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_assignments_chat ON chat_assignments(chat_id);
CREATE INDEX idx_assignments_agent ON chat_assignments(assigned_to);
CREATE INDEX idx_assignments_status ON chat_assignments(status);
CREATE INDEX idx_assignments_active ON chat_assignments(assigned_to, status) WHERE status = 'active';

-- ============================================================
-- ESCALATIONS
-- ============================================================

CREATE TABLE escalations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ticket_id UUID NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    from_user_id UUID NOT NULL REFERENCES users(id),
    to_user_id UUID NOT NULL REFERENCES users(id),
    reason TEXT NOT NULL,
    status escalation_status NOT NULL DEFAULT 'pending',
    resolution_note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

CREATE INDEX idx_escalations_ticket ON escalations(ticket_id);
CREATE INDEX idx_escalations_from ON escalations(from_user_id);
CREATE INDEX idx_escalations_to ON escalations(to_user_id);
CREATE INDEX idx_escalations_status ON escalations(status);

-- ============================================================
-- QUICK REPLIES
-- ============================================================

CREATE TABLE quick_replies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(100) NOT NULL,
    shortcut VARCHAR(50),
    content TEXT NOT NULL,
    category VARCHAR(50),
    created_by UUID REFERENCES users(id),
    is_global BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_quick_replies_shortcut ON quick_replies(shortcut);
CREATE INDEX idx_quick_replies_category ON quick_replies(category);

-- ============================================================
-- AUDIT LOGS
-- ============================================================

CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    action VARCHAR(100) NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    entity_id UUID,
    details JSONB,
    ip_address VARCHAR(45),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_user ON audit_logs(user_id);
CREATE INDEX idx_audit_action ON audit_logs(action);
CREATE INDEX idx_audit_entity ON audit_logs(entity_type, entity_id);
CREATE INDEX idx_audit_created ON audit_logs(created_at DESC);

-- ============================================================
-- SEED: Default Admin User
-- Password: admin123 (argon2 hash)
-- ============================================================

-- Note: Run the backend with SEED_ADMIN=true to create the default admin user
-- via the application's password hashing, or insert manually after hashing.

-- ============================================================
-- UPDATED_AT TRIGGER
-- ============================================================

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_sessions_updated_at BEFORE UPDATE ON whatsapp_sessions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_chats_updated_at BEFORE UPDATE ON chats
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_tickets_updated_at BEFORE UPDATE ON tickets
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_quick_replies_updated_at BEFORE UPDATE ON quick_replies
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
