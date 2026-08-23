# Multi-stage Dockerfile for Trading212 MCP Remote Server
# Optimized for production deployment with minimal image size
# Best practices: multi-stage build, layer caching, non-root user, health checks

# Stage 1: Builder
FROM rust:1.83-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    binutils \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy files to cache dependencies (layer caching optimization)
RUN mkdir -p src/bin benches && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn lib_dummy() {}" > src/lib.rs && \
    echo "fn main() {}" > src/bin/remote-server.rs && \
    echo "fn main() {}" > benches/memory_benchmark.rs && \
    cargo build --release --bin remote-server --features http-server && \
    rm -rf src benches target/release/.fingerprint/trading212-mcp-server-*

# Copy actual source code and benchmarks
COPY src ./src
COPY benches ./benches

# Build release binary with optimizations
# The release profile in Cargo.toml should include:
# lto = true, codegen-units = 1, strip = true for optimal size
RUN cargo build --release --bin remote-server --features http-server && \
    strip target/release/remote-server

# Stage 2: Runtime (minimal debian-slim base)
FROM debian:bookworm-slim

# Install runtime dependencies (ca-certificates for HTTPS, curl for healthcheck)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 -s /bin/bash mcp && \
    mkdir -p /app && \
    chown -R mcp:mcp /app

WORKDIR /app

# Copy binary from builder stage
COPY --from=builder --chown=mcp:mcp /app/target/release/remote-server /app/mcp-server

# Switch to non-root user
USER mcp

# Expose HTTP port
EXPOSE 3000

# Health check endpoint
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1

# Environment variable defaults
ENV RUST_LOG=info \
    MCP_HTTP_HOST=0.0.0.0 \
    MCP_HTTP_PORT=3000 \
    TRADING212_POOL_SIZE=16 \
    TRADING212_TIMEOUT_SECS=30

# Run the server
CMD ["/app/mcp-server"]
