CREATE TABLE rooms (
    id                  UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    owner_id            UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name                VARCHAR(255) NOT NULL,
    is_public           BOOLEAN NOT NULL DEFAULT true,
    is_licensed			BOOLEAN NOT NULL DEFAULT false,
    current_track       BIGINT,
    current_position    INT NOT NULL DEFAULT 0,
    played_at           TIMESTAMPTZ,
    is_playing          BOOLEAN NOT NULL DEFAULT false
);