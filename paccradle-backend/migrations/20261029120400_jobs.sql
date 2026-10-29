-- Background jobs, and the outbox for side effects (backend-architecture.md §1, §4.9). A
-- service adds a job in the same transaction as its own writes, so the job exists exactly
-- when the writes do. Workers claim due jobs with FOR UPDATE SKIP LOCKED.
--
--   queued ──claim──▶ running ──done──▶ done (deleted after 7 days)
--     ▲                  │
--     └──retry (backoff)─┤
--                        └──out of attempts, or given up──▶ dead (kept for the back office)
--
-- A claim leases the job until locked_until. A worker that dies mid-job leaves it running
-- with an old lease; housekeeping puts it back in the queue (or buries it when it is out of
-- attempts). Handlers must therefore be safe to run twice.
CREATE TABLE jobs (
    id           uuid        PRIMARY KEY,
    -- The handler's name, e.g. run_vision. A worker only claims kinds it knows, so a job
    -- added by a newer release waits through a rollback instead of failing.
    kind         text        NOT NULL,
    payload      jsonb       NOT NULL DEFAULT '{}',
    status       text        NOT NULL DEFAULT 'queued',
    -- Claims so far, counting the running one.
    attempts     integer     NOT NULL DEFAULT 0,
    max_attempts integer     NOT NULL DEFAULT 5,
    run_at       timestamptz NOT NULL DEFAULT now(),
    locked_until timestamptz,
    last_error   text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    finished_at  timestamptz,
    CONSTRAINT jobs_kind_shape CHECK (kind ~ '^[a-z][a-z0-9_]{0,63}$'),
    CONSTRAINT jobs_status_known CHECK (status IN ('queued', 'running', 'done', 'dead')),
    CONSTRAINT jobs_attempts_range CHECK (
        max_attempts BETWEEN 1 AND 100 AND attempts BETWEEN 0 AND max_attempts
    ),
    CONSTRAINT jobs_leased_while_running CHECK ((status = 'running') = (locked_until IS NOT NULL)),
    CONSTRAINT jobs_finished_when_over CHECK ((status IN ('done', 'dead')) = (finished_at IS NOT NULL))
);

-- Due jobs, oldest first.
CREATE INDEX jobs_due ON jobs (run_at) WHERE status = 'queued';
-- Leases that ran out.
CREATE INDEX jobs_lease ON jobs (locked_until) WHERE status = 'running';
-- Done jobs to delete.
CREATE INDEX jobs_done ON jobs (finished_at) WHERE status = 'done';

CREATE TRIGGER jobs_set_updated_at BEFORE UPDATE ON jobs
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
