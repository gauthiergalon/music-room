CREATE TABLE queue (
    id          UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    room_id     UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    track_id    BIGINT NOT NULL,
    added_by    UUID NOT NULL REFERENCES users(id),
    position    FLOAT NOT NULL,

    UNIQUE(room_id, position)
);