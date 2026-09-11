FROM rust:1-bookworm AS builder
WORKDIR /src
COPY . .
RUN cargo build --release -p sdkwork-api-order-standalone-gateway

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /src/target/release/sdkwork-api-order-standalone-gateway /usr/local/bin/sdkwork-api-order-standalone-gateway
ENV ORDER_API_BIND=0.0.0.0:18093
EXPOSE 18093
CMD ["sdkwork-api-order-standalone-gateway"]
