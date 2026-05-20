#!/usr/bin/env python3
import subprocess
import time
import sys

issues = [
    {
        "title": "[GSSOC] [Feature] Add support for --config via environment variable JATIN_LEAN_CONFIG",
        "labels": ["gssoc", "good first issue", "difficulty:easy", "domain:rust"],
        "body": """### 📝 Description
Currently, `jatin-lean` config paths can only be supplied via the CLI flag `--config` or checked at default OS-level paths (like `~/.config/jatin-lean/rules.toml`).

To enable seamless automation in Docker containers and CI systems without adding CLI flags to scripts, we should allow users to specify their configuration path via the environment variable `JATIN_LEAN_CONFIG`.

### 🎯 Acceptance Criteria
- In `src/config.rs`, update the configuration resolver function.
- Check if the environment variable `JATIN_LEAN_CONFIG` is set using `std::env::var`.
- If set, load the config from that file path before falling back to default paths.
- Add a unit test verifying that setting `JATIN_LEAN_CONFIG` points the config parser to the correct custom file.

### 📂 Code / Files Involved
- [src/config.rs](src/config.rs)
- [src/main.rs](src/main.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Bug] Fix CLI visual table clipping on small terminal windows",
        "labels": ["gssoc", "good first issue", "difficulty:easy", "domain:docs"],
        "body": """### 📝 Description
The CLI uses the `comfy-table` crate to draw beautiful category breakdowns and dry-run summaries. However, on small or resized terminal windows (less than 80 columns wide), the tables clip or wrap awkwardly, rendering the outputs hard to read.

We need to dynamically check terminal dimensions and adjust styling or compress the columns when width constraints are active.

### 🎯 Acceptance Criteria
- In `src/display.rs`, retrieve terminal width dynamically using the `console` or `crossterm` API.
- If the terminal width is less than 85 columns, dynamically omit non-essential columns (like 'Risk Level' or 'Files Count') or compress margins.
- If width is below 50 columns, fallback to a clean list-based printout instead of drawing the Unicode table.

### 📂 Code / Files Involved
- [src/display.rs](src/display.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Docs] Translate README.md into multiple languages",
        "labels": ["gssoc", "good first issue", "difficulty:easy", "domain:docs"],
        "body": """### 📝 Description
To increase global accessibility and help developers of different backgrounds utilize `jatin-lean`, we want to translate our primary `README.md` into multiple languages.

Translations needed:
- Hindi (`README_HI.md`)
- Spanish (`README_ES.md`)
- French (`README_FR.md`)
- Chinese (`README_ZH.md`)

### 🎯 Acceptance Criteria
- Translate all sections of `README.md` accurately (avoid direct word-for-word automated translator errors).
- Create corresponding markdown translation files in the repository root.
- Add a neat language selector row of badge links at the top of the primary `README.md`.

### 📂 Code / Files Involved
- [README.md](README.md)
- Create `README_<LANG>.md`

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you! (Specify which language you want to translate to)."""
    },
    {
        "title": "[GSSOC] [Feature] Add --dry-run alias as --simulate or -d",
        "labels": ["gssoc", "good first issue", "difficulty:easy", "domain:rust"],
        "body": """### 📝 Description
Currently, `jatin-lean` runs in dry-run simulation mode by default (requiring `--force` to actually delete files). To conform with UNIX tool standards (like `tar`, `rsync`), we should support explicit `--dry-run`, `--simulate`, or `-d` flags to let users explicitly declare simulation intent.

### 🎯 Acceptance Criteria
- Add `--dry-run`, `--simulate`, and `-d` as command line flags/aliases in the CLI options in `src/main.rs`.
- Ensure they are documented clearly in CLI helper outputs.
- Running with `--dry-run` or `--simulate` should override and bypass any `--force` flag to avoid accidental deletions.

### 📂 Code / Files Involved
- [src/main.rs](src/main.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Feature] Add command jatin-lean node version",
        "labels": ["gssoc", "good first issue", "difficulty:easy", "domain:rust"],
        "body": """### 📝 Description
Let's make troubleshooting and env checking simpler for users. Add a sub-command `jatin-lean node version` that prints detailed diagnostic details of the environment.

### 🎯 Acceptance Criteria
- Implement `version` sub-command under `node` category in `src/cli/node.rs`.
- Output should print:
  1. `jatin-lean` compiler target triple.
  2. N-API bindings version.
  3. Node.js version of the host process.
  4. Rust compiler version used for N-API build (compiled-in meta details).
- Support standard `--json` flag to return these version components in a clean JSON object structure.

### 📂 Code / Files Involved
- [src/cli/node.rs](src/cli/node.rs)
- [src/main.rs](src/main.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Testing] Add CLI command integration tests using assert_cmd",
        "labels": ["gssoc", "good first issue", "difficulty:easy", "domain:testing"],
        "body": """### 📝 Description
While we have excellent unit tests for core modules, we lack integration tests that execute the actual compiled CLI binary and assert command line behaviors, outputs, and exit codes.

We need to add CLI integration tests using standard testing crates.

### 🎯 Acceptance Criteria
- Add `assert_cmd` and `predicates` to dev-dependencies in `Cargo.toml`.
- Create a test file `tests/cli_tests.rs`.
- Write tests that compile the binary, run commands like `jatin-lean node scan` on temporary fixtures, and verify that exit codes are `0` and outputs contain anticipated headers/ASCII banners.

### 📂 Code / Files Involved
- [Cargo.toml](Cargo.toml)
- Create `tests/cli_tests.rs`

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Feature] Add a custom directory exclude flag --exclude <DIR>",
        "labels": ["gssoc", "good first issue", "difficulty:easy", "domain:rust"],
        "body": """### 📝 Description
Sometimes developers want to prune most node modules but keep specific directories untouched (e.g., keeping a specific dependency's source files intact).

Let's add a `--exclude` (or `-e`) CLI flag to let users specify folders or glob patterns to exclude from scanning and pruning.

### 🎯 Acceptance Criteria
- Add `--exclude <DIR>` multi-value option to `src/main.rs`.
- Pass these custom excludes to `scan_node_modules` in `src/scanner.rs`.
- During file walking, if a path starts with or matches any of the supplied exclude patterns, skip walking that directory completely.
- Add unit tests verifying that folders matching `--exclude` are untouched.

### 📂 Code / Files Involved
- [src/main.rs](src/main.rs)
- [src/scanner.rs](src/scanner.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Feature] Implement visual dashboard using ascii graphs in node visualize",
        "labels": ["gssoc", "difficulty:medium", "domain:rust"],
        "body": """### 📝 Description
Pruning node modules is a visual experience. The `jatin-lean node visualize` command should display a rich, aesthetic terminal dashboard summarizing the sizes of different folders and optimization metrics using ASCII-based graphs and blocks.

### 🎯 Acceptance Criteria
- Implement the handler for `node visualize` command in `src/cli/node.rs` and `src/visualizer.rs`.
- Draw an ASCII horizontal bar chart representing size distribution across categories (Docs, TestAssets, Maps, TypeScript, etc.).
- Use green/cyan/yellow styled terminal characters to render bar fill levels.
- Render general stats like potential savings percentage in a formatted block diagram.

### 📂 Code / Files Involved
- [src/cli/node.rs](src/cli/node.rs)
- [src/visualizer.rs](src/visualizer.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Feature] Enforce lint limits using a local configuration package.json overrides",
        "labels": ["gssoc", "difficulty:medium", "domain:rust"],
        "body": """### 📝 Description
Global configuration from `~/.config/jatin-lean/rules.toml` is great, but specific projects might need custom pruning bypasses. We should support per-project overrides defined directly in the project's root `package.json` under the key `"jatin-lean"`.

Example:
```json
"jatin-lean": {
  "keep": ["docs/", "*.map"],
  "profile": "conservative"
}
```

### 🎯 Acceptance Criteria
- In `src/policy.rs` or `src/config.rs`, parse the root `package.json` of the target project if it exists.
- Extract the `"jatin-lean"` configuration block if present.
- Override global `rules.toml` values with the settings retrieved from the local `package.json`.
- Log a visual notice in CLI stating "Applying local package.json configuration overrides...".

### 📂 Code / Files Involved
- [src/config.rs](src/config.rs)
- [src/policy.rs](src/policy.rs)
- [src/scanner.rs](src/scanner.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Bug] Handle directory scanning permissions gracefully without halting Rayon worker threads",
        "labels": ["gssoc", "difficulty:medium", "domain:rust"],
        "body": """### 📝 Description
When walking nested directories in `node_modules`, `jatin-lean` can occasionally encounter files or subfolders with restricted filesystem permissions (e.g. root-owned folders or read-protected caches).

Currently, this causes file walking threads to return hard error breaks, halting the entire scanning process. We should handle these filesystem permission errors gracefully by logging warning diagnostics and continuing to scan remaining paths.

### 🎯 Acceptance Criteria
- In `src/scanner.rs`, catch permission-denied / access-denied errors during file walking.
- Do not halt the parallel file walk. Skip the restricted folder.
- Store a list of skipped paths due to permission errors.
- Display a warning recommendation summary in the terminal listing the skipped paths and indicating they could not be optimized due to permissions.

### 📂 Code / Files Involved
- [src/scanner.rs](src/scanner.rs)
- [src/cli/node.rs](src/cli/node.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Feature] Implement a telemetry and metrics logger for disk space optimized",
        "labels": ["gssoc", "difficulty:medium", "domain:rust"],
        "body": """### 📝 Description
To let developers track optimization impact over time, we should log metrics of every execution to a local telemetry log file `~/.config/jatin-lean/history.json`.

### 🎯 Acceptance Criteria
- Add `--metrics` flag to `src/main.rs`.
- When active, capture the total size of deleted files, duration of run, number of files pruned, and error count.
- Append this execution record as an entry to the JSON file `~/.config/jatin-lean/history.json`.
- Create a CLI command `jatin-lean analyze size` to read this history file and output cumulative savings metrics (e.g. "Saved a total of 12.4 GB over 5 runs").

### 📂 Code / Files Involved
- [src/cli/analyze.rs](src/cli/analyze.rs)
- [src/deleter.rs](src/deleter.rs)
- [src/main.rs](src/main.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Feature] Add support for cleaning specific nested dependency trees only",
        "labels": ["gssoc", "difficulty:medium", "domain:rust"],
        "body": """### 📝 Description
Sometimes a developer only wants to target pruning for a single, problematic, bloated dependency in `node_modules` (e.g. `lodash` or `aws-sdk`) rather than scanning all packages.

We need to add a command option to target scanning and pruning to specific package directories.

### 🎯 Acceptance Criteria
- Add `--target-package <PKG_NAME>` option to the CLI in `src/main.rs`.
- In the scanning phase (`src/scanner.rs`), restrict package scanning loops to only target folder matches for the specified package.
- Ensure that safety whitelist calculations are still performed accurately.

### 📂 Code / Files Involved
- [src/main.rs](src/main.rs)
- [src/scanner.rs](src/scanner.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Testing] Expand SIMD JSON parsing benchmarks on edge cases",
        "labels": ["gssoc", "difficulty:medium", "domain:testing"],
        "body": """### 📝 Description
Our high-performance parsing module `simd_json.rs` speeds up package.json reads. To ensure absolute reliability and prevent crashes under edge cases, we need to expand our tests with malformed or extreme JSON files.

### 🎯 Acceptance Criteria
- Add new SIMD parsing tests in `src/simd_json.rs` or inside the integration test suite.
- Create edge-case tests with:
  1. Massive JSON blocks (e.g. 5MB+ mock package manifests).
  2. Malformed JSON syntax (missing brackets, trailing commas) to verify safe error recoveries.
  3. JSON strings with unicode characters, escaped strings, and complex nested arrays.
- Assert that fallbacks to standard parsing occur cleanly when SIMD hardware constraints fail.

### 📂 Code / Files Involved
- [src/simd_json.rs](src/simd_json.rs)
- [tests/integration_test.rs](tests/integration_test.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Feature] Implement active watch mode (jatin-lean node watch) using notify",
        "labels": ["gssoc", "difficulty:medium", "domain:rust"],
        "body": """### 📝 Description
To keep a project's `node_modules` permanently optimized without manual interventions, we should implement a background watch command: `jatin-lean node watch`.

This command should monitor the project's root folder for shifts (like `npm install` editing files or adding folders) and trigger silent, incremental prunes.

### 🎯 Acceptance Criteria
- Implement the `watch` subcommand in `src/cli/node.rs` and `src/watcher.rs`.
- Leverage the `notify` crate to watch for file writes to `package-lock.json`, `yarn.lock`, or `pnpm-lock.yaml`.
- When changes occur, debounce for 3-5 seconds to wait for install to finish, then automatically run a silent pruning phase.

### 📂 Code / Files Involved
- [src/cli/node.rs](src/cli/node.rs)
- [src/watcher.rs](src/watcher.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Performance] Implement a unified cache layers (Memory and Disk) using rkyv",
        "labels": ["gssoc", "difficulty:hard", "domain:rust"],
        "body": """### 📝 Description
Scanning a project's node modules with 50,000+ files can take a few hundred milliseconds. To achieve sub-millisecond execution speeds for incremental runs, we should cache scanning meta results on disk using the zero-copy serialization crate `rkyv`.

### 🎯 Acceptance Criteria
- Refactor scanning cache structures in `src/cache.rs` to implement `rkyv::Archive`, `rkyv::Serialize`, and `rkyv::Deserialize`.
- On successful scan, save candidate paths and sizes to a local binary cache file `.jatin-lean/cache.bin`.
- On subsequent runs, verify file modification times. If unchanged, load metadata directly off disk with zero allocation overhead by mapping the raw cache bytes.

### 📂 Code / Files Involved
- [src/cache.rs](src/cache.rs)
- [src/scanner.rs](src/scanner.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Performance] Implement mmap-backed IPC Ring Buffer for telemetry",
        "labels": ["gssoc", "difficulty:hard", "domain:rust"],
        "body": """### 📝 Description
To allow external dashboards or graphical applications to inspect profiling diagnostics and real-time execution speeds without process-spawning latency, we should implement a high-performance IPC ring buffer backed by a shared memory-mapped (`mmap`) file.

### 🎯 Acceptance Criteria
- Build a lock-free IPC ring buffer implementation inside `src/mmap_ipc.rs` and `src/ringbuffer.rs`.
- Back the ring buffer with a shared memory mapping file `.jatin-lean/ipc_telemetry.map` using the `memmap2` crate.
- Write scanning progress metrics (files walked, processing speed, current packages) to the buffer from Rust worker threads using atomic operations.
- Ensure the layout is cross-platform and handles thread synchronization safely.

### 📂 Code / Files Involved
- [src/mmap_ipc.rs](src/mmap_ipc.rs)
- [src/ringbuffer.rs](src/ringbuffer.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Feature] eBPF network trace collector stub enhancements and mock system",
        "labels": ["gssoc", "difficulty:hard", "domain:rust"],
        "body": """### 📝 Description
Our advanced network features in `bpf_verifier.rs` and `xdp_middleware.rs` run stub/mock architectures for portability. We want to enhance these modules to feature a robust mock engine that behaves like a true eBPF filter while compiling on Linux, macOS, and Windows.

On Linux platforms (when compiled with an `ebpf` feature flag), integrate real kernel packet tracing capabilities.

### 🎯 Acceptance Criteria
- Locate `src/bpf_verifier.rs` and `src/xdp_middleware.rs`.
- Build out the mock layer so it emulates network traffic tracing, socket monitoring, and verifier pass/fail scenarios cleanly.
- Integrate platform compilation checks (`#[cfg(target_os = "linux")]`) to hook into real socket verification features when the `ebpf` flag is set.
- Document compilation constraints and kernel dependencies clearly.

### 📂 Code / Files Involved
- [src/bpf_verifier.rs](src/bpf_verifier.rs)
- [src/xdp_middleware.rs](src/xdp_middleware.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [N-API] Optimize zero-copy N-API Buffer transfers for JSON structures",
        "labels": ["gssoc", "difficulty:hard", "domain:javascript"],
        "body": """### 📝 Description
Currently, our N-API bindings serialize candidate lists (which can hold thousands of file paths) into native JavaScript objects, causing execution overhead at the N-API boundary.

We should optimize this data flow by transferring records using raw binary `ArrayBuffer` blocks, avoiding heavy JS heap allocations.

### 🎯 Acceptance Criteria
- In `src/node_bindings.rs`, serialize candidate records into a flat binary format (e.g. flat buffers or custom packed bytes).
- Allocate a native `napi::bindgen_prelude::Buffer` and write this binary payload directly.
- In `index.js`, receive the raw binary buffer and unpack records on the JS side with high-performance JS byte parsers.
- Benchmark and verify the performance improvement.

### 📂 Code / Files Involved
- [src/node_bindings.rs](src/node_bindings.rs)
- [index.js](index.js)
- [index.d.ts](index.d.ts)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Feature] PCIe bottleneck profiler using hardware core performance counters",
        "labels": ["gssoc", "difficulty:hard", "domain:rust"],
        "body": """### 📝 Description
During massive file sweeps, PCIe bottleneck profiling is essential to diagnose whether slow execution stems from storage access limits or memory bottlenecks.

We want to expand `pcie_bottleneck.rs` to query raw hardware CPU performance counters (PMC) and page-fault rates to log hardware bottleneck metrics.

### 🎯 Acceptance Criteria
- Expand profiling logic in `src/pcie_bottleneck.rs`.
- Query Linux core performance events (using `perf_event_open` syscalls or platform equivalents) to track CPU instructions, clock cycles, and memory stalls.
- Correlate filesystem walk throughput with CPU wait counters.
- Provide recommendations when hardware latency is bottlenecked by the bus or storage.

### 📂 Code / Files Involved
- [src/pcie_bottleneck.rs](src/pcie_bottleneck.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    },
    {
        "title": "[GSSOC] [Performance] Implement a Custom Arena Allocator in allocator.rs for file scanner metadata",
        "labels": ["gssoc", "difficulty:hard", "domain:rust"],
        "body": """### 📝 Description
Our parallel scanner allocates thousands of small file paths (`PathBuf`) and strings on the global heap, creating significant allocation and fragmentation overhead under multi-core scans.

We want to replace these heavy heap allocations with a high-performance **Promotable Arena Allocator** inside `src/allocator.rs`.

### 🎯 Acceptance Criteria
- Refactor the arena allocator structure in `src/allocator.rs` to support thread-safe allocations (`bumpalo` or custom arena blocks).
- Allocate memory for candidate strings, structures, and path segments directly within the arena during scanning.
- Free the entire arena in a single, atomic deallocation step once scanning completes, completely avoiding individual file-level deallocations.
- Run benchmarks to prove the allocation speedup.

### 📂 Code / Files Involved
- [src/allocator.rs](src/allocator.rs)
- [src/scanner.rs](src/scanner.rs)

### 🤝 How to Claim
Comment `/claim` below and a maintainer will assign it to you!"""
    }
]

def main():
    print(f"Starting to raise {len(issues)} additional GSSoC issues on GitHub...")
    for idx, issue in enumerate(issues, 1):
        print(f"[{idx}/{len(issues)}] Creating issue: {issue['title']}")
        
        cmd = [
            "gh", "issue", "create",
            "--title", issue["title"],
            "--body", issue["body"]
        ]
        
        for label in issue["labels"]:
            cmd.extend(["--label", label])
            
        try:
            res = subprocess.run(cmd, capture_output=True, text=True, check=True)
            print(f"  ✓ Success! Issue URL: {res.stdout.strip()}")
        except subprocess.CalledProcessError as e:
            print(f"  ❌ Failed to create issue. Error:\n{e.stderr}", file=sys.stderr)
            
        # Avoid rapid hits to GitHub API rate limits
        time.sleep(2)

if __name__ == "__main__":
    main()
