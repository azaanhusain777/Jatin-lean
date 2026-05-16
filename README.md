# 🚀 jatin-lean

> A **high-performance CLI utility** to prune, analyze, and optimize `node_modules` — reducing disk footprint by up to **50%** without breaking runtime dependencies.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![npm](https://img.shields.io/npm/v/jatin-lean.svg)](https://www.npmjs.com/package/jatin-lean)

---

## ✨ Features

| Feature | Description |
| --- | --- |
| **⚡ Lightning Scan** | Parallel file walking with category-based identification |
| **🗑️ Smart Pruning** | Safely removes test files, docs, configs, build artifacts |
| **📊 Analytics** | Track savings over time, export JSON/CSV/Markdown reports |
| **🔍 Duplicate Detection** | FNV-1a content hashing to find redundant files |
| **🌳 Dependency Graph** | Parse npm/yarn lockfiles, detect orphaned packages |
| **📸 Snapshots** | Pre-deletion backup & one-command restore |
| **👀 Watch Mode** | Auto-prune on `npm install` / file changes |
| **💾 Incremental Cache** | Skip unchanged packages on re-scans |
| **🏥 Health Check** | Assess license risk, security, nesting depth, deprecation |
| **🌿 Tree-Shake Analysis** | Identify unused exports and dead code patterns |
| **📦 Compression Analysis** | Estimate gzip/brotli transfer sizes per package |
| **📜 Policy Engine** | Enforce enterprise rules: size limits, banned packages, licenses |
| **🔌 Plugin System** | Extend with custom analyzers via trait-based API |
| **⏱️ Profiler** | Built-in performance timing with bottleneck detection |

---

## 📥 Installation

### Via npm (recommended)

```bash
npm install -g jatin-lean
```

### Via Cargo (from source)

```bash
cargo install --path .
```

---

## 🚀 Quick Start

```bash
# Scan your project (dry-run — nothing is deleted)
jatin-lean .

# Actually prune files
jatin-lean . --force

# Scan with a safety snapshot first
jatin-lean . --force --snapshot

# Export results to JSON
jatin-lean . --export json
```

---

## 📖 Commands

### Core

```bash
# Default scan (dry-run)
jatin-lean [path]

# Prune with force
jatin-lean [path] --force

# Scan with custom config
jatin-lean [path] --config ./jatin-lean.toml

# Global scan (all node_modules in tree)
jatin-lean [path] --global --depth 5
```

### Analysis

```bash
# Find duplicate files
jatin-lean dedup [path]

# Analyze dependency graph from lockfiles
jatin-lean deps [path]

# Health check (license, security, depth, deprecation)
jatin-lean health [path]

# Tree-shake analysis (unused exports)
jatin-lean treeshake [path]

# Compression potential (gzip/brotli estimates)
jatin-lean compress [path]
```

### Safety & History

```bash
# View analytics dashboard
jatin-lean analytics [path]

# List snapshots
jatin-lean snapshots --list

# Restore a snapshot
jatin-lean snapshots --restore <SNAPSHOT_ID>

# Clean old snapshots (older than 30 days)
jatin-lean snapshots --cleanup 30
```

### Automation

```bash
# Watch mode (auto-prune on changes)
jatin-lean watch [path] --auto-prune --interval 5

# Cache management
jatin-lean cache --stats [path]
jatin-lean cache --clear [path]
```

### Enterprise

```bash
# Generate example policy
jatin-lean policy --init ./policy.toml

# Enforce a policy (exits with code 1 on violation)
jatin-lean policy --file ./policy.toml [path]

# List plugins
jatin-lean plugins --list
```

### Configuration

```bash
# Generate example config file
jatin-lean --init-config ./jatin-lean.toml
```

---

## 🛡️ Global Flags

| Flag | Description |
| --- | --- |
| `--force` | Actually delete files (default is dry-run) |
| `--json` | Output scan results as JSON |
| `--global` | Scan all node_modules recursively |
| `--depth <N>` | Max depth for global scan |
| `--config <FILE>` | Custom config file path |
| `--profile` | Enable performance profiling |
| `--snapshot` | Create a snapshot before pruning |
| `--export <FMT>` | Export report (json, csv, markdown) |
| `--init-config <FILE>` | Generate example config |

---

## 📦 Programmatic API (Node.js)

```javascript
const jatinLean = require('jatin-lean');

// Scan a project
const results = await jatinLean.scan('./my-project');
console.log(results);

// Prune with snapshot
await jatinLean.prune('./my-project', { snapshot: true });

// Calculate savings
const savings = await jatinLean.calculateSavings('./my-project');
console.log(`Could save ${savings.candidateSizeHuman}`);

// Check if binary is available
console.log('Installed:', jatinLean.isInstalled());
console.log('Version:', jatinLean.getVersion());
```

TypeScript definitions are included (`index.d.ts`).

---

## ⚙️ Configuration

Create a `jatin-lean.toml` in your project root:

```toml
[pruning]
extra_patterns = ["*.map", "*.flow"]
skip_packages = ["critical-pkg"]
aggressive = false

[safety]
snapshot_before_delete = true
max_snapshot_age_days = 30

[display]
show_progress = true
color = true
```

---

## 📜 Policy Files

Enforce governance with TOML/JSON policies:

```toml
name = "my-team-policy"
version = "1.0.0"

[rules]
max_total_size = 500000000        # 500MB
max_package_size = 50000000       # 50MB
max_dependency_count = 500
max_nesting_depth = 5
ban_install_scripts = false

banned_packages = ["left-pad", "is-odd"]
allowed_licenses = ["MIT", "ISC", "BSD-2-Clause", "Apache-2.0"]
banned_licenses = ["GPL-3.0", "AGPL-3.0"]
```

Use in CI:

```yaml
- name: Enforce dependency policy
  run: jatin-lean policy --file policy.toml || exit 1
```

---

## 🔌 Plugin Architecture

jatin-lean ships with 5 built-in plugins:

| Plugin | Purpose |
| --- | --- |
| `native-modules` | Identifies native build artifacts (binding.gyp) |
| `typescript-source` | Finds .ts files with compiled .js counterparts |
| `test-files` | Detects test directories (test, __tests__, spec) |
| `example-files` | Finds example/demo directories |
| `benchmark-files` | Identifies benchmark/perf directories |

Custom plugins implement the `Plugin` trait:

```rust
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    fn on_scan(&self, candidates: &mut Vec<PruneCandidate>, root: &Path) -> Result<()>;
    fn on_pre_delete(&self, candidates: &[PruneCandidate]) -> Result<Vec<PathBuf>>;
    fn on_post_delete(&self, deleted: &[PathBuf], total_bytes: u64) -> Result<()>;
    fn generate_report(&self, scan_result: &ScanResult) -> Result<Option<String>>;
}
```

---

## 🏗️ Architecture

```
src/
├── main.rs          # CLI entry point & command routing
├── lib.rs           # Library re-exports
├── scanner.rs       # Parallel file system walker
├── rules.rs         # Category-based file classification
├── config.rs        # TOML configuration loader
├── display.rs       # Terminal UI (tables, progress)
├── deleter.rs       # Safe file deletion engine
├── tracer.rs        # Import/require dependency tracer
├── analytics.rs     # Scan history & trend tracking
├── cache.rs         # Incremental scan cache
├── dedup.rs         # Duplicate file detection (FNV-1a)
├── lockfile.rs      # Lockfile parser & dependency graph
├── profiler.rs      # Performance instrumentation
├── snapshot.rs      # Pre-deletion backup & restore
├── watcher.rs       # File system change monitor
├── health.rs        # Health assessment engine
├── treeshake.rs     # Static export analysis
├── compress.rs      # Compression potential analyzer
├── policy.rs        # Enterprise policy enforcement
└── plugin.rs        # Extensible plugin system
```

---

## 🔧 Building from Source

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test
```

---

## 📄 License

MIT © [Jatin Jalandhra](https://github.com/decodejatin)
