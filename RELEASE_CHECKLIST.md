# Release Checklist — jatin-lean v0.1.6

**Use this checklist before releasing a new version**

---

## 📋 Pre-Release Checklist

### Code Quality
- [ ] All tests passing (`cargo test`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code properly formatted (`cargo fmt -- --check`)
- [ ] No compiler warnings
- [ ] Binary builds successfully (`cargo build --release`)

### Version Updates
- [ ] Version updated in `Cargo.toml`
- [ ] Version updated in `npm/package.json`
- [ ] CHANGELOG.md updated with release notes
- [ ] README.md updated if needed

### Testing
- [ ] Manual testing completed
- [ ] npm package tested locally (`./scripts/test-npm-package.sh`)
- [ ] Binary tested on target platform
- [ ] Configuration system tested
- [ ] Interactive prompts tested

### Documentation
- [ ] README.md is up to date
- [ ] DEVELOPER.md reflects current state
- [ ] All new features documented
- [ ] Examples are correct and working
- [ ] Links are valid

### GitHub
- [ ] All changes committed
- [ ] Working branch merged to main
- [ ] No pending pull requests
- [ ] Issues closed or updated

---

## 🚀 Release Process

### Step 1: Final Verification
```bash
# Run all checks
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check
cargo build --release

# Test binary
./target/release/jatin-lean --version
./target/release/jatin-lean --help

# Test npm package
./scripts/test-npm-package.sh
```

### Step 2: Commit and Tag
```bash
# Commit version bump
git add .
git commit -m "chore: bump version to 0.1.6"
git push origin main

# Create and push tag
git tag v0.1.6
git push origin v0.1.6
```

### Step 3: Monitor GitHub Actions
- [ ] Go to GitHub Actions tab
- [ ] Verify CI workflow passes
- [ ] Verify Release workflow starts
- [ ] Check all platform builds succeed
- [ ] Verify GitHub Release is created
- [ ] Verify binaries are attached
- [ ] Verify npm publish succeeds (if NPM_TOKEN is set)

### Step 4: Publish to crates.io
```bash
# Login to crates.io (first time only)
cargo login

# Publish
./scripts/publish-crates-io.sh

# Or manually
cargo publish
```

### Step 5: Verify Installations
```bash
# Test npm installation
npx jatin-lean@latest --version

# Test cargo installation (after crates.io publish)
cargo install jatin-lean
jatin-lean --version

# Test GitHub Release download
# Download binary from releases page and test
```

---

## 📝 Post-Release Checklist

### Verification
- [ ] npm package is live (https://www.npmjs.com/package/jatin-lean)
- [ ] crates.io package is live (https://crates.io/crates/jatin-lean)
- [ ] GitHub Release is published
- [ ] All binaries are downloadable
- [ ] Installation works on all platforms

### Documentation
- [ ] Release notes published
- [ ] CHANGELOG.md updated
- [ ] GitHub Release description is complete
- [ ] Documentation links are working

### Communication
- [ ] Announce on Twitter/X
- [ ] Post on Reddit (r/rust, r/node)
- [ ] Update project website (if any)
- [ ] Notify users/contributors
- [ ] Update badges in README

### Monitoring
- [ ] Monitor GitHub issues for bug reports
- [ ] Check npm download stats
- [ ] Check crates.io download stats
- [ ] Monitor CI/CD for failures

---

## 🐛 Rollback Plan

If something goes wrong:

### npm Rollback
```bash
# Deprecate version
npm deprecate jatin-lean@0.1.6 "This version has issues, use 0.1.5"

# Or unpublish (within 72 hours)
npm unpublish jatin-lean@0.1.6
```

### crates.io Rollback
```bash
# Yank version (doesn't delete, just marks as not recommended)
cargo yank --vers 0.1.6
```

### GitHub Release Rollback
- Delete the release from GitHub
- Delete the tag: `git tag -d v0.1.6 && git push origin :refs/tags/v0.1.6`

---

## 📊 Success Metrics

After 24 hours, check:
- [ ] npm downloads > 0
- [ ] crates.io downloads > 0
- [ ] GitHub stars increased
- [ ] No critical bug reports
- [ ] CI/CD still passing

After 1 week, check:
- [ ] npm downloads > 100
- [ ] crates.io downloads > 10
- [ ] User feedback collected
- [ ] Issues addressed

---

## 🔐 Required Secrets

Ensure these are set in GitHub repository:

- [ ] `GITHUB_TOKEN` — Automatically provided ✅
- [ ] `NPM_TOKEN` — Required for npm publishing
  - Get from: https://www.npmjs.com/settings/[username]/tokens
  - Add to: GitHub repo → Settings → Secrets → Actions

---

## 📞 Emergency Contacts

If something goes wrong:

- **GitHub Issues:** https://github.com/decodejatin/jatin-lean/issues
- **npm Support:** https://www.npmjs.com/support
- **crates.io Support:** https://crates.io/policies

---

## 📚 Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [npm Publishing Guide](https://docs.npmjs.com/cli/v8/commands/npm-publish)
- [crates.io Publishing Guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [Semantic Versioning](https://semver.org/)

---

## 🎯 Version Numbering

Follow Semantic Versioning (MAJOR.MINOR.PATCH):

- **MAJOR** (1.0.0) — Breaking changes
- **MINOR** (0.2.0) — New features, backward compatible
- **PATCH** (0.1.7) — Bug fixes, backward compatible

Current version: **0.1.6**

Next versions:
- Bug fix: 0.1.7
- New feature: 0.2.0
- Breaking change: 1.0.0

---

## ✅ Quick Checklist

Use this for quick reference:

```
Pre-Release:
□ Tests pass
□ Clippy clean
□ Formatted
□ Versions updated
□ CHANGELOG updated
□ Docs updated

Release:
□ Committed
□ Tagged
□ Pushed
□ CI passes
□ Release created
□ npm published
□ crates.io published

Post-Release:
□ Verified installations
□ Announced
□ Monitoring
```

---

**Made with ❤️ by Jatin Jalandhra**
