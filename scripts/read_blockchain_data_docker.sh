#!/bin/bash

# SP Blockchain Data Reader Script - Docker Version
# Reads blockchain data from RocksDB storage using Docker containers

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# SP Provider configurations
declare -A SP_PROVIDERS=(
    ["tmobile-de"]="T-Mobile-DE"
    ["vodafone-uk"]="Vodafone-UK"
    ["orange-fr"]="Orange-FR"
    ["telenor-no"]="Telenor-NO"
    ["sfr-fr"]="SFR-FR"
)

# SP Container names
declare -A SP_CONTAINERS=(
    ["tmobile-de"]="sp-tmobile-de"
    ["vodafone-uk"]="sp-vodafone-uk"
    ["orange-fr"]="sp-orange-fr"
    ["telenor-no"]="sp-telenor-no"
    ["sfr-fr"]="sp-sfr-fr"
)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Usage information
usage() {
    echo "Usage: $0 [OPTIONS] DATA_TYPE [SP_FILTER]"
    echo ""
    echo "Read and display SP blockchain data from RocksDB storage using Docker containers"
    echo ""
    echo "DATA_TYPE:"
    echo "  blocks          - Display all blockchain blocks"
    echo "  bce-records     - Display BCE records"
    echo "  settlements     - Display settlement records"
    echo "  stats           - Display database statistics"
    echo "  chain-state     - Display current chain state"
    echo ""
    echo "SP_FILTER (required for Docker version):"
    echo "  tmobile-de      - T-Mobile Germany"
    echo "  vodafone-uk     - Vodafone United Kingdom"
    echo "  orange-fr       - Orange France"
    echo "  telenor-no      - Telenor Norway"
    echo "  sfr-fr          - SFR France"
    echo ""
    echo "OPTIONS:"
    echo "  -j, --json           Output in JSON format"
    echo "  -l, --limit N        Limit number of records displayed"
    echo "  -r, --raw           Show raw data without formatting"
    echo "  --no-color          Disable colored output"
    echo "  -h, --help          Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 stats tmobile-de                    # Show T-Mobile stats"
    echo "  $0 blocks vodafone-uk -l 5            # Show last 5 Vodafone blocks"
    echo "  $0 bce-records tmobile-de             # Show T-Mobile BCE records"
    echo "  $0 settlements orange-fr -j          # Show Orange settlements in JSON"
}

# Show help if no arguments provided
if [[ $# -eq 0 ]]; then
    usage
    exit 0
fi

# Parse command line arguments
DATA_TYPE=""
SP_FILTER=""
JSON_OUTPUT=false
LIMIT=""
RAW_OUTPUT=false
USE_COLORS=true

while [[ $# -gt 0 ]]; do
    case $1 in
        -j|--json)
            JSON_OUTPUT=true
            shift
            ;;
        -l|--limit)
            LIMIT="$2"
            shift 2
            ;;
        -r|--raw)
            RAW_OUTPUT=true
            shift
            ;;
        --no-color)
            USE_COLORS=false
            RED=""
            GREEN=""
            YELLOW=""
            BLUE=""
            PURPLE=""
            CYAN=""
            NC=""
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        blocks|bce-records|settlements|stats|chain-state)
            if [[ -z "$DATA_TYPE" ]]; then
                DATA_TYPE="$1"
            else
                echo "Error: Multiple data types specified"
                usage
                exit 1
            fi
            shift
            ;;
        tmobile-de|vodafone-uk|orange-fr|telenor-no|sfr-fr)
            if [[ -z "$SP_FILTER" ]]; then
                SP_FILTER="$1"
            else
                echo "Error: Multiple SP filters specified"
                usage
                exit 1
            fi
            shift
            ;;
        -*)
            echo "Unknown option $1"
            usage
            exit 1
            ;;
        *)
            echo "Unexpected argument: $1"
            usage
            exit 1
            ;;
    esac
done

# Validate required parameters
if [[ -z "$DATA_TYPE" ]]; then
    echo "Error: Data type is required"
    echo ""
    usage
    exit 1
fi

if [[ -z "$SP_FILTER" ]]; then
    echo "Error: SP filter is required for Docker version"
    echo ""
    usage
    exit 1
fi

# Get container name
CONTAINER_NAME="${SP_CONTAINERS[$SP_FILTER]}"
if [[ -z "$CONTAINER_NAME" ]]; then
    echo -e "${RED}Error: Unknown SP filter '$SP_FILTER'${NC}"
    exit 1
fi

# Check if container is running
if ! docker ps --format "table {{.Names}}" | grep -q "^$CONTAINER_NAME$"; then
    echo -e "${RED}Error: Container '$CONTAINER_NAME' is not running${NC}"
    echo -e "${YELLOW}Start the SP consortium with: cd docker && docker-compose up${NC}"
    exit 1
fi

# Header
echo -e "${CYAN}🔍 SP Blockchain Data Reader (Docker)${NC}"
echo -e "${BLUE}🐳 Container: $CONTAINER_NAME${NC}"
echo -e "${BLUE}🏢 SP: ${SP_PROVIDERS[$SP_FILTER]}${NC}"
echo -e "${BLUE}📊 Data Type: $DATA_TYPE${NC}"
echo ""

# Function to format JSON with colors
format_json() {
    local json_data="$1"

    if [[ "$JSON_OUTPUT" == "true" ]]; then
        echo "$json_data" | jq '.' 2>/dev/null || echo "$json_data"
        return
    fi

    if [[ "$RAW_OUTPUT" == "true" ]]; then
        echo "$json_data"
        return
    fi

    if [[ "$USE_COLORS" == "true" ]] && command -v jq &> /dev/null; then
        echo "$json_data" | jq -C '.' 2>/dev/null || echo "$json_data"
    elif command -v jq &> /dev/null; then
        echo "$json_data" | jq '.' 2>/dev/null || echo "$json_data"
    else
        echo "$json_data"
    fi
}

# Function to execute command in container
docker_exec() {
    local cmd="$1"
    docker exec -i "$CONTAINER_NAME" sh -c "$cmd" 2>/dev/null
}

# Function to execute authenticated API call
docker_api_call() {
    local endpoint="$1"
    local api_key="${SP_PROVIDERS[$SP_FILTER]}"
    IFS=':' read -r PORT API_KEY PROVIDER_NAME MCC <<< "$api_key"
    docker exec -i "$CONTAINER_NAME" sh -c "curl -s -H 'Authorization: Bearer $API_KEY' http://localhost:8080$endpoint 2>/dev/null"
}

# Function to display blocks
display_blocks() {
    echo -e "${GREEN}📦 Blockchain Blocks${NC}"
    echo "===================="

    # Use the blockchain blocks API
    local response=$(docker_exec "curl -s http://localhost:8080/api/v1/blockchain/blocks 2>/dev/null")

    if [[ -n "$response" && "$response" != "null" ]]; then
        format_json "$response"
    else
        echo -e "${YELLOW}No blocks found or API not available${NC}"
    fi
}

# Function to display BCE records
display_bce_records() {
    echo -e "${GREEN}📝 BCE Records${NC}"
    echo "==============="

    # Try the new authenticated read API first
    local response=$(docker_api_call "/api/v1/read/bce_records")

    # Check if we got a proper response
    if [[ -n "$response" && "$response" != "null" && "$response" != *"401"* && "$response" != *"404"* ]]; then
        format_json "$response"
    else
        # Fall back to storage/list debug endpoint if available
        echo -e "${YELLOW}⚠️ /api/v1/read/bce_records endpoint not available, using fallback${NC}"
        response=$(docker_exec "curl -s http://localhost:8080/api/v1/storage/list 2>/dev/null")

        # If that also fails, try BCE stats
        if [[ -z "$response" || "$response" == *"404"* ]]; then
            echo -e "${YELLOW}⚠️ Using BCE stats as fallback${NC}"
            response=$(docker_exec "curl -s http://localhost:8080/api/v1/bce/stats 2>/dev/null")
        fi

        if [[ -n "$response" && "$response" != "null" ]]; then
            format_json "$response"
        else
            echo -e "${YELLOW}No BCE records found or API not available${NC}"
        fi
    fi
}

# Function to display settlements
display_settlements() {
    echo -e "${GREEN}💰 Settlement Records${NC}"
    echo "====================="

    # Try the new authenticated read API first for settlement blocks
    local response=$(docker_api_call "/api/v1/read/settlement_blocks")

    # Check if we got a proper response
    if [[ -n "$response" && "$response" != "null" && "$response" != *"401"* && "$response" != *"404"* ]]; then
        format_json "$response"
    else
        # Fall back to settlements list API if available
        echo -e "${YELLOW}⚠️ /api/v1/read/settlement_blocks endpoint not available, using fallback${NC}"
        response=$(docker_exec "curl -s http://localhost:8080/api/v1/settlements 2>/dev/null")

        # If that also fails, try ZKP stats
        if [[ -z "$response" || "$response" == *"404"* ]]; then
            echo -e "${YELLOW}⚠️ Using ZKP stats as fallback${NC}"
            response=$(docker_exec "curl -s http://localhost:8080/api/v1/zkp/stats 2>/dev/null")
        fi

        if [[ -n "$response" && "$response" != "null" ]]; then
            format_json "$response"
        else
            echo -e "${YELLOW}No settlement records found or API not available${NC}"
        fi
    fi
}

# Function to display chain state
display_chain_state() {
    echo -e "${GREEN}🔗 Chain State${NC}"
    echo "==============="

    local response=$(docker_exec "curl -s http://localhost:8080/api/v1/blockchain/stats 2>/dev/null")

    if [[ -n "$response" ]]; then
        format_json "$response"
    else
        echo -e "${YELLOW}No chain state found or API not available${NC}"
    fi
}

# Function to display database statistics
display_stats() {
    echo -e "${GREEN}📊 Database Statistics${NC}"
    echo "======================"

    # Get comprehensive stats from multiple endpoints
    echo -e "${PURPLE}Health Status:${NC}"
    local health=$(docker_exec "curl -s http://localhost:8080/health 2>/dev/null")
    if [[ -n "$health" ]]; then
        format_json "$health"
    fi

    echo ""
    echo -e "${PURPLE}Blockchain Stats:${NC}"
    local blockchain_stats=$(docker_exec "curl -s http://localhost:8080/api/v1/blockchain/stats 2>/dev/null")
    if [[ -n "$blockchain_stats" ]]; then
        format_json "$blockchain_stats"
    fi

    echo ""
    echo -e "${PURPLE}BCE Stats:${NC}"
    local bce_stats=$(docker_exec "curl -s http://localhost:8080/api/v1/bce/stats 2>/dev/null")
    if [[ -n "$bce_stats" ]]; then
        format_json "$bce_stats"
    fi

    echo ""
    echo -e "${PURPLE}ZKP Stats:${NC}"
    local zkp_stats=$(docker_exec "curl -s http://localhost:8080/api/v1/zkp/stats 2>/dev/null")
    if [[ -n "$zkp_stats" ]]; then
        format_json "$zkp_stats"
    fi
}

# Execute based on data type
case $DATA_TYPE in
    "blocks")
        display_blocks
        ;;
    "bce-records")
        display_bce_records
        ;;
    "settlements")
        display_settlements
        ;;
    "chain-state")
        display_chain_state
        ;;
    "stats")
        display_stats
        ;;
    *)
        echo -e "${RED}Error: Unknown data type '$DATA_TYPE'${NC}"
        usage
        exit 1
        ;;
esac

echo ""
echo -e "${CYAN}✅ Data retrieval completed${NC}"