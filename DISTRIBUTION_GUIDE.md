# Distribution Guide — jatin-lean

> Complete guide for distributing jatin-lean across multiple platforms

---

## 📦 Distribution Channels

jatin-lean is distributed through three main channels:

1. **npm** — For Node.js developers (`npx jatin-lean`)
2. **crates.io** — For Rust developers (`cargo install jatin-lean`)
3. **GitHub Releases** — Pre-built binaries for all platforms

---

## 🚀 Publishing Workflow

### Prerequisites

Before publishing, ensure you have:

- [x] All tests passing (`cargo test`)
- [x] Code formatted (`cargo fmt`)
- [x] No clippy warnings (`cargo clippy`)
- [x] Version bumped in `Cargo.toml` and `npm/package.json`
- [x] CHANGELOG updated
- [x] README updated
- [x] GitHub account with push access
- [x] npm account with publish rights
- [x] crates.io account with API token

---

## 📋 Step-by-Step Release Process

### Step 1: Prepare the Release

```bash
# 1. Update version numbers
# Edit Cargo.toml: version = "0.1.7"
# Edit npm/package.json: version = "0.1.7"

# 2. Update CHANGELOG.md
# Add release notes for v0.1.7

# 3. Run all tests
cargo test

# 4. Check formatting
cargo fmt -- --check

# 5. Run clippy
cargo clippy -- -D warnings

# 6. Build release binary
cargo build --release

# 7. Test the binary
./target/release/jatin-lean --version
./target/release/jatin-lean --help
```

### Step 2: Commit and Tag

```bash
# 1. Commit changes
git add .
git commit -m "chore: bump version to 0.1.7"

# 2. Create and push tag
git tag v0.1.7
git push origin main
git push origin v0.1.7
```

### Step 3: GitHub Actions (Automatic)

Once you push the tag, GitHub Actions will automatically:

1. ✅ Build binaries for all platforms:
   - Linux x64
   - Linux ARM64
   - macOS x64 (Intel)
   - macOS ARM64 (Apple Silicon)
   - Windows x64

2. ✅ Create a GitHub Release with:
   - Release notes (auto-generated)
   - All platform binaries attached

3. ✅ Publish to npm:
   - Automatically publishes the npm package
   - Requires `NPM_TOKEN` secret in GitHub

**GitHub Secrets Required:**
- `GITHUB_TOKEN` — Automatically provided
- `NPM_TOKEN` — Add your npm token to repository secrets

### Step 4: Publish to crates.io (Manual)

```bash
# Run the publish script
./scripts/publish-crates-io.sh

# Or manually:
cargo login
cargo publish
```

---

## 🧪 Testing Before Release

### Test npm Package Locally

```bash
# Run the test script
./scripts/test-npm-package.sh

# Or manually:
cargo build --release
mkdir -p npm/bin
cp target/release/jatin-lean npm/bin/
cd npm
npm test
```

### Test All Platform Builds

```bash
# Install cross for cross-compilation
cargo install cross

# Run the build script
./scripts/build-all-platforms.sh

# Binaries will be in dist/ directory
```

### Test in a Real Project

```bash
# Create a test project
mkdir test-project
cd test-project
npm init -y
npm install express lodash

# Test with local binary
../target/release/jatin-lean
../target/release/jatin-lean --force --yes
```

---

## 📦 npm Package Structure

```
npm/
├── bin/
│   └── jatin-lean.js       # Wrapper script
├── install.js              # Post-install script (downloads binary)
├── index.js                # Programmatic API (optional)
├── package.json            # npm package manifest
├── README.md               # npm package documentation
├── LICENSE                 # MIT license
└── .npmignore              # Files to exclude from npm
```

### How npm Package Works

1. User runs `npx jatin-lean` or `npm install jatin-lean`
2. npm runs `postinstall` script (`install.js`)
3. `install.js` downloads the appropriate binary from GitHub Releases
4. Binary is placed in `npm/bin/`
5. `bin/jatin-lean.js` wrapper executes the binary

---

## 🔧 GitHub Actions Workflows

### CI Workflow (`.github/workflows/ci.yml`)

Runs on every push and PR:
- ✅ Tests on Linux, macOS, Windows
- ✅ Clippy linting
- ✅ Format checking
- ✅ Build verification
- ✅ Code coverage
- ✅ Security audit
- ✅ npm package testing

### Release Workflow (`.github/workflows/release.yml`)

Runs on tag push (`v*`):
- ✅ Builds binaries for all platforms
- ✅ Creates GitHub Release
- ✅ Uploads binaries to release
- ✅ Publishes to npm

---

## 🌍 Platform Support

| Platform | Architecture | Binary Name | Status |
|----------|-------------|-------------|--------|
| Linux | x64 | `jatin-lean-linux-x64` | ✅ Supported |
| Linux | ARM64 | `jatin-lean-linux-arm64` | ✅ Supported |
| macOS | x64 (Intel) | `jatin-lean-macos-x64` | ✅ Supported |
| macOS | ARM64 (M1/M2) | `jatin-lean-macos-arm64` | ✅ Supported |
| Windows | x64 | `jatin-lean-windows-x64.exe` | ✅ Supported |

---

## 📊 Distribution Metrics

### Binary Sizes (Approximate)

| Platform | Size (Stripped) |
|----------|----------------|
| Linux x64 | ~3.1 MB |
| Linux ARM64 | ~3.2 MB |
| macOS x64 | ~3.0 MB |
| macOS ARM64 | ~2.9 MB |
| Windows x64 | ~3.3 MB |

### Download Locations

- **npm:** `https://www.npmjs.com/package/jatin-lean`
- **crates.io:** `https://crates.io/crates/jatin-lean`
- **GitHub:** `https://github.com/decodejatin/jatin-lean/releases`

---

## 🔐 Security Considerations

### Binary Verification

Users can verify binaries using checksums:

```bash
# Generate checksums (done automatically in CI)
sha256sum jatin-lean-linux-x64 > checksums.txt

# Verify download
sha256sum -c checksums.txt
```

### npm Package Security

- Binaries are downloaded from GitHub Releases (HTTPS)
- Checksums are verified during download
- Binary permissions are set to 755 (executable)

---

## 🐛 Troubleshooting

### npm Installation Fails

**Problem:** Binary download fails

**Solutions:**
1. Check internet connection
2. Verify GitHub Releases exist for the version
3. Try manual download from GitHub
4. Build from source

### Binary Not Executable

**Problem:** Permission denied

**Solution:**
```bash
chmod +x npm/bin/jatin-lean
```

### Wrong Platform Binary

**Problem:** Binary doesn't work on platform

**Solution:**
```bash
# Check platform
node -e "console.log(process.platform, process.arch)"

# Verify correct binary is downloaded
ls -la npm/bin/
```

---

## 📝 Checklist for New Release

- [ ] Update version in `Cargo.toml`
- [ ] Update version in `npm/package.json`
- [ ] Update `CHANGELOG.md`
- [ ] Run `cargo test`
- [ ] Run `cargo fmt -- --check`
- [ ] Run `cargo clippy -- -D warnings`
- [ ] Test locally with `./scripts/test-npm-package.sh`
- [ ] Commit changes
- [ ] Create and push tag
- [ ] Wait for GitHub Actions to complete
- [ ] Verify GitHub Release created
- [ ] Verify npm package published
- [ ] Publish to crates.io with `./scripts/publish-crates-io.sh`
- [ ] Test installation: `npx jatin-lean@latest --version`
- [ ] Test installation: `cargo install jatin-lean`
- [ ] Update documentation if needed
- [ ] Announce release (Twitter, Reddit, etc.)

---

## 🎯 Future Improvements

### Planned Distribution Enhancements

- [ ] Homebrew formula for macOS
- [ ] Debian/Ubuntu package (.deb)
- [ ] RPM package for Fedora/RHEL
- [ ] Chocolatey package for Windows
- [ ] Scoop package for Windows
- [ ] Docker image
- [ ] Snap package for Linux
- [ ] AUR package for Arch Linux

### Planned CI/CD Improvements

- [ ] Automated changelog generation
- [ ] Automated version bumping
- [ ] Release candidate builds
- [ ] Nightly builds
- [ ] Performance benchmarks in CI
- [ ] Integration tests with real projects

---

## 📚 Resources

### Documentation
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [npm Publishing Guide](https://docs.npmjs.com/cli/v8/commands/npm-publish)
- [crates.io Publishing Guide](https://doc.rust-lang.org/cargo/reference/publishing.html)

### Tools
- [cross](https://github.com/cross-rs/cross) — Cross-compilation tool
- [cargo-release](https://github.com/crate-ci/cargo-release) — Release automation
- [semantic-release](https://github.com/semantic-release/semantic-release) — Automated versioning

---

## 🤝 Contributing to Distribution

If you want to help improve the distribution process:

1. Test installation on your platform
2. Report issues with binary downloads
3. Suggest new distribution channels
4. Improve CI/CD workflows
5. Add platform-specific installers

---

**Made with ❤️ by Jatin Jalandhra**
