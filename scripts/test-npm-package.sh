#!/bin/bash
# Test the npm package locally before publishing

set -e

echo "🧪 Testing npm package locally..."
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Build the Rust binary
echo -e "${BLUE}Step 1: Building Rust binary...${NC}"
cargo build --release
echo -e "${GREEN}✓ Binary built${NC}"
echo ""

# Create npm bin directory
echo -e "${BLUE}Step 2: Preparing npm package...${NC}"
mkdir -p npm/bin
cp target/release/jatin-lean npm/bin/
chmod +x npm/bin/jatin-lean
echo -e "${GREEN}✓ Binary copied to npm/bin/${NC}"
echo ""

# Test the wrapper script
echo -e "${BLUE}Step 3: Testing wrapper script...${NC}"
cd npm
node bin/jatin-lean.js --version
node bin/jatin-lean.js --help | head -5
echo -e "${GREEN}✓ Wrapper script works${NC}"
echo ""

# Test npm install locally
echo -e "${BLUE}Step 4: Testing npm install...${NC}"
cd ..
mkdir -p test-install
cd test-install

# Create a test package.json
cat > package.json << 'EOF'
{
  "name": "test-jatin-lean",
  "version": "1.0.0",
  "private": true
}
EOF

# Install from local npm directory
npm install ../npm
echo -e "${GREEN}✓ npm install successful${NC}"
echo ""

# Test the installed package
echo -e "${BLUE}Step 5: Testing installed package...${NC}"
npx jatin-lean --version
npx jatin-lean --help | head -5
echo -e "${GREEN}✓ Installed package works${NC}"
echo ""

# Test with a real node_modules
echo -e "${BLUE}Step 6: Testing with real node_modules...${NC}"
npm install express
npx jatin-lean --verbose | head -20
echo -e "${GREEN}✓ Real-world test successful${NC}"
echo ""

# Cleanup
cd ..
rm -rf test-install

echo ""
echo -e "${GREEN}✅ All tests passed!${NC}"
echo ""
echo -e "${YELLOW}📦 Package is ready for publishing${NC}"
echo ""
echo "To publish to npm:"
echo "  1. cd npm"
echo "  2. npm login"
echo "  3. npm publish"
echo ""
