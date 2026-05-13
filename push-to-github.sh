#!/bin/bash

# Script to push jatin-lean to GitHub with organized commits

set -e

echo "╔═══════════════════════════════════════════════════════════════════════╗"
echo "║                                                                       ║"
echo "║              📦 Pushing jatin-lean to GitHub 📦                      ║"
echo "║                                                                       ║"
echo "╚═══════════════════════════════════════════════════════════════════════╝"
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Initialize git if needed
if [ ! -d ".git" ]; then
    echo -e "${BLUE}Initializing git repository...${NC}"
    git init
    echo -e "${GREEN}✓ Git initialized${NC}"
fi

# Add remote if not exists
if ! git remote get-url origin &> /dev/null; then
    echo ""
    echo -e "${YELLOW}Enter your GitHub username (default: jatinjalandhra):${NC}"
    read -r github_user
    github_user=${github_user:-jatinjalandhra}
    
    git remote add origin "https://github.com/${github_user}/jatin-lean.git"
    echo -e "${GREEN}✓ Remote added${NC}"
    echo ""
    echo -e "${YELLOW}Make sure you've created the repository on GitHub:${NC}"
    echo -e "  https://github.com/${github_user}/jatin-lean"
    echo ""
    read -p "Press Enter when ready..."
fi

# Create .gitignore if it doesn't exist
if [ ! -f ".gitignore" ]; then
    cat > .gitignore << 'EOF'
# Rust
/target/
**/*.rs.bk
Cargo.lock

# npm
node_modules/
*.log
npm-debug.log*

# OS
.DS_Store
Thumbs.db

# IDE
.vscode/
.idea/
*.swp
*.swo

# Build artifacts
*.tgz
EOF
    echo -e "${GREEN}✓ Created .gitignore${NC}"
fi

echo ""
echo -e "${BLUE}Creating organized commits...${NC}"
echo ""

# Commit 1: Core Rust implementation
echo -e "${YELLOW}[1/8] Committing core Rust implementation...${NC}"
git add Cargo.toml
git add src/main.rs src/rules.rs src/scanner.rs src/tracer.rs src/deleter.rs src/display.rs
git add LICENSE
git commit -m "feat: initial Rust implementation

- Core CLI tool for pruning node_modules
- 7-category file classification system
- Parallel scanning with rayon
- Entry point whitelisting for safety
- Beautiful terminal UI with progress bars
- Dry-run mode by default" || echo "Already committed"

# Commit 2: Performance optimization
echo -e "${YELLOW}[2/8] Committing performance optimization...${NC}"
git add src/tracer.rs
git commit -m "perf: optimize dependency tracing

- Disabled expensive dependency tracing
- Entry points already whitelisted during scanning
- Reduces scan time from minutes to seconds
- Maintains safety through entry point protection" || echo "Already committed"

# Commit 3: CLI improvements
echo -e "${YELLOW}[3/8] Committing CLI improvements...${NC}"
git add src/display.rs src/deleter.rs src/main.rs Cargo.toml
git commit -m "feat: add author branding to CLI

- Display 'Created by Jatin Jalandhra' in banner
- Add author credit to all output modes
- Update package metadata with author name
- Improve success messages" || echo "Already committed"

# Commit 4: npm package structure
echo -e "${YELLOW}[4/8] Committing npm package...${NC}"
git add npm/package.json npm/install.js npm/index.js npm/bin/jatin-lean.js npm/.npmignore
git commit -m "feat: add npm package wrapper

- Post-install script for binary download
- CLI wrapper for npx support
- Cross-platform binary detection
- Automatic platform-specific downloads" || echo "Already committed"

# Commit 5: Documentation
echo -e "${YELLOW}[5/8] Committing documentation...${NC}"
git add README.md DEVELOPER.md npm/README.md
git commit -m "docs: add comprehensive documentation

- Main README with features and usage
- Developer documentation with architecture
- npm package README for users
- Installation and usage instructions" || echo "Already committed"

# Commit 6: User guide
echo -e "${YELLOW}[6/8] Committing user guide...${NC}"
git add HOW_TO_USE.md
git commit -m "docs: add complete user guide

- Quick start instructions
- All CLI options explained
- Common use cases and examples
- Docker and CI/CD integration
- Safety guidelines and troubleshooting
- Performance benchmarks" || echo "Already committed"

# Commit 7: GitHub Actions workflow
echo -e "${YELLOW}[7/8] Committing GitHub Actions...${NC}"
git add .github/workflows/release.yml
git commit -m "ci: add multi-platform build workflow

- Automatic builds for Linux, macOS, Windows
- Support for x64 and ARM64 architectures
- GitHub release creation with binaries
- Optional npm auto-publishing" || echo "Already committed"

# Commit 8: Setup scripts and guides
echo -e "${YELLOW}[8/8] Committing setup scripts...${NC}"
git add MULTI_PLATFORM_SETUP.md setup-multiplatform.sh
git add QUICK_REFERENCE.md PUBLISH_TO_NPM.md
git commit -m "docs: add setup guides and scripts

- Multi-platform setup guide
- Automated setup script
- Quick reference for commands
- Publishing guide for npm" || echo "Already committed"

# Push to GitHub
echo ""
echo -e "${BLUE}Pushing to GitHub...${NC}"
git branch -M main

if git push -u origin main; then
    echo ""
    echo "╔═══════════════════════════════════════════════════════════════════════╗"
    echo "║                                                                       ║"
    echo "║                     ✅ Successfully Pushed! ✅                        ║"
    echo "║                                                                       ║"
    echo "╚═══════════════════════════════════════════════════════════════════════╝"
    echo ""
    echo -e "${GREEN}Your code is now on GitHub!${NC}"
    echo ""
    echo -e "${BLUE}Repository:${NC} https://github.com/$(git remote get-url origin | sed 's/.*github.com[:/]\(.*\)\.git/\1/')"
    echo ""
    echo -e "${YELLOW}Next steps:${NC}"
    echo "1. Create a release tag to trigger multi-platform builds:"
    echo "   git tag v0.1.1"
    echo "   git push origin v0.1.1"
    echo ""
    echo "2. Wait for GitHub Actions to complete (~10-15 min)"
    echo ""
    echo "3. Update npm package:"
    echo "   cd npm"
    echo "   npm version 0.1.1 --no-git-tag-version"
    echo "   npm publish"
    echo ""
else
    echo ""
    echo -e "${YELLOW}⚠️  Push failed. This might be because:${NC}"
    echo "1. Repository doesn't exist on GitHub yet"
    echo "   Create it at: https://github.com/new"
    echo ""
    echo "2. Authentication failed"
    echo "   Set up SSH keys or use personal access token"
    echo ""
    echo "3. Branch protection rules"
    echo "   Check repository settings"
    echo ""
    exit 1
fi
