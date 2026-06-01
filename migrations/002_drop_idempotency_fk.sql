ALTER TABLE idempotency_keys
    DROP CONSTRAINT IF EXISTS idempotency_keys_order_id_fkey,
    ALTER COLUMN order_id DROP NOT NULL;
