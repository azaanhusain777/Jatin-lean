#!/bin/bash
# Build binaries for all supported platforms
# This script is for local testing. CI/CD uses GitHub Actions.

set -e

echo "🔨 Building jatin-lean for all platforms..."
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create output directory
mkdir -p dist

# Function to build for a target
build_target() {
    local target=$1
    local output_name=$2
    
    echo -e "${BLUE}Building for $target...${NC}"
    
    if command -v cross &> /dev/null; then
        cross build --release --target "$target"
    else
        cargo build --release --target "$target"
    fi
    
    # Copy to dist directory
    if [[ "$target" == *"windows"* ]]; then
        cp "target/$target/release/jatin-lean.exe" "dist/$output_name.exe"
        echo -e "${GREEN}✓ Built: dist/$output_name.exe${NC}"
    else
        cp "target/$target/release/jatin-lean" "dist/$output_name"
        # Strip binary on Unix
        if command -v strip &> /dev/null; then
            strip "dist/$output_name"
        fi
        echo -e "${GREEN}✓ Built: dist/$output_name${NC}"
    fi
    
    echo ""
}

# Install targets if needed
echo "📦 Installing Rust targets..."
rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
rustup target add x86_64-pc-windows-gnu

echo ""
echo "🏗️  Building binaries..."
echo ""

# Build for each platform
build_target "x86_64-unknown-linux-gnu" "jatin-lean-linux-x64"
build_target "aarch64-unknown-linux-gnu" "jatin-lean-linux-arm64"
build_target "x86_64-apple-darwin" "jatin-lean-macos-x64"
build_target "aarch64-apple-darwin" "jatin-lean-macos-arm64"
build_target "x86_64-pc-windows-gnu" "jatin-lean-windows-x64"

echo ""
echo -e "${GREEN}✅ All builds complete!${NC}"
echo ""
echo "📦 Binaries are in the dist/ directory:"
ls -lh dist/
echo ""
echo "💡 To test locally:"
echo "   ./dist/jatin-lean-linux-x64 --version"
echo ""
echo "📤 To create a release:"
echo "   git tag v0.1.6"
echo "   git push origin v0.1.6"
echo ""
