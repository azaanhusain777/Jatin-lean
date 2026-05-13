# 🚀 Quick Reference - jatin-lean

## Test Locally (Right Now!)

```bash
# Method 1: npm link
cd npm && npm link
jatin-lean --help

# Method 2: npx local
cd npm && npx . --help

# Method 3: Run test script
./test-npm-package.sh
```

## Publish to NPM

```bash
# Login
npm login

# Publish
cd npm && npm publish
```

## After Publishing

```bash
# Test it works
npx jatin-lean@latest --help

# Use in any project
cd /path/to/node/project
npx jatin-lean
npx jatin-lean --force
```

## Common Commands

```bash
# Dry run (safe preview)
npx jatin-lean

# Execute deletion
npx jatin-lean --force

# Verbose output
npx jatin-lean --verbose

# Scan multiple projects
npx jatin-lean ~/projects --global

# Help
npx jatin-lean --help
```

## Files to Update Before Publishing

1. `npm/package.json` - Change `your-username` to your GitHub username
2. `npm/install.js` - Update download URL with your username
3. `npm/README.md` - Update repository links

## GitHub Actions Setup

1. Push to GitHub
2. Add `NPM_TOKEN` secret (from npmjs.com)
3. Create tag: `git tag v0.1.0 && git push origin v0.1.0`
4. GitHub Actions builds and publishes automatically

## Troubleshooting

```bash
# Fix permissions
chmod +x npm/bin/jatin-lean.js
chmod +x npm/bin/jatin-lean

# Reinstall binary
cd npm && node install.js

# Check package
cd npm && npm pack --dry-run
```

## Key Features

- ✅ Reduces node_modules by up to 50%
- ✅ Safe by default (dry-run mode)
- ✅ Traces dependencies automatically
- ✅ Cross-platform (Linux, macOS, Windows)
- ✅ Fast (Rust + parallel scanning)
- ✅ Zero configuration

## Support

- Docs: See `NPM_SETUP_GUIDE.md` for detailed instructions
- Issues: GitHub issues
- License: MIT
