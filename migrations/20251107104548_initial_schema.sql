-- No-op migration: previous migration already created the full schema.
-- This file intentionally left minimal to avoid duplicate object errors.
-- Remove leftover schema content from prior version.

BEGIN;
-- (intentionally empty)
COMMIT;
-- End of no-op migration.