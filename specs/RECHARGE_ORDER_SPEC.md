# Order Recharge & Unified Order Spec

Status: active  
Owner: SDKWork maintainers  
Capability: `commerce.order`  
Updated: 2026-07-05

Authority: `sdkwork-specs/RPC_SPEC.md` (`OrderService`, `RechargeService`), `sdkwork-specs/API_SPEC.md`

## 1. Purpose

**sdkwork-order** owns all `commerce_order` headers, including account value recharges, for a single order center: list, detail, cancel, refund eligibility, and pay orchestration.

The canonical subject → asset → ledger mapping is owned by [ACCOUNT_VALUE_ORDER_SPEC.md](./ACCOUNT_VALUE_ORDER_SPEC.md) §2.1. Recharge subjects are `token_bank_recharge`, `token_bank_plan_purchase`, `token_bank_plan_renewal`, `account_recharge_package`, `coupon_recharge`, and the legacy `points_recharge` (which credits the **points** asset, not Token Bank).

## 2. Owned domain

| Asset | Owner |
| --- | --- |
| `commerce_order` | Order (all subjects) |
| `commerce_order_item` | Order |
| `commerce_order_amount_breakdown` | Order |
| `commerce_recharge_package` | Order (target); migrate from Payment |
| Checkout session (product) | Order |
| Fulfillment / shipment (physical) | Order |

### Order subjects (extensible)

| `subject` | Scenario | Fulfillment |
| --- | --- | --- |
| `product` | Physical / general goods | Shipment |
| `virtual_goods` | Virtual SKU | Account / entitlement |
| `token_bank_recharge` | Token Bank compute-credit top-up | Account value ledger `token_bank` |
| `token_bank_plan_purchase` / `token_bank_plan_renewal` | Token Bank plan cycle | Account value ledger `token_bank` |
| `account_recharge_package` | Admin-defined value package | Account value ledger (package target asset) |
| `coupon_recharge` | Redemption code | Coupon benefit asset |
| `points_recharge` | Legacy **Membership Points** top-up | Account `points` ledger (`points_recharge`) |
| `membership` | Member purchase | Membership service |
| `refund_request` / `cash_withdrawal` | Refund / payout request | Refund & withdrawal request flows |

## 3. Recharge API (app-api)

OperationIds per `RPC_SPEC.md` — routes owned by **order** repository:

| Operation | operationId | Notes |
| --- | --- | --- |
| List packages | `recharges.packages.list` | `GET .../recharges/packages` |
| Settings | `recharges.settings.retrieve` | Exchange preview |
| Create recharge order | `recharges.orders.create` | Writes `commerce_order`; defaults to the order-bound mobile H5 cashier |
| Retrieve recharge order | `recharges.orders.retrieve` | Same as order detail |
| List recharge orders | `recharges.orders.list` | Filter on unified `orders.list` |
| Cancel | `recharges.orders.cancel` | Recharge-specific cancellation command |
| Create payment | `orders.payments.create` | Orchestrates Payment intent for `orderId` |

Backend admin: `RechargeAdminService` (package publish) on order-backend-api.

## 4. Pay & fulfill flow

```text
1. recharges.orders.create (`mobile_cashier_h5`) → commerce_order (pending_payment) + order-bound cashierUrl
2. Desktop QR encodes cashierUrl; the mobile H5 cashier selects and starts the provider payment
3. Explicit `wechat_native` / `alipay_native` → recharge channel creates the provider intent directly
4. PSP webhook              → Order app-api POST .../orders/payments/webhooks/{provider}
5. Order → Payment port     → ingest webhook, mark attempt succeeded (in-process)
6. Order settlement         → settle_owner_order_after_payment_success (in-process saga)
7. Order                    → fulfillment_status = fulfilled; account ledger credit
```

Production path: **webhook ingestion** on the **order gateway** (`POST /app/v3/api/orders/payments/webhooks/{providerCode}`) verifies the PSP signature, calls the payment repository port, then runs settlement in-process when status maps to `succeeded`. **Manual reconciliation** uses `POST /backend/v3/api/orders/{orderId}/payment_confirmations` (permission `commerce.orders.fulfill`): it resolves one exact payment attempt, queries the original PSP account, validates provider status, merchant order number, amount, and currency, and only then enters the same `settle_owner_order_after_payment_success` flow used by webhooks.

Provider query I/O runs before database mutation. The returned exact payment-attempt identity is then confirmed with conditional writes inside the Payment repository transaction. A verified webhook success already persisted for that attempt is replayed without another provider call. Settlement and subject fulfillment remain idempotent, so either notification delivery or reconciliation converges on the same success processing path without duplicate credits or fulfillment.

Saga entrypoints (order-service):

- `settle_owner_order_after_payment_success` — orchestrates confirm + subject fulfillment
- `mark_points_recharge_payment_succeeded` — order-side payment success markers
- `fulfill_points_recharge_order` — credits account via `AccountPointsCreditPort`, then commits order fulfillment

Idempotency keys:

- Payment success: `points-recharge:payment-success:{orderId}`
- Fulfillment credit: `points-recharge:fulfill:{orderId}`

Account ledger `business_type`: `points_recharge`

Order **must not** call payment provider SDK directly; use Payment service/repository boundary.

## 5. Dependencies

| Direction | Target | Allowed |
| --- | --- | --- |
| Order → Payment | `orders.payments.create`, webhook ingest, confirm payment ports | Yes |
| Order → Account | backend-api adjustments | Yes (saga) |
| Payment → Order | HTTP or service dependency | **No** |
| Order → Account tables | direct SQL | **No** |

## 6. SDK

| Artifact | Content |
| --- | --- |
| `@sdkwork/order-app-sdk` | Generated owner-only `recharges.*` and `orders.*` resources |
| `@sdkwork/order-service` | Injected facade typed directly from the generated app SDK; no account methods |

## 7. Create & pay boundary (O5 complete)

`recharges.orders.create` always writes the **order domain first** (`commerce_order`, items, amount breakdown). Its default `paymentProduct` is `mobile_cashier_h5`: this path returns the order-bound `cashierUrl`, sets `qrCodePayload` to that same URL, and does not create a provider payment intent/attempt. The mobile cashier creates the provider payment after the user selects a channel. Explicit `wechat_native` and `alipay_native` products remain channel-routed and create the provider intent through `PayOwnerOrderCommand`. Account `commerce_billing_history` is **not** written at create; ledger credit runs in the fulfillment saga (`AccountPointsCreditPort`).

Points grant metadata is stored on `commerce_order_item.sku_snapshot_json` and copied into payment attempt `callback_payload` when pay orchestrates.

### Account-value subjects (Token Bank / plans)

`token_bank_recharge`, `token_bank_plan_purchase`, and `token_bank_plan_renewal` share the same create-first boundary but are **server-priced**:

- `paymentProduct = mobile_cashier_h5` returns the order-bound cashier deep-link (`cashierUrl` / `qrCodePayload`, `nextAction = cashier`) and starts **no** provider payment; `wechat_native` / `alipay_native` still create the provider intent.
- The grant is never taken from the client: plan subjects price from the immutable plan snapshot (`grant_amount + bonus_amount`, micro-credits) and also override the payable amount/currency with the catalog values; `token_bank_recharge` prices from the exchange rule plus the package bonus. A request that sends `grantAmount` for these subjects is rejected.
- Credits are written to the `token_bank` ledger asset (micro-credits, 1 credit = 1e6 micro). `points_recharge` remains the membership-points path and is never used as a fallback for Token Bank.

Recharge SQL, routes, and lifecycle ownership belong exclusively to **sdkwork-order**. Clients call order app-api directly.

## 10. Gateway closure (O6 complete)

| Component | Route / env | Role |
| --- | --- | --- |
| PSP webhook | `POST .../orders/payments/webhooks/{providerCode}` | Order-owned public route; verify + ingest + settle |
| Manual reconciliation | `POST .../orders/{orderId}/payment_confirmations` | Query PSP, verify the exact payment, then replay the shared settlement saga |
| Webhook base URL | `ORDER_PAYMENT_WEBHOOK_BASE_URL` | `{base}/app/v3/api/orders/payments/webhooks/{providerCode}` |
| Account credit (store, default) | `SDKWORK_ORDER_ACCOUNT_LEDGER_ADAPTER` unset | In-process ledger on the host PostgreSQL pool (`MODULE_SPEC.md` §4.2) |
| Account credit (HTTP, opt-in) | `SDKWORK_ORDER_ACCOUNT_LEDGER_ADAPTER=http`, `SDKWORK_ACCOUNT_BACKEND_API_ORIGIN`, `SDKWORK_ACCESS_TOKEN` | Split deployment only: `POST .../wallet/adjustments/points`; fails at startup without a token |

Permissions: `commerce.orders.fulfill` (order backend payment_confirmations).

Recharge **app-api** responses use `SdkWorkApiResponse` (`code: 0`, `data.item` / `data.items` + `pageInfo`, `traceId`). Errors use `ProblemDetail` with numeric `code`.

All order **app-api** routers (`orders`, `recharges`, `checkout`, `fulfillments`, `shipments`, `after_sales`) and order **backend** admin routes use the same v3 envelope via `sdkwork-utils-rust` (`SdkWorkApiResponse`, `SdkWorkProblemDetail`).

OpenAPI authority and generated `@sdkwork/order-app-sdk` use `SdkWorkApiResponse`; `pnpm api:materialize` and `pnpm sdk:generate` materialize the owner-only SDK family.

Track phases in [commerce-recharge.spec.json](./commerce-recharge.spec.json).

## 8. PC

| Package | Role |
| --- | --- |
| `@sdkwork/order-pc-order` | Unified order list, order detail, admin |
| `@sdkwork/order-pc-recharge` | Reusable account value package selector, recharge order creation, QR payment, and status polling dialog (used by both Membership Points and Token Bank surfaces) |
| `@sdkwork/account-pc-wallet` | May consume order SDK for recharge UI (composition) |

## 9. Verification

- **Fulfillment saga**: `cargo test -p sdkwork-order-service points_recharge_fulfillment_standard` — payment success markers, deterministic idempotency keys, credit-then-commit ordering, and compensation
- **Store round-trip**: `cargo test -p sdkwork-order-repository-sqlx points_recharge_order_postgres` — create, pay, and fulfill a `points_recharge` order against PostgreSQL
- **Cancel audit** (`cargo test -p sdkwork-order-repository-sqlx cancel_owner_order`): PostgreSQL via `SDKWORK_DATABASE_TEST_POSTGRES_URL` (server test profile must be PostgreSQL per DATABASE_SPEC)
- **Label contract**: `pnpm --filter @sdkwork/order-pc-admin-trade test` — every subject/asset code resolves to a localized label (§2.2)
- **Account-value cashier link**: `cargo test -p sdkwork-order-repository-sqlx --lib postgres_account_value::tests` — recharge outcomes carry `cashierUrl`/`qrCodePayload`, coupon outcomes keep `fulfill`
- **Server pricing**: `cargo test -p sdkwork-routes-order-app-api --lib token_bank_subjects_are_server_priced` — Token Bank subjects are priced server-side, points/package/coupon are not
- **Client routing**: `pnpm exec vitest run apps/sdkwork-order-common/packages/sdkwork-order-service/tests/points-recharge-service.test.ts apps/sdkwork-order-h5/packages/sdkwork-order-h5-subscription/tests/subscription-purchase-service.test.ts` — Token Bank services send `token_bank_*` and never a client `grantAmount`
- OpenAPI parity for `recharges` tag paths; `pnpm align:openapi` + `pnpm sdk:generate` for order-app-sdk
- `pnpm verify` and `cargo test --workspace` in order and account repositories
