#!/bin/bash

# Simple script to submit a single BCE record for testing

if [ $# -lt 2 ]; then
    echo "Usage: $0 <port> <amount_in_cents> [record_type]"
    echo "Examples:"
    echo "  $0 8081 5500 voice_call    # T-Mobile DE"
    echo "  $0 8082 3200 data_session  # Vodafone UK"
    echo "  $0 8083 450 sms            # Orange FR"
    echo
    echo "Available ports:"
    echo "  8081 - T-Mobile DE"
    echo "  8082 - Vodafone UK"
    echo "  8083 - Orange FR"
    echo "  8084 - Telenor NO"
    echo "  8085 - SFR FR"
    exit 1
fi

PORT="$1"
AMOUNT="$2"
RECORD_TYPE="${3:-voice_call}"

# Generate random IMSI and record ID
RANDOM_ID=$((RANDOM % 9000 + 1000))
TIMESTAMP=$(date +%s)
RECORD_ID="TEST_${PORT}_${RANDOM_ID}_$(date +%Y%m%d_%H%M%S)"

echo "📱 Submitting BCE Record:"
echo "  Port: $PORT"
echo "  Amount: $AMOUNT cents"
echo "  Type: $RECORD_TYPE"
echo "  Record ID: $RECORD_ID"
echo

curl -X POST "http://localhost:$PORT/api/v1/bce/submit" \
  -H "Content-Type: application/json" \
  -d "{
    \"record\": {
      \"record_id\": \"$RECORD_ID\",
      \"record_type\": \"$RECORD_TYPE\",
      \"imsi\": \"26201$(printf "%09d" $RANDOM_ID)\",
      \"home_plmn\": \"26201\",
      \"visited_plmn\": \"23415\",
      \"session_duration\": 300,
      \"bytes_uplink\": 1024,
      \"bytes_downlink\": 4096,
      \"wholesale_charge\": $AMOUNT,
      \"retail_charge\": $((AMOUNT + 500)),
      \"currency\": \"EUR\",
      \"timestamp\": $TIMESTAMP,
      \"charging_id\": $RANDOM_ID
    }
  }"

echo
echo "✅ BCE record submitted!"