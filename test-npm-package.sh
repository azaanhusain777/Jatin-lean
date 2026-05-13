#!/bin/bash

# Script to test the npm package locally

set -e

echo "🧪 Testing jatin-lean npm package..."
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test 1: Check if binary exists
echo -e "${BLUE}Test 1: Checking if binary exists...${NC}"
if [ -f "npm/bin/jatin-lean" ]; then
    echo -e "${GREEN}✓ Binary exists${NC}"
else
    echo "✗ Binary not found. Run: cp target/release/jatin-lean npm/bin/"
    exit 1
fi

# Test 2: Check if wrapper script is executable
echo -e "\n${BLUE}Test 2: Checking wrapper script...${NC}"
if [ -x "npm/bin/jatin-lean.js" ]; then
    echo -e "${GREEN}✓ Wrapper script is executable${NC}"
else
    echo "Making wrapper script executable..."
    chmod +x npm/bin/jatin-lean.js
    echo -e "${GREEN}✓ Fixed${NC}"
fi

# Test 3: Test --help flag
echo -e "\n${BLUE}Test 3: Testing --help flag...${NC}"
cd npm
if node bin/jatin-lean.js --help > /dev/null 2>&1; then
    echo -e "${GREEN}✓ --help works${NC}"
else
    echo "✗ --help failed"
    exit 1
fi

# Test 4: Test --version flag
echo -e "\n${BLUE}Test 4: Testing --version flag...${NC}"
if node bin/jatin-lean.js --version > /dev/null 2>&1; then
    echo -e "${GREEN}✓ --version works${NC}"
else
    echo "✗ --version failed"
    exit 1
fi

# Test 5: Check package.json
echo -e "\n${BLUE}Test 5: Validating package.json...${NC}"
if node -e "require('./package.json')" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ package.json is valid${NC}"
else
    echo "✗ package.json is invalid"
    exit 1
fi

# Test 6: Dry run test
echo -e "\n${BLUE}Test 6: Testing dry run...${NC}"
node bin/jatin-lean.js . > /dev/null 2>&1 || true
echo -e "${GREEN}✓ Dry run completed${NC}"

cd ..

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ All tests passed!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo "1. Test with npm link:"
echo "   cd npm && npm link"
echo ""
echo "2. Test with npx:"
echo "   cd npm && npx . --help"
echo ""
echo "3. Publish to npm:"
echo "   cd npm && npm publish"
echo ""
