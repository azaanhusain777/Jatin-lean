# ⚡ jatin-lean

**A high-performance Rust CLI utility to prune non-essential files from `node_modules`, reducing disk footprint by up to 50% without breaking runtime dependencies.**

Created by **Jatin Jalandhra**

[![npm version](https://img.shields.io/npm/v/jatin-lean.svg)](https://www.npmjs.com/package/jatin-lean)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/decodejatin/jatin-lean/workflows/CI/badge.svg)](https://github.com/decodejatin/jatin-lean/actions)

---

## 🚀 Quick Start

```bash
# Run directly with npx (no installation needed)
npx jatin-lean

# Execute deletion with confirmation
npx jatin-lean --force

# Execute deletion without confirmation (automation)
npx jatin-lean --force --yes

# Verbose mode
npx jatin-lean --verbose

# Custom configuration
npx jatin-lean --config my-rules.toml --force
```

---

## ✨ Features

### 🎯 Smart Deletion
- **7 file categories** with risk levels
- **Entry point whitelisting** from package.json
- **Dry-run by default** for safety
- **Interactive confirmation** before deletion

### ⚙️ Customizable
- **TOML configuration** support
- **Custom rules** for any project
- **Extend or override** built-in rules
- **Per-project configs** supported

### 🚀 High Performance
- **Parallel scanning** with Rust + rayon
- **Fast execution** (seconds, not minutes)
- **Minimal memory** usage
- **Progress indicators** for large projects

### 🛡️ Safety First
- **Never touches** `.bin/` directories
- **Preserves** runtime dependencies
- **Keeps** type declarations (*.d.ts)
- **Respects** package.json entry points

---

## 🎯 What It Deletes

| Category | Examples | Risk | Typical Savings |
|---|---|---|---|
| **Documentation** | README.md, CHANGELOG.md, LICENSE | ▪ Low | 10-15% |
| **Test Assets** | test/, *.test.js, *.spec.js | ▪ Low | 15-25% |
| **CI/CD Config** | .travis.yml, .github/, .circleci/ | ▪ Low | 1-3% |
| **Examples** | example/, demos/, samples/ | ▪ Low | 5-10% |
| **Source Maps** | *.js.map, *.css.map | ▪▪ Medium | 10-15% |
| **Build Artifacts** | *.c, *.o, Makefile, binding.gyp | ▪▪ Medium | 5-10% |
| **TS Sources** | *.ts, *.tsx (keeps .d.ts) | ▪▪▪ High | 10-20% |

**Total typical savings: 40-60% of node_modules size**

---

## 📖 Usage

### Basic Commands

```bash
# Dry run (default - safe preview)
npx jatin-lean

# Execute deletion with confirmation
npx jatin-lean --force

# Execute deletion without confirmation (CI/CD)
npx jatin-lean --force --yes

# Verbose - show every file
npx jatin-lean --verbose

# Scan specific project
npx jatin-lean /path/to/project

# Global mode - scan all projects
npx jatin-lean ~/projects --global
```

### Configuration

```bash
# Generate example config
npx jatin-lean --init-config jatin-lean.toml

# Edit the config
nano jatin-lean.toml

# Run with custom config
npx jatin-lean --config jatin-lean.toml --force
```

**Example config (jatin-lean.toml):**
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

### CLI Options

| Flag | Description |
|---|---|
| `--force` / `-f` | Execute deletion (default is dry-run) |
| `--yes` / `-y` | Skip confirmation prompt (auto-confirm) |
| `--config <FILE>` | Path to custom config file |
| `--init-config <FILE>` | Generate example config file |
| `--verbose` / `-v` | Show individual files targeted |
| `--global` / `-g` | Scan all projects in a directory |
| `--max-depth N` | Max directory depth for global scan |
| `--help` / `-h` | Print help information |
| `--version` / `-V` | Print version |

---

## 📊 Example Output

```
  ╔═══════════════════════════════════════════════╗
  ║  ⚡ jatin-lean — Node Modules Pruner ⚡      ║
  ║     Slim your node_modules by up to 50%      ║
  ║          Created by Jatin Jalandhra          ║
  ╚═══════════════════════════════════════════════╝

  Phase 1: Discovery ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ◉ Scanning node_modules... Found 5,057 files across 71 packages.
  ◉ Total size indexed: 68.7MB

  Phase 2: Simulation ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ◉ Analyzing dependency tree... 2,400 files (35MB) identified.

    ╭────────────────┬───────┬─────────┬────────────╮
    │ Category       ┆ Files ┆ Size    ┆ Risk       │
    ╞════════════════╪═══════╪═════════╪════════════╡
    │ Documentation  ┆ 1,200 ┆ 15MB    ┆ ▪ Low      │
    │ Test-Asset     ┆ 800   ┆ 12MB    ┆ ▪ Low      │
    │ Source-Map     ┆ 400   ┆ 8MB     ┆ ▪▪ Medium  │
    ╰────────────────┴───────┴─────────┴────────────╯

  Phase 3: Confirmation ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  [SAFE] No critical runtime files targeted.

  💾 Total Savings: 35MB (51% of node_modules)
  ℹ This will NOT affect npm start or npm build.

  → Run with --force to execute deletion.
  ✨ Made with ❤️  by Jatin Jalandhra
```

---

## 🔧 Use Cases

### In package.json Scripts

```json
{
  "scripts": {
    "postinstall": "jatin-lean --force --yes",
    "clean:modules": "jatin-lean --force"
  }
}
```

### In Dockerfile

```dockerfile
FROM node:18-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --production
RUN npx jatin-lean --force --yes
COPY . .
CMD ["npm", "start"]
```

### CI/CD Pipeline

```yaml
# GitHub Actions
- name: Install dependencies
  run: npm ci
- name: Optimize node_modules
  run: npx jatin-lean --force --yes

# GitLab CI
script:
  - npm ci
  - npx jatin-lean --force --yes
```

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit
npx jatin-lean --verbose
```

### Monorepo Cleanup

```bash
# Scan all projects
npx jatin-lean ~/monorepo --global

# Clean specific project
npx jatin-lean ~/monorepo/packages/api --force
```

---

## 🌍 Platform Support

| Platform | Architecture | Status |
|----------|-------------|--------|
| **Linux** | x64 | ✅ Fully supported |
| **Linux** | ARM64 | ✅ Fully supported |
| **macOS** | x64 (Intel) | ✅ Fully supported |
| **macOS** | ARM64 (M1/M2) | ✅ Fully supported |
| **Windows** | x64 | ✅ Fully supported |

Binaries are automatically downloaded during installation.

---

## ⚡ Performance

- **Fast**: Parallel scanning with Rust + rayon
- **Efficient**: Completes in seconds, not minutes
- **Safe**: Entry points protected automatically

**Expected performance:**
- Small projects (< 100 packages): < 1 second
- Medium projects (100-500 packages): 1-3 seconds
- Large projects (500+ packages): 3-10 seconds

---

## 🛡️ Safety Features

### 3-Layer Safety System

1. **Static Rules**
   - Never touches `.bin/` directories
   - Skips dotfiles (except .github, .circleci, .travis)
   - Ignores nested `node_modules/`

2. **Entry Point Whitelisting**
   - Parses `package.json` fields: main, module, browser, bin, exports, types
   - Auto-whitelists runtime-required files
   - Preserves type declarations (*.d.ts)

3. **Interactive Confirmation**
   - Shows detailed savings summary
   - Displays risk assessment
   - Default answer is "No" for safety

---

## 📚 Documentation

- **[Quick Start Guide](https://github.com/decodejatin/jatin-lean/blob/main/QUICK_START.md)** — Get started in 60 seconds
- **[User Guide](https://github.com/decodejatin/jatin-lean/blob/main/USER_GUIDE.md)** — Comprehensive usage guide
- **[Developer Guide](https://github.com/decodejatin/jatin-lean/blob/main/DEVELOPER.md)** — For contributors
- **[Distribution Guide](https://github.com/decodejatin/jatin-lean/blob/main/DISTRIBUTION_GUIDE.md)** — Publishing workflow

---

## 🔄 Alternatives

### How jatin-lean Compares

| Feature | jatin-lean | node-prune | modclean |
|---------|-----------|-----------|----------|
| **Language** | Rust | Go | JavaScript |
| **Speed** | ⚡⚡⚡ Fast | ⚡⚡ Medium | ⚡ Slow |
| **Configuration** | ✅ TOML | ❌ No | ✅ CLI flags |
| **Interactive** | ✅ Yes | ❌ No | ❌ No |
| **Entry Points** | ✅ Parsed | ⚠️ Basic | ⚠️ Basic |
| **Risk Levels** | ✅ 3 levels | ❌ No | ❌ No |
| **Dry Run** | ✅ Default | ❌ No | ✅ Yes |
| **Global Mode** | ✅ Yes | ❌ No | ❌ No |

---

## 🐛 Troubleshooting

### Q: Nothing gets deleted
**A:** Your node_modules is already lean, or files are whitelisted as runtime-required.

### Q: Can I undo deletion?
**A:** Not yet. Always run dry-run first to verify. Backup feature coming soon.

### Q: How do I keep LICENSE files?
**A:** Create a config with `doc_files = []` and `override_defaults = true`.

### Q: Binary download fails
**A:** Check internet connection or download manually from [GitHub Releases](https://github.com/decodejatin/jatin-lean/releases).

### Q: Does it work on Windows?
**A:** Yes! Windows x64 is fully supported.

---

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

---

## ⚠️ Disclaimer

While `jatin-lean` is designed to be safe:
1. Run in dry-run mode first (default behavior)
2. Review the list of files to be deleted
3. Test your application after pruning
4. Keep backups of critical projects

**Use at your own risk.**

---

## 🔗 Links

- **[GitHub Repository](https://github.com/decodejatin/jatin-lean)** — Source code
- **[Issue Tracker](https://github.com/decodejatin/jatin-lean/issues)** — Bug reports
- **[npm Package](https://www.npmjs.com/package/jatin-lean)** — npm registry
- **[crates.io](https://crates.io/crates/jatin-lean)** — Rust package

---

## 🌟 Star History

If you find this tool useful, please consider giving it a star on GitHub!

---

**Made with ❤️ by Jatin Jalandhra**

