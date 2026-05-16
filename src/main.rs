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
                let scan_result = scanner::scan_node_modules(nm_path, &rules)?;
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
            let scan_result = scanner::scan_node_modules(&nm_path, &rules)?;

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
    let mut profiler = profiler::Profiler::new(profile);
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
        scanner::scan_node_modules(&nm_path, &rules).context("Failed to scan node_modules")?;
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
                "  {} Restore with: {}",
                style("→").dim(),
                style(format!("jatin-lean snapshots --restore {}", snap_id)).yellow()
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

        match scanner::scan_node_modules(nm_path, &rules) {
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
