#!/bin/bash

# Multi-Platform Setup Script for jatin-lean
# This script helps you set up GitHub repository and create a release

set -e

echo "╔═══════════════════════════════════════════════════════════════════════╗"
echo "║                                                                       ║"
echo "║         🌍 jatin-lean Multi-Platform Setup 🌍                        ║"
echo "║                                                                       ║"
echo "╚═══════════════════════════════════════════════════════════════════════╝"
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if git is initialized
if [ ! -d ".git" ]; then
    echo -e "${YELLOW}Git not initialized. Initializing...${NC}"
    git init
    echo -e "${GREEN}✓ Git initialized${NC}"
fi

# Check if remote exists
if ! git remote get-url origin &> /dev/null; then
    echo ""
    echo -e "${BLUE}Setting up GitHub remote...${NC}"
    echo -e "${YELLOW}Enter your GitHub username (default: jatinjalandhra):${NC}"
    read -r github_user
    github_user=${github_user:-jatinjalandhra}
    
    git remote add origin "https://github.com/${github_user}/jatin-lean.git"
    echo -e "${GREEN}✓ Remote added: https://github.com/${github_user}/jatin-lean.git${NC}"
    echo ""
    echo -e "${YELLOW}Make sure you've created the repository on GitHub:${NC}"
    echo -e "  https://github.com/new"
    echo ""
    read -p "Press Enter when repository is created..."
fi

# Commit all changes
echo ""
echo -e "${BLUE}Committing changes...${NC}"
git add .
if git diff --cached --quiet; then
    echo -e "${YELLOW}No changes to commit${NC}"
else
    git commit -m "Setup multi-platform support" || true
    echo -e "${GREEN}✓ Changes committed${NC}"
fi

# Push to GitHub
echo ""
echo -e "${BLUE}Pushing to GitHub...${NC}"
git branch -M main
if git push -u origin main; then
    echo -e "${GREEN}✓ Pushed to GitHub${NC}"
else
    echo -e "${RED}✗ Push failed. Make sure the repository exists on GitHub${NC}"
    echo -e "${YELLOW}Create it at: https://github.com/new${NC}"
    exit 1
fi

# Create and push tag
echo ""
echo -e "${BLUE}Creating release tag...${NC}"
echo -e "${YELLOW}Enter version (default: v0.1.1):${NC}"
read -r version
version=${version:-v0.1.1}

if git tag -a "$version" -m "Release $version - Multi-platform support"; then
    echo -e "${GREEN}✓ Tag created: $version${NC}"
else
    echo -e "${YELLOW}Tag already exists, using existing tag${NC}"
fi

echo ""
echo -e "${BLUE}Pushing tag to GitHub...${NC}"
if git push origin "$version"; then
    echo -e "${GREEN}✓ Tag pushed to GitHub${NC}"
else
    echo -e "${RED}✗ Failed to push tag${NC}"
    exit 1
fi

echo ""
echo "╔═══════════════════════════════════════════════════════════════════════╗"
echo "║                                                                       ║"
echo "║                        ✅ Setup Complete! ✅                          ║"
echo "║                                                                       ║"
echo "╚═══════════════════════════════════════════════════════════════════════╝"
echo ""
echo -e "${GREEN}GitHub Actions is now building binaries for all platforms!${NC}"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "1. Go to: https://github.com/$(git remote get-url origin | sed 's/.*github.com[:/]\(.*\)\.git/\1/')/actions"
echo "2. Wait for the workflow to complete (~10-15 minutes)"
echo "3. Check releases: https://github.com/$(git remote get-url origin | sed 's/.*github.com[:/]\(.*\)\.git/\1/')/releases"
echo "4. Update npm package:"
echo "   cd npm"
echo "   npm version ${version#v} --no-git-tag-version"
echo "   npm publish"
echo ""
echo -e "${YELLOW}Optional: Add NPM_TOKEN to GitHub secrets for auto-publishing${NC}"
echo "  https://github.com/$(git remote get-url origin | sed 's/.*github.com[:/]\(.*\)\.git/\1/')/settings/secrets/actions"
echo ""
echo -e "${GREEN}🎉 Your tool will soon work on Linux, macOS, and Windows! 🎉${NC}"
echo ""
