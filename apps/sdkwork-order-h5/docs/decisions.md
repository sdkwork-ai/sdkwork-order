# H5 Decisions

- 2026-10-03: v1 ships the browser runtime only (no Capacitor host package
  yet). Adoption follows APP_H5_ARCHITECTURE_SPEC §8.2 as a follow-up.
- 2026-10-03: all-routes-auth. Every order H5 route requires a session;
  `resolveSdkworkOrderH5AuthGateDecision` redirects anonymous visitors from
  any product route to `/auth/login?redirect=<sanitized-return-path>` instead
  of the mall template's public-storefront + per-route gating. The order
  surface has no public pages, so a single enforcement point in `AuthGate`
  replaces per-route `RequireSession` wrappers.
- 2026-10-03: account combined balances are NOT injected yet. The im-h5 host
  passes `getBalance` / `getCashBalance` from its portfolio service; this root
  has no account-capability federation (`sdkwork-account` portfolio client)
  composed in bootstrap, so `TokenBankPurchasePage.getBalance` and
  `WithdrawPage.getCashBalance` stay unset and the pages degrade to their
  built-in empty/placeholder states. Wiring waits on the account capability
  federation (账户组合余额待账户能力联邦接入).
- 2026-10-03: transport goes through the generated composed SDK facade
  `@sdkwork/order-app-sdk` only. No federated CloudRouter commerce transport
  is consumed, and no backend SDK clients are constructed (app/user surface).
- 2026-10-03: WeChat in-app JSAPI payment stays unavailable: no
  `wechatPaymentOAuth` channel is composed into
  `configureOrderMobileRuntime` until the IAM payment OAuth endpoint is wired
  for this root. Browser/WeChat-H5 cashier methods keep working.
- 2026-10-03: cashier deployment region is pinned to `cn` (`paymentRegion`):
  the order backend accepts the CN wire methods only today, so the payer
  language fallback must not expose paypal/stripe channels that would fail
  server-side.
- 2026-10-03: workspace registration. Root `pnpm-workspace.yaml` is materialized
  by `sdkwork-specs/tools/sync-workspace.mjs` from
  `sdkwork-specs/tools/lib/workspace-registry.mjs` (`FOUNDATION_PNPM_PACKAGES`)
  plus the per-repo overlay `sdkwork-specs/workspace/consumers/sdkwork-order.json`.
  The sync preserves LOCAL (non-`../`) package entries verbatim and regenerates
  only the `../sdkwork-*` sibling entries, so no registry change is required:
  the existing local globs `apps/sdkwork-order-h5` and
  `apps/sdkwork-order-h5/packages/*` already cover the new application root and
  the new `packages/sdkwork-order-h5-shell` package. Do not hand-edit the
  `../sdkwork-*` block; sibling additions belong in the specs consumer overlay.
- 2026-10-03: `tsconfig.app.json` drops `verbatimModuleSyntax` (mall-h5
  template deviation). This root compiles dependency workspace packages that
  ship raw TS sources through `exports` (`@sdkwork/ui-mobile-react`), and
  those sources are not type-only-import clean; the app cannot impose the
  flag on another repository's sources. The app's own sources keep
  `import type` discipline.
- 2026-10-03: the authGateLogic decision-table unit test lives under
  `packages/sdkwork-order-h5-shell/tests/` instead of an app-root `tests/`
  directory because the repository-root `vitest.config.ts` (not modifiable
  from this task) discovers `apps/sdkwork-order-h5/packages/**/*.test.ts(x)`
  only; placing the test there makes it run in the standard `pnpm test:vitest`
  gate.
