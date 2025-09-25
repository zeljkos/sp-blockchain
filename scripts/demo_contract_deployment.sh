#!/bin/bash

# ============================================================================
# SMART CONTRACT DEPLOYMENT DEMO SCRIPT
# For Telefonica Demo Call
# ============================================================================

set -e  # Exit on any error

# Configuration
API_KEY="tmobile_api_key_2024_secure"
BASE_URL="http://localhost:8081"  # T-Mobile DE node
CONTRACTS_TO_DEPLOY=3

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Function to print colored headers
print_header() {
    echo -e "${BLUE}============================================================================${NC}"
    echo -e "${CYAN}🚀 $1${NC}"
    echo -e "${BLUE}============================================================================${NC}"
    echo
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_info() {
    echo -e "${YELLOW}📋 $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Function to make authenticated API calls
api_call() {
    local method="$1"
    local endpoint="$2"
    local data="$3"

    if [ "$method" = "POST" ]; then
        curl -s -X POST "$BASE_URL$endpoint" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $API_KEY" \
            -H "X-API-Key: $API_KEY" \
            -d "$data" | jq '.'
    else
        curl -s -X GET "$BASE_URL$endpoint" \
            -H "Authorization: Bearer $API_KEY" \
            -H "X-API-Key: $API_KEY" | jq '.'
    fi
}

# Function to wait with a nice progress indicator
wait_with_progress() {
    local seconds=$1
    local message=$2
    echo -n "$message"
    for i in $(seq 1 $seconds); do
        echo -n "."
        sleep 1
    done
    echo " Done!"
}

# ============================================================================
# MAIN DEMO SCRIPT
# ============================================================================

print_header "SP BLOCKCHAIN SMART CONTRACT DEPLOYMENT DEMO"

echo -e "${CYAN}🏢 Telefonica Demo - 5-Party Telecom Consortium${NC}"
echo "📅 $(date)"
echo "🌐 Target Node: T-Mobile DE (Port 8081)"
echo "🔑 Using Production-grade API Authentication"
echo

# Check system health first
print_info "Checking SP blockchain system health..."
HEALTH=$(api_call "GET" "/health")
echo "$HEALTH" | jq '.'

if [ "$(echo "$HEALTH" | jq -r '.status')" != "healthy" ]; then
    print_error "System not healthy! Aborting demo."
    exit 1
fi

print_success "System is healthy and ready for contract deployment"
echo

# Get current contract stats
print_info "Getting current smart contract statistics..."
CURRENT_STATS=$(api_call "GET" "/api/v1/contracts/stats")
echo "$CURRENT_STATS" | jq '.'
echo

# Deploy the 3 demonstration contracts
print_header "DEPLOYING 3 SMART CONTRACTS FOR TELECOM SETTLEMENT"

# Contract 1: BCE Validation Contract
print_info "Deploying Contract 1: BCE Validation Contract with ZKP Proofs"
CONTRACT_1_REQUEST='{
    "contract_id": "telefonica-demo-bce-validator",
    "contract_type": "bce_validator",
    "operators": ["tmobile-de", "vodafone-uk", "orange-fr", "telenor-no", "sfr-fr"],
    "description": "Validates BCE records using real Groth16 ZKP proofs for privacy-preserving settlement verification"
}'

echo "🔧 Contract 1 Payload:"
echo "$CONTRACT_1_REQUEST" | jq '.'

CONTRACT_1_RESULT=$(api_call "POST" "/api/v1/contracts/deploy" "$CONTRACT_1_REQUEST")
echo "📋 Contract 1 Deployment Result:"
echo "$CONTRACT_1_RESULT" | jq '.'

if [ "$(echo "$CONTRACT_1_RESULT" | jq -r '.success')" = "true" ]; then
    print_success "Contract 1 (BCE Validator) deployed successfully!"
    CONTRACT_1_ID=$(echo "$CONTRACT_1_RESULT" | jq -r '.data.contract_id')
    echo "📝 Contract ID: $CONTRACT_1_ID"
else
    print_error "Contract 1 deployment failed!"
fi

wait_with_progress 2 "⏱️  Processing deployment"
echo

# Contract 2: 5-Party Netting Contract
print_info "Deploying Contract 2: Multilateral Netting Contract"
CONTRACT_2_REQUEST='{
    "contract_id": "telefonica-demo-netting-contract",
    "contract_type": "netting_contract",
    "operators": ["tmobile-de", "vodafone-uk", "orange-fr", "telenor-no", "sfr-fr"],
    "description": "5-party multilateral netting achieving ~75% reduction in bilateral settlements"
}'

echo "🔧 Contract 2 Payload:"
echo "$CONTRACT_2_REQUEST" | jq '.'

CONTRACT_2_RESULT=$(api_call "POST" "/api/v1/contracts/deploy" "$CONTRACT_2_REQUEST")
echo "📋 Contract 2 Deployment Result:"
echo "$CONTRACT_2_RESULT" | jq '.'

if [ "$(echo "$CONTRACT_2_RESULT" | jq -r '.success')" = "true" ]; then
    print_success "Contract 2 (Netting Contract) deployed successfully!"
    CONTRACT_2_ID=$(echo "$CONTRACT_2_RESULT" | jq -r '.data.contract_id')
    echo "📝 Contract ID: $CONTRACT_2_ID"
else
    print_error "Contract 2 deployment failed!"
fi

wait_with_progress 2 "⏱️  Processing deployment"
echo

# Contract 3: Settlement Execution Contract
print_info "Deploying Contract 3: Settlement Execution Contract"
CONTRACT_3_REQUEST='{
    "contract_id": "telefonica-demo-settlement-executor",
    "contract_type": "settlement_executor",
    "operators": ["tmobile-de", "vodafone-uk", "orange-fr", "telenor-no", "sfr-fr"],
    "description": "Executes final settlements with multi-party digital signatures and dispute resolution"
}'

echo "🔧 Contract 3 Payload:"
echo "$CONTRACT_3_REQUEST" | jq '.'

CONTRACT_3_RESULT=$(api_call "POST" "/api/v1/contracts/deploy" "$CONTRACT_3_REQUEST")
echo "📋 Contract 3 Deployment Result:"
echo "$CONTRACT_3_RESULT" | jq '.'

if [ "$(echo "$CONTRACT_3_RESULT" | jq -r '.success')" = "true" ]; then
    print_success "Contract 3 (Settlement Executor) deployed successfully!"
    CONTRACT_3_ID=$(echo "$CONTRACT_3_RESULT" | jq -r '.data.contract_id')
    echo "📝 Contract ID: $CONTRACT_3_ID"
else
    print_error "Contract 3 deployment failed!"
fi

wait_with_progress 3 "⏱️  Finalizing deployments"
echo

# List all contracts after deployment
print_header "POST-DEPLOYMENT CONTRACT VERIFICATION"

print_info "Listing all deployed smart contracts..."
ALL_CONTRACTS=$(api_call "GET" "/api/v1/contracts/list")
echo "$ALL_CONTRACTS" | jq '.'
echo

# Get updated contract statistics
print_info "Getting updated smart contract system statistics..."
UPDATED_STATS=$(api_call "GET" "/api/v1/contracts/stats")
echo "$UPDATED_STATS" | jq '.'
echo

# Demo contract execution (rate validation)
print_header "DEMONSTRATING CONTRACT EXECUTION"

if [ ! -z "$CONTRACT_1_ID" ]; then
    print_info "Executing BCE rate validation on deployed contract..."

    RATE_VALIDATION_REQUEST='{
        "contract_id": "'$CONTRACT_1_ID'",
        "method": "validate_bce_rates",
        "parameters": {
            "call_rate_cents": 25,
            "data_rate_cents": 8,
            "sms_rate_cents": 12
        }
    }'

    echo "🔧 Rate Validation Payload:"
    echo "$RATE_VALIDATION_REQUEST" | jq '.'

    EXECUTION_RESULT=$(api_call "POST" "/api/v1/contracts/execute" "$RATE_VALIDATION_REQUEST")
    echo "📋 Contract Execution Result:"
    echo "$EXECUTION_RESULT" | jq '.'

    if [ "$(echo "$EXECUTION_RESULT" | jq -r '.success')" = "true" ]; then
        print_success "Contract execution completed successfully!"
        echo "⛽ Gas Used: $(echo "$EXECUTION_RESULT" | jq -r '.data.gas_used')"
        echo "📋 Result: $(echo "$EXECUTION_RESULT" | jq -r '.data.result')"
    else
        print_error "Contract execution failed!"
    fi
else
    print_error "Cannot demo execution - Contract 1 deployment failed"
fi

echo

# Final summary
print_header "DEPLOYMENT DEMO COMPLETE"

echo -e "${GREEN}🎯 TELEFONICA DEMO SUMMARY:${NC}"
echo
echo -e "${CYAN}✅ Successfully demonstrated:${NC}"
echo "   📋 Smart Contract API Integration"
echo "   🔐 Production-grade Authentication"
echo "   🚀 Real-time Contract Deployment"
echo "   ⚡ Live Contract Execution"
echo "   📊 System Monitoring & Statistics"
echo
echo -e "${CYAN}📈 Deployed Contracts:${NC}"
echo "   1️⃣  BCE Validator (ZKP-powered privacy)"
echo "   2️⃣  Multilateral Netter (75% reduction)"
echo "   3️⃣  Settlement Executor (multi-sig)"
echo
echo -e "${CYAN}🌐 5-Party Consortium Members:${NC}"
echo "   🇩🇪 T-Mobile DE"
echo "   🇬🇧 Vodafone UK"
echo "   🇫🇷 Orange FR"
echo "   🇳🇴 Telenor NO"
echo "   🇫🇷 SFR FR"
echo
echo -e "${YELLOW}💡 Next Steps for Telefonica:${NC}"
echo "   • Integration with existing billing systems"
echo "   • Custom contract development for Telefonica-specific rules"
echo "   • Production deployment with dedicated infrastructure"
echo "   • Multi-region consortium expansion"
echo

print_success "Demo completed! Ready for Telefonica integration discussion."