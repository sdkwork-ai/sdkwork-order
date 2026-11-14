# Integrator Guide

Status: active  
Updated: 2026-07-07

## SDK Packages

| Surface | Consumer package | OpenAPI discovery |
| --- | --- | --- |
| App API | `@sdkwork/order-app-sdk` | `GET /app/v3/api/openapi.json` |
| Backend API | `@sdkwork/order-backend-sdk` | `GET /backend/v3/api/openapi.json` |

Generate or refresh SDKs from the repository root:

```bash
pnpm sdk:generate
pnpm sdk:generate:backend
```

## Authentication

- App routes require IAM session context (`AuthToken` / `Access-Token` per OpenAPI).
- Backend routes require organization-scoped IAM with permissions such as `commerce.orders.read` and `commerce.orders.manage`.
- Points-recharge fulfillment credits the **points** account ledger (not the cash wallet or Token Bank) through `SDKWORK_ACCESS_TOKEN` (Bearer) unless `SDKWORK_ORDER_ACCOUNT_CREDIT_ALLOW_INSECURE=1` is set for local development only. Token Bank recharges use the account value ledger port instead.

## Response Envelope

All business responses use `SdkWorkApiResponse` (`code: 0`, `data`, `traceId`). Errors use HTTP 4xx/5xx with `application/problem+json` (`ProblemDetail`).

List endpoints accept `page` and `page_size` (default 20, max 200). Invalid pagination returns HTTP 400; values are not silently clamped. Nested lists (`orders/{orderId}/events`, `orders/{orderId}/payments`, `shipments/{shipmentId}/packages`, `shipments/{shipmentId}/tracking_events`, `recharges/packages`) use the same pagination contract. Order-scoped payment history is available at `GET /app/v3/api/orders/{orderId}/payments`. After-sales return shipments list at `GET /app/v3/api/after_sales/requests/{afterSalesRequestId}/return_shipments`.

Cancel commands (`orders.cancellations.create`, `recharges.orders.cancel`) return `SdkWorkCommandResponse` with `data.accepted: true`.

## Write Command Headers

All idempotent write operations (checkout, order create/pay/cancel, recharge submit/cancel, after-sales, backend cancel/close/review/shipment) use:

| Header | Purpose |
| --- | --- |
| `Idempotency-Key` | Unique per logical command attempt (UUID recommended) |

OpenAPI marks these operations with `x-sdkwork-idempotent: true`. Generated SDKs expose only `idempotencyKey`; the service computes a canonical request fingerprint inside the authenticated tenant/principal and method/path scope. A replay with the same key and command returns the original/current result, while the same key with a different command returns HTTP 409.

Replays against terminal order states (`cancelled`, `closed`) return success without duplicate audit rows.

## Platform Recharge Catalog

When a tenant has no scoped recharge packages, public catalog queries fall back to the platform tenant configured by `SDKWORK_ORDER_PLATFORM_CATALOG_TENANT_ID` (default `100001`). Set this before production if the platform catalog tenant differs.

## Checkout and Payment

Full topology: `docs/architecture/commerce/COMMERCE_CHECKOUT_ARCHITECTURE.md`.

| Flow | Order operations | Payment |
| --- | --- | --- |
| Physical product checkout | `checkout.sessions.create` → `checkout.sessions.quotes.create` → `checkout.sessions.orders.create` → `orders.payments.create` | Use the authoritative quote, then open `paymentParams.cashierUrl`; Order reserves inventory at order placement |
| Points recharge | `recharges.orders.create` → `orders.payments.create` | Same; settlement via order webhook + in-process saga |
| Membership purchase | `memberships.orders.create` → `orders.payments.create` | Same; settlement activates subscription via membership fulfillment port |

PSP notify URL (production): `POST /app/v3/api/orders/payments/webhooks/{providerCode}` on the **order gateway**.

Operator settlement replay: `POST /backend/v3/api/orders/{orderId}/payment_confirmations` (permission `commerce.orders.fulfill`).

The reusable composed frontend service is `createSdkworkPhysicalPurchaseService` from `@sdkwork/order-service`. `prepareCheckout` validates client input and calls the generated checkout SDK for session plus quote; `placeOrder` submits the quoted session. Buyer identity comes from the authenticated server context, not a request-body owner field. The current physical-order contract accepts one merchant and one store per order.

Webhook settlement carries the exact payment attempt identity through Payment and Order. The order-only operator replay fails with `409` when multiple attempts make the target ambiguous; replay the exact Payment webhook event for that case. A successful late payment preserves a terminal Order status and records the Order-owned late-payment audit event once.

Cashier URL is returned in `orders.payments.create` outcome `paymentParams.cashierUrl`. Configure base URL with `SDKWORK_COMMERCE_CASHIER_BASE_URL` (default `https://platform.sdkwork.com`).

**PC:** `sdkwork-order-pc` buyer surface at `/app/order`; operator admin at `/admin/orders` (`@sdkwork/order-backend-sdk`, permissions `commerce.orders.read` / `commerce.orders.manage`).  
**H5 / Flutter:** consume `@sdkwork/order-app-sdk` and `@sdkwork/payment-app-sdk` from host shells; navigate to `cashierUrl` after pay.

## Local Standalone Gateway

```bash
pnpm start
# listens on ORDER_API_BIND (default 0.0.0.0:18093)
```

Set `SDKWORK_CORS_ALLOWED_ORIGINS` for browser clients. Readiness: platform health routes from `sdkwork-web-bootstrap` plus database `SELECT 1` probe.

## Verification

```bash
cargo test --workspace
pnpm verify
pnpm test:postgres          # optional when SDKWORK_DATABASE_TEST_POSTGRES_URL is set
pnpm test:postgres:required # CI: fails when postgres URL is missing
```

`pnpm verify` now includes `check:governance` (API envelope, pagination, SDK consumer imports).

Authority: `docs/architecture/tech/TECH_ARCHITECTURE.md`, `docs/architecture/commerce/COMMERCE_CHECKOUT_ARCHITECTURE.md`, `specs/RECHARGE_ORDER_SPEC.md`.
