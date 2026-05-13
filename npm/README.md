# ⚡ jatin-lean

**A high-performance Rust CLI utility to prune non-essential files from `node_modules`, reducing disk footprint by up to 50% without breaking runtime dependencies.**

Created by **Jatin Jalandhra**

[![npm version](https://img.shields.io/npm/v/jatin-lean.svg)](https://www.npmjs.com/package/jatin-lean)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

---

## 🚀 Quick Start

```bash
# Run directly with npx (no installation needed)
npx jatin-lean

# Execute deletion
npx jatin-lean --force

# Verbose mode
npx jatin-lean --verbose
```

---

## 🎯 What It Does

Intelligently removes non-runtime files from your `node_modules`:

| Category | Examples | Risk |
|---|---|---|
| **Documentation** | README.md, CHANGELOG.md | ▪ Low |
| **Test Assets** | test/, *.test.js, *.spec.js | ▪ Low |
| **CI/CD Config** | .travis.yml, .github/ | ▪ Low |
| **Examples** | example/, demos/ | ▪ Low |
| **Source Maps** | *.js.map, *.css.map | ▪▪ Medium |
| **Build Artifacts** | *.c, *.o, Makefile | ▪▪ Medium |
| **TS Sources** | *.ts, *.tsx (keeps .d.ts) | ▪▪▪ High |

---

## 🛡️ Safety First

Before deleting anything:
1. ✅ Parses `package.json` entry points (main, module, exports, bin, types)
2. ✅ Auto-whitelists runtime-critical files
3. ✅ Never touches `.bin/` directories
4. ✅ Dry-run mode by default (safe preview)

---

## 📖 Usage

### Basic Commands

```bash
# Dry run (default - safe preview)
npx jatin-lean

# Execute deletion
npx jatin-lean --force

# Verbose - show every file
npx jatin-lean --verbose

# Scan specific project
npx jatin-lean /path/to/project

# Global mode - scan all projects
npx jatin-lean ~/projects --global
```

### CLI Options

| Flag | Description |
|---|---|
| `--force` / `-f` | Execute deletion (default is dry-run) |
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
    "postinstall": "jatin-lean --force",
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
RUN npx jatin-lean --force
COPY . .
CMD ["npm", "start"]
```

### CI/CD Pipeline

```yaml
- name: Install dependencies
  run: npm ci
- name: Optimize node_modules
  run: npx jatin-lean --force
```

---

## 🌍 Platform Support

**Current Version (v0.1.0):**
- ✅ **Linux x64** - Fully supported

**Coming Soon:**
- ⏳ **macOS** (Intel & Apple Silicon)
- ⏳ **Windows** (x64)
- ⏳ **Linux ARM64**

Multi-platform binaries will be available in the next release!

### Building from Source (All Platforms)

If you're on macOS or Windows, you can build from source:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/your-username/jatin-lean.git
cd jatin-lean
cargo build --release

# Binary will be at: target/release/jatin-lean
```

---

## ⚡ Performance

- **Fast**: Parallel scanning with Rust + rayon
- **Efficient**: Completes in seconds, not minutes
- **Safe**: Entry points protected automatically

Expected performance:
- Small projects (< 100 packages): < 1 second
- Medium projects (100-500 packages): 1-3 seconds
- Large projects (500+ packages): 3-10 seconds

---

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

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

- [GitHub Repository](https://github.com/your-username/jatin-lean)
- [Issue Tracker](https://github.com/your-username/jatin-lean/issues)
- [npm Package](https://www.npmjs.com/package/jatin-lean)

---

**Made with ❤️ by Jatin Jalandhra**
