-- Exports keep what they were made from (M1-D7). The design may change between the request
-- and the job, so the export holds its own copy of the spec at the version it records; the
-- copy also says exactly what a customer was sold. Nothing wrote exports before M1-D7, so the
-- table is empty and NOT NULL columns can be added.
ALTER TABLE exports ADD COLUMN spec jsonb NOT NULL;
-- The design's name then, for the files' names.
ALTER TABLE exports ADD COLUMN design_name text NOT NULL DEFAULT '';
-- The test print's paper, when it was asked for.
ALTER TABLE exports ADD COLUMN paper text;
-- How the paid formats were covered: free (none asked for), unlocked (already, within its
-- edit window), pro, or credit.
ALTER TABLE exports ADD COLUMN paid_with text NOT NULL;
ALTER TABLE exports ADD CONSTRAINT exports_paper_known CHECK (paper IN ('a4', 'letter'));
ALTER TABLE exports ADD CONSTRAINT exports_test_print_has_paper CHECK (
    paper IS NOT NULL OR NOT ('test_print' = ANY (formats))
);
ALTER TABLE exports ADD CONSTRAINT exports_paid_with_known CHECK (
    paid_with IN ('free', 'unlocked', 'pro', 'credit')
);
