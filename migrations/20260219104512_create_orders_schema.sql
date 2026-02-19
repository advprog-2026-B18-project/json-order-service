DROP TYPE IF EXISTS order_status CASCADE;

CREATE TYPE order_status AS ENUM (
    'Pending', 'Paid', 'Purchased', 'Shipped', 'Completed', 'Cancelled'
);

CREATE TABLE IF NOT EXISTS orders (
                                      id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    titipers_id UUID NOT NULL,
    jastiper_id UUID NOT NULL,
    product_id UUID NOT NULL,
    quantity INT NOT NULL CHECK (quantity > 0),
    shipping_address TEXT NOT NULL,
    total_price BIGINT NOT NULL,
    status order_status NOT NULL DEFAULT 'Pending',
    voucher_code VARCHAR(50),
    discount_amount BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

CREATE TABLE IF NOT EXISTS order_status_history (
                                                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    old_status order_status,
    new_status order_status NOT NULL,
    changed_by UUID NOT NULL,
    note TEXT,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

CREATE TABLE IF NOT EXISTS ratings (
                                       id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    titipers_id UUID NOT NULL,
    jastiper_id UUID NOT NULL,
    product_id UUID NOT NULL,
    jastiper_rating SMALLINT CHECK (jastiper_rating BETWEEN 1 AND 5),
    product_rating SMALLINT CHECK (product_rating BETWEEN 1 AND 5),
    review TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

CREATE TABLE IF NOT EXISTS war_queue (
                                         id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id UUID NOT NULL,
    titipers_id UUID NOT NULL,
    quantity INT NOT NULL DEFAULT 1,
    status VARCHAR(20) NOT NULL DEFAULT 'Waiting',
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ
    );

CREATE INDEX IF NOT EXISTS idx_orders_titipers ON orders(titipers_id);
CREATE INDEX IF NOT EXISTS idx_orders_jastiper ON orders(jastiper_id);
CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
CREATE INDEX IF NOT EXISTS idx_war_queue_product ON war_queue(product_id, status);
CREATE INDEX IF NOT EXISTS idx_war_queue_joined ON war_queue(joined_at);