//! Analytics engine: scan history, report generation, and trend tracking.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::scanner::{format_number, format_size, ScanResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanEntry {
    pub id: String,
    pub timestamp: u64,
    pub project_path: String,
    pub total_files: u64,
    pub total_size: u64,
    pub candidate_count: u64,
    pub candidate_size: u64,
    pub total_packages: usize,
    pub deletion_executed: bool,
    pub bytes_deleted: u64,
    pub files_deleted: u64,
    pub scan_duration_ms: u64,
    pub category_breakdown: HashMap<String, CategoryStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStat {
    pub file_count: u64,
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsDB {
    pub version: u32,
    pub entries: Vec<ScanEntry>,
}

impl Default for AnalyticsDB {
    fn default() -> Self {
        Self {
            version: 1,
            entries: Vec::new(),
        }
    }
}

impl AnalyticsDB {
    pub fn db_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        let db_dir = home.join(".config").join("jatin-lean");
        if !db_dir.exists() {
            fs::create_dir_all(&db_dir)?;
        }
        Ok(db_dir.join("analytics.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::db_path()?;
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let db: AnalyticsDB = serde_json::from_str(&content)?;
            Ok(db)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::db_path()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    pub fn record_scan(
        &mut self,
        scan_result: &ScanResult,
        deletion_executed: bool,
        bytes_deleted: u64,
        files_deleted: u64,
        scan_duration_ms: u64,
    ) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let id = format!("scan_{:x}_{}", now, self.entries.len());

        let breakdown = scan_result.category_breakdown();
        let category_breakdown: HashMap<String, CategoryStat> = breakdown
            .into_iter()
            .map(|(cat, (count, size))| {
                (
                    cat.label().to_string(),
                    CategoryStat {
                        file_count: count,
                        total_size: size,
                    },
                )
            })
            .collect();

        self.entries.push(ScanEntry {
            id,
            timestamp: now,
            project_path: scan_result.root.display().to_string(),
            total_files: scan_result.total_files,
            total_size: scan_result.total_size,
            candidate_count: scan_result.candidates.len() as u64,
            candidate_size: scan_result.savings(),
            total_packages: scan_result.total_packages,
            deletion_executed,
            bytes_deleted,
            files_deleted,
            scan_duration_ms,
            category_breakdown,
        });
    }

    pub fn total_bytes_saved(&self) -> u64 {
        self.entries.iter().map(|e| e.bytes_deleted).sum()
    }

    pub fn total_files_deleted(&self) -> u64 {
        self.entries.iter().map(|e| e.files_deleted).sum()
    }

    pub fn unique_projects(&self) -> usize {
        let projects: std::collections::HashSet<&str> = self
            .entries
            .iter()
            .map(|e| e.project_path.as_str())
            .collect();
        projects.len()
    }

    pub fn last_n_entries(&self, n: usize) -> Vec<&ScanEntry> {
        let start = self.entries.len().saturating_sub(n);
        self.entries[start..].iter().collect()
    }

    pub fn savings_trend(&self, window: usize) -> Option<f64> {
        let recent = self.last_n_entries(window);
        if recent.is_empty() {
            return None;
        }
        let total_savings: u64 = recent.iter().map(|e| e.candidate_size).sum();
        let total_size: u64 = recent.iter().map(|e| e.total_size).sum();
        if total_size == 0 {
            return None;
        }
        Some(total_savings as f64 / total_size as f64 * 100.0)
    }

    pub fn clear(&mut self) -> Result<()> {
        self.entries.clear();
        self.save()
    }
}

/// Generate a JSON report from a scan result.
pub fn generate_json_report(scan_result: &ScanResult) -> Result<String> {
    let breakdown = scan_result.category_breakdown();
    let report = serde_json::json!({
        "tool": "jatin-lean",
        "version": env!("CARGO_PKG_VERSION"),
        "scan": {
            "root": scan_result.root.display().to_string(),
            "total_files": scan_result.total_files,
            "total_size": scan_result.total_size,
            "total_size_human": format_size(scan_result.total_size),
            "total_packages": scan_result.total_packages,
        },
        "results": {
            "candidate_count": scan_result.candidates.len(),
            "candidate_size": scan_result.savings(),
            "candidate_size_human": format_size(scan_result.savings()),
            "savings_percentage": if scan_result.total_size > 0 {
                scan_result.savings() as f64 / scan_result.total_size as f64 * 100.0
            } else { 0.0 },
            "risk_level": scan_result.risk_label(),
        },
        "breakdown": breakdown.iter().map(|(cat, (count, size))| {
            serde_json::json!({
                "category": cat.label(),
                "file_count": count,
                "total_size": size,
                "total_size_human": format_size(*size),
            })
        }).collect::<Vec<_>>(),
        "top_packages": top_packages_by_savings(scan_result, 20),
    });
    serde_json::to_string_pretty(&report).context("Failed to generate JSON report")
}

/// Generate a CSV report from a scan result.
pub fn generate_csv_report(scan_result: &ScanResult) -> String {
    let mut csv = String::from("file_path,size_bytes,size_human,category,package_name\n");
    for c in &scan_result.candidates {
        csv.push_str(&format!(
            "\"{}\",{},{},{},{}\n",
            c.path.display(),
            c.size,
            format_size(c.size),
            c.category.label(),
            c.package_name,
        ));
    }
    csv
}

/// Generate a Markdown report from a scan result.
pub fn generate_markdown_report(scan_result: &ScanResult) -> String {
    let savings = scan_result.savings();
    let pct = if scan_result.total_size > 0 {
        savings as f64 / scan_result.total_size as f64 * 100.0
    } else {
        0.0
    };
    let breakdown = scan_result.category_breakdown();

    let mut md = String::new();
    md.push_str("# jatin-lean Scan Report\n\n");
    md.push_str("## Summary\n\n");
    md.push_str("| Metric | Value |\n|--------|-------|\n");
    md.push_str(&format!(
        "| **Scanned Path** | `{}` |\n",
        scan_result.root.display()
    ));
    md.push_str(&format!(
        "| **Total Files** | {} |\n",
        format_number(scan_result.total_files)
    ));
    md.push_str(&format!(
        "| **Total Size** | {} |\n",
        format_size(scan_result.total_size)
    ));
    md.push_str(&format!(
        "| **Candidates** | {} files ({}) |\n",
        format_number(scan_result.candidates.len() as u64),
        format_size(savings)
    ));
    md.push_str(&format!("| **Savings** | {:.1}% |\n", pct));
    md.push_str(&format!(
        "| **Risk Level** | {} |\n\n",
        scan_result.risk_label()
    ));

    md.push_str("## Category Breakdown\n\n");
    md.push_str("| Category | Files | Size | Risk |\n|----------|-------|------|------|\n");
    let mut sorted: Vec<_> = breakdown.iter().collect();
    sorted.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));
    for (cat, (count, size)) in &sorted {
        let risk = match cat.risk_level() {
            0 => "Low",
            1 => "Medium",
            _ => "High",
        };
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            cat.label(),
            format_number(*count),
            format_size(*size),
            risk
        ));
    }
    md.push_str(&format!(
        "\n---\n*Generated by jatin-lean v{}*\n",
        env!("CARGO_PKG_VERSION")
    ));
    md
}

fn top_packages_by_savings(scan_result: &ScanResult, n: usize) -> Vec<serde_json::Value> {
    let mut by_pkg: HashMap<&str, (u64, u64)> = HashMap::new();
    for c in &scan_result.candidates {
        let e = by_pkg.entry(&c.package_name).or_insert((0, 0));
        e.0 += 1;
        e.1 += c.size;
    }
    let mut sorted: Vec<_> = by_pkg.into_iter().collect();
    sorted.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));
    sorted.truncate(n);
    sorted.into_iter().map(|(name, (count, size))| {
        serde_json::json!({ "name": name, "file_count": count, "size": size, "size_human": format_size(size) })
    }).collect()
}

/// Print analytics summary to the terminal.
pub fn print_analytics_summary(db: &AnalyticsDB) {
    use console::style;
    println!();
    println!(
        "  {} {}",
        style("Analytics Dashboard").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );

    if db.entries.is_empty() {
        println!("  {} No scan history yet.", style("ℹ").blue());
        println!();
        return;
    }

    println!(
        "  {} Total scans: {}",
        style("◉").cyan(),
        style(format_number(db.entries.len() as u64)).white().bold()
    );
    println!(
        "  {} Unique projects: {}",
        style("◉").cyan(),
        style(format_number(db.unique_projects() as u64))
            .white()
            .bold()
    );
    println!(
        "  {} Total bytes freed: {}",
        style("◉").green(),
        style(format_size(db.total_bytes_saved())).green().bold()
    );
    println!(
        "  {} Total files removed: {}",
        style("◉").green(),
        style(format_number(db.total_files_deleted()))
            .green()
            .bold()
    );
    if let Some(trend) = db.savings_trend(10) {
        println!(
            "  {} Avg savings rate: {}",
            style("◉").yellow(),
            style(format!("{:.1}%", trend)).yellow().bold()
        );
    }

    let recent = db.last_n_entries(5);
    if !recent.is_empty() {
        println!(
            "\n  {} {}",
            style("Recent Scans").white().bold(),
            style("───────────────────────────────").dim()
        );
        for entry in recent {
            let status = if entry.deletion_executed {
                "PRUNED"
            } else {
                "DRY-RUN"
            };
            println!(
                "  {} {} | {} candidates ({}) [{}]",
                style("▸").dim(),
                style(&entry.project_path).dim(),
                format_number(entry.candidate_count),
                format_size(entry.candidate_size),
                status
            );
        }
    }
    println!();
}
