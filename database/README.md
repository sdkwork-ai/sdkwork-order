# order database module

Reference contract for order capability tables under commerce platform bootstrap.

## Initialization state

This module is in **initialization state** for greenfield deployments:

1. **Baseline** — `database/ddl/baseline/{engine}/0001_order_baseline.sql` contains the full DDL snapshot.
2. **Migrations** — `database/migrations/{engine}/` is reserved for post-GA incremental schema changes only. It is intentionally empty at initialization.
3. **Drift** — run `pnpm db:drift:check` before release.


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
