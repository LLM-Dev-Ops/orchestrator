#!/bin/bash

# Publish Remaining Unpublished Crates
# This script publishes only the crates that failed in the initial run

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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

# Check authentication
if [ -z "$CARGO_REGISTRY_TOKEN" ]; then
    if ! grep -q "token" ~/.cargo/credentials.toml 2>/dev/null; then
        print_error "Not logged in to crates.io!"
        echo "Please run: cargo login YOUR_TOKEN"
        exit 1
    fi
fi

print_step "Publishing remaining unpublished crates..."
echo ""

# Already published (skip these):
# - llm-orchestrator-state ✓
# - llm-orchestrator-auth ✓
# - llm-orchestrator-secrets ✓

# Remaining crates to publish in dependency order:
declare -a REMAINING_CRATES=(
    "llm-orchestrator-providers:providers"
    "llm-orchestrator-audit:audit"
    "llm-orchestrator-core:core (depends on providers, state)"
    "llm-orchestrator-sdk:SDK (depends on core, providers)"
    "llm-orchestrator-cli:CLI (depends on core, providers, SDK)"
)

PUBLISHED=()
FAILED=()

publish_crate() {
    local crate_info=$1
    local crate_name="${crate_info%%:*}"
    local crate_desc="${crate_info##*:}"

    print_step "Publishing $crate_name ($crate_desc)..."

    local crate_dir="crates/$crate_name"

    if [ ! -d "$crate_dir" ]; then
        print_error "Directory not found: $crate_dir"
        FAILED+=("$crate_name")
        return 1
    fi

    cd "$crate_dir"

    # Dry run
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

wait_for_indexing() {
    local crate_name=$1
    local wait_time=${2:-180}

    print_warning "Waiting ${wait_time}s for crates.io to index $crate_name..."

    for ((i=wait_time; i>0; i-=10)); do
        echo -n "  ${i}s remaining..."
        sleep 10
        echo " ✓"
    done

    print_success "Indexing wait complete"
}

# Show plan
echo "Remaining crates to publish:"
for crate_info in "${REMAINING_CRATES[@]}"; do
    crate_name="${crate_info%%:*}"
    crate_desc="${crate_info##*:}"
    echo "  - $crate_name ($crate_desc)"
done
echo ""

# Confirm
read -p "Proceed with publishing? (y/N): " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_warning "Publishing cancelled"
    exit 0
fi

echo ""

# Publish crates
for i in "${!REMAINING_CRATES[@]}"; do
    crate_info="${REMAINING_CRATES[$i]}"
    crate_name="${crate_info%%:*}"

    if publish_crate "$crate_info"; then
        # Wait after providers (needed for core)
        if [ "$crate_name" = "llm-orchestrator-providers" ]; then
            wait_for_indexing "$crate_name" 180
        # Wait after core (needed for SDK and CLI)
        elif [ "$crate_name" = "llm-orchestrator-core" ]; then
            wait_for_indexing "$crate_name" 180
        # Wait after SDK (needed for CLI)
        elif [ "$crate_name" = "llm-orchestrator-sdk" ]; then
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

print_success "All remaining crates published successfully! 🎉"
echo ""
echo "Previously published:"
echo "  ✓ llm-orchestrator-state"
echo "  ✓ llm-orchestrator-auth"
echo "  ✓ llm-orchestrator-secrets"
echo ""
echo "Newly published:"
for crate in "${PUBLISHED[@]}"; do
    echo "  ✓ $crate"
done
echo ""
echo "Total: 8/8 crates published ✅"
echo ""
echo "Next steps:"
echo "  1. Test installation: cargo install llm-orchestrator-cli"
echo "  2. Create GitHub release: git tag v0.1.0 && git push --tags"
echo "  3. Update README with crates.io badges"
echo ""
