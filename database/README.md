# order database module

Reference contract for order capability tables under commerce platform bootstrap.

## Initialization state

This module is in **initialization state** for greenfield deployments. The
schema is provisioned from **baseline plus migrations** (`database.manifest.json`
`baselineStrategy`):

1. **Baseline** — `database/ddl/baseline/postgres/0001_order_baseline.sql` holds
   the consolidated initialization DDL: the core order/checkout/account-value
   tables plus every pre-GA migration folded in as an idempotent
   (`IF NOT EXISTS`) block. Note that `commerce_order_event` and
   `commerce_order_cancellation` are created by
   `migrations/postgres/0004_order_lifecycle_tables.up.sql`, not by the
   baseline — a fresh deployment runs both, in order.
2. **Migrations** — `database/migrations/postgres/0004`–`0013` are applied
   after the baseline on every fresh deployment. New schema changes land here
   as `00NN_<name>.up.sql` files with the `sdkwork:migration` header
   (`reversible: false` / forward-fix policy; there are no down migrations).
3. **Contract** — `database/contract/table-registry.json` and
   `database/contract/schema.yaml` register every order-owned table (23) and
   are the DDL authority referenced by `TECH_ARCHITECTURE.md`.
4. **Drift** — run `pnpm db:drift:check` before release.

## Engines

PostgreSQL is the only authoritative-server engine (DATABASE_SPEC: L1+
authoritative tables `MUST` run on PostgreSQL; SQLite is client-local only and
order never persists server-side SQLite rows).

## Commands

```bash
pnpm run db:validate
pnpm run db:materialize:contract
pnpm run db:plan
pnpm run db:init
pnpm run db:migrate
pnpm run db:seed
pnpm run db:status
pnpm run db:drift:check
```
