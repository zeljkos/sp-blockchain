#!/bin/bash

# SP BCE Record Generator Script
# Updated with Phase 3 security hardening: authentication, authorization, and validation

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_URL="http://localhost"

# SP Provider configurations with API keys and port mappings
declare -A SP_PROVIDERS=(
    ["tmobile-de"]="8081:tmobile_api_key_2024_secure:T-Mobile-DE:262"
    ["vodafone-uk"]="8082:vodafone_api_key_2024_secure:Vodafone-UK:234"
    ["orange-fr"]="8083:orange_api_key_2024_secure:Orange-FR:208"
    ["telefonica-es"]="8084:telefonica_api_key_2024_secure:Telefónica-NO:242"
    ["sfr-fr"]="8085:sfr_api_key_2024_secure:SFR-FR:208"
)

# Usage information
usage() {
    echo "Usage: $0 [OPTIONS] PROVIDER_ID"
    echo ""
    echo "Generate and submit BCE records for SP consortium providers"
    echo ""
    echo "PROVIDER_ID:"
    echo "  tmobile-de    - T-Mobile Germany"
    echo "  vodafone-uk   - Vodafone United Kingdom"
    echo "  orange-fr     - Orange France"
    echo "  telefonica-es    - Telefónica Norway"
    echo "  sfr-fr        - SFR France"
    echo ""
    echo "OPTIONS:"
    echo "  -r, --record-id ID     Custom record ID (default: auto-generated)"
    echo "  -v, --visited PROVIDER Target visited provider"
    echo "  -c, --calls MINUTES    Call minutes (default: random 20-60)"
    echo "  -d, --data MB         Data usage in MB (default: random 500-2000)"
    echo "  -s, --sms COUNT       SMS count (default: random 1-15)"
    echo "  --with-signature      Include consortium signature (demo)"
    echo "  --dry-run            Show JSON without submitting"
    echo "  -h, --help           Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 tmobile-de                          # Generate T-Mobile record"
    echo "  $0 vodafone-uk -v orange-fr           # Vodafone→Orange roaming"
    echo "  $0 orange-fr -c 45 -d 1200 -s 8      # Custom usage amounts"
    echo "  $0 telefonica-es --with-signature        # Include signature"
    echo "  $0 sfr-fr --dry-run                   # Preview without submit"
}

# Show help if no arguments provided
if [[ $# -eq 0 ]]; then
    usage
    exit 0
fi

# Parse command line arguments
PROVIDER_ID=""
RECORD_ID=""
VISITED_PROVIDER=""
CALL_MINUTES=""
DATA_MB=""
SMS_COUNT=""
WITH_SIGNATURE=false
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -r|--record-id)
            RECORD_ID="$2"
            shift 2
            ;;
        -v|--visited)
            VISITED_PROVIDER="$2"
            shift 2
            ;;
        -c|--calls)
            CALL_MINUTES="$2"
            shift 2
            ;;
        -d|--data)
            DATA_MB="$2"
            shift 2
            ;;
        -s|--sms)
            SMS_COUNT="$2"
            shift 2
            ;;
        --with-signature)
            WITH_SIGNATURE=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        -*)
            echo "Unknown option $1"
            usage
            exit 1
            ;;
        *)
            if [[ -z "$PROVIDER_ID" ]]; then
                PROVIDER_ID="$1"
            else
                echo "Unexpected argument: $1"
                usage
                exit 1
            fi
            shift
            ;;
    esac
done

# Validate provider ID
if [[ -z "$PROVIDER_ID" ]]; then
    echo "Error: Provider ID is required"
    echo ""
    usage
    exit 1
fi

if [[ ! -v SP_PROVIDERS["$PROVIDER_ID"] ]]; then
    echo "Error: Unknown provider ID '$PROVIDER_ID'"
    echo "Valid providers: ${!SP_PROVIDERS[*]}"
    exit 1
fi

# Parse provider configuration
IFS=':' read -r PORT API_KEY PROVIDER_NAME MCC <<< "${SP_PROVIDERS[$PROVIDER_ID]}"

# Generate random visited provider if not specified
if [[ -z "$VISITED_PROVIDER" ]]; then
    PROVIDERS=(${!SP_PROVIDERS[*]})
    # Remove current provider from list
    PROVIDERS=(${PROVIDERS[@]/$PROVIDER_ID})
    VISITED_PROVIDER=${PROVIDERS[$RANDOM % ${#PROVIDERS[@]}]}
fi

# Parse visited provider info
if [[ ! -v SP_PROVIDERS["$VISITED_PROVIDER"] ]]; then
    echo "Error: Unknown visited provider '$VISITED_PROVIDER'"
    exit 1
fi

IFS=':' read -r _ _ VISITED_NAME _ <<< "${SP_PROVIDERS[$VISITED_PROVIDER]}"

# Generate usage data with realistic ranges
CALL_MINUTES=${CALL_MINUTES:-$((RANDOM % 41 + 20))}        # 20-60 minutes
DATA_MB=${DATA_MB:-$((RANDOM % 1501 + 500))}               # 500-2000 MB
SMS_COUNT=${SMS_COUNT:-$((RANDOM % 15 + 1))}               # 1-15 SMS

# Realistic rate structures (cents)
CALL_RATE_CENTS=$((RANDOM % 11 + 10))      # 10-20 cents/min
DATA_RATE_CENTS=$((RANDOM % 6 + 3))        # 3-8 cents/MB
SMS_RATE_CENTS=$((RANDOM % 8 + 8))         # 8-15 cents/SMS

# Calculate wholesale charge (must match validation logic)
WHOLESALE_CHARGE_CENTS=$(( (CALL_MINUTES * CALL_RATE_CENTS) + (DATA_MB * DATA_RATE_CENTS) + (SMS_COUNT * SMS_RATE_CENTS) ))

# Generate roaming data
ROAMING_MINUTES=$((RANDOM % 21 + 5))       # 5-25 minutes
ROAMING_DATA_MB=$((RANDOM % 301 + 100))    # 100-400 MB
ROAMING_RATE_CENTS=$((RANDOM % 16 + 20))   # 20-35 cents/min
ROAMING_DATA_RATE_CENTS=$((RANDOM % 6 + 6)) # 6-11 cents/MB

# Generate unique identifiers
TIMESTAMP=$(date +%s)
RECORD_ID=${RECORD_ID:-"$(echo $PROVIDER_NAME | cut -d'-' -f1 | tr '[:lower:]' '[:upper:]')-BCE-$(date +%Y%m%d%H%M%S)"}
IMSI="${MCC}01$(printf "%010d" $RANDOM)"
NETWORK_PAIR_HASH="$(echo $PROVIDER_NAME | cut -d'-' -f1)-$(echo $VISITED_NAME | cut -d'-' -f1)-$(date +%Y%m)"

# Build consortium signature if requested
CONSORTIUM_SIG="null"
if [[ "$WITH_SIGNATURE" == "true" ]]; then
    # Generate mock signature data for demo purposes
    SIGNATURE_DATA="[$(for i in {1..64}; do echo -n "$((RANDOM % 256))"; [[ $i -lt 64 ]] && echo -n ","; done)]"
    PUBLIC_KEY="[$(for i in {1..32}; do echo -n "$((RANDOM % 256))"; [[ $i -lt 32 ]] && echo -n ","; done)]"
    MESSAGE_HASH=$(printf "%064x" $((RANDOM * RANDOM)))

    CONSORTIUM_SIG=$(cat <<EOF
{
    "signer_id": "$PROVIDER_NAME",
    "signature_data": $SIGNATURE_DATA,
    "public_key": $PUBLIC_KEY,
    "message_hash": "$MESSAGE_HASH",
    "signature_type": "Ed25519"
}
EOF
)
fi

# Build BCE record JSON
BCE_RECORD=$(cat <<EOF
{
    "record_id": "$RECORD_ID",
    "imsi": "$IMSI",
    "home_operator": "$PROVIDER_NAME",
    "visited_operator": "$VISITED_NAME",
    "call_minutes": $CALL_MINUTES,
    "data_mb": $DATA_MB,
    "sms_count": $SMS_COUNT,
    "call_rate_cents": $CALL_RATE_CENTS,
    "data_rate_cents": $DATA_RATE_CENTS,
    "sms_rate_cents": $SMS_RATE_CENTS,
    "wholesale_charge_cents": $WHOLESALE_CHARGE_CENTS,
    "timestamp": $TIMESTAMP,
    "roaming_minutes": $ROAMING_MINUTES,
    "roaming_data_mb": $ROAMING_DATA_MB,
    "roaming_rate_cents": $ROAMING_RATE_CENTS,
    "roaming_data_rate_cents": $ROAMING_DATA_RATE_CENTS,
    "network_pair_hash": "$NETWORK_PAIR_HASH",
    "zkp_proof": null,
    "proof_verified": false,
    "consortium_signature": $CONSORTIUM_SIG
}
EOF
)

# Display record information
echo "🏢 Generating BCE Record for $PROVIDER_NAME"
echo "📱 IMSI: $IMSI"
echo "🔄 Roaming: $PROVIDER_NAME → $VISITED_NAME"
echo "📊 Usage: ${CALL_MINUTES}min calls, ${DATA_MB}MB data, ${SMS_COUNT} SMS"
echo "💰 Charge: €$(echo "scale=2; $WHOLESALE_CHARGE_CENTS / 100" | bc)"
echo "🔐 Auth: Bearer $API_KEY"
if [[ "$WITH_SIGNATURE" == "true" ]]; then
    echo "✍️  Signature: Included (demo)"
fi
echo ""

# Show JSON if dry run
if [[ "$DRY_RUN" == "true" ]]; then
    echo "📋 Generated BCE Record JSON:"
    echo "$BCE_RECORD" | jq '.'
    exit 0
fi

# Submit the record
echo "🚀 Submitting BCE record to ${BASE_URL}:${PORT}/api/v1/bce/submit"
echo ""

RESPONSE=$(curl -s -X POST "${BASE_URL}:${PORT}/api/v1/bce/submit" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $API_KEY" \
    -d "$BCE_RECORD")

# Parse and display response
echo "📨 Response:"
echo "$RESPONSE" | jq '.'

# Check success
SUCCESS=$(echo "$RESPONSE" | jq -r '.success // false')
if [[ "$SUCCESS" == "true" ]]; then
    echo ""
    echo "✅ BCE record successfully submitted!"
    echo "🆔 Record ID: $(echo "$RESPONSE" | jq -r '.data // "N/A"')"
else
    echo ""
    echo "❌ Failed to submit BCE record"
    MESSAGE=$(echo "$RESPONSE" | jq -r '.message // "Unknown error"')
    echo "💬 Error: $MESSAGE"
    exit 1
fi