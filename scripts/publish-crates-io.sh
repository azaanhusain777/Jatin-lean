#!/bin/bash
# Publish to crates.io

set -e

echo "📦 Publishing jatin-lean to crates.io..."
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if logged in
echo -e "${BLUE}Checking crates.io authentication...${NC}"
if ! cargo login --help &> /dev/null; then
    echo -e "${RED}❌ cargo not found${NC}"
    exit 1
fi

# Run tests
echo -e "${BLUE}Running tests...${NC}"
cargo test
echo -e "${GREEN}✓ All tests passed${NC}"
echo ""

# Check formatting
echo -e "${BLUE}Checking code formatting...${NC}"
cargo fmt -- --check
echo -e "${GREEN}✓ Code is properly formatted${NC}"
echo ""

# Run clippy
echo -e "${BLUE}Running clippy...${NC}"
cargo clippy -- -D warnings
echo -e "${GREEN}✓ No clippy warnings${NC}"
echo ""

# Build release
echo -e "${BLUE}Building release binary...${NC}"
cargo build --release
echo -e "${GREEN}✓ Release build successful${NC}"
echo ""

# Dry run
echo -e "${BLUE}Running publish dry-run...${NC}"
cargo publish --dry-run
echo -e "${GREEN}✓ Dry-run successful${NC}"
echo ""

# Confirm
echo -e "${YELLOW}⚠️  Ready to publish to crates.io${NC}"
echo ""
read -p "Do you want to continue? (y/N) " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled."
    exit 0
fi

# Publish
echo ""
echo -e "${BLUE}Publishing to crates.io...${NC}"
cargo publish

echo ""
echo -e "${GREEN}✅ Successfully published to crates.io!${NC}"
echo ""
echo "🎉 Package is now available at:"
echo "   https://crates.io/crates/jatin-lean"
echo ""
echo "Users can install with:"
echo "   cargo install jatin-lean"
echo ""
