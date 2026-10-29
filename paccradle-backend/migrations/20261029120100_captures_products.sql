-- Photos and what vision measured in them (CLAUDE.md §5, §8.4; M1-D5 writes them).
CREATE TABLE captures (
    id          uuid             PRIMARY KEY,
    user_id     uuid             NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    status      text             NOT NULL DEFAULT 'pending',
    mode        text             NOT NULL,
    paper       text             NOT NULL,
    -- vision::CaptureInput: paper, mode, pen width, tap point, typed height, lens.
    input       jsonb            NOT NULL,
    -- Camera fields the browser read (make, model, focal length). Never GPS.
    exif        jsonb            NOT NULL DEFAULT '{}',
    -- The photo in object storage; NULL once it is deleted (after 30 days, or on request).
    storage_key text,
    -- vision::VisionResult (CLAUDE.md §5.5). Kept while a correction re-runs.
    result      jsonb,
    -- A CAPTURE_* code when status = 'failed'.
    error_code  text,
    confidence  double precision,
    level_used  text,
    created_at  timestamptz      NOT NULL DEFAULT now(),
    updated_at  timestamptz      NOT NULL DEFAULT now(),
    purge_after timestamptz      NOT NULL,
    CONSTRAINT captures_status_known CHECK (status IN ('pending', 'done', 'failed')),
    CONSTRAINT captures_mode_known CHECK (mode IN ('photo', 'trace')),
    CONSTRAINT captures_paper_known CHECK (paper IN ('a4', 'letter')),
    CONSTRAINT captures_level_known CHECK (level_used IN ('l1', 'l2', 'l3')),
    CONSTRAINT captures_confidence_range CHECK (confidence BETWEEN 0 AND 1),
    CONSTRAINT captures_done_has_result CHECK (status <> 'done' OR result IS NOT NULL),
    CONSTRAINT captures_failed_has_code CHECK (status <> 'failed' OR error_code IS NOT NULL)
);

CREATE INDEX captures_user_id ON captures (user_id, created_at DESC);
-- The daily photo purge (M1-D4) looks for photos still stored past their date.
CREATE INDEX captures_purge_due ON captures (purge_after) WHERE storage_key IS NOT NULL;

CREATE TRIGGER captures_set_updated_at BEFORE UPDATE ON captures
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Measured products, reusable across designs. They outlive their photo (capture_id is
-- cleared when the capture goes), since only outlines and sizes are kept (§8.5).
CREATE TABLE products (
    id         uuid             PRIMARY KEY,
    user_id    uuid             NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    capture_id uuid             REFERENCES captures (id) ON DELETE SET NULL,
    name       text             NOT NULL DEFAULT '',
    shape      text             NOT NULL,
    -- [[x, y], …] in mm, centred on the product.
    contour_mm jsonb            NOT NULL,
    -- { "l": …, "w": … } in mm.
    bbox_mm    jsonb            NOT NULL,
    height_mm  double precision,
    -- Side profile for the turned 3D stand-in (v1, M3).
    profile    jsonb,
    created_at timestamptz      NOT NULL DEFAULT now(),
    updated_at timestamptz      NOT NULL DEFAULT now(),
    CONSTRAINT products_shape_known CHECK (shape IN ('round', 'rectangular', 'freeform')),
    CONSTRAINT products_height_positive CHECK (height_mm > 0),
    CONSTRAINT products_name_length CHECK (length(name) <= 200)
);

CREATE INDEX products_user_id ON products (user_id, created_at DESC);
CREATE INDEX products_capture_id ON products (capture_id);

CREATE TRIGGER products_set_updated_at BEFORE UPDATE ON products
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
