# Multi-stage Docker build for Alelysee Dioxus Fullstack App
# ============================================================

# Base stage with common dependencies
FROM rust:1.91.1-slim AS base

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Dioxus CLI for asset bundling
RUN cargo install dioxus-cli --locked

# Set working directory
WORKDIR /app

# Copy workspace configuration
COPY Cargo.toml Cargo.lock ./

# Copy all package manifests
COPY packages/api/Cargo.toml packages/api/
COPY packages/web/Cargo.toml packages/web/
COPY packages/desktop/Cargo.toml packages/desktop/
COPY packages/mobile/Cargo.toml packages/mobile/
COPY packages/ui/Cargo.toml packages/ui/

# Copy assets
COPY packages/ui/assets packages/ui/assets/
COPY packages/web/assets packages/web/assets/

# Force use of the provided Cargo.lock
ENV CARGO_INCREMENTAL=0

# Create dummy source files to cache dependencies
RUN mkdir -p packages/api/src packages/web/src packages/desktop/src packages/mobile/src packages/ui/src && \
    echo "fn main() {}" > packages/api/src/lib.rs && \
    echo "fn main() {}" > packages/web/src/main.rs && \
    echo "fn main() {}" > packages/desktop/src/main.rs && \
    echo "fn main() {}" > packages/mobile/src/main.rs && \
    echo "fn main() {}" > packages/ui/src/lib.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release --workspace && \
    rm -rf packages/*/src && \
    rm -rf target/release/deps/*target*

# Builder stage - compile the application
FROM base AS builder

# Copy source code
COPY packages/api/src packages/api/src/
COPY packages/web/src packages/web/src/
COPY packages/desktop/src packages/desktop/src/
COPY packages/mobile/src packages/mobile/src/
COPY packages/ui/src packages/ui/src/
COPY packages/api/migrations packages/api/migrations/

# Build client + server bundles (fullstack) so assets are generated.
RUN dx build --web --release --package web --fullstack

# Runtime stage - minimal image for running the app
FROM ubuntu:24.04 AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --shell /bin/bash app

# Set working directory
WORKDIR /app

# Copy the compiled binary and web assets from the dx build output
COPY --from=builder /app/target/dx/web/release/web/server /app/server
COPY --from=builder /app/target/dx/web/release/web/public /app/public

# Copy migrations for database setup
COPY --from=builder /app/packages/api/migrations /app/migrations

# Change ownership to non-root user
RUN chown -R app:app /app

# Switch to non-root user
USER app

# Default bind settings; can be overridden by the runtime.
ENV IP=0.0.0.0
ENV PORT=8080

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD sh -c 'curl -f "http://localhost:${PORT:-8080}/api/health" || exit 1'

# Run the application
CMD ["./server"]
