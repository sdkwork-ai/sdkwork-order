# Order PRD

Status: active  
Owner: SDKWork maintainers  
Application: order  
Updated: 2026-07-26
Specs: REQUIREMENTS_SPEC.md, DOCUMENTATION_SPEC.md

## Document Map

- Commerce repository dissolution: `../sdkwork-specs/MIGRATION_SPEC.md` section 8
- Technical architecture: `docs/architecture/tech/TECH_ARCHITECTURE.md`
- Account value order boundary: `specs/ACCOUNT_VALUE_ORDER_SPEC.md`
- Recharge machine contract: `specs/commerce-recharge.spec.json`
- Checkout topology contract: `specs/commerce-checkout-topology.spec.json`

## 1. Background And Problem

Order, checkout, fulfillment, shipment, after-sales, and account-value lifecycles must scale independently from payment, account, promotion, catalog, and membership capabilities.

`sdkwork-order` is a T1 commerce capability building block: domain services, SQL repositories, HTTP routers, standalone gateway, TypeScript SDKs (`@sdkwork/order-app-sdk`, `@sdkwork/order-backend-sdk`), and the PC client surface.

For account value movement, `sdkwork-order` is the business orchestration owner. Every recharge, Token Bank plan purchase or renewal, package recharge, coupon redemption, refund request, and withdrawal request must have order-owned commercial evidence before payment, payout, coupon, or account ledger side effects are executed.

## 2. Target Users

Buyers, Token Bank consumers, merchant operators, finance operators, fulfillment staff, and commerce integrators.

## 3. Goals And Non-Goals

### Goals

- Own order lifecycle domain, SQL repositories, and app/backend HTTP routers.
- Expose checkout, fulfillment, shipment, after-sales, recharge, refund request, and withdrawal request routes with `SdkWorkApiResponse` envelopes.
- Own account value order subjects for `points_recharge`, `token_bank_recharge`, `token_bank_plan_purchase`, `token_bank_plan_renewal`, `account_recharge_package`, `coupon_recharge`, `refund_request`, and `cash_withdrawal`.
- Keep Token Bank naming unambiguous: product copy uses "Token Bank"; account asset code uses `token_bank`.
- Publish OpenAPI authorities at `/app/v3/api/openapi.json` and `/backend/v3/api/openapi.json`.

### Non-Goals

- Payment intent, payment attempt, refund, payout, provider channel, or provider webhook SQL ownership. Those belong to `sdkwork-payment`.
- Provider payment, refund, or payout execution. Order calls payment executor ports.
- Account ledger SQL ownership, balance calculation, holds, settlement, release, or reversal implementation. Those belong to `sdkwork-account`.
- Coupon inventory and promotion rule ownership. Order consumes coupon validation and redemption through a port.
- IAM middleware implementation inside capability routers. Gateway and IAM integration own that layer.

## 4. Scope

- Order create/list/detail/cancel flows with lifecycle audit (`commerce_order_event`, `commerce_order_cancellation`).
- Checkout session lifecycle.
- Fulfillment, shipment tracking, and after-sales requests.
- Account value orders: points recharge, Token Bank direct recharge, Token Bank plan purchase and renewal, account recharge packages, coupon recharge, refund requests, and cash withdrawal requests.
- Existing points recharge checkout and backend fulfillment saga: reserve -> account credit -> commit, with debit compensation on commit failure.
- Account value payment-settlement fulfillment: Token Bank recharge, Token Bank plan purchase and renewal, account recharge package, and coupon recharge use the order-owned settlement saga, `AccountValueFulfillmentStore`, and `AccountValueLedgerPort` to credit the target account asset after payment or coupon validation evidence exists.
- Refund orchestration execution: backend review holds the granted account value through account, executes provider refund through payment, and settles or releases the account hold by outcome.
- Withdrawal orchestration execution: backend review holds cash through account and reaches the `PaymentPayoutExecutorPort` boundary; production payout remains fail-closed until `sdkwork-payment` exposes a concrete provider payout executor contract.
- SQL repositories for `commerce_*` order tables with PostgreSQL as the authoritative server engine (DATABASE_SPEC).
- Payment settlement for subjects without order-owned fulfillment confirms payment only; fulfillment is owned by external commerce capabilities (`awaiting_external_fulfillment`).

Primary API prefixes:

- App: `/app/v3/api/orders`, `/app/v3/api/recharges`, `/app/v3/api/checkout`, `/app/v3/api/memberships`
- Backend: `/backend/v3/api/orders`

## 5. User Scenarios

- A buyer creates a checkout session, places an order, pays via the cashier, and receives fulfillment after the order gateway processes the PSP webhook.
- A user purchases Token Bank value through a one-time recharge, package, or plan; order snapshots the commercial facts, payment collects money, and account credits `token_bank`.
- A user redeems a coupon for account value; order records coupon evidence, skips provider payment for zero-amount redemptions, and credits the target account asset through account.
- A user or operator creates a refund request; order holds the account value first, calls payment with the original provider/account/transaction snapshot, retains the hold while the refund is submitted, processing, or ambiguous, commits only after provider-confirmed success, and releases only after deterministic failure.
- A user creates a cash withdrawal request; order holds cash in account and reaches the payout executor boundary. With the current default runtime, payout is fail-closed, the hold is released on executor failure, and no real provider payout is claimed until `sdkwork-payment` publishes a concrete payout executor.
- An operator lists orders, inspects lifecycle events, and cancels or closes orders through the backend API.
- An operator replays stuck payment settlement via `POST /backend/v3/api/orders/{orderId}/payment_confirmations` with permission `commerce.orders.fulfill`.

## 6. Success Metrics

- `cargo test --workspace` and `pnpm verify` pass.
- Governance checks pass: API envelope, pagination, SDK consumer imports, repo composition, and documentation standard.
- PostgreSQL lifecycle tests pass for critical lifecycles; when `SDKWORK_DATABASE_TEST_POSTGRES_URL` is unset, server e2e tests skip with a notice.
- Account value order specs, OpenAPI contracts, SDK facades, domain sagas, and database table registry stay aligned.

## 7. Phases

- Phase 1 (complete): SQL and app routers owned in `sdkwork-order`.
- Phase 2 (complete): Payment side effects owned by payment repository; order validates via one-way dependency.
- Phase 3 (complete): Backend admin API, backend SDK, OpenAPI discovery, cancel/close audit, strict SQL list pagination, v3 SDK envelope on PC order center.
- Phase 4 (complete): Operator UI (`/admin/orders`), `HttpRouteManifest`, gateway contract fallback, CI verify with Postgres parity, production operations guide, OpenAPI static contract tests.
- Phase 5 (complete): Order-owned PSP webhooks, in-process payment settlement, and `payment_confirmations` backend replay.
- Phase 6 (complete): Backend after-sales management (`afterSales.management.*`, `afterSales.reviews.create`), shipment management (`shipments.list`, `shipments.packages.*`), aligned service contract, table registry, and operator PC filters.
- Phase 7 (complete): Payment-before-cancel/close orchestration, server-owned idempotency fingerprints, recharge cancel idempotency parity, SQL unique-key to HTTP 409 mapping, canonical checkout-bound order creation, and OpenAPI authority-to-SDK synchronization.
- Phase 8 (complete): Membership order create (`memberships.orders.create`), immutable paid-order settlement snapshot via `MembershipPurchaseFulfillmentPort`, and `sdkwork-order-integration-membership` wired at `OrderServiceHost`. Membership atomically reserves and activates the paid period; no pre-payment pending subscription is required.
- Phase 9 (complete): Membership checkout business idempotency. Transport replay validates a server-owned request fingerprint; command timestamps are RFC3339 and expiration must be later than request creation after timezone normalization; active membership orders are unique by tenant, organization, owner, action, and catalog-price snapshot; only Orders with an explicit future expiration boundary are reusable, while missing or invalid boundaries fail closed; expired orders are closed before replacement; payment-method switching reuses the Order, closes the previous PSP attempt through its historical channel, and creates or reuses only the appropriate Payment attempt. Shared checkout retry refreshes an existing Order before issuing another create command.
- Phase 10 (active): Account value order expansion for Token Bank recharge, Token Bank plan purchase and renewal, account recharge packages, coupon recharge, refund request, and cash withdrawal. Points recharge creation returns an ISO 8601 UTC expiration boundary, reuses only a checkout with an explicit future expiry, atomically marks stale Orders expired, and creates a replacement after explicit customer retry. Payment-success settlement for paid Token Bank/package/coupon account-value orders is implemented through `AccountValueFulfillmentStore` and `AccountValueLedgerPort`. Refund approval execution is implemented through account holds and the payment refund executor; processing or ambiguous PSP outcomes retain the hold and original refund identity, confirmed success settles it, and deterministic failure releases it. Withdrawal approval uses the same account hold lifecycle and remains fail-closed at provider payout until payment publishes a concrete payout executor.
- Phase 11 (complete): Physical-goods checkout validates authoritative Merchandise and Shop state, snapshots buyer address and commercial facts, reserves inventory idempotently, consumes it after unified webhook/query payment confirmation, creates shipment fulfillment once, releases reservation on cancellation, and routes terminal-order late payments to controlled recovery. The initial commercial slice is one merchant and one store per order.
- Platform ingress (outside this repo): Redis-backed rate limiting at the mesh layer and Grafana dashboards per deployment topology.

## 8. Linked Requirements

- Component contract: `specs/component.spec.json`
- Account value order spec: `specs/ACCOUNT_VALUE_ORDER_SPEC.md`
- Machine contracts: `apis/`, `sdks/`, `specs/`
- Database table registry: `database/contract/table-registry.json`

## 9. Launch Readiness

Pre-launch verification gate:

```bash
cargo test --workspace
pnpm verify
pnpm test:postgres:required
```

Contract alignment is enforced by:

- OpenAPI to router mount tests (`app_openapi_routes`, `backend_openapi_routes`)
- HTTP manifest to OpenAPI method tests (`http_route_manifest` unit tests)
- Service contract to HTTP manifest test (`gateway-assembly` integration test)
- Static spec tests for account value order ownership and payment/account boundaries

## 10. Open Questions

None blocking the account value order architecture. Platform ingress rate-limit store and Grafana dashboards are owned by deployment topology, documented in `docs/guides/operations/PRODUCTION.md`.

- OpenAPI discovery serving (adjudicated 2026-10-03): the standalone gateway publishes the combined authority at `GET /openapi.json` (framework-hosted, live-verified). The per-prefix paths `/app/v3/api/openapi.json` and `/backend/v3/api/openapi.json` are 404-by-manifest-guard at runtime: the IAM web request-context resolver rejects any app-api/backend-api path absent from the route manifest, and both remediation paths are platform-level (declare the discovery operations in the authored authorities and regenerate every consumer SDK, or add a framework manifest exemption for contract endpoints). The cloudrouter reference implementation serves per-prefix paths only through stub routers. Runtime discovery stays on the root path until the platform picks one remediation.

## 11. Fulfillment Boundaries

| Subject | Automated fulfillment in order gateway |
| --- | --- |
| `points_recharge` | Complete - wallet credit via Account service after payment |
| `token_bank_recharge` | Implemented settlement path - Token Bank credit via Account service after payment |
| `token_bank_plan_purchase` | Implemented settlement path - first-cycle Token Bank grant through Account service after payment |
| `token_bank_plan_renewal` | Implemented settlement path - renewal Token Bank grant through Account service after provider renewal payment |
| `account_recharge_package` | Implemented settlement path - target account asset credit through Account service after payment |
| `coupon_recharge` | Implemented settlement path - coupon evidence plus target account asset credit; zero-amount orders skip provider payment |
| `refund_request` | Implemented review execution - account reversal hold, provider refund through Payment service, then hold settlement or release |
| `cash_withdrawal` | Implemented account hold lifecycle - provider payout boundary is `PaymentPayoutExecutorPort`, fail-closed by default until payment publishes payout execution |
| `membership` | Complete - atomic reserve-and-activate fulfillment via `MembershipPurchaseFulfillmentPort`, replay-safe by source Order |
| `product` | Complete for physical goods - authoritative Merchandise/Shop validation, immutable snapshots, inventory reserve/consume/release, and idempotent shipment fulfillment |
| `virtual_goods`, `coupon_package` | Planned/external - payment confirmed only; downstream fulfillment or promotion capabilities own delivery |

Operator replay for stuck payment settlement: `POST /backend/v3/api/orders/{orderId}/payment_confirmations`. Account value refund and withdrawal review/retry routes call payment/account executor ports and must never write provider or ledger state directly.
