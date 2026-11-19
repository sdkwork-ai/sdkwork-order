-- sdkwork:migration
-- id: 0023_commerce_cart
-- engine: postgres
-- module: sdkwork-order
-- purpose: Materialize the buyer cart. The published commerce app contract still
--   declares `/app/v3/api/cart/current` and `/app/v3/api/cart/items`, and every
--   storefront client calls `commerce().cart.*`, but no service owned the cart
--   state: the retired catalog module dropped `commerce_cart`/`commerce_cart_item`
--   with its handlers, so the gateway manifest answered `404 route-not-in-manifest`.
--   Order now owns the buyer transaction surface (checkout and orders), so the cart
--   lives here: one active cart per tenant + buyer, lines snapshotted against the
--   merchandise SKU at add time.
-- reversible: false
-- rollback: forward-fix (cart lines are buyer-owned state; a dropped cart is data loss)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

CREATE TABLE IF NOT EXISTS commerce_cart (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    owner_user_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    currency_code TEXT NOT NULL DEFAULT 'CNY',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- One active cart per buyer. A paid/abandoned cart keeps its rows for audit, so
-- the uniqueness is partial on the live status instead of the whole table.
CREATE UNIQUE INDEX IF NOT EXISTS uk_commerce_cart_owner_active
    ON commerce_cart (tenant_id, owner_user_id)
    WHERE status = 'active';

CREATE TABLE IF NOT EXISTS commerce_cart_item (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    cart_id TEXT NOT NULL,
    sku_id TEXT NOT NULL,
    spu_id TEXT,
    shop_id TEXT,
    quantity BIGINT NOT NULL,
    product_title TEXT,
    sku_name TEXT,
    image_url TEXT,
    unit_price_minor BIGINT NOT NULL DEFAULT 0,
    price_scale SMALLINT NOT NULL DEFAULT 2,
    currency_code TEXT NOT NULL DEFAULT 'CNY',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT fk_commerce_cart_item_cart
        FOREIGN KEY (cart_id) REFERENCES commerce_cart (id) ON DELETE CASCADE,
    CONSTRAINT ck_commerce_cart_item_quantity CHECK (quantity > 0)
);

-- Re-adding the same SKU increments the existing line instead of duplicating it.
CREATE UNIQUE INDEX IF NOT EXISTS uk_commerce_cart_item_cart_sku
    ON commerce_cart_item (cart_id, sku_id);

CREATE INDEX IF NOT EXISTS idx_commerce_cart_item_cart
    ON commerce_cart_item (cart_id, created_at, id);

COMMIT;
