CREATE TABLE rooms (
    id                  UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    owner_id            UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_public           BOOLEAN NOT NULL DEFAULT true,
    current_track       BIGINT,
    current_position    INT NOT NULL DEFAULT 0,
    played_at           TIMESTAMPTZ,
    is_playing          BOOLEAN NOT NULL DEFAULT false
);