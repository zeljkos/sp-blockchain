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

# Copy pre-built binary from host
COPY target/release/sp-bce-node /usr/local/bin/sp-bce-node

# Entrypoint script to handle dynamic configuration
COPY docker/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Create data directory
RUN mkdir -p /app/data && chown spbce:spbce /app/data

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