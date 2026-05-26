# Klyster Dockerfile - Multi-stage build for smaller production image

# =============================================================================
# Stage 1: Build
# =============================================================================
FROM rust:1.81-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /build

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock* ./

# Create dummy main.rs to cache dependencies
RUN mkdir -p crates/klyster/src && \
    echo "fn main() {}" > crates/klyster/src/main.rs && \
    echo "pub mod config; pub mod models; pub mod logging; pub mod shutdown;" > crates/domain/src/lib.rs && \
    echo "pub mod error; pub mod migrate; pub mod pool; pub mod repositories;" > crates/db/src/lib.rs && \
    echo "" > crates/web/src/lib.rs && \
    echo "" > crates/agent/src/lib.rs && \
    echo "" > crates/analytics/src/lib.rs

# Build dependencies only
RUN cargo build --release --package klyster 2>/dev/null || true

# Copy actual source code
COPY . .

# Build the application
RUN touch crates/*/src/*.rs && \
    cargo build --release --package klyster

# =============================================================================
# Stage 2: Runtime
# =============================================================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    libsqlite3-0 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user for security
RUN useradd -m -u 1000 klyster

# Create directories
RUN mkdir -p /data /var/log/klyster && chown -R klyster:klyster /data /var/log/klyster

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/klyster /app/klyster

# Copy example config
COPY klyster.example.toml /app/klyster.example.toml

# Copy entrypoint script
COPY docker/entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

# Switch to non-root user
USER klyster

# Expose ports
EXPOSE 8080 9091

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/healthz || exit 1

# Default command
ENTRYPOINT ["/app/entrypoint.sh"]
CMD ["--config", "/app/klyster.toml"]