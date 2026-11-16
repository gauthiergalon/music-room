CREATE TYPE privacy_level AS ENUM ('public', 'friends', 'private');

CREATE TABLE users (
    id              UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    username        VARCHAR(24) UNIQUE NOT NULL,
    email           VARCHAR(255) UNIQUE NOT NULL,
    email_confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    password_hash   TEXT,
    google_id       VARCHAR(255) UNIQUE,
    favorite_genres TEXT[] NOT NULL DEFAULT '{}',
    privacy_level   privacy_level NOT NULL DEFAULT 'friends',
	is_subscribed	BOOLEAN NOT NULL DEFAULT FALSE,
	end_subscription_date TIMESTAMPTZ DEFAULT NULL
);

ALTER TABLE users
ADD CONSTRAINT auth_method_required
CHECK (password_hash IS NOT NULL OR google_id IS NOT NULL);
CREATE TABLE rooms (
    id                  UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    owner_id            UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    name                VARCHAR(255) NOT NULL,
    is_public           BOOLEAN NOT NULL DEFAULT true,
    is_licensed			BOOLEAN NOT NULL DEFAULT false,
    current_track       BIGINT,
    current_position    INT NOT NULL DEFAULT 0,
    played_at           TIMESTAMPTZ,
    is_playing          BOOLEAN NOT NULL DEFAULT false
);
CREATE TABLE queue (
    id          UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    room_id     UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    track_id    BIGINT NOT NULL,
    position    FLOAT NOT NULL,

    UNIQUE(room_id, position)
);
CREATE TABLE refresh_tokens (
    token_hash  VARCHAR(64) PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ NOT NULL
);
CREATE TABLE reset_tokens (
    token_hash  VARCHAR(64) PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ NOT NULL
);
CREATE TABLE email_tokens (
    token_hash  VARCHAR(64) PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    new_email   VARCHAR(255) NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ NOT NULL
);
CREATE TABLE friends (
    user_id_1 UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_id_2 UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sender_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_pending BOOLEAN NOT NULL DEFAULT TRUE,
    PRIMARY KEY (user_id_1, user_id_2),
    CHECK (user_id_1 < user_id_2)
);

CREATE TABLE invitations (
    id			UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    room_id		UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    inviter_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    invitee_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_pending	BOOLEAN NOT NULL DEFAULT true,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(room_id, invitee_id, is_pending)
);

CREATE TABLE tracks (
    id          BIGINT PRIMARY KEY,
    title       VARCHAR(255) NOT NULL,
    artist      VARCHAR(255) NOT NULL,
    album       VARCHAR(255),
    duration    INTEGER NOT NULL,
    cover 		VARCHAR(255)
);
