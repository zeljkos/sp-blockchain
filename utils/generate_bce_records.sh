#!/bin/bash

# BCE Record Generator - Creates realistic telecom roaming records
# Usage: ./generate_bce_records.sh [API_ENDPOINT] [COUNT]
# Example: ./generate_bce_records.sh http://localhost:8081/api/v1/bce/submit 20

API_ENDPOINT=${1:-"http://localhost:8081/api/v1/bce/submit"}
RECORD_COUNT=${2:-20}

echo "Generating $RECORD_COUNT BCE records to $API_ENDPOINT"

# Operators with their PLMN codes and rates (cents)
declare -A OPERATORS
OPERATORS["T-Mobile DE"]="26201"
OPERATORS["Vodafone UK"]="23415"
OPERATORS["Orange FR"]="20801"
OPERATORS["Telenor NO"]="24201"
OPERATORS["SFR FR"]="20810"

# Convert operator names to array
OP_NAMES=("T-Mobile DE" "Vodafone UK" "Orange FR" "Telenor NO" "SFR FR")

# Rate structures (cents per unit)
CALL_RATES=(12 15 18 20 25)  # cents per minute
DATA_RATES=(1 2 3 4 5)       # cents per MB
SMS_RATES=(3 5 7 8 10)       # cents per SMS

# Generate IMSI patterns for different home operators
generate_imsi() {
    local home_op="$1"
    case "$home_op" in
        "T-Mobile DE") echo "262020$(printf "%09d" $((RANDOM % 1000000000)))" ;;
        "Vodafone UK") echo "234150$(printf "%09d" $((RANDOM % 1000000000)))" ;;
        "Orange FR") echo "208010$(printf "%09d" $((RANDOM % 1000000000)))" ;;
        "Telenor NO") echo "242010$(printf "%09d" $((RANDOM % 1000000000)))" ;;
        "SFR FR") echo "208100$(printf "%09d" $((RANDOM % 1000000000)))" ;;
    esac
}

# Generate realistic usage patterns
generate_usage() {
    local usage_type="$1"
    case "$usage_type" in
        "light")
            CALL_MIN=$((RANDOM % 30 + 5))          # 5-35 minutes
            DATA_MB=$((RANDOM % 200 + 50))         # 50-250 MB
            SMS_COUNT=$((RANDOM % 15 + 2))         # 2-17 SMS
            ;;
        "moderate")
            CALL_MIN=$((RANDOM % 60 + 30))         # 30-90 minutes
            DATA_MB=$((RANDOM % 800 + 200))        # 200-1000 MB
            SMS_COUNT=$((RANDOM % 25 + 10))        # 10-35 SMS
            ;;
        "heavy")
            CALL_MIN=$((RANDOM % 120 + 60))        # 60-180 minutes
            DATA_MB=$((RANDOM % 2000 + 1000))      # 1000-3000 MB
            SMS_COUNT=$((RANDOM % 50 + 20))        # 20-70 SMS
            ;;
    esac
}

# Generate records
for i in $(seq 1 $RECORD_COUNT); do
    # Random roaming scenario
    home_idx=$((RANDOM % ${#OP_NAMES[@]}))
    visited_idx=$((RANDOM % ${#OP_NAMES[@]}))

    # Ensure different operators for roaming
    while [ $home_idx -eq $visited_idx ]; do
        visited_idx=$((RANDOM % ${#OP_NAMES[@]}))
    done

    home_op="${OP_NAMES[$home_idx]}"
    visited_op="${OP_NAMES[$visited_idx]}"

    # Generate IMSI
    imsi=$(generate_imsi "$home_op")

    # Random usage pattern
    usage_patterns=("light" "light" "moderate" "moderate" "heavy")  # Weighted toward lighter usage
    pattern=${usage_patterns[$((RANDOM % ${#usage_patterns[@]}))]}
    generate_usage "$pattern"

    # Random rates (realistic telecom rates)
    call_rate=${CALL_RATES[$((RANDOM % ${#CALL_RATES[@]}))]}
    data_rate=${DATA_RATES[$((RANDOM % ${#DATA_RATES[@]}))]}
    sms_rate=${SMS_RATES[$((RANDOM % ${#SMS_RATES[@]}))]}

    # Calculate wholesale charge (with small variance for realism)
    base_charge=$(($CALL_MIN * $call_rate + $DATA_MB * $data_rate + $SMS_COUNT * $sms_rate))
    variance=$((RANDOM % 50 - 25))  # ±25 cents variance
    wholesale_charge=$((base_charge + variance))

    # Ensure positive charge
    if [ $wholesale_charge -lt 0 ]; then
        wholesale_charge=$base_charge
    fi

    # Generate timestamp (random within last 30 days)
    current_time=$(date +%s)
    random_offset=$((RANDOM % 2592000))  # 30 days in seconds
    timestamp=$((current_time - random_offset))

    # Create record ID
    record_id=$(printf "BCE_%03d_%s_to_%s_%d" $i "${home_op// /}" "${visited_op// /}" $timestamp)

    echo "Creating record $i: $home_op subscriber roaming on $visited_op"
    echo "  Usage: ${CALL_MIN}min calls, ${DATA_MB}MB data, ${SMS_COUNT} SMS"
    echo "  Rates: ${call_rate}c/min, ${data_rate}c/MB, ${sms_rate}c/SMS"
    echo "  Total: ${wholesale_charge} cents"

    # Submit BCE record via API
    curl -X POST "$API_ENDPOINT" \
        -H "Content-Type: application/json" \
        -d "{
            \"record_id\": \"$record_id\",
            \"imsi\": \"$imsi\",
            \"home_operator\": \"$home_op\",
            \"visited_operator\": \"$visited_op\",
            \"call_minutes\": $CALL_MIN,
            \"data_mb\": $DATA_MB,
            \"sms_count\": $SMS_COUNT,
            \"call_rate_cents\": $call_rate,
            \"data_rate_cents\": $data_rate,
            \"sms_rate_cents\": $sms_rate,
            \"wholesale_charge_cents\": $wholesale_charge,
            \"timestamp\": $timestamp
        }" \
        -w "\nHTTP Status: %{http_code}\n" \
        -s

    echo "----------------------------------------"

    # Small delay to avoid overwhelming the API
    sleep 0.5
done

echo "✅ Generated $RECORD_COUNT BCE records successfully!"
echo "📊 Summary:"
echo "  - Mixed usage patterns (light/moderate/heavy roaming)"
echo "  - Realistic telecom rates and charges"
echo "  - Cross-operator roaming scenarios"
echo "  - IMSI numbers matching operator PLMNs"