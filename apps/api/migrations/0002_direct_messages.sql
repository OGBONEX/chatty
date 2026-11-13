-- Persistent direct messages. Friend requests and accepted friendship state
-- are defined in 0001_identity_and_social.sql.
CREATE TABLE direct_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sender_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    recipient_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    content TEXT NOT NULL CHECK (char_length(trim(content)) BETWEEN 1 AND 4000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    edited_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ,
    CONSTRAINT direct_messages_not_self CHECK (sender_id <> recipient_id)
);
CREATE INDEX direct_messages_conversation_idx
    ON direct_messages (sender_id, recipient_id, created_at DESC);
CREATE INDEX direct_messages_recipient_idx
    ON direct_messages (recipient_id, created_at DESC);
