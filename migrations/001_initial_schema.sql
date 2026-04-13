-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

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
    status              TEXT NOT NULL DEFAULT 'PENDING',
    shipping_address    JSONB NOT NULL,
    note_to_jastiper    TEXT,
    tracking_number     TEXT,
    courier             TEXT,
    cancellation_reason TEXT,
    cancelled_by        TEXT,
    completed_at        TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

CREATE TABLE IF NOT EXISTS "order_status_history" (
                                                      status_his_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id     UUID NOT NULL REFERENCES "order"(order_id) ON DELETE CASCADE,
    status       TEXT NOT NULL,
    changed_by   TEXT NOT NULL,
    actor_role   TEXT NOT NULL,
    notes        TEXT,
    timestamp    TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

CREATE TABLE IF NOT EXISTS "rating_jastiper" (
                                                 rating_jastiper_id  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id            UUID NOT NULL UNIQUE REFERENCES "order"(order_id) ON DELETE CASCADE,
    titipers_id         UUID NOT NULL,
    jastiper_rating     FLOAT8 NOT NULL CHECK (jastiper_rating BETWEEN 1.0 AND 5.0),
    jastiper_review     TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

CREATE TABLE IF NOT EXISTS "rating_product" (
                                                rating_product_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id            UUID NOT NULL UNIQUE REFERENCES "order"(order_id) ON DELETE CASCADE,
    titipers_id         UUID NOT NULL,
    product_rating      FLOAT8 NOT NULL CHECK (product_rating BETWEEN 1.0 AND 5.0),
    product_review      TEXT,
    product_images      TEXT[] NOT NULL DEFAULT '{}',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

CREATE INDEX IF NOT EXISTS idx_order_titipers   ON "order"(titipers_id);
CREATE INDEX IF NOT EXISTS idx_order_jastiper   ON "order"(jastiper_id);
CREATE INDEX IF NOT EXISTS idx_order_status     ON "order"(status);
CREATE INDEX IF NOT EXISTS idx_order_created    ON "order"(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_history_order    ON "order_status_history"(order_id);
CREATE INDEX IF NOT EXISTS idx_rating_jastiper_order ON "rating_jastiper"(order_id);
CREATE INDEX IF NOT EXISTS idx_rating_product_order  ON "rating_product"(order_id);