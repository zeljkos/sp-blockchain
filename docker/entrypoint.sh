#!/bin/bash

# SP BCE Node Entrypoint Script
# Handles dynamic configuration and startup

echo "Starting SP BCE Node: $NODE_ID"
echo "Settlement Threshold: $SETTLEMENT_THRESHOLD_EUR EUR"
echo "API: $API_HOST:$API_PORT"
echo "P2P: $P2P_PORT"

# Build command line arguments
ARGS=(
    "start"
    "--data-dir" "/app/data"
    "--api-host" "$API_HOST"
    "--api-port" "$API_PORT"
    "--node-id" "$NODE_ID"
    "--settlement-threshold-eur" "$SETTLEMENT_THRESHOLD_EUR"
)

# Execute SP BCE node
exec /usr/local/bin/sp-bce-node "${ARGS[@]}"