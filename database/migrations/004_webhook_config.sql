-- Add webhook configuration to whatsapp_sessions
-- Allows per-session webhook callbacks to external systems (e.g., Cakap/Chatwoot)
ALTER TABLE whatsapp_sessions
    ADD COLUMN IF NOT EXISTS webhook_url TEXT,
    ADD COLUMN IF NOT EXISTS webhook_token TEXT;

COMMENT ON COLUMN whatsapp_sessions.webhook_url IS 'URL to POST incoming message/status events to (e.g., Cakap webhook endpoint)';
COMMENT ON COLUMN whatsapp_sessions.webhook_token IS 'Bearer token or verify token sent with webhook requests';
