# 📦 Publishing Update to npm

## Changes Made

### Platform Support Clarification

✅ **Updated to clearly indicate Linux-only support**
- Updated `package.json` to specify `"os": ["linux"]` and `"cpu": ["x64"]`
- Updated `install.js` with better error messages for unsupported platforms
- Updated `README.md` to clearly show current platform support

### New Documentation

✅ **Created comprehensive user guide**
- `HOW_TO_USE.md` - Complete guide for end users
- Covers all use cases, examples, and troubleshooting

---

## How to Publish the Update

### Step 1: Update Version

```bash
cd npm

# Patch version (0.1.0 -> 0.1.1)
npm version patch
```

This will update `package.json` to version `0.1.1`

### Step 2: Verify Changes

```bash
# Check what will be published
npm pack --dry-run
```

### Step 3: Publish

```bash
npm publish
```

### Step 4: Verify

```bash
# Check the published version
npm view jatin-lean

# Test installation
npx jatin-lean@latest --version
```

---

## What Users Will See

### On Linux x64 ✅

```bash
$ npx jatin-lean
# Works perfectly!
```

### On macOS/Windows ⚠️

```bash
$ npm install -g jatin-lean

⚠️  Multi-platform binaries not yet available.
Currently, jatin-lean only works on Linux x64.

To use on other platforms:
1. Install Rust: https://rustup.rs/
2. Clone the repo: git clone https://github.com/your-username/jatin-lean.git
3. Build: cargo build --release
4. Copy binary to: /path/to/bin

Or wait for multi-platform release coming soon!
```

---

## Future: Multi-Platform Support

To add support for other platforms, you'll need to:

1. **Set up GitHub Actions** (already created in `.github/workflows/release.yml`)
2. **Create a GitHub release** with binaries for all platforms
3. **Update package.json** to include all platforms
4. **Publish new version**

The workflow will automatically:
- Build for Linux (x64, ARM64)
- Build for macOS (Intel, Apple Silicon)
- Build for Windows (x64)
- Create GitHub release with all binaries
- Publish to npm

---

## Quick Commands

```bash
# Update version
cd npm && npm version patch

# Publish
npm publish

# Test
npx jatin-lean@latest --version
```

---

**Current Status:**
- ✅ Published to npm
- ✅ Works on Linux x64
- ✅ Clear documentation
- ⏳ Multi-platform support coming soon
