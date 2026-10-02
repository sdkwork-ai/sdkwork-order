# Mini-program Decisions

- 2026-10-03: v1 ships the native WeChat MP root for the order capability
  (domain commerce): manifest, app shell, 3-tab surface (orders / recharge /
  profile), the full order page catalog (orders, order-detail, cashier,
  payment-result, recharge, coupon, withdraw, refund, login, profile) against
  `apis/app-api/order/order-app-api.openapi.json`. Architecture keeps the
  building-block rule: every domain lives in its own `src/services/*` module
  (order / recharge / withdrawal), all traffic funnels through the single
  `services/transport.js` seam (contract test + lint enforce exactly one
  `wx.request` call site), and `bootstrap/sdkClients.ts` composes the
  services into one typed facade. When the generated WeChat MP SDK family
  lands, each service swaps its transport calls for the generated client
  without page changes.
- Single transport seam: the MP runtime cannot execute the generated
  TypeScript transport that PC/H5 consume (`@sdkwork/order-app-sdk` needs
  fetch/AbortController semantics the MP runtime does not provide), so the
  spec-shaped hand-written services are the approved transitional wrapper.
  `Idempotency-Key` (fresh UUID per attempt) rides on every POST command,
  mirroring the generated SDK transport and the required write-command
  headers enforced by `sdkwork-routes-order-app-api` (`command_headers.rs`).
- Platform-token login transition: the IAM SDK family for MP runtimes is
  still pending, so the session holds a platform-issued bearer token in
  platform storage (`pages/login`). wx.login / code2session replaces it once
  the identity contract exists. Logout clears the storage key.
- Payment: the cashier creates the session through
  `POST /orders/{orderId}/payments` (whitelist wechat_pay / wechat_jsapi /
  alipay / alipay_wap / balance) and then polls
  `GET /orders/{orderId}/payment_success` with exponential backoff (3s → 8s
  cap) until the cashier TTL (15 min, or the order `expireTime`, whichever is
  earlier); at the deadline it takes one final
  `GET /orders/{orderId}/status` reading so webhook-settled orders still
  resolve. The real `wx.requestPayment` handoff plugs into
  `pages/cashier/index.js#launchProviderPayment` (backend `paymentParams`)
  when the WeChat pay provider contract for MP lands; until then settlement
  relies on the backend-side payment intent the create call already opened.
- Recharge: plans (Token Bank) and packages (points) both order through
  `POST /recharges/orders`; catalog `priceAmount` is a major-unit decimal and
  is converted to the minor-unit integer string the API requires via
  `planPriceToMinor` (same ×100 semantics as the H5
  `SubscriptionPurchaseService`, which previously undercharged decimal
  prices). Payment result for recharge orders polls
  `GET /recharges/orders/{orderId}` (mode=recharge on the result page).
- Subpackaging: all 10 pages register in the main package, no
  `subpackages` block. Page count and asset-free markup keep the main
  package far below WeChat limits; splitting now would trade navigation
  simplicity for nothing measurable. `bootstrap/routes.ts` carries
  placement metadata (`rootPackage: true` per route), so a future projection
  into `subpackages/<capability>` is a mechanical move without touching
  route ids.
- Withdrawal records: the order app-api exposes
  `POST /withdrawals/requests` and `GET /withdrawals/requests/{id}` but no
  owner list endpoint, so the withdraw page keeps a bounded device-local
  history (latest 20 request ids in platform storage) and refreshes each row
  through the server retrieve. This is a device view, not a server list;
  swap `withdrawal-service.listLocalHistory` for the server list once the
  backend ships `GET /withdrawals/requests`. Refunds do have a server list
  (`GET /orders/refund_requests`) and render it directly.
- Not in the pnpm workspace, zero npm dependencies: a native mini-program
  tree is built by the WeChat DevTools toolchain, not pnpm/vite/tsc. Keeping
  `package.json` dependency-free means `node scripts/*.mjs` and
  `node --test` are the only tooling, so the root never needs
  `pnpm install` for this app and the root `pnpm-workspace.yaml` stays
  untouched. `scripts/typecheck.mjs` uses the workspace TypeScript when
  resolvable and otherwise falls back to the Node strip-types syntax pass.
- Runtime environment: `src/runtime-env.json` is a build artifact
  (git-ignored) materialized by `scripts/build-runtime.mjs` from
  `config/mini-program/runtime-env.<profile>.<env>.json` (host-local real
  file wins, committed `.example` fallbacks build out of the box). Templates
  declare the `SDKWORK_*` keys required by
  `MINI_PROGRAM_APP_ARCHITECTURE_SPEC.md` §10 and are validated (environment/
  profile/target/base URL) before materialization.
- int64 / money: ids, order numbers, and `pageInfo.totalItems` stay strings
  end to end; amounts are minor-unit integer strings converted to display
  text via string math in `utils/format.js` (`formatMinor`), so values beyond
  `Number.MAX_SAFE_INTEGER` never round.
- Logistics tracking (buyer flow, aligned with the H5 物流追踪 surface): the
  order app-api exposes three read endpoints —
  `GET /fulfillments?order_id={orderId}&page=1&page_size=10` (snake_case
  `order_id` query per the OpenAPI parameter), `GET /shipments/{shipmentId}`,
  and `GET /shipments/{shipmentId}/tracking_events?page=1&page_size=50`.
  `shipment-service.getOrderShipment` composes them in that order: the
  fulfillment page first, then the first fulfillment row's shipment header,
  then its events; a missing/failed shipment or events read degrades to the
  preceding layer instead of failing the card. "无履约即隐藏": when the list
  resolves zero fulfillments the assembler resolves null and the order-detail
  page renders no 物流追踪 card (该订单暂无物流) — there is no empty-state
  card for logistics, matching the H5/Flutter alignment baseline. The page
  fetches logistics only while the order status is `fulfilled`/`completed`;
  after 确认收货 flips the status to `completed` the card re-renders from the
  fresh data while the receipt-confirmation button (fulfilled-only) follows
  the existing status-driven rendering. All reads stay on the single
  `services/transport.js` seam; the typed contract lives in
  `bootstrap/sdkClients.ts` (`SdkworkOrderMpShipmentService`).
