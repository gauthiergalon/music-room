CREATE TABLE tracks (
    id          BIGINT PRIMARY KEY,
    title       VARCHAR(255) NOT NULL,
    artist      VARCHAR(255) NOT NULL,
    album       VARCHAR(255),
    duration    INTEGER NOT NULL,
    cover 		VARCHAR(255)
);