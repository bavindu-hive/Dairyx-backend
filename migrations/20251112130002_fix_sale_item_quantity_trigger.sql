-- Fix sale item quantity check to work with truck loads
-- The old trigger checked batches.remaining_quantity which is incorrect
-- because truck loads already deduct from remaining_quantity.
-- Sales should check against truck_load_items availability instead.

-- Drop the old triggers and functions
DROP TRIGGER IF EXISTS check_sale_item_quantity ON sale_items;
DROP FUNCTION IF EXISTS check_batch_quantity();

DROP TRIGGER IF EXISTS update_batch_quantity ON sale_items;
DROP FUNCTION IF EXISTS update_batch_after_sale();

-- Create new trigger that checks truck_load_items instead
CREATE OR REPLACE FUNCTION check_truck_load_item_quantity()
RETURNS TRIGGER AS $$
DECLARE
    v_truck_load_id INTEGER;
    v_available_quantity INTEGER;
BEGIN
    -- Get truck_load_id from the sale
    SELECT truck_load_id INTO v_truck_load_id
    FROM sales
    WHERE id = NEW.sale_id;
    
    -- Check if there's enough quantity available in truck_load_items
    -- Available = loaded - already_sold - returned
    SELECT (quantity_loaded - quantity_sold - quantity_returned) INTO v_available_quantity
    FROM truck_load_items
    WHERE truck_load_id = v_truck_load_id
    AND batch_id = NEW.batch_id;
    
    -- If no truck_load_item found or insufficient quantity, raise error
    IF v_available_quantity IS NULL OR NEW.quantity > v_available_quantity THEN
        RAISE EXCEPTION 'Cannot sell more than available quantity in truck load';
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER check_sale_item_quantity
BEFORE INSERT OR UPDATE ON sale_items
FOR EACH ROW EXECUTE FUNCTION check_truck_load_item_quantity();

-- New trigger: Update truck_load_items.quantity_sold instead of batch quantity
CREATE OR REPLACE FUNCTION update_truck_load_after_sale()
RETURNS TRIGGER AS $$
DECLARE
    v_truck_load_id INTEGER;
BEGIN
    -- Get truck_load_id from the sale
    SELECT truck_load_id INTO v_truck_load_id
    FROM sales
    WHERE id = NEW.sale_id;
    
    -- Update truck_load_items quantity_sold
    UPDATE truck_load_items
    SET quantity_sold = quantity_sold + NEW.quantity
    WHERE truck_load_id = v_truck_load_id
    AND batch_id = NEW.batch_id;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_truck_load_quantity
AFTER INSERT ON sale_items
FOR EACH ROW EXECUTE FUNCTION update_truck_load_after_sale();

COMMENT ON FUNCTION check_truck_load_item_quantity() IS 'Validates sale quantity against truck_load_items availability';
COMMENT ON FUNCTION update_truck_load_after_sale() IS 'Updates truck_load_items.quantity_sold when a sale is made';

