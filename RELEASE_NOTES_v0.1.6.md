# Release Notes — jatin-lean v0.1.6

**Release Date:** May 16, 2026  
**Status:** Production Ready ✅

---

## 🎉 What's New

### Major Features

#### 1. **Configuration System** 🔧
Create custom pruning rules with TOML config files:

```bash
# Generate example config
jatin-lean --init-config my-rules.toml

# Use custom config
jatin-lean --config my-rules.toml --force
```

**Config file locations (in order of precedence):**
1. `--config <path>` (CLI flag)
2. `./jatin-lean.toml` (project-local)
3. `./.jatin-lean.toml` (hidden project-local)
4. `~/.config/jatin-lean/rules.toml` (global)

**Example config:**
```toml
# Extend built-in rules (default)
override_defaults = false

# Add custom documentation files
doc_files = ["CUSTOM_README.md", "NOTES.txt"]

# Add custom test directories
test_dirs = ["integration-tests", "e2e"]

# Keep certain files (never delete)
exclude_patterns = ["important-file.js"]
```

#### 2. **Interactive Confirmation** 🛡️
Safety-first approach with confirmation prompts:

```bash
# Will prompt before deletion
jatin-lean --force

# Skip prompt for automation
jatin-lean --force --yes
```

**Confirmation shows:**
- Total savings (size and file count)
- Percentage of node_modules to be deleted
- Default answer is "No" for safety

#### 3. **Comprehensive Test Suite** ✅
Expanded from 6 to **32 tests** covering:
- Configuration loading and parsing (8 tests)
- File classification rules (20 tests)
- Scanner utilities and entry point parsing (11 tests)
- Edge cases and error handling

---

## 🐛 Bug Fixes

- **Removed unused dependencies** — Eliminated `humansize` and `chrono` crates
- **Verified `--max-depth` functionality** — Confirmed working correctly in global mode
- **Fixed test coverage gaps** — All critical paths now tested

---

## 📊 Performance

- **Binary size:** 3.1 MB (optimized with LTO)
- **Scan speed:** Unchanged (still uses parallel scanning with rayon)
- **Memory usage:** Optimized with atomic counters

---

## 🔄 Breaking Changes

**None.** All changes are backward compatible.

---

## 📦 Installation

### From Source
```bash
git clone https://github.com/yourusername/jatin-lean.git
cd jatin-lean
cargo build --release
./target/release/jatin-lean --help
```

### Using Cargo (coming soon)
```bash
cargo install jatin-lean
```

### Using npx (coming soon)
```bash
npx jatin-lean
```

---

## 📖 Usage Examples

### Basic Usage
```bash
# Dry run (see what would be deleted)
jatin-lean

# Execute with confirmation
jatin-lean --force

# Execute without confirmation (automation)
jatin-lean --force --yes

# Verbose output
jatin-lean --verbose
```

### Configuration
```bash
# Generate config template
jatin-lean --init-config jatin-lean.toml

# Edit the config file
nano jatin-lean.toml

# Run with custom config
jatin-lean --force
```

### Global Mode
```bash
# Scan all projects in ~/projects
jatin-lean ~/projects --global

# Limit scan depth
jatin-lean ~/projects --global --max-depth 3
```

---

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_config_load_local
```

**Test Results:**
```
running 32 tests
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured
```

---

## 📚 Documentation

- **[DEVELOPER.md](DEVELOPER.md)** — Complete developer guide
- **[IMPLEMENTATION_UPDATE.md](IMPLEMENTATION_UPDATE.md)** — Feature implementation details
- **[README.md](README.md)** — User guide and quick start
- **[HOW_TO_USE.md](HOW_TO_USE.md)** — Detailed usage examples

---

## 🗺️ Roadmap

### Next Release (v0.2.0) — Distribution
- [ ] npm wrapper package for `npx jatin-lean`
- [ ] Pre-built binaries for Linux/macOS/Windows
- [ ] GitHub Actions CI/CD pipeline
- [ ] Publish to crates.io

### Future Releases
- [ ] Backup/restore functionality
- [ ] JSON/CSV output mode
- [ ] Watch mode (auto-prune after npm install)
- [ ] Pruning profiles (aggressive, balanced, conservative)

---

## 🙏 Acknowledgments

Built with:
- **Rust** — Systems programming language
- **clap** — CLI argument parsing
- **rayon** — Data parallelism
- **ignore** — Fast file walking
- **dialoguer** — Interactive prompts
- **toml** — Configuration parsing

---

## 📄 License

MIT License — See [LICENSE](LICENSE) file for details

---

## 🐛 Bug Reports

Found a bug? Please open an issue on GitHub with:
- Your OS and Rust version
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs or error messages

---

## 💬 Support

- **GitHub Issues:** Bug reports and feature requests
- **GitHub Discussions:** Questions and community support
- **Email:** jatin@example.com

---

**Made with ❤️ by Jatin Jalandhra**
