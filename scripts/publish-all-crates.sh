#!/bin/bash

# LLM Orchestrator - Publish All Crates to crates.io
#
# Usage:
#   export CARGO_REGISTRY_TOKEN="your-token-here"
#   ./scripts/publish-all-crates.sh
#
# Or:
#   cargo login your-token-here
#   ./scripts/publish-all-crates.sh

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_step() {
    echo -e "${BLUE}==>${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    print_error "cargo not found. Please install Rust: https://rustup.rs/"
    exit 1
fi

# Check if logged in to crates.io
if [ -z "$CARGO_REGISTRY_TOKEN" ]; then
    if ! grep -q "token" ~/.cargo/credentials.toml 2>/dev/null; then
        print_error "Not logged in to crates.io!"
        echo ""
        echo "Please run one of:"
        echo "  export CARGO_REGISTRY_TOKEN=\"your-token-here\""
        echo "  cargo login your-token-here"
        echo ""
        echo "Get your token from: https://crates.io/settings/tokens"
        exit 1
    fi
fi

print_step "Starting crate publication process..."
echo ""

# Array of crates in dependency order
declare -a CRATES=(
    "llm-orchestrator-providers:providers (no dependencies)"
    "llm-orchestrator-state:state (independent)"
    "llm-orchestrator-auth:auth (independent)"
    "llm-orchestrator-secrets:secrets (independent)"
    "llm-orchestrator-audit:audit (independent)"
    "llm-orchestrator-core:core (depends on providers)"
    "llm-orchestrator-sdk:SDK (depends on core)"
    "llm-orchestrator-cli:CLI (depends on core)"
)

# Track published crates
PUBLISHED=()
FAILED=()

# Function to publish a single crate
publish_crate() {
    local crate_info=$1
    local crate_name="${crate_info%%:*}"
    local crate_desc="${crate_info##*:}"

    print_step "Publishing $crate_name ($crate_desc)..."

    # Find the crate directory
    local crate_dir="crates/$crate_name"

    if [ ! -d "$crate_dir" ]; then
        print_error "Directory not found: $crate_dir"
        FAILED+=("$crate_name")
        return 1
    fi

    cd "$crate_dir"

    # Dry run first
    print_step "  Running dry-run for $crate_name..."
    if ! cargo publish --dry-run 2>&1; then
        print_error "  Dry-run failed for $crate_name"
        cd ../..
        FAILED+=("$crate_name")
        return 1
    fi
    print_success "  Dry-run passed for $crate_name"

    # Actual publish
    print_step "  Publishing $crate_name to crates.io..."
    if cargo publish --allow-dirty 2>&1; then
        print_success "  Successfully published $crate_name"
        PUBLISHED+=("$crate_name")
        cd ../..
        return 0
    else
        print_error "  Failed to publish $crate_name"
        cd ../..
        FAILED+=("$crate_name")
        return 1
    fi
}

# Function to wait for indexing
wait_for_indexing() {
    local crate_name=$1
    local wait_time=${2:-180}  # Default 3 minutes

    print_warning "Waiting ${wait_time}s for crates.io to index $crate_name..."

    for ((i=wait_time; i>0; i-=10)); do
        echo -n "  ${i}s remaining..."
        sleep 10
        echo " ✓"
    done

    print_success "Indexing wait complete"
}

# Main publishing loop
echo "Publishing order:"
for crate_info in "${CRATES[@]}"; do
    crate_name="${crate_info%%:*}"
    crate_desc="${crate_info##*:}"
    echo "  - $crate_name ($crate_desc)"
done
echo ""

# Confirm before proceeding
read -p "Proceed with publishing? (y/N): " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_warning "Publishing cancelled by user"
    exit 0
fi

echo ""

# Publish crates
for i in "${!CRATES[@]}"; do
    crate_info="${CRATES[$i]}"
    crate_name="${crate_info%%:*}"

    if publish_crate "$crate_info"; then
        # Wait for indexing after certain crates
        if [ "$crate_name" = "llm-orchestrator-providers" ]; then
            wait_for_indexing "$crate_name" 180
        elif [ "$crate_name" = "llm-orchestrator-audit" ]; then
            wait_for_indexing "independent crates" 180
        elif [ "$crate_name" = "llm-orchestrator-core" ]; then
            wait_for_indexing "$crate_name" 180
        fi
    fi

    echo ""
done

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "PUBLISHING SUMMARY"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if [ ${#PUBLISHED[@]} -gt 0 ]; then
    print_success "Successfully published ${#PUBLISHED[@]} crates:"
    for crate in "${PUBLISHED[@]}"; do
        echo "  ✓ $crate"
        echo "    https://crates.io/crates/$crate"
    done
    echo ""
fi

if [ ${#FAILED[@]} -gt 0 ]; then
    print_error "Failed to publish ${#FAILED[@]} crates:"
    for crate in "${FAILED[@]}"; do
        echo "  ✗ $crate"
    done
    echo ""
    exit 1
fi

print_success "All crates published successfully! 🎉"
echo ""
echo "Next steps:"
echo "  1. Verify all crates on crates.io"
echo "  2. Test installation: cargo install llm-orchestrator-cli"
echo "  3. Create GitHub release: git tag v0.1.0 && git push --tags"
echo "  4. Update README with crates.io badges"
echo ""
