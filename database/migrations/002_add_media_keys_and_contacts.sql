-- Migration 002: Add media encryption keys, quote sender info, and contacts table
-- Required for media download proxy and proper reply/forward rendering

-- ============================================================
-- Add media encryption columns to messages
-- These are needed to download + decrypt WhatsApp CDN media
-- ============================================================

ALTER TABLE messages ADD COLUMN IF NOT EXISTS media_key BYTEA;
ALTER TABLE messages ADD COLUMN IF NOT EXISTS direct_path TEXT;
ALTER TABLE messages ADD COLUMN IF NOT EXISTS file_enc_sha256 BYTEA;

-- ============================================================
-- Add quote sender info columns
-- Needed to render "replied to" bubbles with sender name
-- ============================================================

ALTER TABLE messages ADD COLUMN IF NOT EXISTS quote_sender VARCHAR(100);
ALTER TABLE messages ADD COLUMN IF NOT EXISTS quote_sender_name VARCHAR(255);

-- ============================================================
-- Contacts table for push name resolution
-- Maps JIDs to their WhatsApp display names
-- ============================================================

CREATE TABLE IF NOT EXISTS contacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES whatsapp_sessions(id) ON DELETE CASCADE,
    jid VARCHAR(100) NOT NULL,
    push_name VARCHAR(255),
    phone_number VARCHAR(20),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(session_id, jid)
);

CREATE INDEX IF NOT EXISTS idx_contacts_session ON contacts(session_id);
CREATE INDEX IF NOT EXISTS idx_contacts_jid ON contacts(jid);
