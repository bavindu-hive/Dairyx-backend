-- Add truck loading/inventory tracking system
BEGIN;

-- Represents a truck being loaded on a specific date
CREATE TABLE truck_loads (
    id BIGSERIAL PRIMARY KEY,
    truck_id BIGINT NOT NULL REFERENCES trucks(id),
    load_date DATE NOT NULL,
    loaded_by BIGINT REFERENCES users(id), -- manager who loaded it
    status VARCHAR(20) NOT NULL DEFAULT 'loaded' CHECK (status IN ('loaded', 'in_transit', 'returned', 'reconciled')),
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (truck_id, load_date) -- one load per truck per day
);

-- Individual product batches loaded onto the truck
CREATE TABLE truck_load_items (
    id BIGSERIAL PRIMARY KEY,
    truck_load_id BIGINT NOT NULL REFERENCES truck_loads(id) ON DELETE CASCADE,
    batch_id BIGINT NOT NULL REFERENCES batches(id),
    quantity_loaded INTEGER NOT NULL CHECK (quantity_loaded > 0),
    quantity_sold INTEGER NOT NULL DEFAULT 0 CHECK (quantity_sold >= 0 AND quantity_sold <= quantity_loaded),
    quantity_returned INTEGER NOT NULL DEFAULT 0 CHECK (quantity_returned >= 0),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (truck_load_id, batch_id),
    CHECK (quantity_sold + quantity_returned <= quantity_loaded)
);

-- Add truck_load_id to sales table for traceability
ALTER TABLE sales 
ADD COLUMN truck_load_id BIGINT REFERENCES truck_loads(id);

-- Indexes for performance
CREATE INDEX idx_truck_loads_truck_date ON truck_loads(truck_id, load_date DESC);
CREATE INDEX idx_truck_loads_status ON truck_loads(status);
CREATE INDEX idx_truck_loads_date ON truck_loads(load_date DESC);
CREATE INDEX idx_truck_load_items_batch ON truck_load_items(batch_id);
CREATE INDEX idx_truck_load_items_truck_load ON truck_load_items(truck_load_id);
CREATE INDEX idx_sales_truck_load ON sales(truck_load_id);

-- View: Daily truck inventory summary
CREATE VIEW truck_inventory_summary AS
SELECT 
    tl.id as truck_load_id,
    tl.truck_id,
    t.truck_number,
    u.username as driver,
    tl.load_date,
    tl.status,
    COUNT(tli.id) as product_lines,
    SUM(tli.quantity_loaded) as total_loaded,
    SUM(tli.quantity_sold) as total_sold,
    SUM(tli.quantity_returned) as total_returned,
    SUM(tli.quantity_loaded - tli.quantity_sold - tli.quantity_returned) as total_lost_damaged
FROM truck_loads tl
JOIN trucks t ON tl.truck_id = t.id
LEFT JOIN users u ON t.driver_id = u.id
LEFT JOIN truck_load_items tli ON tl.id = tli.truck_load_id
GROUP BY tl.id, tl.truck_id, t.truck_number, u.username, tl.load_date, tl.status;

-- Trigger to update truck_load_items.quantity_sold when sale is made
CREATE OR REPLACE FUNCTION update_truck_load_item_sold()
RETURNS TRIGGER AS $$
DECLARE
    v_truck_load_id BIGINT;
    v_batch_id BIGINT;
BEGIN
    -- Get the truck_load_id from the sale
    SELECT truck_load_id INTO v_truck_load_id
    FROM sales
    WHERE id = NEW.sale_id;
    
    -- If this sale is linked to a truck load, update the quantity_sold
    IF v_truck_load_id IS NOT NULL THEN
        -- Get batch_id from sale_item
        v_batch_id := NEW.batch_id;
        
        -- Update the truck_load_items quantity_sold
        UPDATE truck_load_items
        SET quantity_sold = quantity_sold + NEW.quantity
        WHERE truck_load_id = v_truck_load_id 
        AND batch_id = v_batch_id;
        
        -- Check if update succeeded (item exists in truck load)
        IF NOT FOUND THEN
            RAISE EXCEPTION 'Batch % was not loaded on truck load %', v_batch_id, v_truck_load_id;
        END IF;
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_truck_load_sold
AFTER INSERT ON sale_items
FOR EACH ROW EXECUTE FUNCTION update_truck_load_item_sold();

COMMIT;
