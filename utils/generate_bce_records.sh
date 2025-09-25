#!/bin/bash

# BCE Record Generator - Creates realistic telecom roaming records
# Usage: ./generate_bce_records.sh [API_ENDPOINT] [COUNT]
# Example: ./generate_bce_records.sh http://localhost:8081/api/v1/bce/submit 20

API_ENDPOINT=${1:-"http://localhost:8081/api/v1/bce/submit"}
RECORD_COUNT=${2:-20}

echo "Generating $RECORD_COUNT BCE records to $API_ENDPOINT"

# Operators with their PLMN codes, API keys, and ports
declare -A OPERATORS
OPERATORS["T-Mobile DE"]="26201"
OPERATORS["Vodafone UK"]="23415"
OPERATORS["Orange FR"]="20801"
OPERATORS["Telenor NO"]="24201"
OPERATORS["SFR FR"]="20810"

# API Keys for each provider
declare -A API_KEYS
API_KEYS["T-Mobile-DE"]="tmobile_api_key_2024_secure"
API_KEYS["Vodafone-UK"]="vodafone_api_key_2024_secure"
API_KEYS["Orange-FR"]="orange_api_key_2024_secure"
API_KEYS["Telenor-NO"]="telenor_api_key_2024_secure"
API_KEYS["SFR-FR"]="sfr_api_key_2024_secure"

# API Ports for each provider
declare -A API_PORTS
API_PORTS["T-Mobile-DE"]="8081"
API_PORTS["Vodafone-UK"]="8082"
API_PORTS["Orange-FR"]="8083"
API_PORTS["Telenor-NO"]="8084"
API_PORTS["SFR-FR"]="8085"

# Convert operator names to array (using exact format expected by auth system)
OP_NAMES=("T-Mobile-DE" "Vodafone-UK" "Orange-FR" "Telenor-NO" "SFR-FR")

# Rate structures (cents per unit)
CALL_RATES=(12 15 18 20 25)  # cents per minute
DATA_RATES=(1 2 3 4 5)       # cents per MB
SMS_RATES=(3 5 7 8 10)       # cents per SMS

# Get API endpoint and key for target provider
get_provider_info() {
    local endpoint="$1"

    # Extract port from endpoint to determine target provider
    if [[ "$endpoint" =~ :([0-9]+)/ ]]; then
        port="${BASH_REMATCH[1]}"
        case "$port" in
            "8081") echo "T-Mobile-DE" ;;
            "8082") echo "Vodafone-UK" ;;
            "8083") echo "Orange-FR" ;;
            "8084") echo "Telenor-NO" ;;
            "8085") echo "SFR-FR" ;;
            *) echo "T-Mobile-DE" ;;  # Default
        esac
    else
        echo "T-Mobile-DE"  # Default
    fi
}

# Generate IMSI patterns for different home operators
generate_imsi() {
    local home_op="$1"
    case "$home_op" in
        "T-Mobile-DE") echo "262020$(printf "%09d" $((RANDOM % 1000000000)))" ;;
        "Vodafone-UK") echo "234150$(printf "%09d" $((RANDOM % 1000000000)))" ;;
        "Orange-FR") echo "208010$(printf "%09d" $((RANDOM % 1000000000)))" ;;
        "Telenor-NO") echo "242010$(printf "%09d" $((RANDOM % 1000000000)))" ;;
        "SFR-FR") echo "208100$(printf "%09d" $((RANDOM % 1000000000)))" ;;
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

# Get target provider (who is submitting the records)
target_provider=$(get_provider_info "$API_ENDPOINT")

# Generate records
for i in $(seq 1 $RECORD_COUNT); do
    # REALISTIC TELECOM: The target provider is the VISITED network (they provide service and generate records)
    visited_op="$target_provider"

    # Random home operator - any subscriber from other networks can roam here
    home_idx=$((RANDOM % ${#OP_NAMES[@]}))
    while [ "${OP_NAMES[$home_idx]}" = "$visited_op" ]; do
        home_idx=$((RANDOM % ${#OP_NAMES[@]}))
    done
    home_op="${OP_NAMES[$home_idx]}"

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

    # Calculate wholesale charge for roaming scenario
    # Real telecom: subscriber uses foreign network, pays roaming rates for ALL usage
    call_charge=$(($CALL_MIN * $call_rate))
    data_charge=$(($DATA_MB * $data_rate))
    sms_charge=$(($SMS_COUNT * $sms_rate))

    # Total wholesale charge = sum of all roaming usage
    wholesale_charge=$((call_charge + data_charge + sms_charge))

    # Generate timestamp (random within last 30 days)
    current_time=$(date +%s)
    random_offset=$((RANDOM % 2592000))  # 30 days in seconds
    timestamp=$((current_time - random_offset))

    # Create realistic telecom record ID (generated by visited network)
    # Format: CDR_VisitedNetwork_YYYYMMDD_HHMMSS_ServiceType_SequenceNumber
    record_date=$(date -d "@$timestamp" '+%Y%m%d_%H%M%S')
    visited_network_code=$(echo "$visited_op" | tr -d '-' | tr '[:lower:]' '[:upper:]')

    # Determine primary service type based on usage
    if [ $CALL_MIN -gt 60 ]; then
        service_type="VOICE_HEAVY"
    elif [ $DATA_MB -gt 1000 ]; then
        service_type="DATA_HEAVY"
    elif [ $SMS_COUNT -gt 30 ]; then
        service_type="SMS_HEAVY"
    elif [ $DATA_MB -gt 500 ]; then
        service_type="DATA_MED"
    else
        service_type="MIXED"
    fi

    record_id=$(printf "CDR_%s_%s_%s_%03d" "$visited_network_code" "$record_date" "$service_type" $i)

    # Get API key for target provider
    api_key="${API_KEYS[$target_provider]}"

    echo "🏪 $visited_op generates record for $home_op subscriber roaming"
    echo "  📱 IMSI: $imsi"
    echo "  📊 Usage: ${CALL_MIN}min calls, ${DATA_MB}MB data, ${SMS_COUNT} SMS"
    echo "  💰 Rates: ${call_rate}c/min, ${data_rate}c/MB, ${sms_rate}c/SMS"
    echo "  🧾 Total charge: ${wholesale_charge} cents (€$(echo "scale=2; $wholesale_charge/100" | bc -l))"
    echo "  🆔 Record ID: $record_id"
    echo "  🌐 Submitting to: $target_provider blockchain"

    # Submit BCE record via API with authentication
    curl -X POST "$API_ENDPOINT" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $api_key" \
        -H "X-API-Key: $api_key" \
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
            \"timestamp\": $timestamp,
            \"roaming_minutes\": $CALL_MIN,
            \"roaming_data_mb\": $DATA_MB,
            \"roaming_rate_cents\": $call_rate,
            \"roaming_data_rate_cents\": $data_rate,
            \"network_pair_hash\": null,
            \"zkp_proof\": null,
            \"proof_verified\": false,
            \"consortium_signature\": null
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