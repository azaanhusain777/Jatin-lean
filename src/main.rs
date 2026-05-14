//! jatin-lean — A high-performance CLI utility to prune non-essential
//! files from node_modules, reducing disk footprint by up to 50%
//! without breaking runtime dependencies.

mod config;
mod deleter;
mod display;
mod rules;
mod scanner;
mod tracer;

use anyhow::{Context, Result};
use clap::Parser;
use console::style;
use dialoguer::Confirm;
use std::path::{Path, PathBuf};

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

    display::print_banner();

    let target = std::fs::canonicalize(&cli.path)
        .with_context(|| format!("Cannot access path: {}", cli.path.display()))?;

    if cli.global {
        run_global_mode(&target, cli.max_depth)?;
    } else {
        run_local_mode(&target, cli.force, cli.yes, cli.verbose, cli.config.as_deref())?;
    }

    Ok(())
}

/// Run in local mode — scan a single project's node_modules.
fn run_local_mode(project_path: &PathBuf, force: bool, yes: bool, verbose: bool, config_path: Option<&Path>) -> Result<()> {
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

    // Load configuration
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

    // Phase 1: Discovery
    let rules = rules::PruneRules::new_with_config(config);
    let scan_result = scanner::scan_node_modules(&nm_path, &rules)
        .context("Failed to scan node_modules")?;

    display::print_discovery(&scan_result);

    if scan_result.candidates.is_empty() {
        println!(
            "  {} node_modules is already lean! Nothing to prune.",
            style("✓").green().bold()
        );
        return Ok(());
    }

    // Phase 2: Simulation — verify runtime safety
    // Note: Entry points are already whitelisted during scanning
    let runtime_files = tracer::verify_runtime_safety(&nm_path, &scan_result.candidates)
        .context("Failed to verify runtime safety")?;

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
            println!("\n  {} [{}]:", style("▸").cyan(), style(cat.label()).yellow());
            for f in files.iter().take(20) {
                println!(
                    "    {} {} ({})",
                    style("·").dim(),
                    style(f.path.display()).dim(),
                    scanner::format_size(f.size)
                );
            }
            if files.len() > 20 {
                println!(
                    "    {} ...and {} more",
                    style("·").dim(),
                    files.len() - 20
                );
            }
        }
        println!();
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
                style(scanner::format_number(filtered_result.candidates.len() as u64)).yellow(),
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
        
        // Phase 4: Execute
        println!(
            "  {} {}",
            style("Phase 4: Execution").cyan().bold(),
            style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
        );

        let result = deleter::execute_deletion(&filtered_result.candidates)
            .context("Failed to execute deletion")?;

        deleter::print_deletion_summary(&result);
        println!();
    } else {
        // Phase 3: Dry run confirmation
        display::print_dry_run_confirmation(&filtered_result);
    }

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
