CREATE TABLE tracks (
    id          BIGINT PRIMARY KEY DEFAULT nextval('tracks_id_seq'),
    title       VARCHAR(255) NOT NULL,
    artist      VARCHAR(255) NOT NULL,
    album       VARCHAR(255),
    duration    INTEGER NOT NULL,
    cover 		VARCHAR(255),
);