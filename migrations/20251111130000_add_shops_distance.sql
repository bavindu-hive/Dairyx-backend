-- Add distance column to shops table
BEGIN;

ALTER TABLE shops 
ADD COLUMN distance NUMERIC(10,2) CHECK (distance >= 0);

COMMIT;
