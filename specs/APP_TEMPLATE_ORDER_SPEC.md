# App Template Order Spec

Order-owned purchase of a published app template. The deployments module owns
the catalog (`deploy_app_template`, `deploy_app_template_category`,
`deploy_app_template_version`); this repository owns the trade, the payment, and
the entitlement.

## Why the entitlement lives here

An app-template purchase used to keep a module-owned entitlement row in the
deployments database. That split meant two ledgers for one trade: commerce held
the order and the payment state, deployments held a second copy of the price,
the order reference, and the status. The trade now has exactly one home — a
settled `commerce_order` with `subject = 'app_template'` **is** the entitlement,
and the deployments module keeps only the catalog plus the listing's install
counter.

## Order shape

| Field | Value |
| --- | --- |
| `commerce_order.subject` | `app_template` |
| `commerce_order.status` | `pending_payment` for a paid listing; `paid` immediately for a free one |
| `commerce_order.payment_status` | `pending` / `success` |
| `commerce_order.fulfillment_status` | `unfulfilled` / `fulfilled` |
| `commerce_order.purchase_intent_key` | SHA-256 over buyer + listing + price + currency, so a retry reuses an unpaid order instead of opening a second one |
| `commerce_order_item.sku_id` | `deploy_app_template.uuid` |
| `commerce_order_item.sku_snapshot_json` | `{templateUuid, templateName, versionUuid, pricingModel, clientRequestNo, source, paymentMethod, paymentProduct}` |

The snapshot deliberately carries no `product_type` / `fulfillment_type` key:
`stable_order_settlement_subject` prefers those keys over the stored subject, so
adding one would route settlement away from the app-template handler.

## Lifecycle

1. **Create** — `appTemplateOrders.create` resolves the listing from
   `deploy_app_template` (tenant + organization scoped, `PUBLISHED` and
   `PUBLIC`, not authored by the caller), snapshots its title, price, currency
   and newest published version onto the order, and stores the order.
   - Idempotency: the same `Idempotency-Key` replays the stored order; a settled
     order for the same buyer and listing is returned as `reused` instead of
     selling the template twice; an expired unpaid intent is retired before a
     new order is created.
   - A **free** listing is settled inside the creating transaction — no payment
     provider is involved, the order is created `paid` + `fulfilled`, and the
     install counter is incremented once.
2. **Pay** — a **paid** listing returns the order-bound `cashierUrl` for the H5
   cashier product, or a provider QR code for `wechat_native` /
   `alipay_native`, through the same `OwnerOrderPaymentStore` seam membership
   uses.
3. **Settle** — the PSP webhook marks the order `paid` and dispatches
   `PAYMENT_NOTIFY_BUSINESS_APP_TEMPLATE`; `sdkwork-order-integration-deployments`
   advances `fulfillment_status` to `fulfilled` and increments
   `deploy_app_template.install_count` in one transaction, counting the install
   only when the order row actually transitioned. Replayed webhooks and the
   payment compensation worker are therefore idempotent.
4. **Read** — `appTemplateOrders.list` returns the buyer's orders; `status =
   paid` is the install entitlement and `versionUuid` is the version it entitles.

## Surfaces

| Surface | Operations |
| --- | --- |
| app-api `/app/v3/api` | `appTemplateOrders.create`, `appTemplateOrders.list` |
| catalog (sdkwork-deployments) | `templateCategories.list`, `marketplaceTemplates.list/retrieve`, `appTemplates.*`, `appTemplateVersions.*` |

No backend operator surface exists for this family: refunds, cancellations and
reconciliation are the order center's existing backend operations.

## Verification

```powershell
cd ..\sdkwork-order
cargo test -p sdkwork-order-service -p sdkwork-order-repository-sqlx -p sdkwork-order-integration-deployments
cargo test -p sdkwork-routes-order-app-api -p sdkwork-routes-order-backend-api
pnpm test:node
```
