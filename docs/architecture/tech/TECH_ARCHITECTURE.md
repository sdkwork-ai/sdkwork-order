# Order Technical Architecture

Status: active  
Owner: SDKWork maintainers  
Updated: 2026-07-27
Specs: ARCHITECTURE_DECISION_SPEC.md, DOCUMENTATION_SPEC.md

## 1. Architecture Overview

`sdkwork-order` is the standalone commerce order capability: domain services, SQL repositories, HTTP routers, standalone gateway, TypeScript SDKs, and the PC client surface.

For account value movement, order is the orchestration layer. It owns commercial evidence and lifecycle state for recharge, Token Bank plans, package recharge, coupon redemption, refund requests, and withdrawal requests. It does not own payment provider execution or account ledger truth.

## 2. Capability Stack

| Layer | Path |
| --- | --- |
| Domain (Rust) | `crates/sdkwork-order-service/` |
| SQL repositories | `crates/sdkwork-order-repository-sqlx/` |
| HTTP routers (app) | `crates/sdkwork-routes-order-app-api/` |
| HTTP routers (backend) | `crates/sdkwork-routes-order-backend-api/` |
| API server | `crates/sdkwork-api-order-standalone-gateway/` |
| PC client | `apps/sdkwork-order-pc/` |
| Composed service facade | `apps/sdkwork-order-common/packages/sdkwork-order-service/` |
| App SDK | `sdks/sdkwork-order-app-sdk/` |
| Backend SDK | `sdks/sdkwork-order-backend-sdk/` |

## 3. API Surfaces

| Surface | Prefix | Contract |
| --- | --- | --- |
| App API | `/app/v3/api/orders`, `/app/v3/api/recharges`, `/app/v3/api/checkout`, `/app/v3/api/memberships` | `apis/app-api/order/order-app-api.openapi.json` |
| Backend API | `/backend/v3/api/orders` | `apis/backend-api/order/order-backend-api.openapi.json` |
| Backend after-sales | `/backend/v3/api/after_sales/requests` | same authority |
| Backend shipments | `/backend/v3/api/shipments` | same authority |

OpenAPI discovery is served at `/app/v3/api/openapi.json` and `/backend/v3/api/openapi.json`.

All success responses use `SdkWorkApiResponse` (`code: 0`, `data`, `traceId`). List endpoints return `data.items` plus `data.pageInfo` with SQL-level `LIMIT`/`OFFSET` and `COUNT(*) OVER()` totals. Errors use `ProblemDetail` with numeric SDKWork error codes and `traceId`.

Write commands marked `x-sdkwork-idempotent` accept the standard `Idempotency-Key`. Request fingerprints are computed and persisted by the owning service from the authenticated scope, method/path, and canonical command input; clients never calculate or send fingerprint headers. Reusing a key with a different command returns HTTP 409.

### Order Creation Entry Points

| Route | Operation | Use case |
| --- | --- | --- |
| `POST /app/v3/api/checkout/sessions/{checkoutSessionId}/orders` | `checkout.sessions.orders.create` | Canonical checkout-bound product order creation after quote |
| `POST /app/v3/api/recharges/orders` | `recharges.orders.create` | Account value order creation, starting with points recharge and extending to Token Bank/package/coupon subjects |
| `POST /app/v3/api/memberships/orders` | `memberships.orders.create` | Membership purchase checkout (`subject=membership`) |

Membership checkout uses two independent idempotency layers. `Idempotency-Key` identifies one command and is bound to a server-computed request fingerprint; replaying that key with a different action, package, source, payment method, or payment product returns HTTP 409. `purchase_intent_key` identifies the commercial intent from tenant, organization, owner, membership action, and the selected package/price snapshot. A partial unique index permits only one active order for that intent. Payment method and product are intentionally excluded, so switching between H5, WeChat, and Alipay reuses the Order. Before creating the new PSP checkout, Payment closes prior active attempts through their historical provider account and commits their local terminal state only after the PSP close succeeds.

Membership Order creation normalizes and snapshots the requested payment method but does not require an active `commerce_payment_method` row. This keeps Order creation behavior identical across engine deployments (DATABASE_SPEC: server persistence is PostgreSQL-only) and allows the H5 cashier to resolve its provider after the Order exists. Native or provider-executed checkout still validates the effective Payment method, channel, provider account, scene, currency, and expiry inside `sdkwork-payment`; Order creation never treats an unconfigured provider as executable.

The command boundary accepts only `purchase`, `renew`, or `upgrade`, requires RFC3339 `requested_at` and `expire_at` values, and requires expiration to be later than request creation after timezone normalization. The create transaction marks expired matching orders as `expired`, returns a winner with `reused=true` only when `expired_at` is present and later than the request time, or inserts a new row. A missing, empty, malformed, or non-increasing Membership expiration boundary fails closed and is never replaced with the incoming request's value during replay. PostgreSQL combines typed `TIMESTAMPTZ` comparisons, the partial unique index, and conflict-safe winner reread. The API returns `action`, `expiresAt`, and `reused`. Shared TypeScript consumers coalesce concurrent identical calls and refresh an existing `orderId` before retrying creation, but database uniqueness remains authoritative across refreshes, devices, replicas, and applications.

Points recharge creation uses ISO 8601 UTC `requestedAt` and `expiresAt` boundaries and returns `expiresAt` in the initial create response, so browser countdowns never interpret a server UTC instant as local wall-clock time. A new command may reuse the same active commercial checkout across a different transport idempotency key only while the stored expiration boundary is non-empty, parseable, and later than the command time. In the same transaction, stale matching Orders advance from `pending_payment` to `expired`; the repository then creates a new Order instead of returning an expired QR. PostgreSQL uses explicit `timestamptz` casts. The PC recharge component keeps a valid QR visible, stops polling at expiration, and creates a replacement only after an explicit customer retry, avoiding both stale checkout reuse and unattended order churn.

New PC and integrator surfaces must use checkout sessions for product checkout and order app-api resources for account value orders. They must not call payment or account mutation APIs directly for recharge, refund, or withdrawal workflows.

## 4. Account Value Order Architecture

Account value order subjects:

| Subject | Target | Status |
| --- | --- | --- |
| `points_recharge` | Account `points` credit | complete |
| `token_bank_recharge` | Account `token_bank` credit | implemented settlement path |
| `token_bank_plan_purchase` | Token Bank first-cycle grant | implemented settlement path |
| `token_bank_plan_renewal` | Token Bank renewal grant | implemented settlement path |
| `account_recharge_package` | Package target account asset credit | implemented settlement path |
| `coupon_recharge` | Coupon-backed target account asset credit | implemented settlement path |
| `refund_request` | Account reversal hold plus provider refund | implemented review execution |
| `cash_withdrawal` | Account cash hold plus future provider payout | account hold lifecycle implemented; provider payout is fail-closed until payment exposes a concrete payout executor |

Dependency direction:

```text
sdkwork-order -> sdkwork-payment
sdkwork-order -> sdkwork-account
sdkwork-payment -X-> sdkwork-account
sdkwork-payment -X-> sdkwork-order service crates
sdkwork-account -X-> sdkwork-order
sdkwork-account -X-> sdkwork-payment
```

`sdkwork-payment` executes provider payment and refund channels today. Provider payout remains an explicit executor contract boundary; order keeps it fail-closed until payment publishes a concrete payout implementation. Payment must not call account ledger APIs, import account crates, or write account tables.

`sdkwork-account` executes idempotent ledger commands for credit, debit, hold, settlement, release, and reversal. It must not create orders, own packages or plans, execute provider channels, or approve refund/withdrawal business state.

### Account Value Ports

Order service ports:

| Port | Direction | Purpose |
| --- | --- | --- |
| `AccountValueLedgerPort` | order -> account | Credit Token Bank/points, hold cash, settle or release holds, reverse granted value |
| `PaymentRefundExecutorPort` | order -> payment | Execute provider refund for an approved refund request |
| `PaymentPayoutExecutorPort` | order -> payment | Reserved provider payout boundary for approved withdrawal requests; default runtime is fail-closed because payment has no concrete payout executor yet |
| `CouponRedemptionPort` | order -> coupon/promotion | Validate and consume coupon value for `coupon_recharge` |
| `TokenBankPlanOrderStore` | order-owned | Persist plan purchase and renewal commercial snapshots |

### Flow Summary

Paid account value orders:

```text
recharges.orders.create
  -> commerce_order pending_payment
  -> orders.payments.create through payment executor
  -> order-owned PSP webhook settlement
  -> account ledger command
  -> order fulfillment complete
```

Coupon recharge:

```text
coupon validation
  -> commerce_order subject=coupon_recharge
  -> optional mixed-payment orders.payments.create
  -> account target asset credit
```

Refund request:

```text
refund request
  -> account reversal hold
  -> provider refund through payment
  -> processing/ambiguous: retain hold and retry the same refund identity
  -> confirmed success: account reversal commit
  -> deterministic failure: hold release
```

Cash withdrawal:

```text
withdrawal request
  -> account cash hold
  -> default runtime fails closed through NoopPaymentPayoutExecutorPort
  -> current failure path releases the account hold
  -> future provider payout executor success settles the account hold
```

## 5. Database

- Engines: PostgreSQL (authoritative server; DATABASE_SPEC — SQLite is client-local only)
- Table prefix: `commerce_`
- DDL authority: `database/contract/table-registry.json`
- Repository implementations target PostgreSQL only on the server.
- List/search paths must use SQL-level pagination.

Existing order-owned or order-managed tables include `commerce_order`, `commerce_order_item`, `commerce_order_amount_breakdown`, `commerce_order_event`, `commerce_order_cancellation`, fulfillment, shipment, after-sales, and idempotency tables.

Account value extension tables:

| Table | Purpose |
| --- | --- |
| `commerce_account_value_package` | Recharge package catalog for points, Token Bank, or other account assets |
| `commerce_token_bank_plan` | Token Bank one-time and continuous plan catalog |
| `commerce_order_refund_request` | Refund request workflow and provider refund execution reference |
| `commerce_order_withdrawal_request` | Cash withdrawal workflow, account hold reference, and future provider payout execution reference |

Immutable package and plan facts are copied into `commerce_order_item.sku_snapshot_json` so account ledger rows never become the commercial catalog source of truth.

## 6. Payment Integration

`sdkwork-order` depends on `sdkwork-payment` for owner-order payment execution (`OwnerOrderPaymentStore`), payment webhook persistence, provider abstractions, and provider refund execution. The order host wires a concrete refund executor through `sdkwork-order-integration-payment`; it resolves the original Payment Attempt's provider-account and native transaction snapshots, claims the refund as `processing`, and reuses the same refund/provider idempotency identity on recovery. Processing recovery queries the original PSP account by the immutable payment and merchant refund identities before any resubmission; query ambiguity retains the account hold, and confirmed absence permits only an idempotent resubmission. Order keeps the account reversal hold while Payment reports `submitted`, `pending`, or `processing`; it settles only on `succeeded`/`refunded` and releases only on deterministic failure. Payout remains behind `PaymentPayoutExecutorPort` and fails closed until payment provides a concrete payout executor. The standalone gateway may wire repositories in-process; split deployments use HTTP backend APIs.

Settlement orchestration is owned by order, not payment:

| Step | Owner | Route / function |
| --- | --- | --- |
| PSP webhook | Order app-api | `POST /app/v3/api/orders/payments/webhooks/{providerCode}` |
| In-process settlement | Order service | `settle_owner_order_after_payment_success` |
| Manual replay | Order backend-api | `POST /backend/v3/api/orders/{orderId}/payment_confirmations` |

Configure PSP notify URL as `{ORDER_PAYMENT_WEBHOOK_BASE_URL}/app/v3/api/orders/payments/webhooks/{providerCode}`.

Duplicate webhook deliveries remain correlated to the exact persisted payment attempt and may re-enter settlement. Payment confirmation, Order state updates, fulfillment, and late-payment audit writes are idempotent, so retries do not duplicate effects. Operators use `payment_confirmations` for recovery; because its public request does not select an attempt, it proceeds only when the order has one unambiguous matching payment attempt.

A successful payment that arrives after an Order is terminal does not reopen or advance the Order lifecycle. Order preserves the terminal status, records `payment_status=success` and the first `paid_at`, and writes one idempotent `payment_succeeded_after_terminal` event.

### 6.1 Payment compensation worker (webhook-failure safety net)

A lost provider notification is recovered by the in-process compensation worker (`sdkwork-order-service-host::spawn_payment_compensation_worker`, spawned by the standalone gateway; opt-in via `SDKWORK_ORDER_PAYMENT_COMPENSATION_WORKER_ENABLED=1`, default off). Every tick (default 30 s) it claims `commerce_payment_attempt` rows stuck in `pending`/`processing` and `commerce_refund` rows stuck in `submitted`/`processing` (`FOR UPDATE SKIP LOCKED` claim, scan window `min_age` 60 s to `max_age` 24 h), queries the PSP through the same account-scoped registry as the webhook path (`query_provider_payment_intent` / `query_provider_refund`), and re-enters **the same notify processing framework** with a synthetic event:

```text
query:{provider}:{out_trade_no}:{mapped_status}        (payment)
query:{provider}:{out_trade_no}:{refund_no}:{mapped_status}  (refund)
```

The synthetic event id makes the query path idempotent exactly like webhook redelivery: the ingest de-duplicates on `(tenant_id, provider_scoped_event_id)`, the status machine preserves terminal states, and fulfillment keys suppress duplicate settlement — a webhook that already settled the order turns the worker's query into a replayed ingest with no double success. Trades still `pending` at the PSP are left untouched and re-claimed on the next pass; trades unknown to the PSP are logged and retried within the scan window.

## 7. Existing Fulfillment

Points recharge fulfillment currently uses a three-phase saga:

```text
fulfillment_status = processing
  -> idempotent Account wallet credit
  -> local fulfilled commit
```

Commit failure triggers compensation debit and reservation release. Membership-subject orders load the immutable action/order-number/package snapshot from Order storage and call `MembershipPurchaseFulfillmentPort` after payment confirmation. Membership performs reserve-and-activate atomically; Payment remains unaware of Membership fields, and replay does not duplicate periods or entitlements.

Order detail projections cap line items at 500 rows per request (`MAX_ORDER_LINE_ITEMS`) to avoid unbounded memory use. Missing `commerce_*` read-model tables surface as storage errors in production. Local scaffolding may set `ORDER_READ_MODEL_LENIENT=1` to return empty pages when tables are absent; this is not allowed for production.

List/search endpoints reject invalid `page` or `page_size` with HTTP 400 (`ProblemDetail`) instead of silently clamping. Validation is centralized in `sdkwork-utils-rust::validated_offset_list_params` and `sdkwork-order-service::validation::offset_list_params`.

## 8. PC Surface

| Path | Package | SDK |
| --- | --- | --- |
| `/app/order` | `@sdkwork/order-pc-order` | `@sdkwork/order-app-sdk` |
| `/admin/orders` | `@sdkwork/order-pc-admin-orders` | `@sdkwork/order-backend-sdk` |

```text
apps/sdkwork-order-pc/
  packages/sdkwork-order-pc-core/
  packages/sdkwork-order-pc-shell/
  packages/sdkwork-order-pc-order/
  packages/sdkwork-order-pc-admin-orders/
```

Wallet recharge, refund, and withdrawal UI surfaces must delegate to order SDK resources or host navigation ports. They must not call payment or account mutation APIs directly.

## 9. Runtime Configuration

| Variable | Purpose | Default |
| --- | --- | --- |
| `ORDER_API_BIND` | Gateway listen address | `0.0.0.0:18093` |
| `SDKWORK_CORS_ALLOWED_ORIGINS` | Comma-separated browser origins (canonical shared key) | empty (same-origin only) |
| `SDKWORK_ORDER_PLATFORM_CATALOG_TENANT_ID` | Tenant id for public recharge package catalog fallback | `100001` |
| `SDKWORK_ACCESS_TOKEN` | Bearer token for service-to-service wallet credit and membership fulfillment during order settlement | required in production |
| `ORDER_PAYMENT_WEBHOOK_BASE_URL` | Public base URL registered with PSP for order-owned webhooks | required in production |
| `SDKWORK_ORDER_PAYMENT_COMPENSATION_WORKER_ENABLED` | Enable the payment/refund compensation worker (webhook-failure safety net) | disabled |
| `SDKWORK_ORDER_PAYMENT_COMPENSATION_TENANT_ID` | Scan scope: tenant id filter for claimed attempts/refunds | unset (all) |
| `SDKWORK_ORDER_PAYMENT_COMPENSATION_ORGANIZATION_ID` | Scan scope: organization id filter | unset |
| `SDKWORK_ORDER_PAYMENT_COMPENSATION_BATCH_SIZE` | Max attempts + refunds claimed per pass | `50` (clamped 1..1000) |
| `SDKWORK_ORDER_PAYMENT_COMPENSATION_INTERVAL_MILLIS` | Worker tick interval | `30000` (clamped 5000..3600000) |
| `SDKWORK_ORDER_PAYMENT_COMPENSATION_MIN_AGE_SECONDS` | Attempts younger than this are not claimed | `60` |
| `SDKWORK_ORDER_PAYMENT_COMPENSATION_MAX_AGE_SECONDS` | Attempts older than this are never claimed (bounds PSP query load) | `86400` |
| `SDKWORK_DATABASE_TEST_POSTGRES_URL` | PostgreSQL URL for repository parity tests | unset |
| `RUST_LOG` | Tracing filter (`order.bootstrap`, `order.runtime`, `order.readiness`, `order.security`) | `info` |

## 10. Observability

The standalone gateway mounts `/healthz`, `/livez`, `/readyz`, and `/metrics` via `sdkwork-web-bootstrap::service_router`. Contract fallback merges app-api and backend-api `HttpRouteManifest` entries through `sdkwork-api-order-assembly::order_contract_fallback_config`.

Structured tracing uses targets `order.bootstrap`, `order.runtime`, `order.readiness`, and `order.security`. API handlers propagate `traceId` through `SdkWorkApiResponse` and `ProblemDetail`. Readiness probes database connectivity via `SELECT 1`.

## 11. Verification

```powershell
cd E:\sdkwork-space\sdkwork-order
cargo test --workspace
pnpm install
pnpm verify
pnpm test:postgres
pnpm test:postgres:required
```

Before completing API, SDK, pagination, or frontend integration work, run the SDKWork validators from `../sdkwork-specs/tools`.

## 12. Related Docs

- Account value order spec: `specs/ACCOUNT_VALUE_ORDER_SPEC.md`
- Checkout and payment topology: `docs/architecture/commerce/COMMERCE_CHECKOUT_ARCHITECTURE.md`
- Commerce repository dissolution: `../../sdkwork-specs/MIGRATION_SPEC.md` section 8
- Recharge machine contract: `specs/commerce-recharge.spec.json`
- Checkout topology contract: `specs/commerce-checkout-topology.spec.json`
- Product: `docs/product/prd/PRD.md`
- Production operations: `docs/guides/operations/PRODUCTION.md`
