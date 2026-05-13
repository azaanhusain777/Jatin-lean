# ⚡ jatin-lean

**A high-performance Rust CLI utility to prune non-essential files from `node_modules`, reducing disk footprint by up to 50% without breaking runtime dependencies.**

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

---

## 🚀 What It Does

`jatin-lean` intelligently identifies and removes non-runtime files from your `node_modules`:

| Category | Examples | Risk |
|---|---|---|
| **Documentation** | `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md` | ▪ Low |
| **Test Assets** | `test/`, `__tests__/`, `*.test.js`, `*.spec.js` | ▪ Low |
| **CI/CD Config** | `.travis.yml`, `circle.yml`, `.github/` | ▪ Low |
| **Examples** | `example/`, `demos/`, `samples/` | ▪ Low |
| **Source Maps** | `*.js.map`, `*.css.map` | ▪▪ Medium |
| **Build Artifacts** | `*.c`, `*.cpp`, `*.o`, `Makefile`, `binding.gyp` | ▪▪ Medium |
| **TS Sources** | `*.ts`, `*.tsx` (keeps `.d.ts` declarations) | ▪▪▪ High |

## 🛡️ Safety First

Before deleting anything, `jatin-lean`:

1. **Parses `package.json`** — identifies `main`, `module`, `exports`, `bin`, and `types` entry points
2. **Traces dependencies** — scans `require()` and `import` statements to map runtime-critical files
3. **Auto-whitelists** — any file reachable from entry points is marked as **LOCKED**
4. **Never touches** `.bin/` directories or dotfiles (except `.github`)

## 📦 Installation

### Using npx (Recommended - No Installation Required)
```bash
# Run directly in any Node.js project
npx jatin-lean

# With options
npx jatin-lean --force
npx jatin-lean --verbose
```

### Using npm (Global Installation)
```bash
# Install globally
npm install -g jatin-lean

# Use anywhere
jatin-lean
jatin-lean --force
```

### From Source (For Development)
```bash
git clone https://github.com/your-username/jatin-lean.git
cd jatin-lean
cargo build --release
# Binary at: target/release/jatin-lean
```

### Add to PATH
```bash
cp target/release/jatin-lean ~/.local/bin/
```

## 🎯 Usage

### Dry Run (Default — Safe)
```bash
# Scan current project
jatin-lean

# Scan a specific project
jatin-lean /path/to/project

# Verbose — show every file targeted
jatin-lean --verbose
```

### Execute Deletion
```bash
jatin-lean --force
```

### Global Mode — Scan All Projects
```bash
jatin-lean ~/projects --global
```

## 📊 Example Output

```
  ╔═══════════════════════════════════════════════╗
  ║  ⚡ jatin-lean — Node Modules Pruner ⚡      ║
  ║     Slim your node_modules by up to 50%      ║
  ╚═══════════════════════════════════════════════╝

  Phase 1: Discovery ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ◉ Scanning node_modules... Found 45,000 files across 420 packages.
  ◉ Total size indexed: 1.2GB

  Phase 2: Simulation ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ◉ Analyzing dependency tree... 12,400 files (680MB) identified as non-runtime assets.

    ╭────────────────┬───────┬─────────┬────────────╮
    │ Category       ┆ Files ┆ Size    ┆ Risk       │
    ╞════════════════╪═══════╪═════════╪════════════╡
    │ Source-Map     ┆ 3,200 ┆ 450MB   ┆ ▪▪ Medium  │
    │ Test-Asset     ┆ 5,100 ┆ 120MB   ┆ ▪ Low      │
    │ Documentation  ┆ 2,800 ┆ 80MB    ┆ ▪ Low      │
    │ Build-Artifact ┆ 900   ┆ 25MB    ┆ ▪▪ Medium  │
    │ CI-Config      ┆ 400   ┆ 5MB     ┆ ▪ Low      │
    ╰────────────────┴───────┴─────────┴────────────╯

  Phase 3: Confirmation ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  [SAFE] No critical runtime files targeted.

  💾 Total Savings: 680MB (56% of node_modules)
  ℹ This will NOT affect npm start or npm build.

  → Run with --force to execute deletion.
```

### Global Mode Output
```
  System Efficiency Report ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    ╭────────────────┬───────┬───────────────────┬──────────────────╮
    │ Project        ┆ Size  ┆ Potential Savings ┆ Last Accessed    │
    ╞════════════════╪═══════╪═══════════════════╪══════════════════╡
    │ old-portfolio  ┆ 850MB ┆ 400MB             ┆ 142 days ago     │
    │ current-app    ┆ 1.2GB ┆ 300MB             ┆ 2 days ago       │
    │ TOTAL          ┆ 2.0GB ┆ 700MB             ┆ —                │
    ╰────────────────┴───────┴───────────────────┴──────────────────╯
```

## 🏗️ Architecture

```
src/
├── main.rs       # CLI entry point (clap), phased execution flow
├── rules.rs      # Heuristic ruleset engine with file classification
├── scanner.rs    # Parallel scanning engine (ignore + rayon)
├── tracer.rs     # Dependency tracing (require/import resolution)
├── deleter.rs    # Atomic batch deletion with progress UI
└── display.rs    # Terminal UI (comfy-table, console, indicatif)
```

### Key Design Decisions

- **`ignore` crate** — respects `.gitignore`, optimized for OS-level file cache
- **`rayon`** — parallel package analysis across all CPU cores
- **Batch deletion** — grouped by package to minimize syscall overhead
- **Graceful error handling** — locked files are logged, never crash the run

## 🔧 CLI Options

| Flag | Short | Description |
|---|---|---|
| `--force` | `-f` | Execute deletion (default is dry-run) |
| `--dry-run` | `-d` | Explicitly run in dry-run mode |
| `--global` | `-g` | Scan all projects in a directory |
| `--verbose` | `-v` | Show individual files targeted |
| `--max-depth N` | | Max directory depth for global scan (default: 4) |
| `--help` | `-h` | Print help information |
| `--version` | `-V` | Print version |

## 📄 License

MIT License — see [LICENSE](LICENSE) for details.
