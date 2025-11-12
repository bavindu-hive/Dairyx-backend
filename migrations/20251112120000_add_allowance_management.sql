-- Add Transport Allowance Management Columns
-- Enhances existing transport_allowances and truck_allowances tables

BEGIN;

-- Add missing columns to transport_allowances
ALTER TABLE transport_allowances 
    ADD COLUMN IF NOT EXISTS allocated_amount NUMERIC(10,2) DEFAULT 0 CHECK (allocated_amount >= 0),
    ADD COLUMN IF NOT EXISTS status VARCHAR(20) DEFAULT 'pending' CHECK (status IN ('pending', 'allocated', 'finalized')),
    ADD COLUMN IF NOT EXISTS notes TEXT,
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT NOW();

-- Make allowance_date UNIQUE (only one allowance entry per day)
ALTER TABLE transport_allowances 
    ADD CONSTRAINT unique_allowance_date UNIQUE (allowance_date);

-- Add missing columns to truck_allowances
ALTER TABLE truck_allowances
    ADD COLUMN IF NOT EXISTS distance_covered NUMERIC(10,2) CHECK (distance_covered >= 0),
    ADD COLUMN IF NOT EXISTS notes TEXT;

-- Add max_allowance_limit to trucks table (default Rs. 4,000)
ALTER TABLE trucks
    ADD COLUMN IF NOT EXISTS max_allowance_limit NUMERIC(10,2) DEFAULT 4000.00 CHECK (max_allowance_limit >= 0);

-- Create function to auto-update allocated_amount when truck allocations change
CREATE OR REPLACE FUNCTION update_transport_allowance_allocated()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE transport_allowances
    SET 
        allocated_amount = (
            SELECT COALESCE(SUM(amount), 0)
            FROM truck_allowances
            WHERE transport_allowance_id = COALESCE(NEW.transport_allowance_id, OLD.transport_allowance_id)
        ),
        updated_at = CURRENT_TIMESTAMP
    WHERE id = COALESCE(NEW.transport_allowance_id, OLD.transport_allowance_id);
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- Trigger to update allocated_amount automatically
CREATE TRIGGER trigger_update_transport_allocated
AFTER INSERT OR UPDATE OR DELETE ON truck_allowances
FOR EACH ROW
EXECUTE FUNCTION update_transport_allowance_allocated();

-- View for allowance summary
CREATE VIEW allowance_summary AS
SELECT 
    ta.id,
    ta.allowance_date,
    ta.total_allowance,
    ta.allocated_amount,
    (ta.total_allowance - ta.allocated_amount) as remaining_amount,
    ta.status,
    ta.notes,
    COUNT(tka.id) as truck_count,
    u.username as created_by_username,
    ta.created_at,
    ta.updated_at
FROM transport_allowances ta
LEFT JOIN truck_allowances tka ON ta.id = tka.transport_allowance_id
LEFT JOIN users u ON ta.created_by = u.id
GROUP BY ta.id, ta.allowance_date, ta.total_allowance, ta.allocated_amount, 
         ta.status, ta.notes, u.username, ta.created_at, ta.updated_at;

-- Index for better query performance
CREATE INDEX IF NOT EXISTS idx_transport_allowances_date ON transport_allowances(allowance_date);
CREATE INDEX IF NOT EXISTS idx_transport_allowances_status ON transport_allowances(status);
CREATE INDEX IF NOT EXISTS idx_truck_allowances_transport ON truck_allowances(transport_allowance_id);

COMMIT;
