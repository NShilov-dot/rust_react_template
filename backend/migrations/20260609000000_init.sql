CREATE TABLE IF NOT EXISTS users (
    id              UUID         PRIMARY KEY,
    email           TEXT         NOT NULL UNIQUE,
    name            TEXT         NOT NULL,
    password_hash   TEXT         NOT NULL,
    created_at      TIMESTAMPTZ  NOT NULL,
    updated_at      TIMESTAMPTZ  NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_users_created_at ON users (created_at DESC);
