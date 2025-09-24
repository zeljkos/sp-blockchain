#!/bin/bash

# SP Blockchain Data Reader Script
# Reads and displays blockchain data from RocksDB storage

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEFAULT_DATA_DIR="./data"

# SP Provider configurations
declare -A SP_PROVIDERS=(
    ["tmobile-de"]="T-Mobile-DE"
    ["vodafone-uk"]="Vodafone-UK"
    ["orange-fr"]="Orange-FR"
    ["telenor-no"]="Telenor-NO"
    ["sfr-fr"]="SFR-FR"
)

# SP Data directory mapping
declare -A SP_DATA_PATHS=(
    ["tmobile-de"]="$PROJECT_ROOT/docker/persistent_data/tmobile-de"
    ["vodafone-uk"]="$PROJECT_ROOT/docker/persistent_data/vodafone-uk"
    ["orange-fr"]="$PROJECT_ROOT/docker/persistent_data/orange-fr"
    ["telenor-no"]="$PROJECT_ROOT/docker/persistent_data/telenor-no"
    ["sfr-fr"]="$PROJECT_ROOT/docker/persistent_data/sfr-fr"
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
    echo "Read and display SP blockchain data from RocksDB storage"
    echo ""
    echo "DATA_TYPE:"
    echo "  blocks          - Display all blockchain blocks"
    echo "  bce-records     - Display BCE records"
    echo "  settlements     - Display settlement records"
    echo "  stats           - Display database statistics"
    echo "  chain-state     - Display current chain state"
    echo ""
    echo "SP_FILTER (optional):"
    echo "  tmobile-de      - Filter for T-Mobile Germany"
    echo "  vodafone-uk     - Filter for Vodafone United Kingdom"
    echo "  orange-fr       - Filter for Orange France"
    echo "  telenor-no      - Filter for Telenor Norway"
    echo "  sfr-fr          - Filter for SFR France"
    echo ""
    echo "OPTIONS:"
    echo "  -d, --data-dir DIR    Data directory (default: ./data)"
    echo "  -j, --json           Output in JSON format"
    echo "  -l, --limit N        Limit number of records displayed"
    echo "  -r, --raw           Show raw data without formatting"
    echo "  --no-color          Disable colored output"
    echo "  -h, --help          Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 stats                                    # Show database stats"
    echo "  $0 blocks -l 5                            # Show last 5 blocks"
    echo "  $0 bce-records tmobile-de                 # Show T-Mobile BCE records"
    echo "  $0 settlements -j                         # Show settlements in JSON"
    echo "  $0 blocks -d /custom/data --no-color     # Custom data dir, no colors"
}

# Show help if no arguments provided
if [[ $# -eq 0 ]]; then
    usage
    exit 0
fi

# Parse command line arguments
DATA_TYPE=""
SP_FILTER=""
DATA_DIR="$DEFAULT_DATA_DIR"
JSON_OUTPUT=false
LIMIT=""
RAW_OUTPUT=false
USE_COLORS=true

while [[ $# -gt 0 ]]; do
    case $1 in
        -d|--data-dir)
            DATA_DIR="$2"
            shift 2
            ;;
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

# Function to resolve data directory
resolve_data_directory() {
    local sp_filter="$1"
    local data_dir="$2"

    # If user specified a custom data directory, use it
    if [[ "$data_dir" != "$DEFAULT_DATA_DIR" ]]; then
        echo "$data_dir"
        return
    fi

    # If SP filter is provided, use the SP-specific data directory
    if [[ -n "$sp_filter" && -v SP_DATA_PATHS["$sp_filter"] ]]; then
        local sp_data_path="${SP_DATA_PATHS[$sp_filter]}"
        if [[ -d "$sp_data_path" ]]; then
            echo "$sp_data_path"
            return
        fi
    fi

    # Fallback to default
    echo "$data_dir"
}

# Resolve the actual data directory to use
RESOLVED_DATA_DIR=$(resolve_data_directory "$SP_FILTER" "$DATA_DIR")

# Check if data directory exists
if [[ ! -d "$RESOLVED_DATA_DIR" ]]; then
    if [[ -n "$SP_FILTER" && -v SP_DATA_PATHS["$SP_FILTER"] ]]; then
        echo -e "${RED}Error: Data directory for ${SP_PROVIDERS[$SP_FILTER]} not found at: '$RESOLVED_DATA_DIR'${NC}"
        echo -e "${YELLOW}Tip: Make sure the Docker containers have been run to generate data${NC}"
    else
        echo -e "${RED}Error: Data directory '$RESOLVED_DATA_DIR' does not exist${NC}"
    fi
    exit 1
fi

# Update DATA_DIR to the resolved path
DATA_DIR="$RESOLVED_DATA_DIR"

# Check if RocksDB files exist
if [[ ! -d "$DATA_DIR" ]] || [[ -z "$(ls -A "$DATA_DIR" 2>/dev/null)" ]]; then
    echo -e "${RED}Error: No RocksDB data found in '$DATA_DIR'${NC}"
    echo "Make sure the blockchain node has been run and data has been stored"
    exit 1
fi

# Header
echo -e "${CYAN}🔍 SP Blockchain Data Reader${NC}"
echo -e "${BLUE}📁 Data Directory: $DATA_DIR${NC}"
if [[ -n "$SP_FILTER" ]]; then
    echo -e "${BLUE}🏢 SP Filter: ${SP_PROVIDERS[$SP_FILTER]}${NC}"
fi
echo -e "${BLUE}📊 Data Type: $DATA_TYPE${NC}"
echo ""

# Function to check if RocksDB tools are available
check_rocksdb_tools() {
    if ! command -v ldb &> /dev/null && ! command -v rocksdb_ldb &> /dev/null; then
        echo -e "${YELLOW}⚠️  RocksDB LDB tool not found. Installing...${NC}"

        # Try to install rocksdb tools
        if command -v apt-get &> /dev/null; then
            sudo apt-get update && sudo apt-get install -y rocksdb-tools
        elif command -v yum &> /dev/null; then
            sudo yum install -y rocksdb-tools
        elif command -v brew &> /dev/null; then
            brew install rocksdb
        else
            echo -e "${RED}❌ Cannot install RocksDB tools automatically${NC}"
            echo "Please install rocksdb-tools manually:"
            echo "  Ubuntu/Debian: sudo apt-get install rocksdb-tools"
            echo "  RHEL/CentOS: sudo yum install rocksdb-tools"
            echo "  macOS: brew install rocksdb"
            exit 1
        fi
    fi
}

# Function to get LDB command
get_ldb_cmd() {
    if command -v ldb &> /dev/null; then
        echo "ldb"
    elif command -v rocksdb_ldb &> /dev/null; then
        echo "rocksdb_ldb"
    else
        echo ""
    fi
}

# Function to format timestamp
format_timestamp() {
    local timestamp=$1
    if [[ "$timestamp" =~ ^[0-9]+$ ]]; then
        # Unix timestamp
        date -d "@$timestamp" 2>/dev/null || date -r "$timestamp" 2>/dev/null || echo "$timestamp"
    else
        echo "$timestamp"
    fi
}

# Function to format JSON with colors
format_json() {
    local json_data="$1"

    if [[ "$JSON_OUTPUT" == "true" ]]; then
        echo "$json_data" | jq '.'
        return
    fi

    if [[ "$RAW_OUTPUT" == "true" ]]; then
        echo "$json_data"
        return
    fi

    if [[ "$USE_COLORS" == "true" ]] && command -v jq &> /dev/null; then
        echo "$json_data" | jq -C '.'
    elif command -v jq &> /dev/null; then
        echo "$json_data" | jq '.'
    else
        echo "$json_data"
    fi
}

# Function to display blocks
display_blocks() {
    echo -e "${GREEN}📦 Blockchain Blocks${NC}"
    echo "===================="

    local ldb_cmd=$(get_ldb_cmd)
    if [[ -z "$ldb_cmd" ]]; then
        check_rocksdb_tools
        ldb_cmd=$(get_ldb_cmd)
    fi

    # List all blocks
    local block_keys=$($ldb_cmd --db="$DATA_DIR" --column_family=settlement_blocks scan --key_hex=false | head -${LIMIT:-1000})

    if [[ -z "$block_keys" ]]; then
        echo -e "${YELLOW}No blocks found in database${NC}"
        return
    fi

    local count=0
    while IFS= read -r line; do
        if [[ -n "$line" && "$line" != *" ==> "* ]]; then
            local block_id=${line%% ==>*}
            echo -e "${PURPLE}Block ID: $block_id${NC}"

            # Get block data
            local block_data=$($ldb_cmd --db="$DATA_DIR" --column_family=settlement_blocks get "$block_id" --key_hex=false --value_hex=false 2>/dev/null)

            if [[ -n "$block_data" ]]; then
                format_json "$block_data"
            else
                echo -e "${RED}  Could not retrieve block data${NC}"
            fi
            echo ""

            ((count++))
            if [[ -n "$LIMIT" && $count -ge $LIMIT ]]; then
                break
            fi
        fi
    done <<< "$block_keys"
}

# Function to display BCE records
display_bce_records() {
    echo -e "${GREEN}📝 BCE Records${NC}"
    echo "==============="

    local ldb_cmd=$(get_ldb_cmd)
    if [[ -z "$ldb_cmd" ]]; then
        check_rocksdb_tools
        ldb_cmd=$(get_ldb_cmd)
    fi

    # List all BCE records
    local record_keys=$($ldb_cmd --db="$DATA_DIR" --column_family=bce_records scan --key_hex=false | head -${LIMIT:-1000})

    if [[ -z "$record_keys" ]]; then
        echo -e "${YELLOW}No BCE records found in database${NC}"
        return
    fi

    local count=0
    while IFS= read -r line; do
        if [[ -n "$line" && "$line" == *" ==> "* ]]; then
            local record_id=${line%% ==>*}
            local record_data=${line#*==> }

            # Filter by SP if specified
            if [[ -n "$SP_FILTER" ]]; then
                local sp_name="${SP_PROVIDERS[$SP_FILTER]}"
                if [[ "$record_data" != *"$sp_name"* ]]; then
                    continue
                fi
            fi

            echo -e "${PURPLE}Record ID: $record_id${NC}"
            format_json "$record_data"
            echo ""

            ((count++))
            if [[ -n "$LIMIT" && $count -ge $LIMIT ]]; then
                break
            fi
        fi
    done <<< "$record_keys"

    if [[ $count -eq 0 && -n "$SP_FILTER" ]]; then
        echo -e "${YELLOW}No BCE records found for ${SP_PROVIDERS[$SP_FILTER]}${NC}"
    fi
}

# Function to display settlements
display_settlements() {
    echo -e "${GREEN}💰 Settlement Records${NC}"
    echo "====================="

    local ldb_cmd=$(get_ldb_cmd)
    if [[ -z "$ldb_cmd" ]]; then
        check_rocksdb_tools
        ldb_cmd=$(get_ldb_cmd)
    fi

    # List all settlements
    local settlement_keys=$($ldb_cmd --db="$DATA_DIR" --column_family=settlements scan --key_hex=false | grep "^settlement:" | head -${LIMIT:-1000})

    if [[ -z "$settlement_keys" ]]; then
        echo -e "${YELLOW}No settlement records found in database${NC}"
        return
    fi

    local count=0
    while IFS= read -r line; do
        if [[ $line == settlement:* ]]; then
            local settlement_id=${line#settlement:}
            echo -e "${PURPLE}Settlement ID: $settlement_id${NC}"

            # Get settlement data
            local settlement_data=$($ldb_cmd --db="$DATA_DIR" --column_family=settlements get "settlement:$settlement_id" --key_hex=false --value_hex=false 2>/dev/null)

            if [[ -n "$settlement_data" ]]; then
                format_json "$settlement_data"
            else
                echo -e "${RED}  Could not retrieve settlement data${NC}"
            fi
            echo ""

            ((count++))
            if [[ -n "$LIMIT" && $count -ge $LIMIT ]]; then
                break
            fi
        fi
    done <<< "$settlement_keys"
}

# Function to display chain state
display_chain_state() {
    echo -e "${GREEN}🔗 Chain State${NC}"
    echo "==============="

    local ldb_cmd=$(get_ldb_cmd)
    if [[ -z "$ldb_cmd" ]]; then
        check_rocksdb_tools
        ldb_cmd=$(get_ldb_cmd)
    fi

    # Get current chain state
    local chain_state=$($ldb_cmd --db="$DATA_DIR" --column_family=chain_state get "current" --key_hex=false --value_hex=false 2>/dev/null)

    if [[ -n "$chain_state" ]]; then
        format_json "$chain_state"
    else
        echo -e "${YELLOW}No chain state found in database${NC}"
    fi
    echo ""
}

# Function to display database statistics
display_stats() {
    echo -e "${GREEN}📊 Database Statistics${NC}"
    echo "======================"

    local ldb_cmd=$(get_ldb_cmd)
    if [[ -z "$ldb_cmd" ]]; then
        check_rocksdb_tools
        ldb_cmd=$(get_ldb_cmd)
    fi

    # Count items in each column family
    local blocks_count=$($ldb_cmd --db="$DATA_DIR" --column_family=settlement_blocks scan --key_hex=false | wc -l || echo "0")
    local records_count=$($ldb_cmd --db="$DATA_DIR" --column_family=bce_records scan --key_hex=false | wc -l || echo "0")
    local settlements_count=0

    # Get database size
    local db_size_bytes=$(du -sb "$DATA_DIR" 2>/dev/null | cut -f1 || echo "0")
    local db_size_mb=$((db_size_bytes / 1024 / 1024))

    if [[ "$JSON_OUTPUT" == "true" ]]; then
        echo "{"
        echo "  \"total_blocks\": $blocks_count,"
        echo "  \"total_bce_records\": $records_count,"
        echo "  \"total_settlements\": $settlements_count,"
        echo "  \"database_size_bytes\": $db_size_bytes,"
        echo "  \"database_size_mb\": $db_size_mb"
        echo "}"
    else
        echo -e "${BLUE}📦 Total Blocks: ${YELLOW}$blocks_count${NC}"
        echo -e "${BLUE}📝 Total BCE Records: ${YELLOW}$records_count${NC}"
        echo -e "${BLUE}💰 Total Settlements: ${YELLOW}$settlements_count${NC}"
        echo -e "${BLUE}💾 Database Size: ${YELLOW}$db_size_mb MB${NC}"
    fi

    echo ""

    # Show chain state as well
    display_chain_state
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

echo -e "${CYAN}✅ Data retrieval completed${NC}"