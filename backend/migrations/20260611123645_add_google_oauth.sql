-- Google OAuth support.
--
-- Existing password users keep their NOT NULL hash (this only DROPs the
-- constraint, not the column). New OAuth-only users will have NULL hash —
-- the Login use case must refuse them with InvalidCredentials.
ALTER TABLE users
    ALTER COLUMN password_hash DROP NOT NULL;

ALTER TABLE users
    ADD COLUMN google_id TEXT;

-- One Google account → one user. NULLs are allowed and not compared for
-- uniqueness, so existing password-only users (NULL) don't collide.
CREATE UNIQUE INDEX idx_users_google_id ON users (google_id) WHERE google_id IS NOT NULL;
