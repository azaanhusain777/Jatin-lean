//! Display and reporting module: terminal UI for scan results.

use crate::scanner::{format_number, format_size, ScanResult};
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};
use console::style;

/// Print the Phase 1: Discovery summary.
pub fn print_discovery(result: &ScanResult) {
    println!();
    println!(
        "  {} {}",
        style("Phase 1: Discovery").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );
    println!(
        "  {} Scanning node_modules... Found {} files across {} packages.",
        style("◉").cyan(),
        style(format_number(result.total_files)).white().bold(),
        style(format_number(result.total_packages as u64)).white().bold(),
    );
    println!(
        "  {} Total size indexed: {}",
        style("◉").cyan(),
        style(format_size(result.total_size)).white().bold(),
    );
    println!();
}

/// Print the Phase 2: Simulation summary.
pub fn print_simulation(result: &ScanResult) {
    let savings = result.savings();
    let breakdown = result.category_breakdown();

    println!(
        "  {} {}",
        style("Phase 2: Simulation").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );
    println!(
        "  {} Analyzing dependency tree... {} files ({}) identified as non-runtime assets.",
        style("◉").cyan(),
        style(format_number(result.candidates.len() as u64)).yellow().bold(),
        style(format_size(savings)).yellow().bold(),
    );

    if result.whitelisted_count > 0 {
        println!(
            "  {} {} files auto-whitelisted (required at runtime).",
            style("◉").green(),
            style(format_number(result.whitelisted_count)).green().bold(),
        );
    }

    // Category breakdown table
    println!();
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Category").fg(Color::Cyan),
            Cell::new("Files").fg(Color::Cyan),
            Cell::new("Size").fg(Color::Cyan),
            Cell::new("Risk").fg(Color::Cyan),
        ]);

    let mut sorted: Vec<_> = breakdown.iter().collect();
    sorted.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));

    for (cat, (count, size)) in &sorted {
        let risk_str = match cat.risk_level() {
            0 => "▪ Low",
            1 => "▪▪ Medium",
            _ => "▪▪▪ High",
        };
        let risk_color = match cat.risk_level() {
            0 => Color::Green,
            1 => Color::Yellow,
            _ => Color::Red,
        };
        table.add_row(vec![
            Cell::new(cat.label()),
            Cell::new(format_number(*count)),
            Cell::new(format_size(*size)),
            Cell::new(risk_str).fg(risk_color),
        ]);
    }

    // Total row
    let total_count: u64 = breakdown.values().map(|(c, _)| c).sum();
    table.add_row(vec![
        Cell::new("TOTAL").fg(Color::White),
        Cell::new(format_number(total_count)).fg(Color::White),
        Cell::new(format_size(savings)).fg(Color::White),
        Cell::new(result.risk_label()).fg(Color::Yellow),
    ]);

    for line in table.to_string().lines() {
        println!("    {}", line);
    }
    println!();
}

/// Print the Phase 3: Confirmation (dry run).
pub fn print_dry_run_confirmation(result: &ScanResult) {
    let savings = result.savings();
    let pct = if result.total_size > 0 {
        (savings as f64 / result.total_size as f64 * 100.0) as u64
    } else {
        0
    };

    println!(
        "  {} {}",
        style("Phase 3: Confirmation").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );

    if result.max_risk() <= 1 {
        println!(
            "  {} No critical runtime files targeted.",
            style("[SAFE]").green().bold()
        );
    } else {
        println!(
            "  {} Some higher-risk files included. Review the breakdown above.",
            style("[REVIEW]").yellow().bold()
        );
    }

    println!(
        "\n  {} Total Savings: {} ({}% of node_modules)",
        style("💾").bold(),
        style(format_size(savings)).green().bold(),
        style(pct).green().bold(),
    );
    println!(
        "  {} This will NOT affect {} or {}.",
        style("ℹ").blue(),
        style("npm start").white().bold(),
        style("npm build").white().bold(),
    );
    println!(
        "\n  {} Run with {} to execute deletion.",
        style("→").dim(),
        style("--force").yellow().bold(),
    );
    println!(
        "  {} Made with ❤️  by {}",
        style("✨").dim(),
        style("Jatin Jalandhra").cyan(),
    );
    println!();
}

/// Print the global scan table.
pub fn print_global_table(
    projects: &[(String, u64, u64, Option<u64>)],
) {
    println!();
    println!(
        "  {} {}",
        style("System Efficiency Report").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );
    println!();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Project").fg(Color::Cyan),
            Cell::new("Size").fg(Color::Cyan),
            Cell::new("Potential Savings").fg(Color::Cyan),
            Cell::new("Last Accessed").fg(Color::Cyan),
        ]);

    let mut total_size: u64 = 0;
    let mut total_savings: u64 = 0;

    for (name, size, savings, days) in projects {
        total_size += size;
        total_savings += savings;

        let days_str = match days {
            Some(d) if *d > 90 => format!("{} days ago", d),
            Some(d) => format!("{} days ago", d),
            None => "Unknown".to_string(),
        };

        let days_color = match days {
            Some(d) if *d > 90 => Color::Red,
            Some(d) if *d > 30 => Color::Yellow,
            _ => Color::Green,
        };

        table.add_row(vec![
            Cell::new(name),
            Cell::new(format_size(*size)),
            Cell::new(format_size(*savings)).fg(Color::Green),
            Cell::new(&days_str).fg(days_color),
        ]);
    }

    table.add_row(vec![
        Cell::new("TOTAL").fg(Color::White),
        Cell::new(format_size(total_size)).fg(Color::White),
        Cell::new(format_size(total_savings)).fg(Color::Green),
        Cell::new("—"),
    ]);

    for line in table.to_string().lines() {
        println!("    {}", line);
    }
    println!();
}

/// Print the banner.
pub fn print_banner() {
    println!();
    println!(
        "  {}",
        style("╔═══════════════════════════════════════════════╗").cyan()
    );
    println!(
        "  {}  {}  {}",
        style("║").cyan(),
        style("⚡ jatin-lean — Node Modules Pruner ⚡").white().bold(),
        style("║").cyan()
    );
    println!(
        "  {}  {}  {}",
        style("║").cyan(),
        style("   Slim your node_modules by up to 50%   ").dim(),
        style("║").cyan()
    );
    println!(
        "  {}  {}  {}",
        style("║").cyan(),
        style("        Created by Jatin Jalandhra        ").dim(),
        style("║").cyan()
    );
    println!(
        "  {}",
        style("╚═══════════════════════════════════════════════╝").cyan()
    );
    println!();
}
