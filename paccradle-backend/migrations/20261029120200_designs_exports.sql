-- Designs and their exported files (CLAUDE.md §6, §8.4; M1-D6 and D7 write them).
CREATE TABLE designs (
    id         uuid        PRIMARY KEY,
    user_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- Shown and searched in "My designs" (M1-D6).
    name       text        NOT NULL DEFAULT '',
    -- insert_core::Design, read back to the last bit (serde_json float_roundtrip).
    spec       jsonb       NOT NULL,
    -- Goes up by one on every saved change; an export records the version it was made from.
    version    integer     NOT NULL DEFAULT 1,
    -- Set when the user shares a view-only link (/s/{slug}).
    share_slug text        UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT designs_version_positive CHECK (version >= 1),
    CONSTRAINT designs_name_length CHECK (length(name) <= 200),
    CONSTRAINT designs_share_slug_shape CHECK (share_slug ~ '^[A-Za-z0-9_-]{8,64}$')
);

CREATE INDEX designs_user_id ON designs (user_id, updated_at DESC);

CREATE TRIGGER designs_set_updated_at BEFORE UPDATE ON designs
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE exports (
    id             uuid        PRIMARY KEY,
    design_id      uuid        NOT NULL REFERENCES designs (id) ON DELETE CASCADE,
    design_version integer     NOT NULL,
    -- What was asked for. Paid: svg, pdf, dxf. Free: png (preview), test_print.
    formats        text[]      NOT NULL,
    status         text        NOT NULL DEFAULT 'pending',
    -- [{ "format", "storage_key", "bytes" }] once done; served through signed URLs.
    files          jsonb       NOT NULL DEFAULT '[]',
    error_code     text,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT exports_formats_known CHECK (
        cardinality(formats) > 0
        AND formats <@ ARRAY['svg', 'pdf', 'dxf', 'png', 'test_print']::text[]
    ),
    CONSTRAINT exports_status_known CHECK (status IN ('pending', 'running', 'done', 'failed')),
    CONSTRAINT exports_design_version_positive CHECK (design_version >= 1),
    CONSTRAINT exports_failed_has_code CHECK (status <> 'failed' OR error_code IS NOT NULL)
);

CREATE INDEX exports_design_id ON exports (design_id, created_at DESC);

CREATE TRIGGER exports_set_updated_at BEFORE UPDATE ON exports
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
