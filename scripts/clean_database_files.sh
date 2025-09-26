#!/bin/bash

# ============================================================================
# SP Blockchain Database Cleanup Script
# Deletes all database files but preserves SP directory structure
# ============================================================================

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${BLUE}============================================================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}============================================================================${NC}"
    echo
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Base persistent data directory
PERSISTENT_DATA_DIR="/home/zeljko/src/sp-blockchain/docker/persistent_data"

# SP node directories
SP_NODES=("tmobile-de" "vodafone-uk" "orange-fr" "telefonica-es" "sfr-fr")

print_header "SP BLOCKCHAIN DATABASE CLEANUP"

echo "This script will clean all database files while preserving SP directory structure."
echo "Location: $PERSISTENT_DATA_DIR"
echo
echo "SP Nodes to clean:"
for node in "${SP_NODES[@]}"; do
    echo "  • $node"
done
echo

# Check if persistent data directory exists
if [ ! -d "$PERSISTENT_DATA_DIR" ]; then
    print_error "Persistent data directory not found: $PERSISTENT_DATA_DIR"
    exit 1
fi

# Confirmation prompt
read -p "Are you sure you want to delete all database files? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_warning "Operation cancelled"
    exit 0
fi

print_header "CLEANING DATABASE FILES"

# Clean each SP node directory
for node in "${SP_NODES[@]}"; do
    node_dir="$PERSISTENT_DATA_DIR/$node"

    if [ -d "$node_dir" ]; then
        echo "Cleaning $node directory..."

        # Count files before deletion
        file_count=$(find "$node_dir" -type f 2>/dev/null | wc -l)

        if [ "$file_count" -gt 0 ]; then
            # Delete all files but keep directories
            find "$node_dir" -type f -delete 2>/dev/null || true
            print_success "Deleted $file_count files from $node"
        else
            print_warning "$node directory is already empty"
        fi

        # Ensure directory still exists (in case it was empty and removed)
        mkdir -p "$node_dir"
    else
        print_warning "$node directory does not exist, creating it..."
        mkdir -p "$node_dir"
        print_success "Created $node directory"
    fi
done

print_header "CLEANUP COMPLETE"

echo "Database cleanup summary:"
for node in "${SP_NODES[@]}"; do
    node_dir="$PERSISTENT_DATA_DIR/$node"
    if [ -d "$node_dir" ]; then
        file_count=$(find "$node_dir" -type f 2>/dev/null | wc -l)
        if [ "$file_count" -eq 0 ]; then
            print_success "$node: Clean (0 files)"
        else
            print_warning "$node: $file_count files remaining"
        fi
    fi
done

echo
print_success "All SP blockchain databases have been cleaned!"
print_success "Directory structure preserved for all SP nodes"
echo
echo "You can now restart the containers with clean databases:"
echo "  cd /home/zeljko/src/sp-blockchain/docker"
echo "  docker-compose up -d"