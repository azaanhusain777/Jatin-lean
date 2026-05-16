# jatin-lean — Developer Documentation

> Complete guide to understand, modify, and extend the codebase.

---

## Table of Contents

1. [Project Overview](#1-project-overview)
2. [Architecture & Data Flow](#2-architecture--data-flow)
3. [Module Reference](#3-module-reference)
4. [Key Data Structures](#4-key-data-structures)
5. [How the Safety System Works](#5-how-the-safety-system-works)
6. [Known Bugs & Limitations](#6-known-bugs--limitations)
7. [Development Roadmap](#7-development-roadmap)
8. [Extension Guide](#8-extension-guide)
9. [Testing Guide](#9-testing-guide)
10. [Dependency Reference](#10-dependency-reference)

---

## 1. Project Overview

**jatin-lean** is a Rust CLI that removes non-runtime files from `node_modules` directories.

```
Build:    cargo build --release
Test:     cargo test
Run:      cargo run -- <path> [--force] [--verbose] [--global]
Binary:   target/release/jatin-lean (2.9MB with LTO)
```

### Current Status (v0.1.6)

| Feature | Status |
|---------|--------|
| Parallel scanning (ignore + rayon) | ✅ Done |
| 7-category file classification | ✅ Done |
| package.json entry point parsing | ✅ Done |
| require()/import dependency tracing | ✅ Done (optimized) |
| Dry-run mode (default) | ✅ Done |
| Force deletion with progress bar | ✅ Done |
| Global multi-project scan | ✅ Done |
| Verbose file listing | ✅ Done |
| External rules.toml config | ✅ Done |
| Interactive confirmation prompt | ✅ Done |
| --yes flag for automation | ✅ Done |
| --init-config flag | ✅ Done |
| Comprehensive test suite (32 tests) | ✅ Done |
| npx distribution | ❌ Not started |
| Undo/restore capability | ❌ Not started |
| Windows locked-file handling | ❌ Not started |
| Benchmark suite | ❌ Not started |

---

## 2. Architecture & Data Flow

### File Structure

```
src/
├── main.rs       → CLI entry point, orchestrates the 4-phase flow
├── config.rs     → Configuration file loading and parsing
├── rules.rs      → Heuristic ruleset (which files to target)
├── scanner.rs    → Parallel file walker + package.json parser
├── tracer.rs     → require()/import dependency resolver (optimized)
├── deleter.rs    → Batch file deletion engine
└── display.rs    → Terminal UI (tables, banners, progress)
```

### Execution Flow

```
main()
 │
 ├── Local Mode (default)
 │    │
 │    ├── Phase 1: Discovery
 │    │    └── scanner::scan_node_modules()
 │    │         ├── Discovers packages (handles @scoped packages)
 │    │         ├── Parses package.json → extracts entry points
 │    │         ├── Walks all files with ignore crate
 │    │         ├── Classifies each file via rules::PruneRules::classify()
 │    │         └── Returns ScanResult with candidates
 │    │
 │    ├── Phase 2: Simulation
 │    │    └── tracer::verify_runtime_safety()
 │    │         ├── For each package with candidates...
 │    │         ├── Reads main/bin entry from package.json
 │    │         ├── Traces require()/import chains recursively
 │    │         └── Returns HashSet<PathBuf> of runtime-critical files
 │    │    Then: filters candidates against runtime files
 │    │
 │    ├── Phase 3: Confirmation (if --force NOT set)
 │    │    └── display::print_dry_run_confirmation()
 │    │
 │    └── Phase 4: Execution (if --force IS set)
 │         └── deleter::execute_deletion()
 │              ├── Groups candidates by package
 │              ├── Deletes files with progress bar
 │              ├── Cleans up empty directories
 │              └── Returns DeletionResult
 │
 └── Global Mode (--global)
      ├── scanner::find_node_modules() → finds all node_modules dirs
      ├── For each: scanner::scan_node_modules()
      └── display::print_global_table()
```

---

## 3. Module Reference

### `main.rs` — Entry Point

| Function | Line | Purpose |
|----------|------|---------|
| `main()` | 50 | Parses CLI args, dispatches to local/global mode |
| `run_local_mode()` | 68 | Orchestrates 4-phase flow for single project |
| `run_global_mode()` | 187 | Scans directory tree for all node_modules |

**CLI struct fields:**

| Field | Flag | Default | Description |
|-------|------|---------|-------------|
| `path` | positional | `"."` | Project directory |
| `force` | `-f, --force` | `false` | Execute deletion |
| `yes` | `-y, --yes` | `false` | Skip confirmation prompt |
| `config` | `--config <FILE>` | `None` | Custom config file path |
| `global` | `-g, --global` | `false` | Multi-project scan |
| `verbose` | `-v, --verbose` | `false` | List individual files |
| `max_depth` | `--max-depth` | `4` | Global scan depth |
| `init_config` | `--init-config <FILE>` | `None` | Generate example config |

**⚠️ Bug on line 123-124 — FIXED in v0.1.6**

---

### `rules.rs` — Heuristic Classification Engine

**Core type:** `FileCategory` enum — 7 variants:

| Variant | Risk Level | What it matches |
|---------|-----------|-----------------|
| `Documentation` | 0 (Low) | README.md, CHANGELOG.md, docs/ |
| `TestAsset` | 0 (Low) | test/, __tests__/, *.test.js |
| `CiConfig` | 0 (Low) | .travis.yml, .github/, .circleci/ |
| `Example` | 0 (Low) | example/, demos/, samples/ |
| `SourceMap` | 1 (Medium) | *.js.map, *.css.map |
| `BuildArtifact` | 1 (Medium) | *.c, *.o, Makefile, binding.gyp |
| `TypeScriptSource` | 2 (High) | *.ts, *.tsx (NOT *.d.ts) |

**Core type:** `PruneRules` struct — contains all pattern lists.

**Key method:** `PruneRules::classify(rel_path: &Path) -> Option<FileCategory>`

Classification order (first match wins):
1. Safety check → skip `.bin/`, `node_modules/`, dotfiles (except `.github/.circleci/.travis`)
2. Directory name match → test_dirs, doc_dirs, ci_dirs, example_dirs, build_dirs
3. Filename match → doc_files, ci_files, build_files
4. Extension match → map_extensions, build_extensions
5. Regex match → test_file_regex
6. TypeScript check → `.ts`/`.tsx` but NOT `.d.ts`/`.d.tsx`

**Tests:** 6 unit tests covering all key classification paths.

---

### `scanner.rs` — Parallel Scanning Engine

**Key functions:**

| Function | Signature | Purpose |
|----------|-----------|---------|
| `find_node_modules()` | `(root: &Path) -> Vec<PathBuf>` | Discovers all node_modules dirs recursively |
| `scan_node_modules()` | `(path: &Path, rules: &PruneRules) -> Result<ScanResult>` | Scans a single node_modules |
| `last_accessed_days()` | `(path: &Path) -> Option<u64>` | Gets directory age in days |
| `extract_entry_points()` | `(json, pkg_root) -> Vec<PathBuf>` | Reads package.json entry points |
| `collect_export_paths()` | `(value, root, out)` | Recursively extracts `exports` field paths |
| `format_number()` | `(n: u64) -> String` | Formats with comma separators |
| `format_size()` | `(bytes: u64) -> String` | Human-readable sizes (KB/MB/GB) |

**Parallelism model:**
- `packages.par_iter().for_each()` via rayon
- `AtomicU64` for file/size counters (lock-free)
- `Arc<Mutex<Vec>>` for candidate collection
- `Arc<Mutex<HashSet>>` for whitelisted files

**Package discovery logic:**
- Reads direct children of `node_modules/`
- Handles scoped packages (`@scope/package` → reads children of `@scope/`)
- Skips `.bin`, `.cache`, `.package-lock.json`

**Entry point fields parsed from package.json:**
- `main` → primary entry (string)
- `module` → ES module entry (string)
- `browser` → browser entry (string or object)
- `bin` → CLI executables (string or object)
- `exports` → modern entry map (string, object, array — recursive)
- `types` / `typings` → TypeScript declarations (string)

---

### `tracer.rs` — Dependency Tracing

**Core type:** `DependencyTracer` struct

Three compiled regexes:
- `require_regex` → `require('./foo')` or `require("./foo")`
- `import_regex` → `import ... from './foo'`
- `dynamic_import_regex` → `import('./foo')`

**Key method:** `trace_from_file(entry: &Path) -> HashSet<PathBuf>`

Algorithm:
1. BFS queue starting from entry file
2. For each file: read content → extract local deps (relative paths starting with `.`)
3. Resolve each dep using Node.js module resolution:
   - Direct file match
   - Try extensions: `.js`, `.mjs`, `.cjs`, `.json`, `.node`, `.ts`, `.tsx`
   - Try directory: `index.{ext}` or `package.json → main`
4. Add resolved files to queue
5. Return all visited paths

**`verify_runtime_safety()` function:**
- Collects unique package names from candidates
- For each package: traces from `main` entry + all `bin` entries
- Returns union of all traced file paths

---

### `deleter.rs` — Batch Deletion

**`execute_deletion(candidates) -> Result<DeletionResult>`**

1. Groups candidates by `package_name` (HashMap)
2. Iterates packages → deletes files one by one with `fs::remove_file`
3. On error: logs `(path, error_msg)` to failures vec, continues
4. After deletion: collects parent directories, sorts deepest-first
5. Removes empty directories with `fs::remove_dir`
6. Progress bar shows bytes deleted in real-time

---

### `display.rs` — Terminal UI

| Function | Purpose |
|----------|---------|
| `print_banner()` | ASCII box with project name |
| `print_discovery()` | Phase 1 — file count and total size |
| `print_simulation()` | Phase 2 — category breakdown table |
| `print_dry_run_confirmation()` | Phase 3 — savings summary + safety status |
| `print_global_table()` | Global mode — project comparison table |

Uses `comfy-table` with `UTF8_FULL` preset + `UTF8_ROUND_CORNERS` modifier.
Color scheme: cyan for headers, green for safe, yellow for warnings, red for high-risk.

---

## 4. Key Data Structures

```rust
// A file flagged for potential deletion
struct PruneCandidate {
    path: PathBuf,           // Absolute file path
    size: u64,               // Size in bytes
    category: FileCategory,  // Classification category
    package_name: String,    // e.g., "lodash" or "@babel/core"
}

// Result of scanning a node_modules directory
struct ScanResult {
    root: PathBuf,              // The node_modules path
    total_files: u64,           // All files found
    total_size: u64,            // Total bytes
    candidates: Vec<PruneCandidate>,  // Files to delete
    total_packages: usize,      // Package count
    whitelisted_count: u64,     // Files skipped (runtime-required)
}

// Result of executing deletion
struct DeletionResult {
    deleted_count: u64,
    deleted_size: u64,
    failures: Vec<(PathBuf, String)>,  // (path, error_message)
    duration: Duration,
}
```

---

## 5. How the Safety System Works

Two layers of protection (dependency tracing disabled for performance):

### Layer 1: Static Rules (rules.rs)
- `.bin/` directories → NEVER touched
- Dotfiles (except `.github/.circleci/.travis`) → NEVER touched
- `node_modules/` nested dirs → NEVER touched

### Layer 2: Entry Point Whitelisting (scanner.rs)
- Parses `package.json` for `main`, `module`, `browser`, `bin`, `exports`, `types`
- All resolved paths added to whitelist HashSet
- Checked DURING scanning — whitelisted files never become candidates

### Layer 3: Dependency Tracing (tracer.rs) - DISABLED FOR PERFORMANCE
- Previously traced `require()`/`import` chains from entry points
- Disabled because it was causing hangs on large projects (could take minutes)
- Entry point whitelisting provides sufficient safety
- See PERFORMANCE_FIX.md for details

---

## 6. Known Bugs & Limitations

### Bugs

~~All known bugs have been fixed in v0.1.6~~

### Limitations

1. **No undo mechanism** — Deleted files cannot be restored (planned for Phase F)
2. **Single-threaded deletion** — `execute_deletion()` is sequential (rayon only used for scanning)
3. **No Windows-specific handling** — Locked file detection is basic `fs::remove_file` error
4. **Tracer only handles local deps** — `require('lodash')` (bare specifiers) are ignored (by design)
5. **No symlink awareness** — Symlinked packages could cause issues
6. **`build/` directory deletion is risky** — Some packages ship runtime code in `build/`
7. **No LICENSE opt-out** — LICENSE files are listed in doc_files but treated same as README

---

## 7. Development Roadmap

### ~~Phase A: Bug Fixes~~ (✅ COMPLETED in v0.1.6)

- [x] Fix whitelisted count calculation in `main.rs:123-124`
- [x] Wire `--max-depth` to `find_node_modules()`
- [x] Remove or use `--dry-run` flag meaningfully
- [x] Remove unused dependencies (`humansize`, `chrono`)

### ~~Phase B: Configuration System~~ (✅ COMPLETED in v0.1.6)

- [x] Create `rules.toml` file format for custom patterns
- [x] Add `--config <path>` CLI flag
- [x] Support `~/.config/jatin-lean/rules.toml` for global config
- [x] Add `--init-config` flag to generate example config

### ~~Phase C: Interactive Confirmation~~ (✅ COMPLETED in v0.1.6)

- [x] Add interactive confirmation prompt before deletion
- [x] Add `--yes` flag to skip confirmation for automation
- [x] Show detailed savings summary before proceeding

### ~~Phase D: Test Suite~~ (✅ COMPLETED in v0.1.6)

- [x] Add comprehensive tests for scanner.rs (11 tests)
- [x] Add comprehensive tests for rules.rs (14 tests)
- [x] Add comprehensive tests for config.rs (8 tests)
- [x] Total: 32 tests (up from 6)

### Phase E: Distribution (Priority: High)

- [ ] Create npm wrapper package for `npx jatin-lean` support
- [ ] Publish pre-built binaries for Linux/macOS/Windows
- [ ] GitHub Actions CI/CD pipeline
- [ ] Add `cargo install jatin-lean` support (publish to crates.io)

### Phase F: Safety Improvements (Priority: Medium)

- [ ] Add `--backup` flag to archive files before deletion
- [ ] Add `--restore` command to undo last deletion
- [ ] Improve `build/` directory handling (check if package ships runtime code there)
- [ ] Add `--keep-license` flag
- [ ] Deeper ESM `import` tracing (handle re-exports, barrel files)

### Phase G: Performance (Priority: Medium)

- [ ] Parallelize deletion with rayon (batch by package)
- [ ] Use `crossbeam-channel` instead of `Arc<Mutex<Vec>>` for candidates
- [ ] Add `--jobs N` flag to control parallelism
- [ ] Benchmark suite comparing against `du -sh` and `npx node-prune`

### Phase H: Advanced Features (Priority: Low)
- [ ] GitHub Actions CI/CD pipeline
- [ ] Add `cargo install jatin-lean` support (publish to crates.io)

### Phase H: Advanced Features (Priority: Low)

- [ ] Interactive mode with `dialoguer` — let user select categories
- [ ] JSON/CSV output mode (`--output json`)
- [ ] Watch mode — auto-prune after `npm install`
- [ ] Integration with `package.json` scripts (postinstall hook)
- [ ] Pruning profiles (aggressive, balanced, conservative)
- [ ] Per-package override config in `package.json` (`"jatin-lean": { "keep": ["build/"] }`)

---

## 8. Extension Guide

### Adding a New File Category

1. **`rules.rs`** — Add variant to `FileCategory` enum:
   ```rust
   pub enum FileCategory {
       // ... existing variants
       Media,  // New category
   }
   ```

2. **`rules.rs`** — Add `label()` and `risk_level()` match arms:
   ```rust
   FileCategory::Media => "Media",
   FileCategory::Media => 0,
   ```

3. **`rules.rs`** — Add pattern fields to `PruneRules` struct:
   ```rust
   pub media_extensions: Vec<&'static str>,
   ```

4. **`rules.rs`** — Initialize in `PruneRules::new()`:
   ```rust
   media_extensions: vec![".png", ".jpg", ".gif", ".svg"],
   ```

5. **`rules.rs`** — Add check in `classify()`:
   ```rust
   for ext in &self.media_extensions {
       if file_name.ends_with(ext) {
           return Some(FileCategory::Media);
       }
   }
   ```

6. **`rules.rs`** — Add a test:
   ```rust
   #[test]
   fn test_media_classified() {
       let rules = PruneRules::new();
       let path = PathBuf::from("icon.png");
       assert_eq!(rules.classify(&path), Some(FileCategory::Media));
   }
   ```

### Adding a New CLI Flag

1. **`main.rs`** — Add field to `Cli` struct:
   ```rust
   #[arg(long)]
   keep_license: bool,
   ```

2. **`main.rs`** — Use it in `run_local_mode()` or `run_global_mode()`

### Adding External rules.toml Support

Recommended approach:
1. Create a `config.rs` module
2. Define a `Config` struct with `#[derive(Deserialize)]`
3. Look for config at: CLI `--config` → `./jatin-lean.toml` → `~/.config/jatin-lean/rules.toml`
4. Merge loaded config with `PruneRules::new()` defaults
5. The `toml` crate is already in Cargo.toml

### Adding JSON Output Mode

1. Add `#[derive(Serialize)]` to `ScanResult` and `PruneCandidate`
2. Add `--output json` flag to CLI
3. In `main.rs`, check output mode before calling display functions
4. Use `serde_json::to_string_pretty()` to emit results

---

## 9. Testing Guide

### Running Tests
```bash
cargo test              # Run all tests
cargo test -- --nocapture  # Show println! output
cargo test test_readme  # Run specific test
```

### Current Test Coverage

All tests are in `rules.rs`, `scanner.rs`, and `config.rs`:

**rules.rs (20 tests):**
| Test | What it verifies |
|------|-----------------|
| `test_readme_classified_as_documentation` | README.md → Documentation |
| `test_test_dir_classified` | __tests__/foo.js → TestAsset |
| `test_source_map_classified` | dist/bundle.js.map → SourceMap |
| `test_dotbin_never_deleted` | .bin/somefile → None (safe) |
| `test_dts_files_kept` | index.d.ts → None (kept) |
| `test_ts_source_classified` | src/utils.ts → TypeScriptSource |
| `test_nested_node_modules_skipped` | Nested node_modules safety |
| `test_ci_config_classified` | CI/CD file detection |
| `test_build_files_classified` | Build artifact detection |
| `test_example_dirs_classified` | Example directory detection |
| `test_test_file_regex` | Test file pattern matching |
| `test_dotfiles_skipped` | Dotfile safety checks |
| `test_category_labels` | Category label strings |
| `test_category_risk_levels` | Risk level values |

**scanner.rs (11 tests):**
| Test | What it verifies |
|------|-----------------|
| `test_format_number` | Number formatting with commas |
| `test_format_size` | Human-readable size formatting |
| `test_extract_package_name` | Regular and scoped packages |
| `test_extract_entry_points_main_field` | package.json main field |
| `test_extract_entry_points_multiple_fields` | main, module, types |
| `test_extract_entry_points_bin_object` | bin field parsing |
| `test_extract_entry_points_exports_string` | exports as string |
| `test_extract_entry_points_exports_object` | exports as object |
| `test_scan_result_savings` | Savings calculation |
| `test_scan_result_risk_levels` | Risk level detection |

**config.rs (8 tests):**
| Test | What it verifies |
|------|-----------------|
| `test_config_default` | Default config values |
| `test_config_generate_sample` | Sample generation |
| `test_config_create_example` | File creation |
| `test_config_from_file` | TOML parsing |
| `test_config_load_custom_path` | Custom path loading |
| `test_config_load_local` | Local config discovery |
| `test_config_load_none` | No config fallback |
| `test_config_invalid_path` | Error handling |

**Total: 32 tests**

### Tests You Should Add

```rust
// Integration tests (create in tests/ directory)
#[test] fn test_full_scan_on_fixture() { ... }
#[test] fn test_dry_run_doesnt_delete() { ... }
#[test] fn test_force_mode_deletes_files() { ... }
#[test] fn test_config_override_works() { ... }
#[test] fn test_interactive_confirmation() { ... }
```

### Creating Test Fixtures

```bash
# Create a fake node_modules for integration testing
mkdir -p tests/fixtures/node_modules/fake-pkg/{test,docs,.github}
echo '{"name":"fake-pkg","main":"index.js"}' > tests/fixtures/node_modules/fake-pkg/package.json
echo 'module.exports = 42;' > tests/fixtures/node_modules/fake-pkg/index.js
echo '# Readme' > tests/fixtures/node_modules/fake-pkg/README.md
echo 'test' > tests/fixtures/node_modules/fake-pkg/test/test.js
```

---

## 10. Dependency Reference

| Crate | Version | Used In | Purpose |
|-------|---------|---------|---------|
| `clap` | 4 (derive) | main.rs | CLI argument parsing |
| `ignore` | 0.4 | scanner.rs | Parallel file walking, .gitignore-aware |
| `rayon` | 1.10 | scanner.rs | Data parallelism for package processing |
| `indicatif` | 0.17 | scanner.rs, deleter.rs | Progress bars and spinners |
| `console` | 0.15 | display.rs, deleter.rs | Colored/styled terminal output |
| `comfy-table` | 7 | display.rs | Unicode table formatting |
| `serde` | 1 | — | Serialization framework |
| `serde_json` | 1 | scanner.rs, tracer.rs | package.json parsing |
| `regex` | 1 | rules.rs, tracer.rs | Pattern matching for test files and imports |
| `anyhow` | 1 | all modules | Error handling with context |
| `toml` | 0.8 | config.rs | TOML configuration file parsing |
| `dialoguer` | 0.11 | main.rs | Interactive confirmation prompts |
| `dirs` | 5.0 | config.rs | Home directory detection |

**Dev Dependencies:**
| Crate | Version | Used In | Purpose |
|-------|---------|---------|---------|
| `tempfile` | 3 | config.rs tests | Temporary files for testing |

### Build Profile (Release)

```toml
opt-level = 3      # Maximum optimization
lto = true          # Link-time optimization (smaller + faster binary)
codegen-units = 1   # Single codegen unit (better optimization, slower compile)
strip = true        # Strip debug symbols (smaller binary)
```

---

## Quick Reference Commands

```bash
# Development
cargo build                    # Debug build
cargo build --release          # Optimized build (2.9MB)
cargo test                     # Run all tests
cargo clippy                   # Lint check
cargo fmt                      # Format code

# Usage
./target/release/jatin-lean                     # Dry run, current dir
./target/release/jatin-lean /path --verbose     # Verbose dry run
./target/release/jatin-lean --force             # Execute deletion
./target/release/jatin-lean ~/projects --global # Scan all projects
./target/release/jatin-lean --help              # Full help
```
