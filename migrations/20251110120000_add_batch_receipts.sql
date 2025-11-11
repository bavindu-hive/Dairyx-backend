-- Create batch_receipts to decouple batches from deliveries
BEGIN;

CREATE TABLE IF NOT EXISTS batch_receipts (
  id BIGSERIAL PRIMARY KEY,
  batch_id BIGINT NOT NULL REFERENCES batches(id) ON DELETE RESTRICT,
  delivery_id BIGINT NOT NULL REFERENCES deliveries(id) ON DELETE RESTRICT,
  delivery_item_id BIGINT NOT NULL REFERENCES delivery_items(id) ON DELETE RESTRICT,
  quantity INTEGER NOT NULL CHECK (quantity > 0),
  created_at TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE (batch_id, delivery_item_id)
);

-- Helpful index for lookups
CREATE INDEX IF NOT EXISTS idx_batch_receipts_delivery_item_id ON batch_receipts(delivery_item_id);
CREATE INDEX IF NOT EXISTS idx_batch_receipts_delivery_id ON batch_receipts(delivery_id);
CREATE INDEX IF NOT EXISTS idx_batch_receipts_batch_id ON batch_receipts(batch_id);

-- Backfill existing batches to receipts (one receipt per existing batch link)
INSERT INTO batch_receipts (batch_id, delivery_id, delivery_item_id, quantity)
SELECT b.id, b.delivery_id, b.delivery_item_id, b.quantity
FROM batches b
LEFT JOIN batch_receipts br ON br.batch_id = b.id AND br.delivery_item_id = b.delivery_item_id
WHERE br.id IS NULL;

COMMIT;
