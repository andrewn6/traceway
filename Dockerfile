# Build stage
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary with cloud features
RUN cargo build --release -p daemon --features cloud

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/daemon /usr/local/bin/traceway

# Copy UI assets (pre-built)
COPY ui/build /app/ui/build

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
# REDIS_URL - Redis connection string for pub/sub and BullMQ
# TURBOPUFFER_API_KEY - Turbopuffer API key
# TURBOPUFFER_NAMESPACE - Namespace prefix for multi-tenancy

CMD ["traceway", "--cloud"]
