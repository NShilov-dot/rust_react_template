-- Per-user todo tasks (Jira-for-MVP).
--
-- `owner_id` is the creator. We enforce ownership at the SQL level via
-- `WHERE owner_id = $1` in every read/mutate path, so IDOR is impossible
-- even if a handler forgets the check.
--
-- `status` and `priority` are plain TEXT with CHECK constraints so future
-- values can be added with a single ALTER + redeploy (a Postgres enum
-- would force ALTER TYPE ... ADD VALUE in a separate transaction).
CREATE TABLE IF NOT EXISTS tasks (
    id           UUID         PRIMARY KEY,
    owner_id     UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title        TEXT         NOT NULL,
    description  TEXT,
    status       TEXT         NOT NULL CHECK (status IN ('todo', 'in_progress', 'done')),
    priority     TEXT         NOT NULL CHECK (priority IN ('low', 'medium', 'high')),
    due_date     TIMESTAMPTZ,
    created_at   TIMESTAMPTZ  NOT NULL,
    updated_at   TIMESTAMPTZ  NOT NULL
);

-- Covers the dominant query: "list this user's tasks, filterable by status,
-- newest first". Postgres can use the prefix (owner_id) alone for the unfiltered
-- list too.
CREATE INDEX IF NOT EXISTS idx_tasks_owner_status_created
    ON tasks (owner_id, status, created_at DESC);
