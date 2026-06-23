# Multi-stage build for Klyster (Alpine-based)
FROM rust:1.90-alpine3.20 AS builder

WORKDIR /build

# Install build dependencies:
# - protoc: required by tonic-build for the analytics crate
# - musl-dev: required for static linking
# - nodejs/npm: required for building Tailwind CSS
RUN apk add --no-cache \
    protobuf-dev \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    nodejs \
    npm

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY svcs ./svcs
COPY proto ./proto

# Build release binary (statically linked against musl)
RUN cargo build --release --bin klyster

# Runtime stage: minimal Alpine
FROM alpine:3.20

WORKDIR /app

# Install only what's needed at runtime
RUN apk add --no-cache \
    ca-certificates \
    curl

# Create non-root user
RUN adduser -D -u 1000 klyster && \
    chown -R klyster:klyster /app

# Copy statically-linked binary from builder
COPY --from=builder /build/target/release/klyster /app/klyster

# Copy default config
COPY build/config/klyster.toml /app/klyster.toml

# Create data directory
RUN mkdir -p /app/data && chown -R klyster:klyster /app/data

USER klyster

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/healthz || exit 1

CMD ["/app/klyster", "--web"]
