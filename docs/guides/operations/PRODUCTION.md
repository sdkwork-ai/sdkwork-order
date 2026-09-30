# Production Operations

Status: active  
Updated: 2026-07-30

## Deployment Topology

`sdkwork-order` ships as a standalone Axum gateway (`sdkwork-api-order-standalone-gateway`) exposing:

| Surface | Prefix | Auth |
| --- | --- | --- |
| App API | `/app/v3/api/*` | IAM dual-token |
| Backend API | `/backend/v3/api/*` | IAM dual-token + org scope |
| Infra | `/healthz`, `/livez`, `/readyz`, `/metrics` | Public (metrics should be in-cluster only) |

Run multiple instances behind a load balancer. All instances share the same PostgreSQL database and external IAM/account/payment dependencies.

## Environment Variables

| Variable | Required | Notes |
| --- | --- | --- |
| `ORDER_API_BIND` | No | Default `0.0.0.0:18093` |
| `SDKWORK_CORS_ALLOWED_ORIGINS` | Production | Comma-separated browser origins (canonical shared key); unset denies CORS |
| `SDKWORK_ORDER_PLATFORM_CATALOG_TENANT_ID` | No | Platform recharge catalog tenant (default `100001`) |
| `SDKWORK_ACCESS_TOKEN` | Production | Bootstrap access credential for approved SDK-backed service integrations; never used as an ad hoc bearer secret |
| `ORDER_READ_MODEL_LENIENT` | No | **Forbidden in production.** Set `1` only for local scaffolding without commerce DDL |
| `ORDER_PAYMENT_WEBHOOK_BASE_URL` | Production | Public base URL for PSP notify: `{base}/app/v3/api/orders/payments/webhooks/{providerCode}` |
| `RUST_LOG` | No | e.g. `info,order.bootstrap=info,order.runtime=info` |

### Background workers (in-process)

The gateway spawns two worker loops at bootstrap; both are multi-replica safe
(claim transitions are transactional and idempotent):

| Variable | Default | Notes |
| --- | --- | --- |
| `SDKWORK_ORDER_EXPIRATION_SCHEDULER_ENABLED` | `true` | Set `0`/`false` to disable the order expiry sweep |
| `SDKWORK_ORDER_EXPIRATION_SCHEDULER_INTERVAL_SECONDS` | `60` | Clamped 10–3600 |
| `SDKWORK_ORDER_EXPIRATION_BATCH_SIZE` | `200` | Clamped 1–2000 |
| `SDKWORK_ORDER_PAYMENT_COMPENSATION_WORKER_ENABLED` | `false` | **Opt-in.** PSP query sweep for stuck attempts/refunds — enable in production after credentials are configured |
| `SDKWORK_ORDER_PAYMENT_COMPENSATION_INTERVAL_MILLIS` | `30000` | Clamped 5000–3600000 |
| `SDKWORK_ORDER_PAYMENT_COMPENSATION_BATCH_SIZE` | `50` | Clamped 1–1000 |
| `SDKWORK_ORDER_PAYMENT_COMPENSATION_MIN_AGE_SECONDS` | `60` | Keeps fresh attempts inside their webhook window |
| `SDKWORK_ORDER_PAYMENT_COMPENSATION_TENANT_ID` / `_ORGANIZATION_ID` | unset | Optional scan scope narrowing |

Worker loops stop deterministically during graceful shutdown (handles aborted
after the HTTP plane drains; passes are transactional and idempotent, so the
next run resumes cleanly).

## Payment Webhooks

PSP notify URLs **must** target the **order gateway**, not `sdkwork-payment`:

```text
POST {ORDER_PAYMENT_WEBHOOK_BASE_URL}/app/v3/api/orders/payments/webhooks/{providerCode}
```

The order-owned payment webhook is the only notify surface; the historical platform-level path `POST /app/v3/api/payments/webhooks/{providerCode}` is not mounted by this gateway (plain 404 from the standalone deployment). PSP notify URLs must always use the order gateway path above.

Operator manual settlement replay:

```text
POST /backend/v3/api/orders/{orderId}/payment_confirmations
```

Requires IAM permission `commerce.orders.fulfill`.

Duplicate PSP deliveries with the same `provider_event_id` are idempotent at the payment ingest layer (`replayed: true`) and may re-enter settlement for the exact payment attempt. Confirmation, Order state updates, and fulfillment are idempotent, so retrying the webhook does not duplicate effects. If notification delivery is missing or settlement still requires operator recovery, use `payment_confirmations`. It queries the original provider account and validates successful status, merchant order number, amount, and currency before entering the same success-processing function as the webhook. Provider I/O occurs before the short database confirmation transaction; the transaction conditionally updates the exact attempt and intent, and repeated calls read back the durable success. The endpoint returns a conflict when the provider is not successful or multiple attempts make order-only recovery ambiguous.

## Health And Observability

- **Liveness:** `GET /healthz`, `GET /livez`
- **Readiness:** `GET /readyz` (includes database `SELECT 1`)
- **Metrics:** `GET /metrics` (Prometheus text; scrape in-cluster only, do not expose on public ingress)
- **Tracing:** structured logs with targets `order.bootstrap`, `order.runtime`, `order.readiness`, `order.security`
- **Contract fallback:** manifest-declared routes without handlers return HTTP 501; unknown paths return HTTP 404 (merged app + backend manifests)

## High Availability

1. Run **N ≥ 2** gateway replicas with the same DB connection pool limits tuned per instance.
2. Use PostgreSQL with automated backups and point-in-time recovery.
3. Configure `SDKWORK_CORS_ALLOWED_ORIGINS` explicitly per environment.
4. Enable Redis-backed rate limiting at the platform gateway layer when `sdkwork-web-framework` production assembly requires it.
5. Points-recharge fulfillment is idempotent; payment callbacks may retry safely. Commit failure after wallet credit triggers automatic compensation debit and releases the `processing` reservation; operators may still replay via `payment_confirmations` if compensation fails.
6. Order cancellation and close (buyer or admin) close payment intents **before** mutating order status to avoid payable terminal orders.
7. Write commands use `Idempotency-Key`; the server persists canonical request fingerprints and rejects same-key/different-command replays. A late successful payment does not reopen a terminal order, and repeated settlement does not duplicate the `payment_succeeded_after_terminal` audit row.

## Verification Before Release

```bash
pnpm verify
pnpm test:postgres:required   # CI uses SDKWORK_DATABASE_TEST_POSTGRES_URL
```

Contract drift is guarded by automated tests:

- OpenAPI ↔ Axum router mount (`app_openapi_routes`, `backend_openapi_routes`)
- HTTP manifest ↔ OpenAPI methods (`http_route_manifest` unit tests)
- Service contract ↔ HTTP manifest (`gateway-assembly` integration test)

## PC Surfaces

Operator actions require IAM permissions documented below. Set `VITE_SDKWORK_ACCESS_TOKEN` (and optional `VITE_SDKWORK_AUTH_TOKEN`) for the standalone PC build; without a token the shell shows a configuration hint instead of failing on opaque SDK errors.

| Path | Surface | SDK |
| --- | --- | --- |
| `/app/order` | Buyer order center | `@sdkwork/order-app-sdk` |
| `/admin/orders` | Operator order admin | `@sdkwork/order-backend-sdk` |

## IAM Permissions (backend)

| Permission | Scope |
| --- | --- |
| `commerce.afterSales.read` | List/retrieve after-sales requests |
| `commerce.afterSales.review` | Review after-sales requests |
| `commerce.orders.read` | Orders, shipments, events |
| `commerce.orders.manage` | Cancel/close orders; shipment package write |
