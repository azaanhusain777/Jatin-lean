# NPM Package Setup Guide for jatin-lean

This guide explains how to use and publish the jatin-lean npm package.

## ✅ Package Structure Created

The npm package has been set up in the `npm/` directory with the following structure:

```
npm/
├── package.json          # Package metadata and dependencies
├── install.js            # Post-install script to download/setup binary
├── index.js              # Main entry point for programmatic usage
├── bin/
│   ├── jatin-lean.js     # CLI wrapper script
│   └── jatin-lean        # The actual Rust binary (copied during build)
├── README.md             # User-facing documentation
├── LICENSE               # MIT License
└── .npmignore            # Files to exclude from npm package
```

---

## 🚀 Local Testing (Before Publishing)

### Method 1: Test with npm link

```bash
# In the npm/ directory
cd npm
npm link

# Now you can use it globally
jatin-lean --help
jatin-lean --version

# Test in any Node.js project
cd /path/to/your/node/project
jatin-lean
jatin-lean --force

# Unlink when done
npm unlink -g jatin-lean
```

### Method 2: Test with npx locally

```bash
# In the npm/ directory
cd npm

# Test the package
npx . --help
npx . --version

# Test in a project directory
npx . /path/to/project
```

### Method 3: Install locally in a test project

```bash
# Create a test project
mkdir test-project
cd test-project
npm init -y
npm install express lodash

# Install jatin-lean from local path
npm install ../npm

# Test it
npx jatin-lean
npx jatin-lean --force
```

---

## 📦 Publishing to npm

### Prerequisites

1. **Create an npm account**: https://www.npmjs.com/signup
2. **Login to npm**:
   ```bash
   npm login
   ```

### Step 1: Prepare the Package

```bash
# Make sure you're in the npm directory
cd npm

# Update package.json with your details:
# - Change "your-username" to your GitHub username
# - Update author name
# - Verify version number

# Test the package locally first (see above)
```

### Step 2: Build Binaries for All Platforms

For a complete npm package, you need binaries for all platforms. You have two options:

#### Option A: Use GitHub Actions (Recommended)

Create `.github/workflows/release.yml` to automatically build binaries for all platforms when you create a release.

#### Option B: Manual Cross-Compilation

```bash
# Install cross-compilation tools
cargo install cross

# Build for different platforms
cross build --release --target x86_64-unknown-linux-gnu
cross build --release --target aarch64-unknown-linux-gnu
cross build --release --target x86_64-apple-darwin
cross build --release --target aarch64-apple-darwin
cross build --release --target x86_64-pc-windows-gnu

# Copy binaries to npm/bin/ with appropriate names
# (See PLATFORM_MAP in install.js for naming convention)
```

### Step 3: Publish to npm

```bash
cd npm

# Dry run to see what would be published
npm publish --dry-run

# Publish for real
npm publish

# If the package name is taken, you can use a scoped package:
# 1. Update package.json: "name": "@your-username/jatin-lean"
# 2. Publish: npm publish --access public
```

### Step 4: Verify Publication

```bash
# Check on npm
npm view jatin-lean

# Test installation
npx jatin-lean@latest --help

# Or install globally
npm install -g jatin-lean
jatin-lean --help
```

---

## 🎯 Usage After Publishing

Once published, users can use your package in several ways:

### 1. Using npx (No Installation)

```bash
# Run directly without installing
npx jatin-lean

# In any Node.js project
cd my-project
npx jatin-lean --force
```

### 2. Global Installation

```bash
# Install globally
npm install -g jatin-lean

# Use anywhere
jatin-lean
jatin-lean ~/projects --global
```

### 3. Local Project Installation

```bash
# Install as dev dependency
npm install --save-dev jatin-lean

# Use via npm scripts
npm exec jatin-lean
# or
npx jatin-lean
```

### 4. In package.json Scripts

```json
{
  "scripts": {
    "clean:modules": "jatin-lean --force",
    "analyze:modules": "jatin-lean --verbose"
  }
}
```

### 5. In Dockerfile

```dockerfile
FROM node:18-alpine

WORKDIR /app
COPY package*.json ./
RUN npm ci --production

# Prune node_modules to reduce image size
RUN npx jatin-lean --force

COPY . .
CMD ["npm", "start"]
```

---

## 🔄 Updating the Package

### Version Bump

```bash
cd npm

# Patch version (0.1.0 -> 0.1.1)
npm version patch

# Minor version (0.1.0 -> 0.2.0)
npm version minor

# Major version (0.1.0 -> 1.0.0)
npm version major

# Publish the new version
npm publish
```

### Update Process

1. Make changes to the Rust code
2. Rebuild the binary: `cargo build --release`
3. Copy new binary to npm/bin/
4. Update version in npm/package.json
5. Update CHANGELOG if you have one
6. Publish: `npm publish`

---

## 🏗️ GitHub Actions for Automated Releases

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact_name: jatin-lean-linux-x64
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            artifact_name: jatin-lean-linux-arm64
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact_name: jatin-lean-macos-x64
          - os: macos-latest
            target: aarch64-apple-darwin
            artifact_name: jatin-lean-macos-arm64
          - os: windows-latest
            target: x86_64-pc-windows-gnu
            artifact_name: jatin-lean-windows-x64.exe

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: ${{ matrix.target }}
          override: true

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: ${{ matrix.artifact_name }}
          path: target/${{ matrix.target }}/release/jatin-lean*

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Download artifacts
        uses: actions/download-artifact@v3
        with:
          path: artifacts

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: artifacts/**/*
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  publish-npm:
    needs: release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '18'
          registry-url: 'https://registry.npmjs.org'

      - name: Publish to npm
        run: |
          cd npm
          npm publish
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

---

## 📝 Current Status

✅ **Completed:**
- npm package structure created
- Binary wrapper scripts implemented
- Post-install script for binary download
- README and documentation
- Local testing ready

⏳ **Next Steps:**
1. Test locally using `npm link` or `npx`
2. Set up GitHub repository
3. Configure GitHub Actions for multi-platform builds
4. Publish to npm registry
5. Create GitHub releases with binaries

---

## 🧪 Testing Checklist

Before publishing, test these scenarios:

- [ ] `npm link` works and binary is accessible
- [ ] `npx .` runs the binary correctly
- [ ] `--help` flag shows help text
- [ ] `--version` flag shows version
- [ ] Dry run mode works (default behavior)
- [ ] `--force` mode actually deletes files
- [ ] `--verbose` mode shows file list
- [ ] `--global` mode scans multiple projects
- [ ] Works in a real Node.js project with node_modules
- [ ] Binary has correct permissions (executable)
- [ ] Package size is reasonable (check with `npm pack`)

---

## 📊 Package Size Optimization

The binary is ~2.9MB. To keep the npm package small:

1. **Don't include the binary in the package** - Download it post-install
2. **Use .npmignore** - Exclude unnecessary files
3. **Compress binaries** - Consider using UPX or similar

Current approach: Binary is downloaded during post-install from GitHub releases.

---

## 🔗 Useful Commands

```bash
# Check what will be published
npm pack --dry-run

# Create a tarball to inspect
npm pack

# Check package size
npm publish --dry-run

# View published package info
npm view jatin-lean

# Unpublish (within 72 hours)
npm unpublish jatin-lean@0.1.0

# Deprecate a version
npm deprecate jatin-lean@0.1.0 "Please upgrade to 0.2.0"
```

---

## 🎉 Success!

Your npm package is ready! Users can now run:

```bash
npx jatin-lean
```
`
And it will work in any Node.js project! 🚀
