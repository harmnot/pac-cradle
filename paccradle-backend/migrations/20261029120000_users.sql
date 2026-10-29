-- Conventions for every table (backend-architecture.md §5):
--   * ids are UUID v7, made by the API (shared::id::new_id), so they sort by creation time;
--   * times are timestamptz (UTC); updated_at is kept current by set_updated_at();
--   * enum-like text columns hold lower-case snake_case values, checked by a constraint;
--   * lengths are millimetres (double precision); money, when stored, is integer minor units;
--   * rows belong to a user and go when the user is deleted (ON DELETE CASCADE).
-- Migrations only add, so the previous release still runs if a deploy rolls back.

-- Keeps updated_at current on every UPDATE.
CREATE FUNCTION set_updated_at() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at := now();
    RETURN NEW;
END;
$$;

-- People using the app (CLAUDE.md §8.4). A row without an email is a guest, so a visitor can
-- capture and design before signing in; signing in (M1-D3) puts the email on the same row.
CREATE TABLE users (
    id            uuid        PRIMARY KEY,
    email         text,
    locale        text        NOT NULL DEFAULT 'en',
    paper_default text        NOT NULL DEFAULT 'a4',
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT users_email_shape CHECK (
        email IS NULL OR (email = btrim(email) AND email LIKE '_%@_%' AND length(email) <= 254)
    ),
    CONSTRAINT users_locale_shape CHECK (locale ~ '^[a-z]{2}(-[A-Z]{2})?$'),
    CONSTRAINT users_paper_default_known CHECK (paper_default IN ('a4', 'letter'))
);

-- One account per address, whatever its letter case.
CREATE UNIQUE INDEX users_email_unique ON users (lower(email)) WHERE email IS NOT NULL;

CREATE TRIGGER users_set_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
