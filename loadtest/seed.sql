-- Test accounts lt_0 ... lt_19999. Their ids are derived from their number
-- (md5 of 'lt-user-<n>') so k6 can sign their access tokens without logging
-- in: password hashing is not part of what these scenarios measure.
INSERT INTO users (id, username, email, email_confirmed, password_hash)
SELECT md5('lt-user-' || n)::uuid, 'lt_' || n, 'lt' || n || '@load.test', true, 'not-a-password'
FROM generate_series(0, 19999) AS n
ON CONFLICT DO NOTHING;
