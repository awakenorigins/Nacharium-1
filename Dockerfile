# Nacharium node — multi-stage build (Epoch 0 -> Sprint 4 deployable).
FROM rust:1.96-slim AS builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY Cargo.toml .
COPY crates ./crates
COPY migrations ./migrations
RUN cargo build --release -p nacharium-node

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/target/release/nacharium-node /usr/local/bin/nacharium-node
COPY --from=builder /build/migrations ./migrations
# DATABASE_URL is provided at runtime via docker-compose/env; no default here.
EXPOSE 8787
CMD ["nacharium-node"]
