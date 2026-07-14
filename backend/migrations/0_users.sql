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