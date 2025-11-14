-- DairyX Distributors - Corrected Schema with Delivery Items
-- Restores the crucial delivery_items table

BEGIN;


-- Core Tables
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(50) UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    role VARCHAR(20) NOT NULL CHECK (role IN ('manager', 'driver')),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Products table stores current prices and commission rates
CREATE TABLE products (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) UNIQUE NOT NULL,
    current_wholesale_price NUMERIC(10,2) NOT NULL CHECK (current_wholesale_price >= 0),
    commission_per_unit NUMERIC(10,2) NOT NULL CHECK (commission_per_unit >= 0),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE deliveries (
    id BIGSERIAL PRIMARY KEY,
    delivery_date DATE NOT NULL,
    received_by BIGINT REFERENCES users(id),
    delivery_note_number VARCHAR(100) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- DELIVERY ITEMS RESTORED - This records what was actually delivered
CREATE TABLE delivery_items (
    id BIGSERIAL PRIMARY KEY,
    delivery_id BIGINT NOT NULL REFERENCES deliveries(id),
    product_id BIGINT NOT NULL REFERENCES products(id),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    unit_price NUMERIC(10,2) NOT NULL CHECK (unit_price >= 0), -- Actual price from CreamyLand
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (delivery_id, product_id) -- One entry per product per delivery
);

-- Batches track physical inventory with expiry dates
CREATE TABLE batches (
    id BIGSERIAL PRIMARY KEY,
    product_id BIGINT NOT NULL REFERENCES products(id),
    delivery_id BIGINT NOT NULL REFERENCES deliveries(id),
    delivery_item_id BIGINT NOT NULL REFERENCES delivery_items(id), -- Link to specific delivery item
    batch_number VARCHAR(100) NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity >= 0),
    remaining_quantity INTEGER NOT NULL CHECK (remaining_quantity >= 0 AND remaining_quantity <= quantity),
    expiry_date DATE NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (product_id, batch_number)
);

CREATE TABLE trucks (
    id BIGSERIAL PRIMARY KEY,
    truck_number VARCHAR(50) UNIQUE NOT NULL,
    driver_id BIGINT UNIQUE REFERENCES users(id),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE shops (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) UNIQUE NOT NULL,
    location TEXT,
    contact_info TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE sales (
    id BIGSERIAL PRIMARY KEY,
    shop_id BIGINT NOT NULL REFERENCES shops(id),
    truck_id BIGINT NOT NULL REFERENCES trucks(id),
    user_id BIGINT NOT NULL REFERENCES users(id),
    total_amount NUMERIC(10,2) NOT NULL CHECK (total_amount >= 0),
    amount_paid NUMERIC(10,2) NOT NULL DEFAULT 0 CHECK (amount_paid >= 0 AND amount_paid <= total_amount),
    payment_status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (payment_status IN ('paid', 'pending')),
    sale_date DATE NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE sale_items (
    id BIGSERIAL PRIMARY KEY,
    sale_id BIGINT NOT NULL REFERENCES sales(id),
    batch_id BIGINT NOT NULL REFERENCES batches(id),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    unit_price NUMERIC(10,2) NOT NULL CHECK (unit_price >= 0), -- Sale price to shop
    commission_earned NUMERIC(10,2) NOT NULL CHECK (commission_earned >= 0),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE transport_allowances (
    id BIGSERIAL PRIMARY KEY,
    allowance_date DATE NOT NULL,
    total_allowance NUMERIC(10,2) NOT NULL CHECK (total_allowance >= 0),
    created_by BIGINT REFERENCES users(id),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE truck_allowances (
    id BIGSERIAL PRIMARY KEY,
    transport_allowance_id BIGINT NOT NULL REFERENCES transport_allowances(id),
    truck_id BIGINT NOT NULL REFERENCES trucks(id),
    amount NUMERIC(10,2) NOT NULL CHECK (amount >= 0),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (transport_allowance_id, truck_id)
);

-- Essential Indexes
CREATE INDEX idx_batches_product_id ON batches(product_id);
CREATE INDEX idx_batches_expiry_date ON batches(expiry_date);
CREATE INDEX idx_batches_remaining ON batches(remaining_quantity) WHERE remaining_quantity > 0;
CREATE INDEX idx_sales_date ON sales(sale_date);
CREATE INDEX idx_sales_shop ON sales(shop_id);
CREATE INDEX idx_sales_truck ON sales(truck_id);
CREATE INDEX idx_sale_items_batch ON sale_items(batch_id);
CREATE INDEX idx_delivery_items_delivery ON delivery_items(delivery_id);

-- Critical Business Logic Constraints

-- Prevent selling more than available batch quantity
CREATE OR REPLACE FUNCTION check_batch_quantity()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.quantity > (SELECT remaining_quantity FROM batches WHERE id = NEW.batch_id) THEN
        RAISE EXCEPTION 'Cannot sell more than available batch quantity';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER check_sale_item_quantity
BEFORE INSERT OR UPDATE ON sale_items
FOR EACH ROW EXECUTE FUNCTION check_batch_quantity();

-- Update batch quantity after sale
CREATE OR REPLACE FUNCTION update_batch_after_sale()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE batches 
    SET remaining_quantity = remaining_quantity - NEW.quantity 
    WHERE id = NEW.batch_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_batch_quantity
AFTER INSERT ON sale_items
FOR EACH ROW EXECUTE FUNCTION update_batch_after_sale();

-- Essential Views

-- Current Stock Levels
CREATE VIEW current_stock AS
SELECT 
    p.id,
    p.name,
    p.current_wholesale_price,
    p.commission_per_unit,
    COALESCE(SUM(b.remaining_quantity), 0) as current_stock,
    COUNT(b.id) as active_batches,
    MIN(b.expiry_date) as earliest_expiry
FROM products p
LEFT JOIN batches b ON p.id = b.product_id AND b.remaining_quantity > 0
GROUP BY p.id, p.name, p.current_wholesale_price, p.commission_per_unit;

-- FIFO Batch Selection (sell oldest first)
CREATE VIEW available_batches_fifo AS
SELECT 
    b.*,
    p.name as product_name,
    p.commission_per_unit,
    di.unit_price as wholesale_price -- From delivery_items
FROM batches b
JOIN products p ON b.product_id = p.id
JOIN delivery_items di ON b.delivery_item_id = di.id
WHERE b.remaining_quantity > 0
ORDER BY b.expiry_date ASC, b.created_at ASC;

-- Delivery Summary View
CREATE VIEW delivery_summary AS
SELECT 
    d.id as delivery_id,
    d.delivery_date,
    d.delivery_note_number,
    COUNT(di.id) as product_types,
    SUM(di.quantity) as total_units,
    SUM(di.quantity * di.unit_price) as total_value
FROM deliveries d
JOIN delivery_items di ON d.id = di.delivery_id
GROUP BY d.id, d.delivery_date, d.delivery_note_number;

-- Seed Data
INSERT INTO products (name, current_wholesale_price, commission_per_unit) VALUES
    ('Milk 1L Packet', 220.00, 10.00),
    ('Yogurt Cup', 150.00, 5.00),
    ('Butter 500g', 700.00, 15.00),
    ('Cheese 200g', 450.00, 12.00)
ON CONFLICT (name) DO UPDATE SET
    current_wholesale_price = EXCLUDED.current_wholesale_price,
    commission_per_unit = EXCLUDED.commission_per_unit;

-- Default manager user (password: "manager123")
INSERT INTO users (username, password_hash, role) VALUES
    ('manager', '$2b$12$LQv3c1yqBWVHxkd0g8f7QuYlC5nB.8qkQ8p8Nc6b5a6d5e4f3g2h1i', 'manager')
ON CONFLICT (username) DO UPDATE SET
    password_hash = EXCLUDED.password_hash,
    role = EXCLUDED.role;
COMMIT;