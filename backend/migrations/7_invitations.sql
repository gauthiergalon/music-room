CREATE TABLE invitations (
    id                  UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    room_id             UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    inviter_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    invitee_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_pending          BOOLEAN NOT NULL DEFAULT true,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(room_id, invitee_id)
);
