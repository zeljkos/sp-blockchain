# Multi-stage build with dashboard support
FROM rust:1.87 as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    libssl-dev \
    pkg-config \
    ca-certificates \
    libclang-dev \
    clang \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /usr/src/app

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to cache dependencies
RUN mkdir -p src/bin \
    && echo "fn main() {}" > src/main.rs \
    && echo "fn main() {}" > src/simple_main.rs \
    && echo "fn main() {}" > src/bin/trusted_setup_demo.rs \
    && echo "pub fn add(left: usize, right: usize) -> usize { left + right }" > src/lib.rs

# Build dependencies
RUN cargo build --release && rm -rf src

# Copy actual source code
COPY src/ src/

# Build the actual application
RUN cargo build --release

# Runtime image
FROM ubuntu:24.04

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false spbce

# Copy binary from builder stage
COPY --from=builder /usr/src/app/target/release/sp-bce-node /usr/local/bin/sp-bce-node

# Copy dashboard files
COPY dashboard/ /app/dashboard/

# Copy entrypoint script
COPY docker/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Create data directory and set permissions
RUN mkdir -p /app/data && \
    chown -R spbce:spbce /app

# Default environment variables
ENV NODE_ID=sp-bce-node
ENV SETTLEMENT_THRESHOLD_EUR=100.0
ENV API_HOST=0.0.0.0
ENV API_PORT=8080
ENV P2P_PORT=30303
ENV BOOTSTRAP_PEERS=""

USER spbce
WORKDIR /app

# Expose API and P2P ports
EXPOSE 8080 30303

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]