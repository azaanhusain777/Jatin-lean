//! Terminal-based rich visualization engine.
//!
//! Renders:
//!   - Dependency tree graphs (Unicode box-drawing)
//!   - Size treemaps (proportional block display)
//!   - Horizontal bar charts
//!   - Sparklines for trend data
//!   - Flamegraph-style profiler output

use console::{style, Term};
use std::collections::HashMap;

use crate::scanner::{format_number, format_size};

// ─── Treemap Visualization ───────────────────────────────────────────────────

/// A node in the treemap (package or category).
#[derive(Debug, Clone)]
pub struct TreemapNode {
    pub label: String,
    pub size: u64,
    pub children: Vec<TreemapNode>,
}

impl TreemapNode {
    pub fn new(label: &str, size: u64) -> Self {
        Self {
            label: label.to_string(),
            size,
            children: Vec::new(),
        }
    }

    pub fn with_children(label: &str, children: Vec<TreemapNode>) -> Self {
        let size = children.iter().map(|c| c.size).sum();
        Self {
            label: label.to_string(),
            size,
            children,
        }
    }
}

/// Render a treemap showing proportional sizes.
pub fn render_treemap(root: &TreemapNode, width: usize) {
    println!();
    println!(
        "  {} {} ({})",
        style("Treemap").cyan().bold(),
        style(&root.label).white().bold(),
        style(format_size(root.size)).dim()
    );
    println!("  {}", style("─".repeat(width.min(70))).dim());

    if root.children.is_empty() {
        return;
    }

    let total = root.size.max(1) as f64;
    let usable_width = width.min(70);

    // Sort children by size (largest first)
    let mut sorted: Vec<&TreemapNode> = root.children.iter().collect();
    sorted.sort_by(|a, b| b.size.cmp(&a.size));

    // Color palette for blocks
    let colors = [
        console::Color::Cyan,
        console::Color::Green,
        console::Color::Yellow,
        console::Color::Magenta,
        console::Color::Blue,
        console::Color::Red,
        console::Color::White,
    ];

    for (i, child) in sorted.iter().take(15).enumerate() {
        let pct = child.size as f64 / total * 100.0;
        let bar_width = ((child.size as f64 / total) * usable_width as f64) as usize;
        let bar_width = bar_width.max(1).min(usable_width);

        let color = colors[i % colors.len()];
        let bar = "█".repeat(bar_width);

        println!(
            "  {} {:28} {:>8} ({:5.1}%)",
            style(&bar).fg(color),
            style(&child.label).fg(color),
            format_size(child.size),
            pct,
        );
    }

    // Show "others" if more than 15
    if sorted.len() > 15 {
        let others_size: u64 = sorted[15..].iter().map(|c| c.size).sum();
        let others_pct = others_size as f64 / total * 100.0;
        println!(
            "  {} {:28} {:>8} ({:5.1}%)",
            style("░".repeat(3)).dim(),
            style(format!("... {} others", sorted.len() - 15)).dim(),
            format_size(others_size),
            others_pct,
        );
    }

    println!();
}

// ─── Horizontal Bar Chart ────────────────────────────────────────────────────

/// Data point for a bar chart.
pub struct BarChartEntry {
    pub label: String,
    pub value: f64,
    pub display_value: String,
}

/// Render a horizontal bar chart.
pub fn render_bar_chart(title: &str, entries: &[BarChartEntry], max_width: usize) {
    println!();
    println!(
        "  {} {}",
        style(title).cyan().bold(),
        style("━".repeat(40)).dim()
    );
    println!();

    if entries.is_empty() {
        println!("  {} No data", style("ℹ").dim());
        return;
    }

    let max_val = entries.iter().map(|e| e.value).fold(0.0f64, f64::max);

    if max_val == 0.0 {
        return;
    }

    let bar_width = max_width.min(40);
    let max_label_len = entries
        .iter()
        .map(|e| e.label.len())
        .max()
        .unwrap_or(10)
        .min(25);

    for entry in entries {
        let filled = ((entry.value / max_val) * bar_width as f64) as usize;
        let filled = filled.max(0).min(bar_width);

        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_width - filled));

        println!(
            "  {:width$} {} {}",
            style(&entry.label).white(),
            style(&bar).cyan(),
            style(&entry.display_value).dim(),
            width = max_label_len,
        );
    }

    println!();
}

// ─── Sparkline ───────────────────────────────────────────────────────────────

/// Render a sparkline from a series of values.
///
/// Uses Unicode block characters: ▁▂▃▄▅▆▇█
pub fn sparkline(values: &[f64]) -> String {
    const BLOCKS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    if values.is_empty() {
        return String::new();
    }

    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (max - min).max(0.001);

    values
        .iter()
        .map(|&v| {
            let normalized = ((v - min) / range * 7.0) as usize;
            BLOCKS[normalized.min(7)]
        })
        .collect()
}

/// Render a sparkline with label and stats.
pub fn render_sparkline(label: &str, values: &[f64], unit: &str) {
    if values.is_empty() {
        return;
    }

    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let avg = values.iter().sum::<f64>() / values.len() as f64;
    let last = values.last().copied().unwrap_or(0.0);

    let trend = if values.len() >= 2 {
        let prev = values[values.len() - 2];
        if last > prev * 1.05 {
            style("↑").red().to_string()
        } else if last < prev * 0.95 {
            style("↓").green().to_string()
        } else {
            style("→").dim().to_string()
        }
    } else {
        style("–").dim().to_string()
    };

    let spark = sparkline(values);

    println!(
        "  {} {:15} {} {} (min: {:.0}, avg: {:.0}, max: {:.0} {})",
        style("◉").cyan(),
        label,
        spark,
        trend,
        min,
        avg,
        max,
        unit,
    );
}

// ─── Dependency Tree Renderer ────────────────────────────────────────────────

/// Node in a dependency tree for rendering.
#[derive(Debug, Clone)]
pub struct DepTreeNode {
    pub name: String,
    pub version: String,
    pub size: Option<u64>,
    pub children: Vec<DepTreeNode>,
    pub is_dev: bool,
    pub is_duplicate: bool,
}

/// Render a dependency tree using Unicode box-drawing characters.
pub fn render_dep_tree(root: &DepTreeNode, max_depth: usize) {
    println!();
    println!(
        "  {} {}",
        style("Dependency Tree").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );
    println!();

    render_tree_node(root, "", true, 0, max_depth);
    println!();
}

fn render_tree_node(
    node: &DepTreeNode,
    prefix: &str,
    is_last: bool,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth {
        return;
    }

    let connector = if depth == 0 {
        ""
    } else if is_last {
        "└── "
    } else {
        "├── "
    };

    let name_style = if node.is_duplicate {
        style(&node.name).yellow()
    } else if node.is_dev {
        style(&node.name).dim()
    } else {
        style(&node.name).white().bold()
    };

    let version = style(format!("@{}", node.version)).dim();
    let size_info = node
        .size
        .map(|s| format!(" ({})", format_size(s)))
        .unwrap_or_default();
    let dup_marker = if node.is_duplicate {
        style(" [dup]").yellow().to_string()
    } else {
        String::new()
    };
    let dev_marker = if node.is_dev {
        style(" [dev]").dim().to_string()
    } else {
        String::new()
    };

    println!(
        "  {}{}{}{}{}{} {}",
        style(prefix).dim(),
        style(connector).dim(),
        name_style,
        version,
        style(size_info).dim(),
        dup_marker,
        dev_marker,
    );

    let child_prefix = if depth == 0 {
        String::new()
    } else if is_last {
        format!("{}    ", prefix)
    } else {
        format!("{}│   ", prefix)
    };

    let total = node.children.len();
    for (i, child) in node.children.iter().enumerate() {
        let child_is_last = i == total - 1;

        if depth + 1 == max_depth && !child.children.is_empty() {
            // At max depth, show collapsed indicator
            let name_s = if child.is_dev {
                style(&child.name).dim()
            } else {
                style(&child.name).white()
            };
            let connector = if child_is_last {
                "└── "
            } else {
                "├── "
            };
            println!(
                "  {}{}{} {} {} ...",
                style(&child_prefix).dim(),
                style(connector).dim(),
                name_s,
                style(format!("@{}", child.version)).dim(),
                style(format!("({} deps)", child.children.len())).dim()
            );
        } else {
            render_tree_node(child, &child_prefix, child_is_last, depth + 1, max_depth);
        }
    }
}

// ─── Flamegraph-style Profiler Output ────────────────────────────────────────

/// Render a flamegraph-style stacked view of profiling data.
pub fn render_flamegraph(spans: &[(String, f64, u64)], total_ms: f64, width: usize) {
    println!();
    println!(
        "  {} {}",
        style("Performance Flamegraph").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );
    println!("  {} Total: {:.1}ms", style("◉").cyan(), total_ms);
    println!();

    let bar_width = width.min(50);
    let total = total_ms.max(0.001);

    // Sort by duration (longest first)
    let mut sorted: Vec<_> = spans.iter().collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let flame_colors = [
        console::Color::Red,
        console::Color::Yellow,
        console::Color::Magenta,
        console::Color::Cyan,
        console::Color::Green,
        console::Color::Blue,
    ];

    for (i, (name, duration, items)) in sorted.iter().enumerate() {
        let pct = *duration / total * 100.0;
        let filled = ((*duration / total) * bar_width as f64) as usize;
        let filled = filled.max(1).min(bar_width);

        let color = flame_colors[i % flame_colors.len()];
        let bar = "▓".repeat(filled);
        let empty = "░".repeat(bar_width - filled);

        let throughput = if *duration > 0.0 {
            *items as f64 / (*duration / 1000.0)
        } else {
            0.0
        };

        println!(
            "  {}{} {:20} {:>7.1}ms ({:5.1}%) {:>7} items ({:.0}/s)",
            style(&bar).fg(color),
            style(&empty).dim(),
            style(name).fg(color),
            duration,
            pct,
            format_number(*items),
            throughput,
        );
    }

    println!();
}

// ─── Progress Dashboard ──────────────────────────────────────────────────────

/// Multi-line dashboard showing real-time scan progress.
pub struct Dashboard {
    start_time: std::time::Instant,
    files_scanned: u64,
    bytes_scanned: u64,
    candidates_found: u64,
    candidate_bytes: u64,
    packages_scanned: u64,
    current_package: String,
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            files_scanned: 0,
            bytes_scanned: 0,
            candidates_found: 0,
            candidate_bytes: 0,
            packages_scanned: 0,
            current_package: String::new(),
        }
    }

    pub fn update(
        &mut self,
        files: u64,
        bytes: u64,
        candidates: u64,
        candidate_bytes: u64,
        packages: u64,
        current: &str,
    ) {
        self.files_scanned = files;
        self.bytes_scanned = bytes;
        self.candidates_found = candidates;
        self.candidate_bytes = candidate_bytes;
        self.packages_scanned = packages;
        self.current_package = current.to_string();
    }

    pub fn render(&self) {
        let elapsed = self.start_time.elapsed();
        let files_per_sec = if elapsed.as_secs_f64() > 0.0 {
            self.files_scanned as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };
        let mb_per_sec = if elapsed.as_secs_f64() > 0.0 {
            self.bytes_scanned as f64 / 1_048_576.0 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        // Clear previous output (6 lines)
        let term = Term::stderr();
        let _ = term.clear_last_lines(7);

        println!(
            "  {} {}",
            style("⚡ Scanning").cyan().bold(),
            style(&self.current_package).white()
        );
        println!(
            "  {} Files: {} │ Packages: {} │ Size: {}",
            style("▸").dim(),
            style(format_number(self.files_scanned)).white().bold(),
            style(format_number(self.packages_scanned)).white(),
            style(format_size(self.bytes_scanned)).white(),
        );
        println!(
            "  {} Candidates: {} │ Savings: {}",
            style("▸").green(),
            style(format_number(self.candidates_found)).green().bold(),
            style(format_size(self.candidate_bytes)).green().bold(),
        );
        println!(
            "  {} Throughput: {:.0} files/s │ {:.1} MB/s",
            style("▸").dim(),
            files_per_sec,
            mb_per_sec,
        );
        println!(
            "  {} Elapsed: {:.1}s",
            style("▸").dim(),
            elapsed.as_secs_f64(),
        );
        println!("  {}", style("─".repeat(55)).dim());
    }
}

// ─── Heatmap ─────────────────────────────────────────────────────────────────

/// Render a heatmap of values (e.g., package sizes by first letter).
pub fn render_heatmap(title: &str, data: &HashMap<String, f64>, _rows: usize, _cols: usize) {
    println!();
    println!(
        "  {} {}",
        style(title).cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );
    println!();

    if data.is_empty() {
        return;
    }

    let max_val = data.values().cloned().fold(0.0f64, f64::max);
    let heat_chars = ['·', '░', '▒', '▓', '█'];

    let mut keys: Vec<_> = data.keys().collect();
    keys.sort();

    for key in &keys {
        let val = data[*key];
        let normalized = if max_val > 0.0 { val / max_val } else { 0.0 };
        let idx = (normalized * 4.0) as usize;
        let idx = idx.min(4);

        let heat = heat_chars[idx];
        let color = match idx {
            0 => console::Color::Color256(238), // dark gray
            1 => console::Color::Blue,
            2 => console::Color::Yellow,
            3 => console::Color::Red,
            4 => console::Color::Magenta,
            _ => console::Color::White,
        };

        println!(
            "  {} {:20} {:>10.0}",
            style(format!("{}{}", heat, heat)).fg(color),
            key,
            val,
        );
    }

    println!();
}

// ─── Summary Table ───────────────────────────────────────────────────────────

/// Render a summary comparison table (before/after).
pub fn render_comparison(before_label: &str, after_label: &str, items: &[(String, u64, u64)]) {
    println!();
    println!(
        "  {} {}",
        style("Comparison").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );
    println!();

    println!(
        "  {:25} {:>12} {:>12} {:>10}",
        "",
        style(before_label).dim(),
        style(after_label).green(),
        style("Savings").cyan(),
    );
    println!("  {}", style("─".repeat(62)).dim());

    for (label, before, after) in items {
        let savings = before.saturating_sub(*after);
        let pct = if *before > 0 {
            savings as f64 / *before as f64 * 100.0
        } else {
            0.0
        };

        println!(
            "  {:25} {:>12} {:>12} {:>8} ({:.1}%)",
            label,
            format_size(*before),
            style(format_size(*after)).green(),
            style(format_size(savings)).cyan(),
            pct,
        );
    }

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparkline_empty() {
        assert_eq!(sparkline(&[]), "");
    }

    #[test]
    fn test_sparkline_single() {
        let s = sparkline(&[5.0]);
        assert_eq!(s.chars().count(), 1);
    }

    #[test]
    fn test_sparkline_ascending() {
        let s = sparkline(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
        assert_eq!(s, "▁▂▃▄▅▆▇█");
    }

    #[test]
    fn test_sparkline_all_same() {
        let s = sparkline(&[5.0, 5.0, 5.0]);
        // All same → all should be the same character
        let chars: Vec<char> = s.chars().collect();
        assert!(chars.iter().all(|c| *c == chars[0]));
    }

    #[test]
    fn test_treemap_node_with_children() {
        let children = vec![TreemapNode::new("a", 100), TreemapNode::new("b", 200)];
        let root = TreemapNode::with_children("root", children);
        assert_eq!(root.size, 300);
        assert_eq!(root.children.len(), 2);
    }

    #[test]
    fn test_dep_tree_node() {
        let node = DepTreeNode {
            name: "express".to_string(),
            version: "4.18.2".to_string(),
            size: Some(1024),
            children: vec![],
            is_dev: false,
            is_duplicate: false,
        };
        assert_eq!(node.name, "express");
        assert!(!node.is_dev);
    }

    #[test]
    fn test_dashboard_creation() {
        let dash = Dashboard::new();
        assert_eq!(dash.files_scanned, 0);
        assert_eq!(dash.bytes_scanned, 0);
    }
}
