# UI build stage
FROM oven/bun:1 AS ui-builder

WORKDIR /app/ui
COPY ui/package.json ui/bun.lock ./
RUN bun install --frozen-lockfile
COPY ui/ ./
RUN bun run build

# Rust build stage
FROM rust:1.88-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Copy pre-built UI assets (needed by rust-embed at compile time)
COPY --from=ui-builder /app/ui/build ./ui/build

# Build release binary with cloud features
RUN cargo build --release -p traceway --features cloud

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/traceway /usr/local/bin/traceway

# Create non-root user
RUN useradd -r -s /bin/false traceway
USER traceway

EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/api/health || exit 1

# Environment variables (override at runtime)
ENV RUST_LOG=info
ENV STORAGE_BACKEND=turbopuffer

CMD ["traceway", "--cloud"]
