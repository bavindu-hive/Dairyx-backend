-- Migration: Create daily reconciliation system
-- Tracks end-of-day truck returns and profit summary

-- Create reconciliation status enum
CREATE TYPE reconciliation_status AS ENUM (
    'in_progress',  -- Started, trucks being verified
    'completed',    -- All trucks verified, ready to finalize
    'finalized'     -- Locked, summary generated, stock returned
);

-- Create daily_reconciliations table
CREATE TABLE daily_reconciliations (
    id SERIAL PRIMARY KEY,
    reconciliation_date DATE NOT NULL UNIQUE,  -- One reconciliation per day
    status reconciliation_status NOT NULL DEFAULT 'in_progress',
    
    -- Truck summary
    trucks_out INTEGER NOT NULL DEFAULT 0,  -- Number of trucks that went out
    trucks_verified INTEGER NOT NULL DEFAULT 0,  -- Number verified so far
    
    -- Stock summary
    total_items_loaded NUMERIC(10, 2) NOT NULL DEFAULT 0,
    total_items_sold NUMERIC(10, 2) NOT NULL DEFAULT 0,
    total_items_returned NUMERIC(10, 2) NOT NULL DEFAULT 0,
    total_items_discarded NUMERIC(10, 2) NOT NULL DEFAULT 0,  -- Damaged/expired
    
    -- Financial summary
    total_sales_amount NUMERIC(12, 2) NOT NULL DEFAULT 0,
    total_commission_earned NUMERIC(12, 2) NOT NULL DEFAULT 0,
    total_allowance_allocated NUMERIC(12, 2) NOT NULL DEFAULT 0,
    total_payments_collected NUMERIC(12, 2) NOT NULL DEFAULT 0,
    pending_payments NUMERIC(12, 2) NOT NULL DEFAULT 0,
    
    -- Net profit calculation: commission earned - allowance allocated
    net_profit NUMERIC(12, 2) NOT NULL DEFAULT 0,
    
    -- Metadata
    started_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    started_at TIMESTAMP NOT NULL DEFAULT NOW(),
    finalized_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    finalized_at TIMESTAMP,
    notes TEXT,
    
    -- Constraints
    CONSTRAINT valid_truck_counts CHECK (trucks_verified <= trucks_out),
    CONSTRAINT valid_item_counts CHECK (
        total_items_loaded = total_items_sold + total_items_returned + total_items_discarded
    )
);

-- Create reconciliation_items table (per-truck verification)
CREATE TABLE reconciliation_items (
    id SERIAL PRIMARY KEY,
    reconciliation_id INTEGER NOT NULL REFERENCES daily_reconciliations(id) ON DELETE CASCADE,
    truck_id INTEGER NOT NULL REFERENCES trucks(id) ON DELETE RESTRICT,
    driver_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    
    -- Truck load reference
    truck_load_id INTEGER NOT NULL REFERENCES truck_loads(id) ON DELETE RESTRICT,
    
    -- Stock verification
    items_loaded NUMERIC(10, 2) NOT NULL,
    items_sold NUMERIC(10, 2) NOT NULL,
    items_returned NUMERIC(10, 2) NOT NULL,
    items_discarded NUMERIC(10, 2) NOT NULL DEFAULT 0,  -- Damaged/expired/wasted
    
    -- Status
    is_verified BOOLEAN NOT NULL DEFAULT false,
    has_discrepancy BOOLEAN NOT NULL DEFAULT false,
    discrepancy_notes TEXT,
    
    -- Financial
    sales_amount NUMERIC(12, 2) NOT NULL DEFAULT 0,
    commission_earned NUMERIC(12, 2) NOT NULL DEFAULT 0,
    allowance_received NUMERIC(12, 2) NOT NULL DEFAULT 0,
    payments_collected NUMERIC(12, 2) NOT NULL DEFAULT 0,
    pending_payments NUMERIC(12, 2) NOT NULL DEFAULT 0,
    
    -- Metadata
    verified_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    verified_at TIMESTAMP,
    
    -- Constraints
    CONSTRAINT unique_truck_per_reconciliation UNIQUE(reconciliation_id, truck_id),
    CONSTRAINT valid_stock_balance CHECK (
        items_loaded = items_sold + items_returned + items_discarded
    )
);

-- Indexes for performance
CREATE INDEX idx_reconciliations_date ON daily_reconciliations(reconciliation_date);
CREATE INDEX idx_reconciliations_status ON daily_reconciliations(status);
CREATE INDEX idx_reconciliation_items_reconciliation ON reconciliation_items(reconciliation_id);
CREATE INDEX idx_reconciliation_items_truck ON reconciliation_items(truck_id);
CREATE INDEX idx_reconciliation_items_verified ON reconciliation_items(is_verified);

-- View: Reconciliation summary with details
CREATE OR REPLACE VIEW reconciliation_summary AS
SELECT 
    dr.id,
    dr.reconciliation_date,
    dr.status,
    dr.trucks_out,
    dr.trucks_verified,
    dr.total_items_loaded,
    dr.total_items_sold,
    dr.total_items_returned,
    dr.total_items_discarded,
    dr.total_sales_amount,
    dr.total_commission_earned,
    dr.total_allowance_allocated,
    dr.total_payments_collected,
    dr.pending_payments,
    dr.net_profit,
    CASE 
        WHEN dr.net_profit >= 0 THEN 'profit'
        ELSE 'loss'
    END as profit_status,
    dr.started_by,
    su.username as started_by_username,
    dr.started_at,
    dr.finalized_by,
    fu.username as finalized_by_username,
    dr.finalized_at,
    COUNT(ri.id) as total_truck_items,
    COUNT(CASE WHEN ri.is_verified THEN 1 END) as verified_truck_items,
    COUNT(CASE WHEN ri.has_discrepancy THEN 1 END) as trucks_with_discrepancies
FROM daily_reconciliations dr
LEFT JOIN users su ON dr.started_by = su.id
LEFT JOIN users fu ON dr.finalized_by = fu.id
LEFT JOIN reconciliation_items ri ON dr.id = ri.reconciliation_id
GROUP BY dr.id, dr.reconciliation_date, dr.status, dr.trucks_out, dr.trucks_verified,
         dr.total_items_loaded, dr.total_items_sold, dr.total_items_returned, 
         dr.total_items_discarded, dr.total_sales_amount, dr.total_commission_earned,
         dr.total_allowance_allocated, dr.total_payments_collected, dr.pending_payments,
         dr.net_profit, dr.started_by, su.username, dr.started_at, 
         dr.finalized_by, fu.username, dr.finalized_at;

-- View: Truck performance report
CREATE OR REPLACE VIEW truck_performance_report AS
SELECT 
    ri.truck_id,
    t.truck_number,
    ri.driver_id,
    u.username as driver_username,
    dr.reconciliation_date,
    ri.items_loaded,
    ri.items_sold,
    ri.items_returned,
    ri.items_discarded,
    ROUND((ri.items_sold / NULLIF(ri.items_loaded, 0) * 100), 2) as sales_percentage,
    ROUND((ri.items_returned / NULLIF(ri.items_loaded, 0) * 100), 2) as return_percentage,
    ROUND((ri.items_discarded / NULLIF(ri.items_loaded, 0) * 100), 2) as waste_percentage,
    ri.sales_amount,
    ri.commission_earned,
    ri.allowance_received,
    ri.payments_collected,
    ri.pending_payments,
    (ri.commission_earned - ri.allowance_received) as net_profit,
    ri.has_discrepancy,
    ri.discrepancy_notes,
    ri.is_verified,
    ri.verified_at
FROM reconciliation_items ri
JOIN trucks t ON ri.truck_id = t.id
JOIN users u ON ri.driver_id = u.id
JOIN daily_reconciliations dr ON ri.reconciliation_id = dr.id
ORDER BY dr.reconciliation_date DESC, t.truck_number;

COMMENT ON TABLE daily_reconciliations IS 'Daily end-of-day reconciliation and profit summary';
COMMENT ON TABLE reconciliation_items IS 'Per-truck verification details for daily reconciliation';
COMMENT ON VIEW reconciliation_summary IS 'Complete reconciliation overview with verification status';
COMMENT ON VIEW truck_performance_report IS 'Detailed truck and driver performance metrics';
