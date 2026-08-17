# sdkwork-order-backend-sdk

Generated SDKWork v3 dual-token transport SDK.

## Installation

```bash
npm install @sdkwork/order-backend-sdk
# or
yarn add @sdkwork/order-backend-sdk
# or
pnpm add @sdkwork/order-backend-sdk
```

## Quick Start

```typescript
import { SdkworkOrderBackendClient } from '@sdkwork/order-backend-sdk';

const client = new SdkworkOrderBackendClient({
  baseUrl: 'http://127.0.0.1:18079',
  timeout: 30000,
});

// Authentication
client.setAuthToken('your-auth-token');
client.setAccessToken('your-access-token');

// Use the SDK
const params = {
  status: 'status',
  page: 2,
  page_size: 3,
};
const result = await client.orderAdminBackend.backend.tokenBankPlans.list(params);
```

## Authentication

```text
Authorization: Bearer <authToken>
Access-Token: <accessToken>
```


## Configuration (Non-Auth)

```typescript
import { SdkworkOrderBackendClient } from '@sdkwork/order-backend-sdk';

const client = new SdkworkOrderBackendClient({
  baseUrl: 'http://127.0.0.1:18079',
  timeout: 30000, // Request timeout in ms
  headers: {      // Custom headers
    'X-Custom-Header': 'value',
  },
});
```

## API Modules

- `client.orderAdminOrders` - order_admin_orders API
- `client.orderAdminAfterSales` - order_admin_after_sales API
- `client.orderAdminShipments` - order_admin_shipments API
- `client.orderAdminBackend` - order_admin_backend API

## Usage Examples

### order_admin_orders

```typescript
// List order cancellation audit records
const params = {
  status: 'status',
  page: 2,
  page_size: 3,
};
const result = await client.orderAdminOrders.orders.admin.cancellations.list(params);
```

### order_admin_after_sales

```typescript
// List after-sales requests for operator review
const params = {
  status: 'status',
  after_sales_type: 'after_sales_type',
  order_id: 'order_id',
  page: 4,
  page_size: 5,
};
const result = await client.orderAdminAfterSales.afterSales.management.list(params);
```

### order_admin_shipments

```typescript
// List shipments for operator review
const params = {
  status: 'status',
  order_id: 'order_id',
  fulfillment_id: 'fulfillment_id',
  page: 4,
  page_size: 5,
};
const result = await client.orderAdminShipments.shipments.list(params);
```

### order_admin_backend

```typescript
// Token Bank plans list.
const params = {
  status: 'status',
  page: 2,
  page_size: 3,
};
const result = await client.orderAdminBackend.backend.tokenBankPlans.list(params);
```

## Error Handling

```typescript
import { SdkworkOrderBackendClient, NetworkError, TimeoutError, AuthenticationError } from '@sdkwork/order-backend-sdk';

try {
  const params = {
    status: 'status',
    page: 2,
    page_size: 3,
  };
  const result = await client.orderAdminBackend.backend.tokenBankPlans.list(params);
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
