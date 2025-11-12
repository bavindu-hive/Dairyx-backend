-- Migration: Create comprehensive stock movement tracking system
-- Replaces batch_receipts with complete audit trail

-- Drop old batch_receipts table if it exists
DROP TABLE IF EXISTS batch_receipts CASCADE;

-- Create stock movement types enum
CREATE TYPE stock_movement_type AS ENUM (
    'delivery_in',      -- Stock received from CreamyLand delivery
    'truck_load_out',   -- Stock loaded onto truck
    'sale_out',         -- Stock sold from truck to shop
    'truck_return_in',  -- Stock returned from truck to batch
    'adjustment',       -- Manual adjustment (damaged, expired, correction)
    'expired_out'       -- Stock removed due to expiry
);

-- Create reference types enum for traceability
CREATE TYPE reference_type AS ENUM (
    'delivery',
    'truck_load',
    'sale',
    'reconciliation',
    'manual'
);

-- Create stock_movements table with complete audit trail
CREATE TABLE stock_movements (
    id SERIAL PRIMARY KEY,
    
    -- What moved
    batch_id INTEGER NOT NULL REFERENCES batches(id) ON DELETE RESTRICT,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    movement_type stock_movement_type NOT NULL,
    
    -- How much
    quantity NUMERIC(10, 2) NOT NULL CHECK (quantity > 0),
    
    -- Traceability
    reference_type reference_type NOT NULL,
    reference_id INTEGER NOT NULL,  -- ID of the source record (delivery_id, truck_load_id, sale_id, etc.)
    
    -- Additional context
    notes TEXT,
    created_by INTEGER REFERENCES users(id) ON DELETE SET NULL,  -- Who initiated this movement
    
    -- When
    movement_date DATE NOT NULL DEFAULT CURRENT_DATE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT valid_reference CHECK (reference_id > 0)
);

-- Indexes for performance
CREATE INDEX idx_stock_movements_batch ON stock_movements(batch_id, movement_date);
CREATE INDEX idx_stock_movements_product ON stock_movements(product_id, movement_date);
CREATE INDEX idx_stock_movements_type ON stock_movements(movement_type);
CREATE INDEX idx_stock_movements_date ON stock_movements(movement_date);
CREATE INDEX idx_stock_movements_reference ON stock_movements(reference_type, reference_id);

-- View: Running balance for each batch
CREATE OR REPLACE VIEW batch_stock_balance AS
SELECT 
    b.id as batch_id,
    b.product_id,
    p.name as product_name,
    b.quantity as initial_quantity,
    b.remaining_quantity,
    COALESCE(SUM(
        CASE 
            WHEN sm.movement_type IN ('delivery_in', 'truck_return_in', 'adjustment') 
            THEN sm.quantity 
            ELSE -sm.quantity 
        END
    ), 0) as calculated_balance,
    -- Verify integrity
    (b.remaining_quantity = COALESCE(SUM(
        CASE 
            WHEN sm.movement_type IN ('delivery_in', 'truck_return_in', 'adjustment') 
            THEN sm.quantity 
            ELSE -sm.quantity 
        END
    ), 0)) as balance_matches
FROM batches b
JOIN products p ON b.product_id = p.id
LEFT JOIN stock_movements sm ON sm.batch_id = b.id
GROUP BY b.id, b.product_id, p.name, b.quantity, b.remaining_quantity;

-- View: Daily stock movement summary
CREATE OR REPLACE VIEW daily_stock_summary AS
SELECT 
    sm.movement_date,
    sm.product_id,
    p.name as product_name,
    sm.movement_type,
    COUNT(*) as transaction_count,
    SUM(sm.quantity) as total_quantity,
    STRING_AGG(DISTINCT u.username, ', ') as users_involved
FROM stock_movements sm
JOIN products p ON sm.product_id = p.id
LEFT JOIN users u ON sm.created_by = u.id
GROUP BY sm.movement_date, sm.product_id, p.name, sm.movement_type
ORDER BY sm.movement_date DESC, sm.product_id, sm.movement_type;

-- Trigger: Auto-log batch creation as delivery_in movement
CREATE OR REPLACE FUNCTION log_batch_delivery() 
RETURNS TRIGGER AS $$
BEGIN
    -- Create stock movement for initial delivery
    INSERT INTO stock_movements (
        batch_id,
        product_id,
        movement_type,
        quantity,
        reference_type,
        reference_id,
        notes,
        created_by,
        movement_date
    ) VALUES (
        NEW.id,
        NEW.product_id,
        'delivery_in',
        NEW.quantity,
        'delivery',
        NEW.delivery_id,
        'Initial delivery receipt - Batch: ' || NEW.batch_number,
        NULL,  -- System generated
        NEW.created_at::DATE
    );
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_log_batch_delivery
AFTER INSERT ON batches
FOR EACH ROW
EXECUTE FUNCTION log_batch_delivery();

COMMENT ON TABLE stock_movements IS 'Complete audit trail of all stock movements in the system';
COMMENT ON VIEW batch_stock_balance IS 'Running balance verification for each batch with integrity check';
COMMENT ON VIEW daily_stock_summary IS 'Daily aggregated stock movements by product and type';
