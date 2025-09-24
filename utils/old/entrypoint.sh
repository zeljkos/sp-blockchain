#!/bin/bash

# SP BCE Node Entrypoint Script
# Handles dynamic configuration for consortium deployment

set -e

echo "=== SP BCE Node Starting ==="
echo "Node ID: $NODE_ID"
echo "Settlement Threshold: $SETTLEMENT_THRESHOLD_EUR EUR"
echo "API Host: $API_HOST:$API_PORT"
echo "P2P Port: $P2P_PORT"
echo "Bootstrap Peers: ${BOOTSTRAP_PEERS:-"none (bootstrap node)"}"
echo "Data Directory: /app/data"
echo "================================"

# Build command arguments
CMD_ARGS=(
    "sp-bce-node"
    "start"
    "--data-dir" "/app/data"
    "--api-host" "$API_HOST"
    "--api-port" "$API_PORT"
    "--p2p-port" "$P2P_PORT"
    "--node-id" "$NODE_ID"
    "--settlement-threshold-eur" "$SETTLEMENT_THRESHOLD_EUR"
)

# Add bootstrap peers if specified
if [ -n "$BOOTSTRAP_PEERS" ]; then
    IFS=',' read -ra PEER_ARRAY <<< "$BOOTSTRAP_PEERS"
    for peer in "${PEER_ARRAY[@]}"; do
        CMD_ARGS+=("--bootstrap-peers" "$peer")
    done
fi

echo "Starting SP BCE node with command:"
echo "${CMD_ARGS[@]}"
echo "================================"

# Execute the SP BCE node
exec "${CMD_ARGS[@]}"