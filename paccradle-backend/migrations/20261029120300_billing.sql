-- Entitlements and Paddle's events (CLAUDE.md §9.3, §9.4, §8.4; M1-D7 and F write them).
-- Paddle is the merchant of record and keeps the money records; these tables only say what
-- a user may download.

-- A design that may be exported: bought, paid with a credit, or exported on Pro. Re-download
-- is forever; edits stay open until edits_until (30 days, §9.4).
CREATE TABLE design_unlocks (
    id                    uuid        PRIMARY KEY,
    design_id             uuid        NOT NULL UNIQUE REFERENCES designs (id) ON DELETE CASCADE,
    user_id               uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    source                text        NOT NULL,
    -- The checkout that paid for it, when source = 'purchase' (to undo it on a refund).
    paddle_transaction_id text,
    unlocked_at           timestamptz NOT NULL DEFAULT now(),
    edits_until           timestamptz NOT NULL,
    updated_at            timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT design_unlocks_source_known CHECK (source IN ('purchase', 'credit', 'pro')),
    CONSTRAINT design_unlocks_purchase_has_transaction CHECK (
        (source = 'purchase') = (paddle_transaction_id IS NOT NULL)
    )
);

CREATE INDEX design_unlocks_user_id ON design_unlocks (user_id);

CREATE TRIGGER design_unlocks_set_updated_at BEFORE UPDATE ON design_unlocks
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Export credits. The balance is SUM(delta); rows are never changed, only added (a mistake is
-- undone by an 'adjustment' row). Keeping it from going below zero is the export service's
-- job (M1-D7), with the user's row locked.
CREATE TABLE credit_ledger (
    id                    uuid        PRIMARY KEY,
    user_id               uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    delta                 integer     NOT NULL,
    reason                text        NOT NULL,
    -- The design a credit was spent on.
    design_id             uuid        REFERENCES designs (id) ON DELETE SET NULL,
    paddle_transaction_id text,
    -- Why an adjustment was made, and by whom (back office).
    note                  text,
    created_at            timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT credit_ledger_delta_nonzero CHECK (delta <> 0),
    CONSTRAINT credit_ledger_reason_known CHECK (
        reason IN ('purchase', 'spend', 'refund', 'chargeback', 'adjustment')
    ),
    CONSTRAINT credit_ledger_delta_sign CHECK (
        CASE reason
            WHEN 'purchase' THEN delta > 0
            WHEN 'adjustment' THEN true
            ELSE delta < 0
        END
    ),
    CONSTRAINT credit_ledger_paddle_has_transaction CHECK (
        reason NOT IN ('purchase', 'refund', 'chargeback') OR paddle_transaction_id IS NOT NULL
    ),
    CONSTRAINT credit_ledger_adjustment_has_note CHECK (reason <> 'adjustment' OR note IS NOT NULL)
);

CREATE INDEX credit_ledger_user_id ON credit_ledger (user_id, created_at);
-- A pack's credits are added once, however often Paddle sends its webhook.
CREATE UNIQUE INDEX credit_ledger_purchase_once ON credit_ledger (paddle_transaction_id)
    WHERE reason = 'purchase';
CREATE INDEX credit_ledger_design_id ON credit_ledger (design_id);

-- The only change allowed is the one deleting a design makes (ON DELETE SET NULL).
CREATE FUNCTION refuse_ledger_update() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.design_id IS NULL
        AND (NEW.id, NEW.user_id, NEW.delta, NEW.reason, NEW.paddle_transaction_id, NEW.note, NEW.created_at)
            IS NOT DISTINCT FROM
            (OLD.id, OLD.user_id, OLD.delta, OLD.reason, OLD.paddle_transaction_id, OLD.note, OLD.created_at)
    THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'credit_ledger rows are never changed; add an adjustment row instead';
END;
$$;

CREATE TRIGGER credit_ledger_append_only BEFORE UPDATE ON credit_ledger
    FOR EACH ROW EXECUTE FUNCTION refuse_ledger_update();

-- Pro (and Studio, v2) subscriptions, as Paddle last told us.
CREATE TABLE subscriptions (
    id                     uuid        PRIMARY KEY,
    user_id                uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    paddle_subscription_id text        NOT NULL UNIQUE,
    plan                   text        NOT NULL,
    status                 text        NOT NULL,
    current_period_end     timestamptz,
    -- Paddle's own updated_at from the last event applied: older events arriving late are
    -- skipped (M1-F4).
    paddle_updated_at      timestamptz NOT NULL,
    created_at             timestamptz NOT NULL DEFAULT now(),
    updated_at             timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT subscriptions_plan_known CHECK (plan IN ('pro_monthly', 'pro_yearly', 'studio_monthly')),
    CONSTRAINT subscriptions_status_known CHECK (
        status IN ('trialing', 'active', 'past_due', 'paused', 'canceled')
    )
);

CREATE INDEX subscriptions_user_id ON subscriptions (user_id);

CREATE TRIGGER subscriptions_set_updated_at BEFORE UPDATE ON subscriptions
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Every webhook Paddle sent, stored before it is processed (§9.4). event_id makes a resent
-- event a no-op; a job processes each one (M1-F3).
CREATE TABLE paddle_events (
    id           uuid        PRIMARY KEY,
    event_id     text        NOT NULL UNIQUE,
    event_type   text        NOT NULL,
    occurred_at  timestamptz NOT NULL,
    payload      jsonb       NOT NULL,
    received_at  timestamptz NOT NULL DEFAULT now(),
    processed_at timestamptz
);

CREATE INDEX paddle_events_unprocessed ON paddle_events (received_at) WHERE processed_at IS NULL;
