#!/bin/bash

echo "🏢 SP BCE Consortium Status Check"
echo "================================="
echo

# SP Node Endpoints
TMOBILE_DE="http://localhost:8081"
VODAFONE_UK="http://localhost:8082"
ORANGE_FR="http://localhost:8083"
TELENOR_NO="http://localhost:8084"
SFR_FR="http://localhost:8085"

# Function to check node health
check_node() {
    local name="$1"
    local url="$2"
    local emoji="$3"

    echo -n "$emoji $name: "
    response=$(curl -s "$url/health" 2>/dev/null)
    if [ $? -eq 0 ] && echo "$response" | grep -q "healthy"; then
        timestamp=$(echo "$response" | jq -r '.timestamp // "unknown"' 2>/dev/null)
        echo "✅ HEALTHY (ts: $timestamp)"
    else
        echo "❌ DOWN/UNREACHABLE"
    fi
}

# Function to get settlement stats
get_stats() {
    local name="$1"
    local url="$2"
    local emoji="$3"

    echo
    echo "$emoji $name Settlement Stats:"
    response=$(curl -s "$url/api/v1/bce/stats" 2>/dev/null)
    if [ $? -eq 0 ]; then
        echo "$response" | jq . 2>/dev/null || echo "Failed to parse stats"
    else
        echo "  ❌ Stats unavailable"
    fi
}

echo "Health Status:"
echo "-------------"
check_node "T-Mobile DE" "$TMOBILE_DE" "🇩🇪"
check_node "Vodafone UK" "$VODAFONE_UK" "🇬🇧"
check_node "Orange FR" "$ORANGE_FR" "🇫🇷"
check_node "Telenor NO" "$TELENOR_NO" "🇳🇴"
check_node "SFR FR" "$SFR_FR" "🇫🇷"

echo
echo "Settlement Statistics:"
echo "====================="
get_stats "T-Mobile DE" "$TMOBILE_DE" "🇩🇪"
get_stats "Vodafone UK" "$VODAFONE_UK" "🇬🇧"
get_stats "Orange FR" "$ORANGE_FR" "🇫🇷"
get_stats "Telenor NO" "$TELENOR_NO" "🇳🇴"
get_stats "SFR FR" "$SFR_FR" "🇫🇷"

echo
echo "Docker Container Status:"
echo "========================"
cd /home/zeljko/src/albatross-fresh/sp-bce-node/docker 2>/dev/null && docker compose ps 2>/dev/null || echo "Docker Compose not available"

echo
echo "📊 Status check complete!"