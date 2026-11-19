# Build stage. The Debian release must match the runtime stage (OpenSSL ABI).
FROM rust:1-trixie AS builder
WORKDIR /app

# Queries are checked against the metadata committed in .sqlx, so the build
# does not need a running database.
ENV SQLX_OFFLINE=true

COPY Cargo.toml Cargo.lock ./
COPY .sqlx .sqlx
COPY migrations migrations
COPY src src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked \
    && cp target/release/backend /usr/local/bin/backend

# Runtime stage: only the binary and the libraries it links against.
FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3t64 \
    && apt-get clean

COPY --from=builder /usr/local/bin/backend /usr/local/bin/backend

USER nobody
EXPOSE 3000
CMD ["backend"]
