ALTER TABLE users ADD COLUMN password_hash varchar(255);

CREATE UNIQUE INDEX users_email_unique_idx
ON users (lower(email))
WHERE email IS NOT NULL;
