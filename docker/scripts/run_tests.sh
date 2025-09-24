#!/bin/bash

echo "🧪 SP BCE Settlement Testing Suite"
echo "================================="

# Function to wait for service
wait_for_service() {
    local url=$1
    local name=$2
    echo "⏳ Waiting for $name to be ready..."
    for i in {1..30}; do
        if curl -s "$url" > /dev/null 2>&1; then
            echo "✅ $name is ready!"
            return 0
        fi
        echo "   Attempt $i/30..."
        sleep 2
    done
    echo "❌ $name failed to start"
    return 1
}

# Check if validators are running
echo "🏥 Checking validator health..."
wait_for_service "http://sp-bce-validator-1:8080/health" "Validator 1"
wait_for_service "http://sp-bce-validator-2:8080/health" "Validator 2"
wait_for_service "http://sp-bce-validator-3:8080/health" "Validator 3"

echo ""
echo "📊 Validator Status:"
echo "Validator 1: $(curl -s http://sp-bce-validator-1:8080/health | jq -r '.status // "unknown"' 2>/dev/null || echo "unknown")"
echo "Validator 2: $(curl -s http://sp-bce-validator-2:8080/health | jq -r '.status // "unknown"' 2>/dev/null || echo "unknown")"
echo "Validator 3: $(curl -s http://sp-bce-validator-3:8080/health | jq -r '.status // "unknown"' 2>/dev/null || echo "unknown")"

echo ""
echo "🌐 Network Status:"
curl -s http://sp-bce-validator-1:8080/api/v1/network/status | jq '.' 2>/dev/null || echo "Network status unavailable"

echo ""
echo "🧪 Running Settlement Tests..."

echo ""
echo "💰 Test 1: Small settlements (should NOT trigger blockchain)"
./test_small_settlements.sh

echo ""
echo "🏦 Test 2: Large settlements (SHOULD trigger blockchain)"
./test_large_settlements.sh

echo ""
echo "🌍 Test 3: Realistic scenario with mixed settlement sizes"
./test_realistic_scenario.sh

echo ""
echo "📊 Test 4: Multi-validator settlement distribution"
./test_multi_validator.sh

echo ""
echo "📋 Final Statistics:"
echo "Validator 1 stats:"
curl -s http://sp-bce-validator-1:8080/api/v1/bce/stats | jq '.' 2>/dev/null || echo "Stats unavailable"

echo ""
echo "Validator 2 stats:"
curl -s http://sp-bce-validator-2:8080/api/v1/bce/stats | jq '.' 2>/dev/null || echo "Stats unavailable"

echo ""
echo "Validator 3 stats:"
curl -s http://sp-bce-validator-3:8080/api/v1/bce/stats | jq '.' 2>/dev/null || echo "Stats unavailable"

echo ""
echo "✅ SP BCE Testing Complete!"