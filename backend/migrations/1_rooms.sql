CREATE TYPE room_visibility AS ENUM ('public', 'private');

CREATE TABLE rooms (
    id                  UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    owner_id            UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    visibility          room_visibility DEFAULT 'public',
    current_track       BIGINT,
    current_position    INT DEFAULT 0,
    is_playing          BOOLEAN DEFAULT false
);