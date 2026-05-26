# Multi-stage build for Klyster
FROM rust:1.90-slim as builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY svcs ./svcs

# Build release binary
RUN cargo build --release --bin klyster

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 klyster && \
    chown -R klyster:klyster /app

# Copy binary from builder
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
