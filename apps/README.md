# apps/

Application: order
Status: active
Owner: SDKWork maintainers
Specs: APPLICATION_SPEC.md, SDKWORK_WORKSPACE_SPEC.md

## Primary App Surface

The repository root is not the primary runnable app surface.
Runnable application roots live under `apps/<application-root>/`.

## Directory Index

| Directory | Surface role | Runnable | Purpose | Entry |
| --- | --- | --- | --- | --- |
| sdkwork-order-pc | pc | yes | SDKWork Order PC pc application root. | `sdkwork-order-pc/` |
| sdkwork-order-common | composed services | no (library) | Composed order service facade shared by PC/H5 apps (UI -> service -> SDK). | `sdkwork-order-common/` |
| sdkwork-order-h5 | h5 | yes | Mobile H5 application root: order center, cashier, account value (VIP/Token Bank/coupon), withdrawal wallet. | `sdkwork-order-h5/` |
| sdkwork-order-flutter-mobile | flutter-mobile | yes | Flutter mobile app: orders, cashier, recharge, coupon redemption, withdrawal, refund requests. | `sdkwork-order-flutter-mobile/` |
| sdkwork-order-mini-program | mini-program | yes | WeChat mini program storefront for order and account value flows. | `sdkwork-order-mini-program/` |

## Allowed Content

- Selected language/architecture application roots with `README.md`, `AGENTS.md`, `.sdkwork/`, and `specs/` when authored packages exist.
- Architecture-local `packages/`, `config/`, `src/`, `lib/`, `App/`, or `entry/` directories required by the owning architecture standard.

## Forbidden Content

- Repository-root API contracts, generated SDK workspaces, Rust crates, or deployment descriptors moved under `apps/`.
- Runtime secrets, user-private state, generated SDK transport output, or cross-application copied business logic.

## Related Specs

- `../sdkwork-specs/APPLICATION_SPEC.md`
- `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`
- `../sdkwork-specs/APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md`

## Verification

```bash
node ../sdkwork-specs/tools/check-apps-directory-index.mjs --root .
```
