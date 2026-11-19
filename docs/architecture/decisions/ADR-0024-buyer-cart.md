# ADR-0024 — Buyer cart ownership, concurrency, and lifecycle

- Status: Accepted
- Date: 2026-11-20
- Owners: sdkwork-order
- Related: `0023_commerce_cart.up.sql`, `crates/sdkwork-order-service/src/domain/cart.rs`,
  `crates/sdkwork-order-repository-sqlx/src/postgres_cart.rs`,
  `crates/sdkwork-routes-order-app-api/src/cart_router.rs`,
  `crates/sdkwork-order-repository-sqlx/tests/cart_postgres.rs`

## Context

The published commerce app contract declares `cart.current.retrieve` and
`cart.items.create|update|delete`, and every storefront client calls them, but no
service owned the cart state: the retired catalog module had dropped
`commerce_cart`/`commerce_cart_item` together with its handlers, so the gateway
answered `404 route-not-in-manifest` for `POST /app/v3/api/cart/items`.

Order already owns the buyer transaction surface (checkout sessions consume
`cartItemIds`, orders carry the money model), so the cart belongs here.

## Decision

### 1. One live cart per buyer, enforced by the database

`commerce_cart` is keyed by `(tenant_id, owner_user_id)` through a **partial**
unique index on `status = 'active'`. Converted, abandoned, and expired carts keep
their rows for audit without blocking the slot. `status` is constrained to the
four lifecycle states, never free text.

### 2. The line is a price snapshot, the cart is the staging area

`commerce_cart_item` stores the SKU id, the product id derived **server-side**
from the SKU row, quantity, selection, display fields, and the captured
`unit_price_minor`/`price_scale`/`currency_code`. The checkout session remains the
authority for what is charged; the cart never re-prices what the buyer saw.
`spuId`/`shopId` are not accepted from the client — attribution is not a client
decision.

### 3. One row lock serializes every cart mutation

Each mutation runs in one transaction that resolves the buyer's live cart with
`SELECT … FOR UPDATE`, then validates and writes. The line cap (100), the per-SKU
quantity window (`min_order_quantity` ≤ existing + requested ≤ `max_order_quantity`
≤ 999), the single-currency rule, and the read-after-write all execute under that
lock, so validation cannot be invalidated between the check and the write. Carts
are per buyer, so the lock never couples two buyers.

### 4. The database enforces the invariants on its own

`CHECK quantity BETWEEN 1 AND 999`, `unit_price_minor >= 0`, `price_scale` 0–6,
three-character currency codes, one line per `(cart_id, sku_id)`, and cascade
delete. A future writer that bypasses the repository still cannot persist a state
the totals would mis-parse.

### 5. Writes are retry-safe

`POST /cart/items` honours an optional `Idempotency-Key` through
`commerce_idempotency_key`, claimed in the same transaction as the add: a retry
replays the recorded line, a reused key with a different request hashes to `409`,
and a rolled-back attempt leaves no claim. The header is required by the order
app-api contract but tolerated as absent at runtime because the already-shipped
storefront transport does not send one.

### 6. Checkout consumes the selected lines

`POST /checkout/sessions` accepts `cartItemIds`; the server takes the SKU and
quantity from the cart snapshot, rejects lines that are no longer sellable, and
removes exactly those lines once the session exists. A failure there is reported
but cannot fail the committed checkout.

### 7. Read model tells the buyer what changed

Each line exposes `available` (the SKU is still sellable), `currentUnitPrice` and
`priceChanged` (drift against the snapshot), and `availableStock`/`stockShortage`
(sum of active stock rows; absent when the SKU tracks no inventory, so unlimited
goods never render a shortage). The cart aggregates `itemCount`,
`totalQuantity`, `selectedQuantity`, `totalAmount` of the selected lines,
`hasUnavailableSelection`, and `hasStockShortage`.

### 8. Idle carts expire

A bounded background sweep (`SDKWORK_ORDER_CART_IDLE_DAYS`, default 30) moves
carts whose `updated_at` is older than the retention window to `expired`,
oldest-first, so a backlog drains across ticks without locking a large row set.
Without it the unique active slot would be occupied forever by an abandoned cart.

## Consequences

- Cart reads are index-driven (< 1 ms measured at 200k carts) with no sequential
  scans; the hot paths are the partial unique index, the
  `(cart_id, created_at, id)` index, the `(cart_id, sku_id)` unique index, and the
  existing `(tenant_id, sku_id, status)` stock index.
- The cart is intentionally not a price authority and not an inventory
  reservation; both stay with checkout.
- Rate limiting lives at the gateway (`cart-write-ip`, 60 burst / 30 rps), not in
  the domain.
- Regression lives in `tests/cart_postgres.rs` (8 tests, gated on
  `SDKWORK_DATABASE_TEST_POSTGRES_URL`), covering increments and snapshots,
  rejection classes, 20-way concurrency, idempotent replay, owner scoping and
  consumption, the line cap, idle expiry, and stock shortage.
