# sdkwork-order-h5

H5 (mobile web) surface for SDKWork Order, per `APP_H5_ARCHITECTURE_SPEC.md`.
The root is a thin Vite React application shell: bootstrap, providers,
AuthGate wiring, route assembly, SDK client construction, IAM runtime wiring,
and runtime config selection. Business screens and services live in
`packages/`.

## Identity

- Application code: `order` (domain `commerce`, capability `order`)
- App API prefix: `/app/v3/api` through the generated `@sdkwork/order-app-sdk`
- Template: `sdkwork-mall/apps/sdkwork-mall-h5` (structure and import patterns);
  the auth shell mirrors `sdkwork-im/apps/sdkwork-im-h5` module assembly.

## Layer boundary (UI -> service -> SDK)

- UI packages (`@sdkwork/order-h5-subscription`, `@sdkwork/order-h5-withdraw`,
  `@sdkwork/order-mobile-react-orders`) never construct SDK clients or raw
  HTTP. They receive injected clients/services through props.
- `src/bootstrap/` owns the only SDK construction:
  `sdkClients.ts` + `iamRuntime.ts` build the token-manager-bound Order App SDK
  client, `orderRuntime.ts` calls `configureOrderMobileRuntime` and exposes the
  purchase/withdraw service resolvers injected into the pages.
- No backend SDK clients exist on this surface (app/user surface only).

## Surfaces

- `/orders`, `/orders/:orderId`, `/orders/:orderId/cashier`, `/orders/voucher`
  (order owner routes), `/value`, `/value/vip`, `/value/token-bank`,
  `/value/coupon` (account value), `/withdraw`. All routes are
  session-gated by `AuthGate` (all-routes-auth, see `docs/decisions.md`).

## Verification

```bash
pnpm --dir apps/sdkwork-order-h5 run typecheck
pnpm --dir apps/sdkwork-order-h5 run build:dev
pnpm --dir apps/sdkwork-order-h5 run build:dev:cloud
pnpm test:vitest
```
