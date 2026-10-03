# sdkwork-order Component Specs

Local specification index for the order capability (unified `commerce_order`, including account value orders).

## Spec map

| Document | Purpose |
| --- | --- |
| [component.spec.json](./component.spec.json) | Workspace manifest |
| [ACCOUNT_VALUE_ORDER_SPEC.md](./ACCOUNT_VALUE_ORDER_SPEC.md) | Order-owned Token Bank recharge, plans, coupon redemption, refund, and withdrawal orchestration |
| [RECHARGE_ORDER_SPEC.md](./RECHARGE_ORDER_SPEC.md) | Recharge packages, recharge orders, unified order center |
| [APP_TEMPLATE_ORDER_SPEC.md](./APP_TEMPLATE_ORDER_SPEC.md) | Published app-template purchase: order, payment, and the settled-order entitlement |
| [commerce-recharge.spec.json](./commerce-recharge.spec.json) | Machine-readable recharge contract + migration from payment |
| [commerce-checkout-topology.spec.json](./commerce-checkout-topology.spec.json) | Order ↔ payment dependency and checkout flows |
| [commerce-payment-webhook.spec.json](./commerce-payment-webhook.spec.json) | Order-owned PSP webhooks and in-process settlement |

## Sibling specs

| Repository | Entry |
| --- | --- |
| `sdkwork-account` | `specs/COMMERCE_BOUNDARY_SPEC.md` |
| `sdkwork-payment` | `specs/PAYMENT_EXECUTOR_SPEC.md` |

## Verification

```powershell
cd ..\sdkwork-order
pnpm verify
cargo test --workspace
```
