-- Capture runs (M1-D5). Each vision run of a capture has a number: the first is 1, and a tap
-- (or a height typed while detection is still running) starts the next. A run's job writes
-- its result only if its number is still the capture's, so a slower, older run never
-- overwrites a newer one or an outline the user edited (an edit also moves the number on).
ALTER TABLE captures ADD COLUMN run integer NOT NULL DEFAULT 1;
ALTER TABLE captures ADD CONSTRAINT captures_run_positive CHECK (run >= 1);

-- The numbers a stored error's message names (the field and what it allows, the photo's
-- size…), so `GET /captures/{id}` can say it in the caller's language.
ALTER TABLE captures ADD COLUMN error_meta jsonb NOT NULL DEFAULT '{}';
