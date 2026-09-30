# sdkwork-order
repository-kind: application

SDKWork commerce **order** capability building-block repository (domain `commerce`).

- Standards: `../sdkwork-specs/README.md`
- Federated composition consumer: `@sdkwork/cloudrouter-app-sdk/domains` / `@sdkwork/cloudrouter-backend-sdk/domains` (federated domain surfaces)
- Domain service: `crates/sdkwork-order-service/`
- Repository SQL: `crates/sdkwork-order-repository-sqlx/`
- HTTP API server: `crates/sdkwork-api-order-standalone-gateway/`

## Quick start

```bash
cargo test --workspace
```

## Documentation Canon

- [docs/README.md](docs/README.md)
- [docs/product/prd/PRD.md](docs/product/prd/PRD.md)
- [docs/architecture/tech/TECH_ARCHITECTURE.md](docs/architecture/tech/TECH_ARCHITECTURE.md)

## Application Roots

- [apps directory index](apps/README.md)
