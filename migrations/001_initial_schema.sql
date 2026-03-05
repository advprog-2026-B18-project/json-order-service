-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

DO $$ BEGIN
CREATE TYPE order_status AS ENUM ('PENDING','PAID','PURCHASED','SHIPPED','COMPLETED','CANCELLED');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
CREATE TYPE cancellation_reason AS ENUM ('OUT_OF_STOCK_PHYSICAL','TRIP_CANCELLED','ITEM_UNAVAILABLE','OTHER');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

CREATE TABLE IF NOT EXISTS "order" (
                                       order_id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    titipers_id         UUID NOT NULL,
    jastiper_id         UUID NOT NULL,
    product_id          UUID NOT NULL,
    product_snapshot    JSONB NOT NULL,
    quantity            INTEGER NOT NULL CHECK (quantity >= 1),
    unit_price          BIGINT NOT NULL,
    service_fee         BIGINT NOT NULL DEFAULT 0,
    total_price         BIGINT NOT NULL,
    status              order_status NOT NULL DEFAULT 'PENDING',
    shipping_address    JSONB NOT NULL,
    note_to_jastiper    TEXT,
    tracking_number     TEXT,
    courier             TEXT,
    cancellation_reason cancellation_reason,
    cancelled_by        TEXT,
    completed_at        TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

CREATE TABLE IF NOT EXISTS "order_status_history" (
                                                      statushis_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id     UUID NOT NULL REFERENCES "order"(order_id) ON DELETE CASCADE,
    status       TEXT NOT NULL,
    changed_by   TEXT NOT NULL,
    actor_role   TEXT NOT NULL,
    notes        TEXT,
    timestamp    TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

CREATE TABLE IF NOT EXISTS "rating" (
                                        rating_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id        UUID NOT NULL UNIQUE REFERENCES "order"(order_id),
    titipers_id     UUID NOT NULL,
    jastiper_rating DECIMAL(2,1) NOT NULL CHECK (jastiper_rating BETWEEN 1.0 AND 5.0),
    jastiper_review TEXT,
    product_rating  DECIMAL(2,1) NOT NULL CHECK (product_rating BETWEEN 1.0 AND 5.0),
    product_review  TEXT,
    product_images  JSONB NOT NULL DEFAULT '[]',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

CREATE INDEX IF NOT EXISTS idx_order_titipers   ON "order"(titipers_id);
CREATE INDEX IF NOT EXISTS idx_order_jastiper   ON "order"(jastiper_id);
CREATE INDEX IF NOT EXISTS idx_order_status     ON "order"(status);
CREATE INDEX IF NOT EXISTS idx_order_created    ON "order"(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_history_order    ON "order_status_history"(order_id);
CREATE INDEX IF NOT EXISTS idx_ratings_order    ON "rating"(order_id);