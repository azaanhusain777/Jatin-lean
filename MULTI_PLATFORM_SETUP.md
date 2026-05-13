# 🌍 Multi-Platform Setup Guide

This guide will help you set up automatic builds for Linux, macOS, and Windows.

---

## 📋 Prerequisites

1. ✅ GitHub repository created
2. ✅ npm package published (v0.1.0)
3. ✅ GitHub Actions workflow file ready (`.github/workflows/release.yml`)

---

## 🚀 Step-by-Step Setup

### Step 1: Create GitHub Repository

If you haven't already:

```bash
# Initialize git (if not done)
git init

# Add all files
git add .
git commit -m "Initial commit - jatin-lean v0.1.0"

# Create repo on GitHub, then:
git remote add origin https://github.com/jatinjalandhra/jatin-lean.git
git branch -M main
git push -u origin main
```

### Step 2: Update Repository URLs

Update these files with your actual GitHub username:

**1. npm/package.json:**
```json
{
  "repository": {
    "type": "git",
    "url": "https://github.com/jatinjalandhra/jatin-lean.git"
  },
  "bugs": {
    "url": "https://github.com/jatinjalandhra/jatin-lean/issues"
  },
  "homepage": "https://github.com/jatinjalandhra/jatin-lean#readme"
}
```

**2. npm/install.js:**
Already updated to use: `https://github.com/jatinjalandhra/jatin-lean/releases/...`

**3. npm/README.md:**
Update all `your-username` references to `jatinjalandhra`

### Step 3: Add NPM Token to GitHub Secrets (Optional)

If you want automatic npm publishing:

1. Go to https://www.npmjs.com/settings/YOUR_USERNAME/tokens
2. Click "Generate New Token" → "Automation"
3. Copy the token
4. Go to your GitHub repo → Settings → Secrets and variables → Actions
5. Click "New repository secret"
6. Name: `NPM_TOKEN`
7. Value: Paste your npm token
8. Click "Add secret"

### Step 4: Create a GitHub Release

This will trigger the workflow to build binaries for all platforms:

```bash
# Make sure all changes are committed
git add .
git commit -m "Setup multi-platform support"
git push

# Create and push a tag
git tag v0.1.1
git push origin v0.1.1
```

### Step 5: Monitor the Build

1. Go to your GitHub repo
2. Click "Actions" tab
3. You should see a workflow running called "Release"
4. It will build binaries for:
   - Linux x64
   - Linux ARM64
   - macOS x64 (Intel)
   - macOS ARM64 (Apple Silicon)
   - Windows x64

This takes about 10-15 minutes.

### Step 6: Verify the Release

Once the workflow completes:

1. Go to your repo → Releases
2. You should see "v0.1.1" with 5 binary files attached:
   - `jatin-lean-linux-x64`
   - `jatin-lean-linux-arm64`
   - `jatin-lean-macos-x64`
   - `jatin-lean-macos-arm64`
   - `jatin-lean-windows-x64.exe`

### Step 7: Update npm Package

Now update the npm package to version 0.1.1:

```bash
cd npm

# Update version
npm version 0.1.1 --no-git-tag-version

# Publish
npm publish
```

### Step 8: Test on Different Platforms

**On Linux:**
```bash
npx jatin-lean@latest --version
# Should download linux-x64 binary
```

**On macOS:**
```bash
npx jatin-lean@latest --version
# Should download macos-x64 or macos-arm64 binary
```

**On Windows:**
```bash
npx jatin-lean@latest --version
# Should download windows-x64.exe binary
```

---

## 🔧 Troubleshooting

### Workflow Fails

**Problem:** GitHub Actions workflow fails to build

**Common causes:**
1. Rust toolchain issues
2. Cross-compilation problems
3. Missing dependencies

**Solution:**
- Check the Actions logs for specific errors
- The workflow uses `cross` for cross-compilation which handles most issues
- For macOS builds, they run on actual macOS runners

### Binary Download Fails

**Problem:** `npm install` fails to download binary

**Causes:**
1. Release doesn't exist yet
2. Binary name mismatch
3. Network issues

**Solution:**
```bash
# Check if release exists
curl -I https://github.com/jatinjalandhra/jatin-lean/releases/download/v0.1.1/jatin-lean-linux-x64

# Should return 200 OK if it exists
```

### Wrong Binary Downloaded

**Problem:** Wrong platform binary is downloaded

**Solution:**
Check the platform detection in `install.js`:
```javascript
function getPlatformKey() {
  const platform = process.platform; // 'linux', 'darwin', 'win32'
  const arch = process.arch;         // 'x64', 'arm64'
  return `${platform}-${arch}`;
}
```

---

## 📝 Manual Build (Alternative)

If you don't want to use GitHub Actions, you can build manually:

### On Linux (for Linux):
```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

### On macOS (for macOS):
```bash
# Intel
cargo build --release --target x86_64-apple-darwin

# Apple Silicon
cargo build --release --target aarch64-apple-darwin
```

### On Windows (for Windows):
```bash
cargo build --release --target x86_64-pc-windows-gnu
```

### Cross-compilation (from Linux):
```bash
# Install cross
cargo install cross

# Build for all platforms
cross build --release --target x86_64-unknown-linux-gnu
cross build --release --target aarch64-unknown-linux-gnu
cross build --release --target x86_64-apple-darwin
cross build --release --target aarch64-apple-darwin
cross build --release --target x86_64-pc-windows-gnu
```

Then manually create a GitHub release and upload the binaries.

---

## 🎯 Quick Commands Reference

```bash
# Push code to GitHub
git add .
git commit -m "Multi-platform support"
git push

# Create release tag
git tag v0.1.1
git push origin v0.1.1

# Wait for GitHub Actions to complete (~10-15 min)

# Update npm package
cd npm
npm version 0.1.1 --no-git-tag-version
npm publish

# Test
npx jatin-lean@latest --version
```

---

## ✅ Checklist

Before creating a release:

- [ ] All code committed and pushed to GitHub
- [ ] Repository URLs updated in package.json and install.js
- [ ] NPM_TOKEN added to GitHub secrets (if auto-publishing)
- [ ] `.github/workflows/release.yml` file exists
- [ ] Ready to create a git tag

After release:

- [ ] GitHub Actions workflow completed successfully
- [ ] Release created with 5 binary files
- [ ] npm package updated to new version
- [ ] Tested on at least one platform

---

## 🎉 Success!

Once set up, every time you create a new tag (e.g., `v0.2.0`), GitHub Actions will automatically:

1. ✅ Build binaries for all platforms
2. ✅ Create a GitHub release
3. ✅ Upload all binaries
4. ✅ (Optional) Publish to npm

Users on any platform can then run:
```bash
npx jatin-lean
```

And it will automatically download the correct binary for their platform! 🚀

---

## 📞 Need Help?

- Check GitHub Actions logs for build errors
- Verify release exists: https://github.com/jatinjalandhra/jatin-lean/releases
- Test binary download manually with curl
- Open an issue if you're stuck

---

**Made with ❤️ by Jatin Jalandhra**
