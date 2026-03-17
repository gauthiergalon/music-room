CREATE TABLE users (
    id          UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    username    VARCHAR(24) UNIQUE NOT NULL,
    email       VARCHAR(255) UNIQUE NOT NULL,
    password    TEXT,
    google_id   VARCHAR(255) UNIQUE
);

ALTER TABLE users
ADD CONSTRAINT auth_method_required
CHECK (password IS NOT NULL OR google_id IS NOT NULL);