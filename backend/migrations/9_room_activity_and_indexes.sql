-- Rooms are deleted when nobody has been connected to them for a while.
-- `last_active_at` is refreshed when the owner joins or leaves and at server start.
ALTER TABLE rooms ADD COLUMN IF NOT EXISTS last_active_at TIMESTAMPTZ NOT NULL DEFAULT now();

CREATE INDEX IF NOT EXISTS idx_friends_user_id_2 ON friends (user_id_2);
CREATE INDEX IF NOT EXISTS idx_invitations_invitee_id ON invitations (invitee_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_id ON refresh_tokens (user_id);
CREATE INDEX IF NOT EXISTS idx_reset_tokens_user_id ON reset_tokens (user_id);
CREATE INDEX IF NOT EXISTS idx_email_tokens_user_id ON email_tokens (user_id);
