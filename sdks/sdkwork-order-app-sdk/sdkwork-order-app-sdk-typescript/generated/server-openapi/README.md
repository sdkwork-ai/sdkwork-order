# sdkwork-order-app-sdk

Generated SDKWork v3 dual-token transport SDK.

## Installation

```bash
npm install @sdkwork/order-app-sdk
# or
yarn add @sdkwork/order-app-sdk
# or
pnpm add @sdkwork/order-app-sdk
```

## Quick Start

```typescript
import { SdkworkAppClient } from '@sdkwork/order-app-sdk';

const client = new SdkworkAppClient({
  baseUrl: 'http://127.0.0.1:18079',
  timeout: 30000,
});

// Authentication
client.setAuthToken('your-auth-token');
client.setAccessToken('your-access-token');

// Use the SDK
const result = await client.orderOrders.orders.statistics.retrieve();
```

## Authentication

```text
Authorization: Bearer <authToken>
Access-Token: <accessToken>
```


## Configuration (Non-Auth)

```typescript
import { SdkworkAppClient } from '@sdkwork/order-app-sdk';

const client = new SdkworkAppClient({
  baseUrl: 'http://127.0.0.1:18079',
  timeout: 30000, // Request timeout in ms
  headers: {      // Custom headers
    'X-Custom-Header': 'value',
  },
});
```

## API Modules

- `client.orderCheckout` - order_checkout API
- `client.orderOrders` - order_orders API
- `client.orderPayments` - order_payments API
- `client.orderAfterSales` - order_after_sales API
- `client.orderFulfillments` - order_fulfillments API
- `client.orderShipments` - order_shipments API
- `client.orderRecharges` - order_recharges API
- `client.orderMemberships` - order_memberships API
- `client.orderWithdrawals` - order_withdrawals API

## Usage Examples

### order_checkout

```typescript
// Checkout sessions retrieve.
const checkoutSessionId = '1';
const result = await client.orderCheckout.checkout.sessions.retrieve(checkoutSessionId);
```

### order_orders

```typescript
// Orders statistics retrieve.
const result = await client.orderOrders.orders.statistics.retrieve();
```

### order_payments

```typescript
// Payments order Payments list.
const orderId = '1';
const params = {
  page: 1,
  page_size: 2,
};
const result = await client.orderPayments.payments.orderPayments.list(orderId, params);
```

### order_after_sales

```typescript
// After Sales requests list.
const params = {
  status: 'status',
  order_id: 'order_id',
  page: 3,
  page_size: 4,
};
const result = await client.orderAfterSales.afterSales.requests.list(params);
```

### order_fulfillments

```typescript
// Fulfillments list.
const params = {
  status: 'status',
  page: 2,
  page_size: 3,
};
const result = await client.orderFulfillments.fulfillments.list(params);
```

### order_shipments

```typescript
// Shipments retrieve.
const shipmentId = '1';
const result = await client.orderShipments.shipments.retrieve(shipmentId);
```

### order_recharges

```typescript
// Recharges settings retrieve.
const result = await client.orderRecharges.recharges.settings.retrieve();
```

### order_memberships

```typescript
// Create or reuse a membership purchase-intent order.
const body = {
  action: 'purchase',
  packageId: 'packageId',
  paymentMethod: 'paymentMethod',
  paymentProduct: 'mobile_cashier_h5',
  clientRequestNo: 'clientRequestNo',
  source: 'source',
  grantQuantity: 1,
  amount: 'amount',
};
const idempotencyKey = 'Idempotency-Key';
const params = {
  idempotencyKey,
};
const result = await client.orderMemberships.memberships.orders.create(body, params);
```

### order_withdrawals

```typescript
// Withdrawal requests retrieve.
const withdrawalRequestId = '1';
const result = await client.orderWithdrawals.withdrawals.requests.retrieve(withdrawalRequestId);
```

## Error Handling

```typescript
import { SdkworkAppClient, NetworkError, TimeoutError, AuthenticationError } from '@sdkwork/order-app-sdk';

try {
  const result = await client.orderOrders.orders.statistics.retrieve();
} catch (error) {
  if (error instanceof AuthenticationError) {
    console.error('Authentication failed:', error.message);
  } else if (error instanceof TimeoutError) {
    console.error('Request timed out:', error.message);
  } else if (error instanceof NetworkError) {
    console.error('Network error:', error.message);
  } else {
    throw error;
  }
}
```

## Publishing

This SDK includes cross-platform publish scripts in `bin/`:
- `bin/publish-core.mjs`
- `bin/publish.sh`
- `bin/publish.ps1`

TypeScript check and publish commands use pnpm to materialize workspace dependency versions in a temporary tarball. They reject local-only dependency protocols before npm publication and do not rewrite the source `package.json`.

### Check

```bash
./bin/publish.sh --action check
```

### Publish

```bash
./bin/publish.sh --action publish --channel release
```

```powershell
.\bin\publish.ps1 --action publish --channel test --dry-run
```

> Configure npm registry credentials before release publish.

## License

MIT

## Regeneration Contract

- HTTP/OpenAPI generator-owned files are tracked in `.sdkwork/sdkwork-generator-manifest.json`.
- HTTP/OpenAPI generation also writes `.sdkwork/sdkwork-generator-changes.json` so automation can inspect created, updated, deleted, unchanged, scaffolded, and backed-up files plus the classified impact areas, verification plan, and execution decision for the latest generation.
- HTTP/OpenAPI apply mode also writes `.sdkwork/sdkwork-generator-report.json` with the full execution report, including `schemaVersion`, `generator`, stable artifact paths, and the execution handoff commands that match CLI `--json` output.
- CLI JSON output also includes an execution handoff with concrete next commands, including reviewed apply commands for dry-run flows.
- Put HTTP/OpenAPI hand-written wrappers, adapters, and orchestration in `custom/`.
- Files scaffolded under `custom/` are created once and preserved across HTTP/OpenAPI regenerations.
- If an HTTP/OpenAPI generated-owned file was modified locally, its previous content is copied to `.sdkwork/manual-backups/` before overwrite or removal.
- RPC SDK source workspaces use convention-first evidence by default: RPC SDK family naming, language workspace naming, `rpc/*.manifest.json`, proto source references, generated client source, and native package manifests.
- Use `sdkgen inspect --protocol rpc` to verify RPC convention evidence. Request persisted generator evidence only with `--emit-control-plane` for release, CI, audit, or migration workflows; evidence paths are derived by generator convention.
