#!/bin/bash

echo "🚀 Forcing Settlement by Submitting Large BCE Record"
echo "=================================================="

# Submit a 150 EUR record to force settlement
AMOUNT=15000  # 150 EUR in cents
PORT=8081     # T-Mobile DE
TIMESTAMP=$(date +%s)
RECORD_ID="FORCE_SETTLEMENT_$(date +%Y%m%d_%H%M%S)"

echo "📱 Submitting large BCE record to force settlement:"
echo "  Port: $PORT (T-Mobile DE)"
echo "  Amount: $AMOUNT cents (150 EUR)"
echo "  Record ID: $RECORD_ID"
echo

curl -X POST "http://localhost:$PORT/api/v1/bce/submit" \
  -H "Content-Type: application/json" \
  -d "{
    \"record\": {
      \"record_id\": \"$RECORD_ID\",
      \"record_type\": \"data_session\",
      \"imsi\": \"262019999888777\",
      \"home_plmn\": \"26201\",
      \"visited_plmn\": \"23415\",
      \"session_duration\": 7200,
      \"bytes_uplink\": 104857600,
      \"bytes_downlink\": 5368709120,
      \"wholesale_charge\": $AMOUNT,
      \"retail_charge\": $((AMOUNT + 5000)),
      \"currency\": \"EUR\",
      \"timestamp\": $TIMESTAMP,
      \"charging_id\": 9999
    }
  }"

echo
echo
echo "✅ Large record submitted! This should trigger settlement if threshold logic is working."
echo "Let me check the settlement stats..."
echo

curl -s http://localhost:8081/api/v1/bce/stats | jq .