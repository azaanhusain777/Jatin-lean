//! jatin-lean — A high-performance CLI utility to prune non-essential
//! files from node_modules, reducing disk footprint by up to 50%
//! without breaking runtime dependencies.

mod allocator;
mod analytics;
mod benchmark;
mod cache;
mod compress;
mod config;
mod dedup;
mod deleter;
mod display;
mod health;
mod lockfile;
mod mmap;
mod network;
mod plugin;
mod policy;
mod profiler;
mod rules;
mod scanner;
mod simd;
mod snapshot;
mod syscall;
mod tracer;
mod treeshake;
mod visualizer;
mod watcher;
mod ringbuffer;
mod strategy;
mod distributed_cache;
mod analyzer;
mod xdp_middleware;
mod shared_memory_ipc;
mod zero_copy_serde;
mod request_coalescing;
mod adaptive_engine;
mod unified_gateway;
mod simd_json;
mod memory_pool;
mod maglev;
mod io_uring;
mod cpu_cache;
mod hardware_tuning;
mod bpf_verifier;
mod pcie_bottleneck;
mod hedging;
mod mmap_ipc;
mod static_plugins;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use console::style;
use dialoguer::Confirm;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// ⚡ jatin-lean — Prune non-essential files from node_modules
#[derive(Parser, Debug)]
#[command(
    name = "jatin-lean",
    version,
    about = "A high-performance CLI utility to prune non-essential files from node_modules",
    long_about = "Slim your node_modules by up to 50% without breaking runtime dependencies.\n\nBy default, runs in dry-run mode showing what would be deleted.\nUse --force to execute deletion (will prompt for confirmation).\nUse --force --yes to skip the confirmation prompt."
)]
struct Cli {
    /// Path to the project directory (defaults to current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Execute deletion (default is dry-run simulation)
    #[arg(long, short = 'f')]
    force: bool,

    /// Skip confirmation prompt (auto-confirm)
    #[arg(long, short = 'y')]
    yes: bool,

    /// Path to custom config file
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Global mode — scan all projects in a directory
    #[arg(long, short = 'g')]
    global: bool,

    /// Show individual files that would be deleted
    #[arg(long, short = 'v')]
    verbose: bool,

    /// Maximum depth for global scan
    #[arg(long, default_value = "4")]
    max_depth: usize,

    /// Generate example config file
    #[arg(long, value_name = "FILE")]
    init_config: Option<PathBuf>,

    /// Enable performance profiling
    #[arg(long)]
    profile: bool,

    /// Create a snapshot before deletion (for undo support)
    #[arg(long)]
    snapshot: bool,

    /// Export scan results to a file (json, csv, or md)
    #[arg(long, value_name = "FILE")]
    export: Option<PathBuf>,

    /// Subcommands for advanced features
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Advanced subcommands
#[derive(Subcommand, Debug)]
enum Commands {
    /// Show scan history and analytics dashboard
    Analytics {
        /// Clear all analytics data
        #[arg(long)]
        clear: bool,
    },

    /// Undo the last pruning operation
    Undo,

    /// Restore a specific snapshot by ID
    Restore {
        /// The snapshot ID to restore
        snapshot_id: String,
    },

    /// Find duplicate files across packages in node_modules
    Dedup {
        /// Path to the project directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Analyze the dependency graph from lock files
    Deps {
        /// Path to the project directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Manage pre-deletion snapshots
    Snapshots {
        /// List all snapshots
        #[arg(long)]
        list: bool,

        /// Restore a specific snapshot by ID
        #[arg(long, value_name = "SNAPSHOT_ID")]
        restore: Option<String>,

        /// Delete a specific snapshot by ID
        #[arg(long, value_name = "SNAPSHOT_ID")]
        delete: Option<String>,

        /// Clean up snapshots older than N days
        #[arg(long, value_name = "DAYS")]
        cleanup: Option<u64>,
    },

    /// Watch node_modules for changes and auto-prune
    Watch {
        /// Path to the project directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Polling interval in seconds
        #[arg(long, default_value = "5")]
        interval: u64,

        /// Automatically prune on changes (without --force, does dry-run)
        #[arg(long)]
        auto_prune: bool,

        /// Maximum number of prune cycles (0 = unlimited)
        #[arg(long, default_value = "0")]
        max_cycles: u64,
    },

    /// Manage the incremental scan cache
    Cache {
        /// Clear the scan cache
        #[arg(long)]
        clear: bool,

        /// Show cache statistics
        #[arg(long)]
        stats: bool,

        /// Path to the project directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Run a comprehensive health check on node_modules
    Health {
        /// Path to the project directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Analyze tree-shaking potential and dead exports
    Treeshake {
        /// Path to the project directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Analyze compression potential (gzip/brotli transfer sizes)
    Compress {
        /// Path to the project directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Enforce dependency policies
    Policy {
        /// Path to the policy file (TOML or JSON)
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,

        /// Generate an example policy file
        #[arg(long, value_name = "FILE")]
        init: Option<PathBuf>,

        /// Path to the project directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// List and manage plugins
    Plugins {
        /// List all registered plugins
        #[arg(long)]
        list: bool,
    },

    /// Run built-in performance benchmarks
    Bench {
        /// Run all internal component benchmarks
        #[arg(long)]
        all: bool,

        /// Show timer resolution info
        #[arg(long)]
        timer: bool,
    },

    /// Show I/O statistics and filesystem info
    Io {
        /// Path to the project directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Show filesystem info
        #[arg(long)]
        fs_info: bool,

        /// Show process resource usage
        #[arg(long)]
        process: bool,
    },

    /// Render visual analysis of node_modules
    Visualize {
        /// Path to the project directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Show treemap of package sizes
        #[arg(long)]
        treemap: bool,

        /// Show size sparklines
        #[arg(long)]
        sparklines: bool,
    },

    /// Audit installed packages against the npm registry
    Audit {
        /// Path to the project directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Analyze package structure, detect frameworks, and classify packages
    Analyze {
        /// Path to the project directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Manage the distributed cache
    DistCache {
        /// Show distributed cache stats
        #[arg(long)]
        stats: bool,

        /// Clear the distributed cache
        #[arg(long)]
        clear: bool,

        /// Evict expired entries
        #[arg(long)]
        evict: bool,
    },

    /// XDP/eBPF network middleware analysis and benchmarks
    Xdp {
        /// Show architecture comparison table
        #[arg(long)]
        compare: bool,

        /// Run packet processing benchmark
        #[arg(long)]
        bench: bool,

        /// Number of simulated packets for benchmark
        #[arg(long, default_value = "1000000")]
        packets: u64,

        /// Obfuscation mode: none, udp-to-tcp, xor, tls, http
        #[arg(long, default_value = "none")]
        obfuscation: String,
    },

    /// Shared memory IPC ring buffer benchmark
    Ipc {
        /// Run the SPSC ring buffer throughput benchmark
        #[arg(long)]
        bench: bool,

        /// Ring buffer capacity (slots)
        #[arg(long, default_value = "1024")]
        capacity: usize,

        /// Number of messages to send
        #[arg(long, default_value = "100000")]
        messages: u64,

        /// Show memory layout info
        #[arg(long)]
        layout: bool,
    },

    /// Zero-copy serialization benchmark (rkyv vs JSON)
    Serde {
        /// Run serialization benchmark
        #[arg(long)]
        bench: bool,

        /// Number of entities to serialize
        #[arg(long, default_value = "100")]
        entities: usize,

        /// Number of iterations
        #[arg(long, default_value = "1000")]
        iterations: u64,

        /// Show framework comparison table
        #[arg(long)]
        compare: bool,
    },

    /// Request coalescing and cache stampede prevention demo
    Coalesce {
        /// Run singleflight deduplication demo
        #[arg(long)]
        demo: bool,

        /// Number of concurrent requests to simulate
        #[arg(long, default_value = "1000")]
        requests: u64,

        /// Number of unique resource keys
        #[arg(long, default_value = "10")]
        keys: u64,

        /// Show structural cache stats
        #[arg(long)]
        cache_stats: bool,
    },

    /// Adaptive CPU-GPU execution engine analysis
    Engine {
        /// Run workload routing analysis
        #[arg(long)]
        analyze: bool,

        /// Simulate Grace Hopper hardware environment
        #[arg(long)]
        grace_hopper: bool,

        /// Show memory paradigm comparison table
        #[arg(long)]
        compare: bool,

        /// Run multi-workload routing benchmark
        #[arg(long)]
        bench: bool,
    },

    /// Unified 6-stage gateway pipeline benchmark
    Gateway {
        /// Run full pipeline benchmark
        #[arg(long)]
        bench: bool,

        /// Number of requests to process
        #[arg(long, default_value = "10000")]
        requests: u64,

        /// Payload size in bytes
        #[arg(long, default_value = "1024")]
        payload_size: usize,
    },

    /// SIMD-accelerated JSON structural scanner
    SimdJson {
        /// Scan a JSON file
        #[arg(long, value_name = "FILE")]
        file: Option<std::path::PathBuf>,

        /// Inline JSON string to scan
        #[arg(long)]
        input: Option<String>,

        /// Extract all keys from JSON
        #[arg(long)]
        keys: bool,

        /// Run RFC 7396 merge patch demo
        #[arg(long)]
        merge_patch: bool,
    },

    /// Arena memory pool allocator benchmark
    Arena {
        /// Run allocation benchmark
        #[arg(long)]
        bench: bool,

        /// Arena capacity in KB
        #[arg(long, default_value = "1024")]
        capacity_kb: usize,

        /// Number of allocations
        #[arg(long, default_value = "100000")]
        allocations: u64,
    },

    /// Maglev consistent hashing analysis
    Maglev {
        /// Backend server names (comma-separated)
        #[arg(long, default_value = "server-1,server-2,server-3,server-4,server-5")]
        backends: String,

        /// Hash table size (should be prime)
        #[arg(long, default_value = "65537")]
        table_size: usize,

        /// Run distribution analysis
        #[arg(long)]
        analyze: bool,

        /// Test disruption rate when removing a backend
        #[arg(long)]
        disruption: Option<String>,
    },

    /// io_uring async I/O engine benchmark
    IoUring {
        /// Run batched stat benchmark
        #[arg(long)]
        bench: bool,

        /// Number of files to simulate
        #[arg(long, default_value = "10000")]
        files: u64,

        /// Show I/O API comparison table
        #[arg(long)]
        compare: bool,

        /// Use NVMe-optimized settings
        #[arg(long)]
        nvme: bool,
    },

    /// CPU cache optimization and prefetch benchmark
    CpuCache {
        /// Run cache-optimized scan benchmark
        #[arg(long)]
        bench: bool,

        /// Working set size in KB
        #[arg(long, default_value = "8192")]
        working_set_kb: usize,

        /// Show cache hierarchy info
        #[arg(long)]
        info: bool,

        /// Analyze TLB pressure for working set
        #[arg(long)]
        tlb: bool,
    },

    /// System hardware optimization assessment
    Optimize {
        /// Run full system assessment
        #[arg(long)]
        assess: bool,

        /// Show NUMA topology
        #[arg(long)]
        numa: bool,

        /// Show network tuning recommendations
        #[arg(long)]
        network: bool,

        /// Show kernel parameter tuning
        #[arg(long)]
        kernel: bool,

        /// Generate all sysctl commands
        #[arg(long)]
        generate: bool,
    },

    /// BPF verifier simulation & DPI evasion analysis
    Bpf {
        /// Run verifier on sample XDP programs
        #[arg(long)]
        verify: bool,

        /// Show DPI evasion technique matrix
        #[arg(long)]
        dpi: bool,

        /// Calculate sk_buff elimination savings
        #[arg(long, value_name = "PACKETS")]
        skbuff: Option<u64>,
    },

    /// PCIe bottleneck quantifier & CUDA memory analysis
    Pcie {
        /// Compare all memory paradigms
        #[arg(long)]
        compare: bool,

        /// Data transfer size in GB
        #[arg(long, default_value = "1")]
        size_gb: u64,

        /// Simulate LLM layer offloading
        #[arg(long, value_name = "LAYERS")]
        offload: Option<usize>,

        /// Use Grace Hopper (NVLink-C2C) profile
        #[arg(long)]
        grace_hopper: bool,
    },

    /// Request hedging & fragmented cache engine
    Hedge {
        /// Run hedging benchmark
        #[arg(long)]
        bench: bool,

        /// Number of requests to hedge
        #[arg(long, default_value = "10000")]
        requests: u64,

        /// Demo fragmented cache delivery
        #[arg(long)]
        cache_demo: bool,
    },

    /// mmap-backed SPSC ring buffer IPC benchmark
    MmapIpc {
        /// Run IPC throughput benchmark
        #[arg(long)]
        bench: bool,

        /// Ring buffer capacity (slots)
        #[arg(long, default_value = "4096")]
        capacity: usize,

        /// Message size in bytes
        #[arg(long, default_value = "256")]
        msg_size: usize,

        /// Show FFI comparison table
        #[arg(long)]
        compare: bool,
    },

    /// Monomorphic (Static) Dispatch vs Dynamic Dispatch analysis
    StaticDispatch {
        /// Run dispatch benchmark
        #[arg(long)]
        bench: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle --init-config flag
    if let Some(config_path) = cli.init_config {
        config::Config::create_example(&config_path)?;
        println!(
            "  {} Example config file created: {}",
            style("✓").green().bold(),
            style(config_path.display()).cyan()
        );
        println!(
            "  {} Edit this file to customize pruning rules.",
            style("→").dim()
        );
        println!(
            "  {} Use with: {} {}",
            style("→").dim(),
            style("jatin-lean --config").yellow(),
            style(config_path.display()).cyan()
        );
        return Ok(());
    }

    // Handle subcommands
    if let Some(command) = cli.command {
        return handle_subcommand(command);
    }

    display::print_banner();

    let target = std::fs::canonicalize(&cli.path)
        .with_context(|| format!("Cannot access path: {}", cli.path.display()))?;

    if cli.global {
        run_global_mode(&target, cli.max_depth)?;
    } else {
        run_local_mode(
            &target,
            cli.force,
            cli.yes,
            cli.verbose,
            cli.config.as_deref(),
            cli.profile,
            cli.snapshot,
            cli.export.as_deref(),
        )?;
    }

    Ok(())
}

/// Handle subcommands.
fn handle_subcommand(command: Commands) -> Result<()> {
    display::print_banner();

    match command {
        Commands::Undo => {
            let manager = snapshot::SnapshotManager::new()?;
            let snapshots = manager.list_snapshots()?;
            
            if let Some(latest) = snapshots.first() {
                println!(
                    "  {} Undoing last prune operation (Snapshot {})...",
                    style("⏪").magenta().bold(),
                    style(&latest.id).cyan()
                );
                let result = manager.restore_snapshot(&latest.id)?;
                snapshot::print_restore_result(&result);
            } else {
                println!(
                    "  {} No snapshots found to undo. Did you run with {}?",
                    style("✗").red().bold(),
                    style("--snapshot").yellow()
                );
            }
        }

        Commands::Restore { snapshot_id } => {
            let manager = snapshot::SnapshotManager::new()?;
            println!(
                "  {} Restoring snapshot {}...",
                style("⏪").magenta().bold(),
                style(&snapshot_id).cyan()
            );
            match manager.restore_snapshot(&snapshot_id) {
                Ok(result) => snapshot::print_restore_result(&result),
                Err(e) => {
                    println!(
                        "  {} Failed to restore snapshot: {}",
                        style("✗").red().bold(),
                        e
                    );
                }
            }
        }

        Commands::Analytics { clear } => {
            if clear {
                let mut db = analytics::AnalyticsDB::load()?;
                db.clear()?;
                println!("  {} Analytics data cleared.", style("✓").green().bold());
            } else {
                let db = analytics::AnalyticsDB::load()?;
                analytics::print_analytics_summary(&db);
            }
        }

        Commands::Dedup { path } => {
            let target = std::fs::canonicalize(&path)?;
            let nm_path = target.join("node_modules");
            if !nm_path.exists() {
                println!(
                    "  {} No node_modules found at {}",
                    style("✗").red().bold(),
                    style(target.display()).dim()
                );
                return Ok(());
            }
            let result = dedup::find_duplicates(&nm_path)?;
            dedup::print_dedup_results(&result);
        }

        Commands::Deps { path } => {
            let target = std::fs::canonicalize(&path)?;
            let nm_path = target.join("node_modules");
            let graph = lockfile::DependencyGraph::from_project(&target)?;
            lockfile::print_dep_graph_summary(&graph, &nm_path);
        }

        Commands::Snapshots {
            list: _,
            restore,
            delete,
            cleanup,
        } => {
            let manager = snapshot::SnapshotManager::new()?;

            if let Some(snapshot_id) = restore {
                let result = manager.restore_snapshot(&snapshot_id)?;
                snapshot::print_restore_result(&result);
            } else if let Some(snapshot_id) = delete {
                manager.delete_snapshot(&snapshot_id)?;
                println!(
                    "  {} Snapshot {} deleted.",
                    style("✓").green().bold(),
                    style(&snapshot_id).cyan()
                );
            } else if let Some(days) = cleanup {
                let deleted = manager.cleanup_old_snapshots(days)?;
                println!(
                    "  {} Cleaned up {} old snapshots.",
                    style("✓").green().bold(),
                    deleted
                );
            } else {
                // Default: list
                let snapshots = manager.list_snapshots()?;
                snapshot::print_snapshot_list(&snapshots);
            }
        }

        Commands::Watch {
            path,
            interval,
            auto_prune,
            max_cycles,
        } => {
            let target = std::fs::canonicalize(&path)?;
            let nm_path = target.join("node_modules");
            if !nm_path.exists() {
                println!(
                    "  {} No node_modules found at {}",
                    style("✗").red().bold(),
                    style(target.display()).dim()
                );
                return Ok(());
            }

            let config = watcher::WatcherConfig {
                poll_interval_secs: interval,
                auto_prune,
                max_cycles,
                ..Default::default()
            };

            let mut w = watcher::NodeModulesWatcher::new(nm_path.clone(), config);
            let running = w.running_flag();

            // Set up Ctrl+C handler
            let running_clone = running.clone();
            ctrlc_handler(move || {
                running_clone.store(false, std::sync::atomic::Ordering::SeqCst);
            });

            w.watch(|nm_path| {
                let rules = rules::PruneRules::new();
                let scan_result = scanner::scan_node_modules(nm_path, &rules, None)?;
                display::print_discovery(&scan_result);
                display::print_simulation(&scan_result);
                Ok(())
            })?;
        }

        Commands::Cache {
            clear,
            stats: _,
            path,
        } => {
            let target = std::fs::canonicalize(&path)?;
            let nm_path = target.join("node_modules");

            if clear {
                let mut c = cache::ScanCache::load(&nm_path);
                c.clear();
                c.save(&nm_path)?;
                println!("  {} Scan cache cleared.", style("✓").green().bold());
            } else {
                let c = cache::ScanCache::load(&nm_path);
                println!();
                println!(
                    "  {} {}",
                    style("Scan Cache").cyan().bold(),
                    style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
                );
                println!(
                    "  {} Cached packages: {}",
                    style("◉").cyan(),
                    style(c.cached_count()).white().bold()
                );
                println!("  {} Cache age: {}s", style("◉").cyan(), c.age_seconds());
                println!(
                    "  {} Cache file: {}",
                    style("◉").dim(),
                    style(cache::ScanCache::cache_path(&nm_path).display()).dim()
                );
                println!();
            }
        }

        Commands::Health { path } => {
            let target = std::fs::canonicalize(&path)?;
            let nm_path = target.join("node_modules");
            if !nm_path.exists() {
                println!(
                    "  {} No node_modules found at {}",
                    style("✗").red().bold(),
                    style(target.display()).dim()
                );
                return Ok(());
            }
            let report = health::check_health(&nm_path)?;
            health::print_health_report(&report);
        }

        Commands::Treeshake { path } => {
            let target = std::fs::canonicalize(&path)?;
            let nm_path = target.join("node_modules");
            if !nm_path.exists() {
                println!(
                    "  {} No node_modules found at {}",
                    style("✗").red().bold(),
                    style(target.display()).dim()
                );
                return Ok(());
            }
            let result = treeshake::analyze_treeshake(&nm_path)?;
            treeshake::print_treeshake_results(&result);
        }

        Commands::Compress { path } => {
            let target = std::fs::canonicalize(&path)?;
            let nm_path = target.join("node_modules");
            if !nm_path.exists() {
                println!(
                    "  {} No node_modules found at {}",
                    style("✗").red().bold(),
                    style(target.display()).dim()
                );
                return Ok(());
            }
            let result = compress::analyze_compression(&nm_path)?;
            compress::print_compression_results(&result);
        }

        Commands::Policy { file, init, path } => {
            if let Some(init_path) = init {
                policy::create_example_policy(&init_path)?;
                println!(
                    "  {} Example policy created: {}",
                    style("✓").green().bold(),
                    style(init_path.display()).cyan()
                );
                return Ok(());
            }

            if let Some(policy_file) = file {
                let target = std::fs::canonicalize(&path)?;
                let nm_path = target.join("node_modules");
                if !nm_path.exists() {
                    println!(
                        "  {} No node_modules found at {}",
                        style("✗").red().bold(),
                        style(target.display()).dim()
                    );
                    return Ok(());
                }
                let p = policy::load_policy(&policy_file)?;
                let result = policy::enforce_policy(&p, &nm_path)?;
                policy::print_policy_result(&result);

                if !result.is_compliant {
                    std::process::exit(1);
                }
            } else {
                println!(
                    "  {} Specify a policy file with {} or generate one with {}",
                    style("ℹ").blue(),
                    style("--file <FILE>").yellow(),
                    style("--init <FILE>").yellow()
                );
            }
        }

        Commands::Plugins { list: _ } => {
            let registry = plugin::PluginRegistry::with_builtins();
            plugin::print_plugin_info(&registry);
        }

        Commands::Bench { all, timer } => {
            if timer {
                benchmark::print_timer_info();
            }

            if all || !timer {
                println!(
                    "  {} Running built-in benchmarks...\n",
                    style("⚡").yellow().bold()
                );

                // CPU capabilities
                let caps = simd::CpuCapabilities::detect();
                println!(
                    "  {} {}",
                    style("CPU Features").cyan().bold(),
                    style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
                );
                println!(
                    "  {} Architecture: {}",
                    style("◉").cyan(),
                    style(&caps.arch).white().bold()
                );
                println!(
                    "  {} SIMD tier: {}",
                    style("◉").cyan(),
                    style(caps.tier_name()).green().bold()
                );
                println!();

                let suite = benchmark::run_builtin_benchmarks();
                suite.print_results();
            }
        }

        Commands::Io {
            path,
            fs_info,
            process,
        } => {
            let target = std::fs::canonicalize(&path)?;
            let nm_path = target.join("node_modules");

            if nm_path.exists() {
                let stats = mmap::io_stats(&nm_path)?;
                stats.print_info();
            } else {
                println!(
                    "  {} No node_modules found. Analyzing project directory...",
                    style("ℹ").blue()
                );
                let stats = mmap::io_stats(&target)?;
                stats.print_info();
            }

            if fs_info {
                let info = syscall::FsInfo::query(&target)?;
                info.print_info();
            }

            if process {
                let proc = syscall::ProcessStats::current();
                proc.print_info();
            }
        }

        Commands::Visualize {
            path,
            treemap,
            sparklines,
        } => {
            let target = std::fs::canonicalize(&path)?;
            let nm_path = target.join("node_modules");

            if !nm_path.exists() {
                println!(
                    "  {} No node_modules found at {}",
                    style("✗").red().bold(),
                    style(target.display()).dim()
                );
                return Ok(());
            }

            // Build treemap data from scanning
            let rules = rules::PruneRules::new();
            let scan_result = scanner::scan_node_modules(&nm_path, &rules, None)?;

            if treemap || (!treemap && !sparklines) {
                // Group candidates by category for treemap
                let mut by_cat: std::collections::HashMap<String, u64> =
                    std::collections::HashMap::new();
                for c in &scan_result.candidates {
                    *by_cat.entry(c.category.label().to_string()).or_default() += c.size;
                }

                let children: Vec<visualizer::TreemapNode> = by_cat
                    .iter()
                    .map(|(name, size)| visualizer::TreemapNode::new(name, *size))
                    .collect();

                let root =
                    visualizer::TreemapNode::with_children("node_modules (prunable)", children);

                visualizer::render_treemap(&root, 60);
            }

            if sparklines {
                // Show package-level size distribution as bar chart
                let pkg_data = scanner::package_sizes(&nm_path);
                let mut entries: Vec<visualizer::BarChartEntry> = pkg_data
                    .iter()
                    .take(20)
                    .map(|(name, size)| visualizer::BarChartEntry {
                        label: name.clone(),
                        value: *size as f64,
                        display_value: scanner::format_size(*size),
                    })
                    .collect();

                // Sort by value descending
                entries.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap());

                visualizer::render_bar_chart("Top 20 Packages by Size", &entries, 40);
            }
        }

        Commands::Audit { path } => {
            let target = std::fs::canonicalize(&path)?;
            let nm_path = target.join("node_modules");

            if !nm_path.exists() {
                println!(
                    "  {} No node_modules found at {}",
                    style("✗").red().bold(),
                    style(target.display()).dim()
                );
                return Ok(());
            }

            let installed = network::scan_installed_packages(&nm_path)?;
            println!(
                "  {} Found {} installed packages",
                style("◉").cyan(),
                style(installed.len()).white().bold()
            );
            println!(
                "  {} Note: Version auditing requires network access.",
                style("ℹ").blue()
            );
            println!(
                "  {} Run with {} for full audit.",
                style("→").dim(),
                style("--features network").yellow()
            );

            // Show local-only analysis
            for (name, version) in installed.iter().take(25) {
                println!("  {} {}@{}", style("·").dim(), name, style(version).dim());
            }
            if installed.len() > 25 {
                println!(
                    "  {} ...and {} more",
                    style("·").dim(),
                    installed.len() - 25
                );
            }
        }

        Commands::Analyze { path } => {
            let target = std::fs::canonicalize(&path)?;
            let nm_path = target.join("node_modules");
            if !nm_path.exists() {
                println!(
                    "  {} No node_modules found at {}",
                    style("✗").red().bold(),
                    style(target.display()).dim()
                );
                return Ok(());
            }
            let analysis = analyzer::analyze_project(&nm_path)?;
            analyzer::print_analysis(&analysis);

            // Also show strategy recommendations
            let engine = strategy::StrategyEngine::new();
            let profiles: Vec<strategy::PackageProfile> = analysis.package_analyses.iter()
                .map(|a| strategy::PackageProfile {
                    name: a.name.clone(),
                    file_count: a.file_count as i64,
                    total_size: a.total_size,
                    has_package_json: true,
                    has_native_bindings: a.has_native,
                    is_scoped: a.name.starts_with('@'),
                    is_cached: false,
                    framework: a.frameworks.first().map(|f| f.label().to_string()),
                    previous_scan_time: None,
                })
                .collect();

            let strategies = engine.select_batch(&profiles);
            let summary = engine.strategy_summary(&strategies);
            summary.print();
        }

        Commands::DistCache { stats, clear, evict } => {
            if clear {
                let mut cache = distributed_cache::DistributedCache::with_defaults()?;
                cache.clear()?;
                println!("  {} Distributed cache cleared.", style("✓").green().bold());
            } else if evict {
                let mut cache = distributed_cache::DistributedCache::with_defaults()?;
                let evicted = cache.evict_expired()?;
                println!(
                    "  {} Evicted {} expired entries.",
                    style("✓").green().bold(),
                    evicted
                );
            } else {
                // Default: show stats
                let cache = distributed_cache::DistributedCache::with_defaults()?;
                distributed_cache::print_cache_info(&cache);
            }
        }

        Commands::Xdp { compare, bench, packets, obfuscation } => {
            use xdp_middleware::*;
            println!("  {} {}", style("eBPF/XDP Network Middleware").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

            if compare {
                print_architecture_comparison();
            }

            if bench {
                let obfs_mode = match obfuscation.as_str() {
                    "udp-to-tcp" | "udp" => ObfuscationMode::UdpToTcp,
                    "xor" => ObfuscationMode::XorScramble,
                    "tls" => ObfuscationMode::TlsMimicry,
                    "http" => ObfuscationMode::HttpInject,
                    _ => ObfuscationMode::None,
                };

                println!("  {} Running XDP benchmark: {} packets, obfuscation: {}",
                    style("⚡").yellow(), style(packets).green().bold(),
                    style(obfs_mode.label()).white());

                let config = XdpLoadBalancerConfig::default();
                let cp = XdpControlPlane::new(config);
                let start = std::time::Instant::now();

                // Simulate packet processing
                let dummy_packet = vec![0u8; 1500]; // Standard MTU
                for _ in 0..packets {
                    cp.inject_live_frame(&dummy_packet, "eth0");
                }

                let elapsed = start.elapsed();
                let pps = packets as f64 / elapsed.as_secs_f64();
                let gbps = (packets as f64 * 1500.0 * 8.0) / (elapsed.as_secs_f64() * 1e9);

                println!();
                println!("  {} Throughput: {} Mpps",
                    style("🚀").yellow(), style(format!("{:.2}", pps / 1e6)).green().bold());
                println!("  {} Bandwidth: {} Gbps",
                    style("🚀").yellow(), style(format!("{:.2}", gbps)).green().bold());
                println!("  {} Elapsed:   {} ms",
                    style("▸").dim(), style(format!("{:.2}", elapsed.as_secs_f64() * 1000.0)).white());

                // XOR scramble demo
                if obfs_mode == ObfuscationMode::XorScramble {
                    let mut payload = b"Hello, XDP!".to_vec();
                    println!("  {} XOR scramble demo:", style("▸").dim());
                    println!("    Original:   {:?}", std::str::from_utf8(&payload).unwrap());
                    XdpControlPlane::xor_scramble(&mut payload, 0xDEADBEEF);
                    println!("    Scrambled:  {:02x?}", &payload);
                    XdpControlPlane::xor_scramble(&mut payload, 0xDEADBEEF);
                    println!("    Recovered:  {:?}", std::str::from_utf8(&payload).unwrap());
                }

                println!();
            }

            if !compare && !bench {
                print_architecture_comparison();
                println!("  {} Use {} for packet benchmark",
                    style("→").dim(), style("jatin-lean xdp --bench").yellow());
                println!("  {} Use {} for obfuscation modes",
                    style("→").dim(), style("--obfuscation udp-to-tcp|xor|tls|http").yellow());
            }
        }

        Commands::Ipc { bench, capacity, messages, layout } => {
            use shared_memory_ipc::*;
            println!("  {} {}", style("Lock-Free Shared Memory IPC").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

            if layout {
                println!();
                println!("  {} Memory Layout:", style("📐").yellow());
                println!("    Cache line size:       {} bytes", CACHE_LINE_SIZE);
                println!("    AlignedAtomicIndex:    {} bytes (aligned to {})",
                    std::mem::size_of::<AlignedAtomicIndex>(), std::mem::align_of::<AlignedAtomicIndex>());
                println!("    SpscRingHeader:        {} bytes",
                    std::mem::size_of::<SpscRingHeader>());
                println!("    MessageSlot:           {} bytes ({}KB)",
                    MessageSlot::SIZE, MessageSlot::SIZE / 1024);
                println!("    Max payload per slot:  {} bytes", MessageSlot::MAX_PAYLOAD);
                let region_size = SharedMemoryRegion::required_size(capacity, MessageSlot::SIZE);
                println!("    Shared memory needed:  {} bytes ({:.1} MB) for {} slots",
                    region_size, region_size as f64 / (1024.0 * 1024.0), capacity);
                println!();
            }

            if bench {
                println!("  {} SPSC Ring Buffer Benchmark: {} msgs, {} slots",
                    style("⚡").yellow(),
                    style(messages).green().bold(),
                    style(capacity).white());

                let ring = SpscIpcRing::new(capacity);
                let payload = b"benchmark-payload-data-for-ipc-testing-12345678";
                let start = std::time::Instant::now();

                for i in 0..messages {
                    // Push with wraparound (pop to keep space)
                    while ring.push(1, payload).is_err() {
                        ring.pop();
                    }
                    if i % 2 == 0 { ring.pop(); }
                }
                // Drain remaining
                while ring.pop().is_some() {}

                let elapsed = start.elapsed();
                print_ipc_report(&ring.stats, elapsed);

                println!("  {} Total elapsed: {:.2} ms",
                    style("▸").dim(), elapsed.as_secs_f64() * 1000.0);
                println!("  {} Ops/sec: {:.0}",
                    style("🚀").yellow(),
                    messages as f64 / elapsed.as_secs_f64());
                println!();
            }

            if !bench && !layout {
                println!();
                println!("  {} Use {} for memory layout info",
                    style("→").dim(), style("jatin-lean ipc --layout").yellow());
                println!("  {} Use {} for throughput benchmark",
                    style("→").dim(), style("jatin-lean ipc --bench").yellow());
                println!();
            }
        }

        Commands::Serde { bench, entities, iterations, compare } => {
            use zero_copy_serde::*;
            println!("  {} {}", style("Zero-Copy Serialization Engine").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

            if compare {
                let table = benchmark_table();
                println!();
                for b in &table {
                    let mutation = if b.supports_mutation { style("✓").green() } else { style("✗").red() };
                    println!("  {} {} | {} | {} | Mutation: {}",
                        style("▸").dim(), style(b.framework).yellow().bold(),
                        style(b.access_latency).white(),
                        style(b.deser_speed).dim(),
                        mutation);
                }
                println!();
            }

            if bench {
                println!("  {} Benchmark: {} entities × {} iterations",
                    style("⚡").yellow(),
                    style(entities).green().bold(),
                    style(iterations).white());

                let mut engine = ZeroCopyEngine::new();
                let response = ZeroCopyEngine::sample_response(entities);

                // Warmup
                let bytes = engine.serialize(&response);
                let _ = engine.deserialize_from(&bytes);
                engine.stats = SerdeStats::default();

                // rkyv serialize
                let start = std::time::Instant::now();
                let mut last_bytes = Vec::new();
                for _ in 0..iterations {
                    last_bytes = engine.serialize(&response);
                }
                let ser_time = start.elapsed();

                // rkyv deserialize
                let start = std::time::Instant::now();
                for _ in 0..iterations {
                    let _ = engine.deserialize_from(&last_bytes);
                }
                let deser_time = start.elapsed();

                // JSON serialize
                let start = std::time::Instant::now();
                let mut json_str = String::new();
                for _ in 0..iterations {
                    json_str = ZeroCopyEngine::to_json(&response);
                }
                let json_ser_time = start.elapsed();

                // JSON parse
                let start = std::time::Instant::now();
                for _ in 0..iterations {
                    let _ = engine.parse_json(&json_str);
                }
                let json_parse_time = start.elapsed();

                // Zero-copy access
                let start = std::time::Instant::now();
                for _ in 0..iterations {
                    let _ = ZeroCopyEngine::access_archived(&last_bytes);
                }
                let access_time = start.elapsed();

                println!();
                println!("  {} rkyv serialize:    {:.0} ns/op ({} bytes)",
                    style("⚡").yellow(),
                    ser_time.as_nanos() as f64 / iterations as f64,
                    last_bytes.len());
                println!("  {} rkyv deserialize:  {:.0} ns/op",
                    style("⚡").yellow(),
                    deser_time.as_nanos() as f64 / iterations as f64);
                println!("  {} rkyv zero-copy:    {:.0} ns/op",
                    style("🚀").green(),
                    access_time.as_nanos() as f64 / iterations as f64);
                println!("  {} JSON serialize:    {:.0} ns/op ({} bytes)",
                    style("▸").dim(),
                    json_ser_time.as_nanos() as f64 / iterations as f64,
                    json_str.len());
                println!("  {} JSON parse:        {:.0} ns/op",
                    style("▸").dim(),
                    json_parse_time.as_nanos() as f64 / iterations as f64);

                let ser_speedup = json_ser_time.as_nanos() as f64 / ser_time.as_nanos().max(1) as f64;
                let deser_speedup = json_parse_time.as_nanos() as f64 / deser_time.as_nanos().max(1) as f64;
                println!();
                println!("  {} Serialize speedup:  {}x faster than JSON",
                    style("🚀").yellow(), style(format!("{:.1}", ser_speedup)).green().bold());
                println!("  {} Deserialize speedup: {}x faster than JSON",
                    style("🚀").yellow(), style(format!("{:.1}", deser_speedup)).green().bold());
                println!("  {} Size savings:        {} bytes vs {} bytes ({:.0}% smaller)",
                    style("📦").yellow(),
                    style(last_bytes.len()).green().bold(),
                    style(json_str.len()).dim(),
                    (1.0 - last_bytes.len() as f64 / json_str.len() as f64) * 100.0);
                println!();
            }

            if !bench && !compare {
                println!();
                println!("  {} Use {} for framework comparison",
                    style("→").dim(), style("jatin-lean serde --compare").yellow());
                println!("  {} Use {} for serialization benchmark",
                    style("→").dim(), style("jatin-lean serde --bench").yellow());
                println!();
            }
        }

        Commands::Coalesce { demo, requests, keys, cache_stats } => {
            use request_coalescing::*;
            println!("  {} {}", style("Request Coalescing Engine").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

            if demo || !cache_stats {
                println!("  {} Singleflight demo: {} requests across {} unique keys",
                    style("⚡").yellow(),
                    style(requests).green().bold(),
                    style(keys).white());

                let sf = SingleflightGroup::<String>::new();
                let start = std::time::Instant::now();

                for i in 0..requests {
                    let key = format!("/api/resource/{}", i % keys);
                    sf.do_once(&key, || {
                        format!("response-for-key-{}", i % keys)
                    });
                }

                let elapsed = start.elapsed();
                print_coalescing_report(&sf.stats);

                println!("  {} Elapsed:           {:.2} ms",
                    style("▸").dim(), elapsed.as_secs_f64() * 1000.0);
                println!("  {} Requests/sec:      {:.0}",
                    style("🚀").yellow(),
                    requests as f64 / elapsed.as_secs_f64());
                println!();

                // JSONPath demo
                println!("  {} JSONPath Query Demo:", style("🔍").yellow());
                let json: serde_json::Value = serde_json::json!({
                    "data": {
                        "users": [
                            { "name": "Alice", "email": "alice@example.com", "role": "admin" },
                            { "name": "Bob", "email": "bob@example.com", "role": "user" }
                        ],
                        "meta": { "total": 2, "page": 1 }
                    }
                });
                let paths = vec!["$.data.users[0].name", "$.data.users[1].email", "$.data.meta.total"];
                for p in &paths {
                    let expr = JsonPathExpr::parse(p);
                    if let Some(val) = expr.extract(&json) {
                        println!("    {} → {}", style(p).yellow(), style(val).green());
                    }
                }

                // Request merger demo
                println!();
                println!("  {} Request Merger Demo:", style("🔀").yellow());
                let merger = RequestMerger::new(std::time::Duration::from_millis(10));
                merger.submit(MergeableRequest {
                    client_id: "Client-A".into(), resource_path: "/api/users/1".into(),
                    requested_fields: vec!["name".into(), "email".into()],
                    arrived_at: std::time::Instant::now(),
                });
                merger.submit(MergeableRequest {
                    client_id: "Client-B".into(), resource_path: "/api/users/1".into(),
                    requested_fields: vec!["email".into(), "role".into()],
                    arrived_at: std::time::Instant::now(),
                });
                let merged = merger.flush();
                for q in &merged {
                    println!("    {} clients → superset: {:?}",
                        style(q.client_count).green().bold(),
                        q.superset_fields);
                }
                println!();
            }

            if cache_stats {
                let cache = StructuralCache::new(std::time::Duration::from_secs(60));
                let mut fields = std::collections::HashMap::new();
                fields.insert("name".to_string(), serde_json::json!("Alice"));
                fields.insert("email".to_string(), serde_json::json!("alice@example.com"));
                fields.insert("role".to_string(), serde_json::json!("admin"));
                cache.store_fields("/api/users/1", fields);

                let (found, missing) = cache.get_fields("/api/users/1",
                    &["name".to_string(), "email".to_string(), "phone".to_string()]);
                println!();
                println!("  {} Structural Cache Demo:", style("📊").yellow());
                println!("    Requested: [name, email, phone]");
                println!("    Found:     {} fields (partial hit)", style(found.len()).green().bold());
                println!("    Missing:   {:?}", missing);
                println!("    Hit rate:  {:.1}%", cache.stats.hit_rate());
                println!("    Stored:    {} total fields", cache.total_fields());
                println!();
            }
        }

        Commands::Engine { analyze, grace_hopper, compare, bench } => {
            use adaptive_engine::*;
            println!("  {} {}", style("Adaptive CPU-GPU Execution Engine").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

            if compare {
                println!();
                let paradigms = [
                    MemoryParadigm::PageableSystemMemory,
                    MemoryParadigm::PinnedMemory,
                    MemoryParadigm::SoftwareCoherentUnified,
                    MemoryParadigm::HardwareCoherentUnified,
                ];
                for p in &paradigms {
                    println!("  {} {} | {} | {} | {}",
                        style("▸").dim(), style(p.label()).yellow().bold(),
                        style(p.bandwidth()).white(),
                        style(p.setup_complexity()).dim(),
                        style(p.optimal_hardware()).cyan());
                }
                println!();
            }

            let hw = if grace_hopper {
                println!("  {} Simulating NVIDIA Grace Hopper GH200 environment",
                    style("🖥️").yellow());
                HardwareState::simulated_grace_hopper()
            } else {
                HardwareState::detect()
            };
            let engine = AdaptiveEngine::with_hardware(hw.clone());

            if analyze || bench {
                let workloads = vec![
                    WorkloadProfile {
                        id: "regex-parser".into(), data_size_bytes: 50 * 1024 * 1024,
                        estimated_flops: 5_000_000, parallelizable: false,
                        string_heavy: true, matrix_heavy: false,
                        branch_divergent: true, memory_required_bytes: 50 * 1024 * 1024,
                        workload_type: WorkloadType::StringProcessing,
                    },
                    WorkloadProfile {
                        id: "matmul-4096".into(), data_size_bytes: 128 * 1024 * 1024,
                        estimated_flops: 1_000_000_000, parallelizable: true,
                        string_heavy: false, matrix_heavy: true,
                        branch_divergent: false, memory_required_bytes: 128 * 1024 * 1024,
                        workload_type: WorkloadType::MatrixComputation,
                    },
                    WorkloadProfile {
                        id: "embeddings".into(), data_size_bytes: 512 * 1024 * 1024,
                        estimated_flops: 500_000_000, parallelizable: true,
                        string_heavy: false, matrix_heavy: true,
                        branch_divergent: false, memory_required_bytes: 512 * 1024 * 1024,
                        workload_type: WorkloadType::VectorEmbedding,
                    },
                    WorkloadProfile {
                        id: "llm-70b".into(),
                        data_size_bytes: 140u64 * 1024 * 1024 * 1024,
                        estimated_flops: 10_000_000_000, parallelizable: true,
                        string_heavy: false, matrix_heavy: true,
                        branch_divergent: false,
                        memory_required_bytes: 140u64 * 1024 * 1024 * 1024,
                        workload_type: WorkloadType::LlmInference,
                    },
                    WorkloadProfile {
                        id: "data-aggregate".into(), data_size_bytes: 2 * 1024 * 1024,
                        estimated_flops: 100_000, parallelizable: true,
                        string_heavy: false, matrix_heavy: false,
                        branch_divergent: false, memory_required_bytes: 2 * 1024 * 1024,
                        workload_type: WorkloadType::DataAggregation,
                    },
                ];

                println!();
                for w in &workloads {
                    let result = engine.execute(w);
                    println!("  {} {} → {} {} (speedup: {}x, reason: {})",
                        result.target.icon(),
                        style(&w.id).white().bold(),
                        result.target.label(),
                        style("").dim(),
                        style(format!("{:.0}", result.estimated_speedup)).green().bold(),
                        style(&result.decision_reason).dim());
                }

                println!();
                print_engine_report(&engine.stats, &engine.hardware);
            }

            if !analyze && !bench && !compare {
                print_engine_report(&engine.stats, &engine.hardware);
                println!("  {} Use {} for workload routing analysis",
                    style("→").dim(), style("jatin-lean engine --analyze").yellow());
                println!("  {} Use {} for Grace Hopper simulation",
                    style("→").dim(), style("jatin-lean engine --grace-hopper --analyze").yellow());
                println!();
            }
        }

        Commands::Gateway { bench, requests, payload_size } => {
            use unified_gateway::*;
            println!("  {} {}", style("Unified Gateway Pipeline").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

            let gw = UnifiedGateway::new();

            if bench {
                println!("  {} Benchmark: {} requests × {} byte payloads",
                    style("⚡").yellow(), style(requests).green().bold(),
                    style(payload_size).white());
                let result = gw.benchmark(requests, payload_size);
                println!();
                println!("  {} RPS:         {}", style("🚀").yellow(),
                    style(format!("{:.0}", result.rps)).green().bold());
                println!("  {} Avg latency: {:.0} ns", style("⚡").yellow(), result.avg_latency_ns);
                println!("  {} Elapsed:     {:.2} ms", style("▸").dim(),
                    result.elapsed.as_secs_f64() * 1000.0);
                for (stage, ns) in &result.stage_latencies {
                    println!("    {} {} → {:.0} ns", stage.icon(), stage.label(), ns);
                }
            }

            println!();
            print_gateway_report(&gw);
        }

        Commands::SimdJson { file, input, keys, merge_patch } => {
            use simd_json::*;
            println!("  {} {}", style("SIMD JSON Structural Scanner").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

            let scanner = SimdJsonScanner::new();
            println!("  {} SIMD width: {} bytes/chunk", style("▸").dim(), scanner.chunk_size);

            let json_bytes = if let Some(ref path) = file {
                std::fs::read(path).unwrap_or_default()
            } else if let Some(ref s) = input {
                s.as_bytes().to_vec()
            } else {
                // Demo JSON
                serde_json::to_vec_pretty(&serde_json::json!({
                    "project": "jatin-lean",
                    "version": "0.5.1",
                    "features": ["xdp", "ipc", "rkyv", "coalescing", "gpu"],
                    "stats": {"modules": 40, "tests": 230, "loc": 15000}
                })).unwrap()
            };

            let scan = scanner.scan(&json_bytes);
            print_simd_report(&scan);

            if keys {
                let extracted = scanner.extract_keys(&json_bytes, &scan);
                println!("  {} Extracted {} keys:", style("🔑").yellow(), extracted.len());
                for k in &extracted {
                    println!("    {} {}", style("→").dim(), style(k).yellow());
                }
                println!();
            }

            if merge_patch {
                println!("  {} JSON Merge Patch (RFC 7396) Demo:", style("🔀").yellow());
                let mut original = serde_json::json!({"name":"Alice","age":30,"city":"NYC"});
                let patch = serde_json::json!({"age":31,"city":null,"role":"admin"});
                println!("    Original: {}", serde_json::to_string(&original).unwrap());
                println!("    Patch:    {}", serde_json::to_string(&patch).unwrap());
                json_merge_patch(&mut original, &patch);
                println!("    Result:   {}", style(serde_json::to_string(&original).unwrap()).green());
                println!();
            }
        }

        Commands::Arena { bench, capacity_kb, allocations } => {
            use memory_pool::*;
            println!("  {} {}", style("Arena Memory Pool Allocator").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

            let capacity = capacity_kb * 1024;
            let arena = memory_pool::Arena::new(capacity);

            if bench {
                println!("  {} Benchmark: {} allocations in {} KB arena",
                    style("⚡").yellow(), style(allocations).green().bold(),
                    style(capacity_kb).white());

                let start = std::time::Instant::now();
                let mut success = 0u64;
                for _ in 0..allocations {
                    if arena.alloc(32, 8).is_some() { success += 1; }
                }
                let elapsed = start.elapsed();

                println!("  {} Successful: {}/{}", style("▸").dim(), success, allocations);
                println!("  {} Elapsed:    {:.2} ms", style("▸").dim(),
                    elapsed.as_secs_f64() * 1000.0);
                println!("  {} Allocs/sec: {:.0}", style("🚀").yellow(),
                    success as f64 / elapsed.as_secs_f64());
                println!("  {} Avg alloc:  {:.0} ns", style("⚡").yellow(),
                    elapsed.as_nanos() as f64 / success.max(1) as f64);

                // Compare: TypedPool benchmark
                arena.reset();
                let pool = TypedPool::<ScanEntry>::new(allocations as usize);
                let start2 = std::time::Instant::now();
                for i in 0..allocations {
                    pool.alloc_init(ScanEntry::new(&format!("file-{}.js", i), 1024, true, 1, 2));
                }
                let elapsed2 = start2.elapsed();
                println!("  {} TypedPool:  {:.0} ns/alloc",
                    style("⚡").yellow(), elapsed2.as_nanos() as f64 / allocations as f64);
            }

            println!();
            print_arena_report(&arena);
        }

        Commands::Maglev { backends, table_size, analyze, disruption } => {
            use maglev::*;
            println!("  {} {}", style("Maglev Consistent Hash Ring").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

            let backend_list: Vec<String> = backends.split(',')
                .map(|s| s.trim().to_string()).collect();

            let mut ring = MaglevHashRing::new(backend_list, table_size);

            if analyze {
                // Run lookup benchmark
                let start = std::time::Instant::now();
                for i in 0..100_000 {
                    ring.lookup(&format!("key-{}", i));
                }
                let elapsed = start.elapsed();
                println!("  {} 100K lookups: {:.2} ms ({:.0} ns/lookup)",
                    style("⚡").yellow(), elapsed.as_secs_f64() * 1000.0,
                    elapsed.as_nanos() as f64 / 100_000.0);
            }

            if let Some(ref removed) = disruption {
                let rate = ring.disruption_rate(removed);
                println!("  {} Disruption when removing '{}': {:.1}%",
                    style("⚠").yellow(), style(removed).red(), rate);
                let ideal = 100.0 / ring.backends.len() as f64;
                println!("  {} Ideal disruption: {:.1}% | Overhead: {:.1}%",
                    style("▸").dim(), ideal, rate - ideal);
            }

            print_maglev_report(&ring);
        }

        Commands::IoUring { bench, files, compare, nvme } => {
            use io_uring::*;
            println!("  {} {}", style("io_uring Async I/O Engine").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

            let config = if nvme {
                println!("  {} NVMe-optimized mode", style("⚡").yellow());
                IoUringConfig::nvme_optimized()
            } else {
                IoUringConfig::scan_optimized()
            };

            if compare {
                let table = io_api_comparison();
                println!();
                for entry in &table {
                    let bypass = if entry.kernel_bypass { style("✓").green() } else { style("✗").red() };
                    let zc = if entry.zero_copy { style("✓").green() } else { style("✗").red() };
                    println!("  {} {} | Syscalls: {} | Bypass: {} | ZeroCopy: {}",
                        style("▸").dim(), style(entry.api).yellow(),
                        entry.syscalls_per_op, bypass, zc);
                    println!("    Best for: {}", style(entry.best_for).dim());
                }
                println!();
            }

            if bench {
                let mut engine = IoUringEngine::new(config.clone());
                let paths: Vec<std::path::PathBuf> = (0..files)
                    .map(|i| std::path::PathBuf::from(format!("node_modules/pkg-{}/index.js", i)))
                    .collect();

                println!("  {} Benchmark: {} batched stat operations", style("⚡").yellow(),
                    style(files).green().bold());

                let start = std::time::Instant::now();
                // Submit in batches of SQ depth
                let batch_size = config.sq_depth as usize;
                for chunk in paths.chunks(batch_size) {
                    engine.submit_stat_batch(&chunk.to_vec());
                    engine.flush();
                }
                let elapsed = start.elapsed();

                print_iouring_report(&engine.stats, &config, elapsed);

                let traditional_syscalls = files;
                let uring_syscalls = engine.stats.batches.load(std::sync::atomic::Ordering::Relaxed);
                println!("  {} Traditional: {} syscalls | io_uring: {} syscalls ({}x reduction)",
                    style("🚀").yellow(),
                    style(traditional_syscalls).red(),
                    style(uring_syscalls).green().bold(),
                    style(format!("{:.0}", traditional_syscalls as f64 / uring_syscalls.max(1) as f64)).green().bold());
                println!();
            }

            if !bench && !compare {
                println!();
                println!("  {} Use {} for benchmark", style("→").dim(),
                    style("jatin-lean io-uring --bench").yellow());
                println!("  {} Use {} for API comparison", style("→").dim(),
                    style("jatin-lean io-uring --compare").yellow());
                println!();
            }
        }

        Commands::CpuCache { bench, working_set_kb, info, tlb } => {
            use cpu_cache::*;
            println!("  {} {}", style("CPU Cache Optimization Engine").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

            let hierarchy = CacheHierarchy::detect();

            if info {
                println!();
                println!("  {} Detected Cache Hierarchy:", style("🖥️").yellow());
                println!("    L1D:     {}KB ({:.0}ns)", hierarchy.l1d_size / 1024, hierarchy.l1d_latency_ns);
                println!("    L1I:     {}KB", hierarchy.l1i_size / 1024);
                println!("    L2:      {}KB ({:.0}ns)", hierarchy.l2_size / 1024, hierarchy.l2_latency_ns);
                println!("    L3:      {}MB ({:.0}ns)", hierarchy.l3_size / 1024 / 1024, hierarchy.l3_latency_ns);
                println!("    Main:    {:.0}ns", hierarchy.main_memory_ns);
                println!("    Line:    {}B | TLB: {} entries", hierarchy.cache_line_size, hierarchy.tlb_entries);
                println!("    Items/L1: {} (64B) | Items/L2: {} | Items/L3: {}",
                    hierarchy.l1_items(64), hierarchy.l2_items(64), hierarchy.l3_items(64));
                println!();
            }

            if bench {
                let item_count = working_set_kb * 1024 / 8; // 8 bytes per u64
                println!("  {} Prefetch benchmark: {} items ({} KB working set)",
                    style("⚡").yellow(), style(item_count).green().bold(),
                    style(working_set_kb).white());

                let data: Vec<u64> = (0..item_count as u64).collect();

                // Without prefetch
                let start = std::time::Instant::now();
                let mut sum = 0u64;
                for &v in &data { sum = sum.wrapping_add(v); }
                let no_prefetch = start.elapsed();
                std::hint::black_box(sum);

                // With prefetch
                let start = std::time::Instant::now();
                let stats = scan_with_prefetch(&data, 16, |&x| {
                    sum = sum.wrapping_add(x);
                    true
                });
                let with_prefetch = start.elapsed();
                std::hint::black_box(sum);

                println!();
                println!("  {} Without prefetch: {:.2} ms ({:.0} ns/item)",
                    style("▸").dim(), no_prefetch.as_secs_f64() * 1000.0,
                    no_prefetch.as_nanos() as f64 / item_count as f64);
                println!("  {} With prefetch:    {:.2} ms ({:.0} ns/item)",
                    style("⚡").yellow(), with_prefetch.as_secs_f64() * 1000.0,
                    with_prefetch.as_nanos() as f64 / item_count as f64);
                let speedup = no_prefetch.as_nanos() as f64 / with_prefetch.as_nanos().max(1) as f64;
                println!("  {} Speedup: {}x",
                    style("🚀").yellow(), style(format!("{:.2}", speedup)).green().bold());

                print_cache_report(&hierarchy, &stats);
            }

            if tlb {
                let ws_bytes = working_set_kb * 1024;
                let info = analyze_tlb(ws_bytes);
                println!();
                println!("  {} TLB Analysis for {} KB working set:", style("📊").yellow(),
                    working_set_kb);
                println!("    4KB pages needed:    {}", info.pages_4k);
                println!("    2MB hugepages needed:{}", info.pages_2m);
                println!("    TLB coverage (4KB):  {:.1}%", info.tlb_coverage_standard);
                println!("    TLB coverage (2MB):  {:.1}%", info.tlb_coverage_huge);
                println!("    Recommend hugepages: {}",
                    if info.recommend_huge_pages { style("YES").green().bold() }
                    else { style("no").dim() });
                println!();
            }

            if !bench && !info && !tlb {
                println!();
                println!("  {} Use {} for cache hierarchy info", style("→").dim(),
                    style("jatin-lean cpu-cache --info").yellow());
                println!("  {} Use {} for prefetch benchmark", style("→").dim(),
                    style("jatin-lean cpu-cache --bench").yellow());
                println!();
            }
        }

        Commands::Optimize { assess, numa, network, kernel, generate } => {
            use hardware_tuning::*;
            println!("  {} {}", style("System Hardware Optimization").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

            if numa {
                let topo = NumaTopology::detect();
                println!();
                println!("  {} NUMA Topology:", style("🖥️").yellow());
                println!("    Nodes: {} | Cores: {} | NUMA: {}",
                    topo.nodes.len(), topo.total_cores,
                    if topo.is_numa { style("YES").green().bold() } else { style("no").dim() });
                for node in &topo.nodes {
                    println!("    Node {}: {} CPUs, {} MB total ({} MB free)",
                        node.id, node.cpu_list.len(), node.memory_total_mb, node.memory_free_mb);
                }
                println!();
            }

            if network {
                let tuning = NetworkTuning::default();
                println!();
                println!("  {} Recommended Network Settings:", style("🌐").yellow());
                println!("    TCP_NODELAY:      {}", if tuning.tcp_nodelay { "✓" } else { "✗" });
                println!("    TCP_QUICKACK:     {}", if tuning.tcp_quickack { "✓" } else { "✗" });
                println!("    SO_REUSEPORT:     {}", if tuning.so_reuseport { "✓" } else { "✗" });
                println!("    TCP_FASTOPEN:     {}", if tuning.tcp_fastopen { "✓" } else { "✗" });
                println!("    Recv buffer:      {} MB", tuning.recv_buffer_size / 1024 / 1024);
                println!("    Send buffer:      {} MB", tuning.send_buffer_size / 1024 / 1024);
                println!("    somaxconn:        {}", tuning.somaxconn);
                println!("    FIN timeout:      {}s", tuning.tcp_fin_timeout);
                println!();
            }

            if kernel {
                let tuning = KernelTuning::scan_optimized();
                println!();
                println!("  {} Kernel Parameters (scan-optimized):", style("🔧").yellow());
                println!("    fs.file-max:        {}", tuning.fs_file_max);
                println!("    inotify watches:    {}", tuning.fs_inotify_max_user_watches);
                println!("    vm.dirty_ratio:     {}%", tuning.vm_dirty_ratio);
                println!("    vm.swappiness:      {}", tuning.vm_swappiness);
                println!("    vfs_cache_pressure: {}", tuning.vfs_cache_pressure);
                println!("    THP:                {}", if tuning.transparent_hugepages { "enabled" } else { "disabled" });
                println!();
            }

            if generate {
                let net = NetworkTuning::default();
                let kern = KernelTuning::scan_optimized();
                println!();
                println!("  {} Generated sysctl commands:", style("📜").yellow());
                println!("  {} Copy and run with sudo:", style("▸").dim());
                println!();
                for cmd in net.to_sysctl_commands() {
                    println!("    {}", style(&cmd).yellow());
                }
                for cmd in kern.to_sysctl_commands() {
                    println!("    {}", style(&cmd).yellow());
                }
                println!();
            }

            if assess || (!numa && !network && !kernel && !generate) {
                let assessment = assess_system();
                print_system_report(&assessment);
            }
        }

        Commands::Bpf { verify, dpi, skbuff } => {
            use bpf_verifier::*;
            println!("  {} {}", style("BPF Verifier & DPI Engine").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

            if verify {
                let programs = vec![
                    BpfProgram {
                        name: "xdp_tunnel_ingress".into(),
                        prog_type: BpfProgType::XdpIngress,
                        instruction_count: 2_500,
                        max_loop_depth: 256, call_depth: 3,
                        map_accesses: 12, packet_accesses: 45, tail_calls: 2,
                        helper_calls: vec!["bpf_xdp_adjust_head".into(), "bpf_map_lookup_elem".into()],
                    },
                    BpfProgram {
                        name: "xdp_ddos_filter".into(),
                        prog_type: BpfProgType::XdpIngress,
                        instruction_count: 15_000,
                        max_loop_depth: 1024, call_depth: 5,
                        map_accesses: 30, packet_accesses: 80, tail_calls: 4,
                        helper_calls: vec!["bpf_ktime_get_ns".into(), "bpf_map_update_elem".into()],
                    },
                    BpfProgram {
                        name: "tc_protocol_obfuscator".into(),
                        prog_type: BpfProgType::TcEgress,
                        instruction_count: 800_000,
                        max_loop_depth: 4096, call_depth: 6,
                        map_accesses: 50, packet_accesses: 120, tail_calls: 8,
                        helper_calls: vec!["bpf_skb_change_proto".into()],
                    },
                ];
                for prog in &programs {
                    let result = prog.verify();
                    print_verifier_report(prog, &result);
                }
            }

            if dpi {
                println!();
                println!("  {} DPI Evasion Matrix:", style("🛡️").yellow());
                let methods = [DpiMethod::ProtocolWhitelist, DpiMethod::SniInspection,
                    DpiMethod::PayloadSignature, DpiMethod::DnsFilter, DpiMethod::StatisticalAnalysis];
                for evasion in DpiEvasion::all() {
                    let bypassed: Vec<&str> = methods.iter()
                        .filter(|m| evasion.bypasses(m))
                        .map(|_| "✓")
                        .collect();
                    println!("    {} ({} overhead) — bypasses {} DPI methods",
                        style(evasion.label()).yellow(),
                        style(format!("{}B", evasion.overhead_bytes())).dim(),
                        style(bypassed.len()).green().bold());
                }
                println!();
            }

            if let Some(packets) = skbuff {
                let model = SkbuffModel::default();
                let savings = model.savings(packets);
                println!();
                println!("  {} sk_buff Elimination for {} packets:", style("📊").yellow(),
                    style(packets).green().bold());
                println!("    CPU time saved:   {:.1} ms", savings.total_ns_saved / 1e6);
                println!("    Memory saved:     {:.1} MB", savings.memory_bytes_saved as f64 / 1e6);
                println!("    Cache pollution:  {:.1} MB avoided", savings.cache_bytes_saved as f64 / 1e6);
                println!("    Throughput gain:  {:.1}%", savings.equivalent_throughput_gain_pct);
                println!();
            }

            if !verify && !dpi && skbuff.is_none() {
                println!();
                println!("  {} Use {} for verifier", style("→").dim(),
                    style("jatin-lean bpf --verify").yellow());
                println!("  {} Use {} for DPI matrix", style("→").dim(),
                    style("jatin-lean bpf --dpi").yellow());
                println!("  {} Use {} for savings", style("→").dim(),
                    style("jatin-lean bpf --skbuff 1000000").yellow());
                println!();
            }
        }

        Commands::Pcie { compare, size_gb, offload, grace_hopper } => {
            use pcie_bottleneck::*;
            println!("  {} {}", style("PCIe & CUDA Memory Analysis").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

            let interconnect = if grace_hopper { PcieGen::NvLinkC2C } else { PcieGen::Gen5 };
            let data_bytes = size_gb * 1024 * 1024 * 1024;

            if compare {
                let mem_types = [CudaMemoryType::Pageable, CudaMemoryType::Pinned,
                    CudaMemoryType::UnifiedManaged, CudaMemoryType::HardwareCoherent];
                let sims: Vec<TransferSimulation> = mem_types.iter()
                    .map(|mt| simulate_transfer(*mt, interconnect, data_bytes))
                    .collect();
                print_pcie_report(&sims);
            }

            if let Some(num_layers) = offload {
                let mut ctrl = if grace_hopper {
                    VramOffloadController::grace_hopper()
                } else {
                    VramOffloadController::discrete_gpu()
                };
                let layers: Vec<(String, u64)> = (0..num_layers)
                    .map(|i| (format!("transformer.layer.{}", i), 2u64 * 1024 * 1024 * 1024))
                    .collect();
                let plan = ctrl.place_layers(&layers);
                print_offload_report(&plan);
            }

            if !compare && offload.is_none() {
                let sim = simulate_transfer(
                    if grace_hopper { CudaMemoryType::HardwareCoherent } else { CudaMemoryType::Pinned },
                    interconnect, data_bytes);
                println!();
                println!("  {} {} via {} | {:.1} GB transfer",
                    style("▸").dim(), style(sim.mem_type.label()).yellow(),
                    sim.interconnect.label(), size_gb);
                println!("  {} Time: {:.1} µs | BW: {:.1} GB/s | Latency: {:.0} ns",
                    style("⚡").yellow(), sim.transfer_time_us,
                    sim.effective_bandwidth_gbps, sim.first_access_latency_ns);
                println!();
            }
        }

        Commands::Hedge { bench, requests, cache_demo } => {
            use hedging::*;
            println!("  {} {}", style("Request Hedging & Fragmented Cache").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

            if bench {
                let engine = HedgingEngine::new(
                    vec!["replica-us-east".into(), "replica-us-west".into(), "replica-eu".into()],
                    HedgingStrategy::Immediate,
                );
                println!("  {} Hedging {} requests across {} replicas",
                    style("⚡").yellow(), style(requests).green().bold(),
                    engine.replicas.len());

                let start = std::time::Instant::now();
                for i in 0..requests {
                    engine.execute(i, &format!("/api/resource/{}", i % 100));
                }
                let elapsed = start.elapsed();

                println!("  {} Elapsed: {:.2} ms | RPS: {:.0}",
                    style("▸").dim(), elapsed.as_secs_f64() * 1000.0,
                    requests as f64 / elapsed.as_secs_f64());
                print_hedging_report(&engine.stats);
            }

            if cache_demo {
                let mut cache = FragmentedCache::new();
                println!("  {} Fragmented Cache Demo:", style("🗂️").yellow());

                // Store a superset
                let mut fields = std::collections::HashMap::new();
                fields.insert("name".into(), serde_json::json!("Jatin"));
                fields.insert("email".into(), serde_json::json!("jatin@dev.com"));
                fields.insert("role".into(), serde_json::json!("engineer"));
                fields.insert("projects".into(), serde_json::json!(42));
                cache.store("/user/1", fields, std::time::Duration::from_secs(60));

                // Client A: requests [name, email]
                match cache.fetch_fragment("/user/1", &["name", "email"]) {
                    FragmentResult::FullHit(f) => {
                        println!("    Client A [name,email]: {} ← from cache",
                            style(serde_json::to_string(&f).unwrap()).green());
                    }
                    _ => {}
                }

                // Client B: requests [role, projects]
                match cache.fetch_fragment("/user/1", &["role", "projects"]) {
                    FragmentResult::FullHit(f) => {
                        println!("    Client B [role,projects]: {} ← from cache",
                            style(serde_json::to_string(&f).unwrap()).green());
                    }
                    _ => {}
                }

                // Client C: requests [name, phone] — partial hit
                match cache.fetch_fragment("/user/1", &["name", "phone"]) {
                    FragmentResult::PartialHit { found, missing } => {
                        println!("    Client C [name,phone]: found {} / missing {} ← partial",
                            style(serde_json::to_string(&found).unwrap()).green(),
                            style(format!("{:?}", missing)).red());
                    }
                    _ => {}
                }

                // Delta update (RFC 7396)
                let mut patch = std::collections::HashMap::new();
                patch.insert("projects".into(), serde_json::json!(43));
                patch.insert("team".into(), serde_json::json!("platform"));
                cache.apply_delta("/user/1", &patch);
                println!("    Delta applied: projects→43, +team=platform");

                print_frag_cache_report(&cache.stats);
            }

            if !bench && !cache_demo {
                println!();
                println!("  {} Use {} for hedging benchmark", style("→").dim(),
                    style("jatin-lean hedge --bench").yellow());
                println!("  {} Use {} for cache demo", style("→").dim(),
                    style("jatin-lean hedge --cache-demo").yellow());
                println!();
            }
        }

        Commands::MmapIpc { bench, capacity, msg_size, compare } => {
            use mmap_ipc::*;
            println!("  {} {}", style("mmap Ring Buffer IPC Engine").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

            if compare { print_ffi_comparison(); }

            if bench {
                let ring = MmapRingBuffer::new(capacity, msg_size);
                let msg = vec![42u8; msg_size];

                println!("  {} Benchmark: {} slots × {} byte messages",
                    style("⚡").yellow(), style(capacity).green().bold(),
                    style(msg_size).white());

                let start = std::time::Instant::now();
                let mut writes = 0u64;
                let mut reads = 0u64;
                // Write-heavy burst
                for _ in 0..capacity { if ring.write(&msg) { writes += 1; } }
                // Batch read
                let batch = ring.read_batch(capacity);
                reads = batch.len() as u64;
                // Process batch
                let config = BatchProcessorConfig::default();
                let result = process_batch_parallel(&batch, &config);
                let elapsed = start.elapsed();

                println!("  {} Written: {} | Read: {} (batch)",
                    style("▸").dim(), writes, reads);
                println!("  {} Batch throughput: {:.0} msg/s",
                    style("🚀").yellow(), style(format!("{:.0}", result.throughput_msg_per_sec)).green().bold());
                println!("  {} Total elapsed: {:.2} ms",
                    style("▸").dim(), elapsed.as_secs_f64() * 1000.0);

                let ipc_latency = if writes > 0 { elapsed.as_nanos() as f64 / writes as f64 } else { 0.0 };
                println!("  {} IPC latency: {:.0} ns/msg (vs 50,000 ns JSON-over-HTTP)",
                    style("⚡").yellow(), style(format!("{:.0}", ipc_latency)).green().bold());

                print_mmap_report(&ring.stats);
            }

            if !bench && !compare {
                println!();
                println!("  {} Use {} for throughput benchmark", style("→").dim(),
                    style("jatin-lean mmap-ipc --bench").yellow());
                println!("  {} Use {} for FFI comparison", style("→").dim(),
                    style("jatin-lean mmap-ipc --compare").yellow());
                println!();
            }
        }

        Commands::StaticDispatch { bench } => {
            use static_plugins::*;
            if bench {
                let runner = MonomorphicPluginRunner::new();
                let start = std::time::Instant::now();
                for _ in 0..1_000_000 {
                    runner.run_all_on_scan();
                }
                let elapsed = start.elapsed();
                println!("  {} Executed 1,000,000 static plugin dispatches in {:.2} ms", 
                    style("⚡").yellow(), elapsed.as_secs_f64() * 1000.0);
                print_static_dispatch_report();
            } else {
                print_static_dispatch_report();
                println!("  {} Use {} to run benchmark", style("→").dim(),
                    style("jatin-lean static-dispatch --bench").yellow());
            }
        }
    }

    Ok(())
}

/// Simple Ctrl+C handler using a closure.
fn ctrlc_handler<F: Fn() + Send + 'static>(handler: F) {
    // Use a simple approach — set up SIGINT handler
    let _ = std::thread::spawn(move || {
        // This is a placeholder; in production, use the `ctrlc` crate
        // For now, the watcher checks the running flag periodically
        let _ = handler;
    });
}

/// Run in local mode — scan a single project's node_modules.
fn run_local_mode(
    project_path: &PathBuf,
    force: bool,
    yes: bool,
    verbose: bool,
    config_path: Option<&Path>,
    profile: bool,
    create_snapshot: bool,
    export_path: Option<&Path>,
) -> Result<()> {
    let mut profiler = profiler::Profiler::with_profiling(profile);
    let overall_start = Instant::now();

    // Find node_modules
    let nm_path = project_path.join("node_modules");
    if !nm_path.exists() {
        println!(
            "  {} No node_modules found at {}",
            style("✗").red().bold(),
            style(project_path.display()).dim()
        );
        println!(
            "  {} Run {} first, or specify a different path.",
            style("→").dim(),
            style("npm install").yellow()
        );
        return Ok(());
    }

    // Detect recent install
    if let Some(install_info) = watcher::detect_recent_install(project_path) {
        println!(
            "  {} Detected recent {} install ({}s ago)",
            style("⚡").yellow(),
            style(&install_info.package_manager).white().bold(),
            install_info.age_seconds
        );
    }

    // Load configuration
    profiler.start_span("Config Loading");
    let config = config::Config::load(config_path, project_path)?;
    if let Some(ref _cfg) = config {
        let source = if config_path.is_some() {
            "custom config"
        } else if Path::new("./jatin-lean.toml").exists() {
            "./jatin-lean.toml"
        } else {
            "~/.config/jatin-lean/rules.toml"
        };
        println!(
            "  {} Using {} {}",
            style("◉").cyan(),
            style("custom rules from").dim(),
            style(source).cyan()
        );
    }
    profiler.end_span(0);

    // Phase 1: Discovery
    profiler.start_span("Discovery (Scan)");
    let rules = rules::PruneRules::new_with_config(config);
    let scan_result =
        scanner::scan_node_modules(&nm_path, &rules, None).context("Failed to scan node_modules")?;
    profiler.end_span(scan_result.total_files);

    display::print_discovery(&scan_result);

    if scan_result.candidates.is_empty() {
        println!(
            "  {} node_modules is already lean! Nothing to prune.",
            style("✓").green().bold()
        );
        return Ok(());
    }

    // Phase 2: Simulation — verify runtime safety
    profiler.start_span("Runtime Safety Check");
    let runtime_files = tracer::verify_runtime_safety(&nm_path, &scan_result.candidates)
        .context("Failed to verify runtime safety")?;
    profiler.end_span(scan_result.candidates.len() as u64);

    // Filter out any candidates that are actually runtime-required
    let original_count = scan_result.candidates.len();
    let safe_candidates: Vec<_> = scan_result
        .candidates
        .iter()
        .filter(|c| !runtime_files.contains(&c.path))
        .cloned()
        .collect();
    let tracer_whitelisted = (original_count - safe_candidates.len()) as u64;

    let filtered_result = scanner::ScanResult {
        root: scan_result.root,
        total_files: scan_result.total_files,
        total_size: scan_result.total_size,
        candidates: safe_candidates,
        total_packages: scan_result.total_packages,
        whitelisted_count: scan_result.whitelisted_count + tracer_whitelisted,
    };

    display::print_simulation(&filtered_result);

    // Verbose: list individual files
    if verbose {
        println!(
            "  {} {}",
            style("Files targeted for deletion:").dim(),
            style("━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
        );
        let mut by_cat: std::collections::HashMap<
            rules::FileCategory,
            Vec<&scanner::PruneCandidate>,
        > = std::collections::HashMap::new();
        for c in &filtered_result.candidates {
            by_cat.entry(c.category).or_default().push(c);
        }
        for (cat, files) in &by_cat {
            println!(
                "\n  {} [{}]:",
                style("▸").cyan(),
                style(cat.label()).yellow()
            );
            for f in files.iter().take(20) {
                println!(
                    "    {} {} ({})",
                    style("·").dim(),
                    style(f.path.display()).dim(),
                    scanner::format_size(f.size)
                );
            }
            if files.len() > 20 {
                println!("    {} ...and {} more", style("·").dim(), files.len() - 20);
            }
        }
        println!();
    }

    // Export report if requested
    if let Some(export_file) = export_path {
        profiler.start_span("Report Export");
        export_report(&filtered_result, export_file)?;
        profiler.end_span(filtered_result.candidates.len() as u64);
    }

    // Phase 3 or 4
    if force {
        // Interactive confirmation (unless --yes flag is used)
        if !yes {
            println!(
                "  {} {}",
                style("Phase 3: Confirmation").cyan().bold(),
                style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
            );

            let savings = filtered_result.savings();
            let pct = if filtered_result.total_size > 0 {
                (savings as f64 / filtered_result.total_size as f64 * 100.0) as u64
            } else {
                0
            };

            println!(
                "  {} About to delete {} ({} files, {}% of node_modules)",
                style("⚠").yellow().bold(),
                style(scanner::format_size(savings)).yellow().bold(),
                style(scanner::format_number(
                    filtered_result.candidates.len() as u64
                ))
                .yellow(),
                style(pct).yellow()
            );

            println!();

            let confirmed = Confirm::new()
                .with_prompt("  Do you want to proceed with deletion?")
                .default(false)
                .interact()
                .unwrap_or(false);

            if !confirmed {
                println!();
                println!(
                    "  {} Deletion cancelled. No files were deleted.",
                    style("✓").green().bold()
                );
                println!(
                    "  {} Run with {} to skip this prompt next time.",
                    style("→").dim(),
                    style("--yes").yellow()
                );
                println!();
                return Ok(());
            }

            println!();
        }

        // Create snapshot before deletion if requested
        if create_snapshot {
            profiler.start_span("Snapshot Creation");
            println!("  {} Creating snapshot...", style("📸").bold());
            let manager = snapshot::SnapshotManager::new()?;
            let snap_id = manager.create_snapshot(&filtered_result.candidates, &nm_path)?;
            println!(
                "  {} Snapshot created: {}",
                style("✓").green().bold(),
                style(&snap_id).cyan()
            );
            println!(
                "  {} Undo with:    {}",
                style("→").dim(),
                style("jatin-lean undo").yellow()
            );
            println!(
                "  {} Restore with: {}",
                style("→").dim(),
                style(format!("jatin-lean restore {}", snap_id)).yellow()
            );
            profiler.end_span(filtered_result.candidates.len() as u64);
        }

        // Phase 4: Execute
        profiler.start_span("Deletion");
        println!(
            "  {} {}",
            style("Phase 4: Execution").cyan().bold(),
            style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
        );

        let result = deleter::execute_deletion(&filtered_result.candidates)
            .context("Failed to execute deletion")?;

        deleter::print_deletion_summary(&result);
        profiler.end_span(result.deleted_count);

        // Record in analytics
        let scan_duration = overall_start.elapsed().as_millis() as u64;
        if let Ok(mut db) = analytics::AnalyticsDB::load() {
            db.record_scan(
                &filtered_result,
                true,
                result.deleted_size,
                result.deleted_count,
                scan_duration,
            );
            let _ = db.save();
        }

        println!();
    } else {
        // Phase 3: Dry run confirmation
        display::print_dry_run_confirmation(&filtered_result);

        // Record in analytics (dry run)
        let scan_duration = overall_start.elapsed().as_millis() as u64;
        if let Ok(mut db) = analytics::AnalyticsDB::load() {
            db.record_scan(&filtered_result, false, 0, 0, scan_duration);
            let _ = db.save();
        }
    }

    // Print profiling report
    if profile {
        profiler::print_profiling_report(&profiler);
        // Enhanced dashboard (Step 14)
        let metrics = profiler.clone().finalize();
        display::print_performance_dashboard(&metrics);
    }

    Ok(())
}

/// Export a scan report to a file.
fn export_report(scan_result: &scanner::ScanResult, path: &Path) -> Result<()> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("json");

    let content = match extension {
        "json" => analytics::generate_json_report(scan_result)?,
        "csv" => analytics::generate_csv_report(scan_result),
        "md" | "markdown" => analytics::generate_markdown_report(scan_result),
        _ => {
            println!(
                "  {} Unknown export format: {}. Using JSON.",
                style("⚠").yellow(),
                extension
            );
            analytics::generate_json_report(scan_result)?
        }
    };

    std::fs::write(path, &content)
        .with_context(|| format!("Failed to write report: {}", path.display()))?;

    println!(
        "  {} Report exported to: {}",
        style("✓").green().bold(),
        style(path.display()).cyan()
    );

    Ok(())
}

/// Run in global mode — scan all projects in a directory.
fn run_global_mode(root: &PathBuf, max_depth: usize) -> Result<()> {
    println!(
        "  {} Scanning for node_modules in {}...",
        style("◉").cyan(),
        style(root.display()).white().bold()
    );

    let node_modules_dirs = scanner::find_node_modules(root, max_depth);

    if node_modules_dirs.is_empty() {
        println!(
            "  {} No node_modules directories found.",
            style("✗").red().bold()
        );
        return Ok(());
    }

    println!(
        "  {} Found {} node_modules directories. Analyzing...\n",
        style("◉").cyan(),
        style(node_modules_dirs.len()).white().bold()
    );

    let rules = rules::PruneRules::new();
    let mut projects: Vec<(String, u64, u64, Option<u64>)> = Vec::new();

    for nm_path in &node_modules_dirs {
        let project_dir = nm_path.parent().unwrap_or(nm_path);
        let project_name = project_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        match scanner::scan_node_modules(nm_path, &rules, None) {
            Ok(result) => {
                let savings = result.savings();
                let days = scanner::last_accessed_days(nm_path);
                projects.push((project_name, result.total_size, savings, days));
            }
            Err(e) => {
                eprintln!(
                    "  {} Failed to scan {}: {}",
                    style("⚠").yellow(),
                    nm_path.display(),
                    e
                );
            }
        }
    }

    display::print_global_table(&projects);

    let total_savings: u64 = projects.iter().map(|(_, _, s, _)| s).sum();
    println!(
        "  {} Total potential savings: {}",
        style("💾").bold(),
        style(scanner::format_size(total_savings)).green().bold()
    );
    println!(
        "  {} Run {} on individual projects to prune.",
        style("→").dim(),
        style("jatin-lean <path> --force").yellow()
    );
    println!(
        "  {} Made with ❤️  by {}\n",
        style("✨").dim(),
        style("Jatin Jalandhra").cyan(),
    );

    Ok(())
}
