-- Sign-in (M1-D3, DEC-07): email links that confirm the address, passwords, passkeys, sessions
-- with rotating refresh tokens, the admin role, and a log of admin writes.
--
-- How an account comes to be: the visitor asks for an email link; the link both confirms the
-- address and signs them in, so users.email only ever holds a confirmed address. A password and
-- passkeys are added once signed in. A guest (no email) keeps its row: confirming an address from
-- the guest's browser puts the email on it, and signing in to an existing account from it moves
-- the guest's captures, products and designs over.
--
-- Every secret is stored as its SHA-256 (refresh tokens, link tokens) or as an Argon2id hash
-- (passwords); the raw values exist only in the user's cookie, inbox or memory.

ALTER TABLE users
    ADD COLUMN role               text NOT NULL DEFAULT 'user',
    ADD COLUMN email_confirmed_at timestamptz,
    -- Argon2id, PHC string format ($argon2id$v=19$m=…).
    ADD COLUMN password_hash      text;

-- An email written before sign-in existed counts as confirmed (there are none in production).
UPDATE users SET email_confirmed_at = created_at WHERE email IS NOT NULL;

ALTER TABLE users
    ADD CONSTRAINT users_role_known CHECK (role IN ('user', 'admin')),
    -- users.email is set only by a confirmed link, so the two go together.
    ADD CONSTRAINT users_email_confirmed CHECK ((email IS NULL) = (email_confirmed_at IS NULL)),
    ADD CONSTRAINT users_password_needs_email CHECK (password_hash IS NULL OR email IS NOT NULL),
    ADD CONSTRAINT users_admin_has_email CHECK (role <> 'admin' OR email IS NOT NULL);

-- A signed-in browser: one refresh token at a time, replaced on every refresh. The token before
-- it is kept to tell a harmless race (two tabs refreshing at once) from a stolen token.
CREATE TABLE sessions (
    id                          uuid        PRIMARY KEY,
    user_id                     uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- Which API surface it signs in to: client (the web app) or admin (the back office).
    surface                     text        NOT NULL,
    -- How the user proved who they are.
    method                      text        NOT NULL,
    refresh_token_hash          bytea       NOT NULL UNIQUE,
    previous_refresh_token_hash bytea       UNIQUE,
    rotated_at                  timestamptz,
    authenticated_at            timestamptz NOT NULL DEFAULT now(),
    expires_at                  timestamptz NOT NULL,
    revoked_at                  timestamptz,
    created_at                  timestamptz NOT NULL DEFAULT now(),
    updated_at                  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT sessions_surface_known CHECK (surface IN ('client', 'admin')),
    CONSTRAINT sessions_method_known CHECK (method IN ('guest', 'email_link', 'password', 'passkey')),
    -- The back office signs in with a passkey only.
    CONSTRAINT sessions_admin_by_passkey CHECK (surface <> 'admin' OR method = 'passkey'),
    CONSTRAINT sessions_hash_size CHECK (
        length(refresh_token_hash) = 32
        AND (previous_refresh_token_hash IS NULL OR length(previous_refresh_token_hash) = 32)
    ),
    CONSTRAINT sessions_rotation CHECK ((previous_refresh_token_hash IS NULL) = (rotated_at IS NULL))
);

CREATE INDEX sessions_user_id ON sessions (user_id);
CREATE INDEX sessions_expires_at ON sessions (expires_at);

CREATE TRIGGER sessions_set_updated_at BEFORE UPDATE ON sessions
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Email links: one confirms an address and creates its account (sign_up), or signs in to the
-- account that has it (sign_in). The row is made when asked for; the send_email_link job makes
-- the token, stores its hash and sends it, so the raw token is never stored, not even in a job.
CREATE TABLE email_links (
    id         uuid        PRIMARY KEY,
    email      text        NOT NULL,
    purpose    text        NOT NULL,
    -- The language of the email (the asker's Accept-Language).
    locale     text        NOT NULL DEFAULT 'en',
    -- Both set when the email is sent; a resend replaces them.
    token_hash bytea       UNIQUE,
    expires_at timestamptz,
    used_at    timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT email_links_purpose_known CHECK (purpose IN ('sign_up', 'sign_in')),
    CONSTRAINT email_links_email_shape CHECK (
        email = btrim(email) AND email LIKE '_%@_%' AND length(email) <= 254
    ),
    CONSTRAINT email_links_locale_shape CHECK (locale ~ '^[a-z]{2}(-[A-Z]{2})?$'),
    CONSTRAINT email_links_sent CHECK ((token_hash IS NULL) = (expires_at IS NULL)),
    CONSTRAINT email_links_hash_size CHECK (token_hash IS NULL OR length(token_hash) = 32)
);

-- Links asked for an address lately (the per-address limit), and the ones to retire once one is used.
CREATE INDEX email_links_email ON email_links (lower(email), created_at);
CREATE INDEX email_links_created_at ON email_links (created_at);

-- Passkeys (WebAuthn credentials). The user handle given to the authenticator is the user's id.
CREATE TABLE passkeys (
    id              uuid        PRIMARY KEY,
    user_id         uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    credential_id   bytea       NOT NULL UNIQUE,
    -- The COSE_Key the authenticator made, as it sent it.
    public_key      bytea       NOT NULL,
    -- COSE algorithm: -7 ES256, -8 EdDSA, -257 RS256.
    algorithm       integer     NOT NULL,
    sign_count      bigint      NOT NULL DEFAULT 0,
    -- What the browser said it can reach the authenticator over (internal, hybrid, usb…).
    transports      text[]      NOT NULL DEFAULT '{}',
    -- Whether the passkey can be synced (BE flag, fixed) and is synced now (BS flag).
    backup_eligible boolean     NOT NULL,
    backed_up       boolean     NOT NULL,
    -- The authenticator model, when it says (all zeros is stored as NULL).
    aaguid          uuid,
    name            text        NOT NULL DEFAULT '',
    last_used_at    timestamptz,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT passkeys_algorithm_known CHECK (algorithm IN (-7, -8, -257)),
    CONSTRAINT passkeys_credential_id_size CHECK (length(credential_id) BETWEEN 16 AND 1023),
    CONSTRAINT passkeys_sign_count_range CHECK (sign_count BETWEEN 0 AND 4294967295),
    CONSTRAINT passkeys_backup_flags CHECK (backup_eligible OR NOT backed_up),
    CONSTRAINT passkeys_name_size CHECK (length(name) <= 100)
);

CREATE INDEX passkeys_user_id ON passkeys (user_id);

CREATE TRIGGER passkeys_set_updated_at BEFORE UPDATE ON passkeys
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- A passkey ceremony's challenge, used once: sign_in (any passkey) or register (a new passkey
-- for user_id).
CREATE TABLE passkey_challenges (
    id         uuid        PRIMARY KEY,
    purpose    text        NOT NULL,
    surface    text        NOT NULL,
    user_id    uuid        REFERENCES users (id) ON DELETE CASCADE,
    challenge  bytea       NOT NULL,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT passkey_challenges_purpose_known CHECK (purpose IN ('sign_in', 'register')),
    CONSTRAINT passkey_challenges_surface_known CHECK (surface IN ('client', 'admin')),
    CONSTRAINT passkey_challenges_register_has_user CHECK ((purpose = 'register') = (user_id IS NOT NULL)),
    CONSTRAINT passkey_challenges_size CHECK (length(challenge) = 32)
);

CREATE INDEX passkey_challenges_expires_at ON passkey_challenges (expires_at);

-- Every write an admin makes on the back office (who, what, when), recorded before it runs and
-- given its status after. Rows outlive the admin's account and are never changed otherwise.
CREATE TABLE admin_audit_log (
    id          uuid        PRIMARY KEY,
    admin_id    uuid        REFERENCES users (id) ON DELETE SET NULL,
    -- Who it was, even after their account is gone.
    admin_email text        NOT NULL,
    session_id  uuid,
    request_id  text        NOT NULL,
    method      text        NOT NULL,
    -- The route as written in the router (…/users/{user_id}), and the path as called.
    route       text,
    path        text        NOT NULL,
    status      smallint,
    created_at  timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz,
    CONSTRAINT admin_audit_log_finished CHECK ((status IS NULL) = (finished_at IS NULL))
);

CREATE INDEX admin_audit_log_created_at ON admin_audit_log (created_at);
CREATE INDEX admin_audit_log_admin_id ON admin_audit_log (admin_id, created_at);

-- The log only grows: a row's status is written once, the admin link may be cleared when the
-- account goes, and nothing else changes or is deleted.
CREATE FUNCTION guard_admin_audit_log() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'admin_audit_log rows are never deleted';
    END IF;
    IF (NEW.id, NEW.admin_email, NEW.session_id, NEW.request_id, NEW.method, NEW.route, NEW.path,
        NEW.created_at)
       IS DISTINCT FROM
       (OLD.id, OLD.admin_email, OLD.session_id, OLD.request_id, OLD.method, OLD.route, OLD.path,
        OLD.created_at)
       OR (NEW.admin_id IS DISTINCT FROM OLD.admin_id AND NEW.admin_id IS NOT NULL)
       OR (OLD.status IS NOT NULL AND (NEW.status, NEW.finished_at)
           IS DISTINCT FROM (OLD.status, OLD.finished_at)) THEN
        RAISE EXCEPTION 'admin_audit_log rows are never changed, only finished once';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER admin_audit_log_append_only BEFORE UPDATE OR DELETE ON admin_audit_log
    FOR EACH ROW EXECUTE FUNCTION guard_admin_audit_log();
