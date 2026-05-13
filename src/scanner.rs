//! Scanning engine: parallel file discovery using the `ignore` crate.
//!
//! Discovers node_modules directories and walks them in parallel,
//! collecting file metadata for the pruning engine.

use crate::rules::{FileCategory, PruneRules};
use anyhow::{Context, Result};
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Information about a file flagged for deletion.
#[derive(Debug, Clone)]
pub struct PruneCandidate {
    /// Absolute path to the file.
    pub path: PathBuf,
    /// Size in bytes.
    pub size: u64,
    /// Category classification.
    pub category: FileCategory,
    /// The package this file belongs to (e.g., "lodash").
    pub package_name: String,
}

/// Results of scanning a node_modules directory.
#[derive(Debug)]
pub struct ScanResult {
    /// Root path that was scanned.
    pub root: PathBuf,
    /// Total number of files found.
    pub total_files: u64,
    /// Total size of all files.
    pub total_size: u64,
    /// Files flagged for deletion.
    pub candidates: Vec<PruneCandidate>,
    /// Total packages scanned.
    pub total_packages: usize,
    /// Whitelisted files (files that matched a rule but are runtime-required).
    pub whitelisted_count: u64,
}

impl ScanResult {
    /// Total savings in bytes.
    pub fn savings(&self) -> u64 {
        self.candidates.iter().map(|c| c.size).sum()
    }

    /// Breakdown by category.
    pub fn category_breakdown(&self) -> HashMap<FileCategory, (u64, u64)> {
        let mut map: HashMap<FileCategory, (u64, u64)> = HashMap::new();
        for c in &self.candidates {
            let entry = map.entry(c.category).or_insert((0, 0));
            entry.0 += 1; // count
            entry.1 += c.size; // size
        }
        map
    }

    /// Maximum risk level across all candidates.
    pub fn max_risk(&self) -> u8 {
        self.candidates
            .iter()
            .map(|c| c.category.risk_level())
            .max()
            .unwrap_or(0)
    }

    /// Risk label string.
    pub fn risk_label(&self) -> &'static str {
        match self.max_risk() {
            0 => "LOW — Only junk files targeted",
            1 => "MEDIUM — Source maps and build files included",
            2 => "HIGH — TypeScript sources included (declarations kept)",
            _ => "UNKNOWN",
        }
    }
}

/// Discover all `node_modules` directories under the given root.
pub fn find_node_modules(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut results = Vec::new();

    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .follow_links(false)
        .max_depth(Some(max_depth))
        .build();

    for entry in walker.flatten() {
        if entry.file_type().map_or(false, |ft| ft.is_dir()) {
            if entry.file_name().to_str() == Some("node_modules") {
                results.push(entry.into_path());
            }
        }
    }

    results
}

/// Get the last access time of a directory in days.
pub fn last_accessed_days(path: &Path) -> Option<u64> {
    let metadata = fs::metadata(path).ok()?;
    let accessed = metadata.accessed().ok()?;
    let duration = SystemTime::now().duration_since(accessed).ok()?;
    Some(duration.as_secs() / 86400)
}

/// Scan a single node_modules directory and identify prune candidates.
pub fn scan_node_modules(node_modules_path: &Path, rules: &PruneRules) -> Result<ScanResult> {
    let total_files = AtomicU64::new(0);
    let total_size = AtomicU64::new(0);
    let whitelisted = AtomicU64::new(0);
    let candidates = Arc::new(Mutex::new(Vec::new()));

    // Create progress bar
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template(
            "  {spinner:.cyan} {msg}",
        )
        .unwrap()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message("Scanning files...");
    pb.enable_steady_tick(std::time::Duration::from_millis(80));

    // Discover all packages
    let mut packages: Vec<PathBuf> = Vec::new();
    if node_modules_path.is_dir() {
        if let Ok(entries) = fs::read_dir(node_modules_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = entry.file_name();
                    let name_str = name.to_str().unwrap_or("");

                    if name_str.starts_with('@') {
                        // Scoped packages: @scope/package
                        if let Ok(scoped_entries) = fs::read_dir(&path) {
                            for scoped_entry in scoped_entries.flatten() {
                                if scoped_entry.path().is_dir() {
                                    packages.push(scoped_entry.path());
                                }
                            }
                        }
                    } else if name_str != ".bin" && name_str != ".cache" && name_str != ".package-lock.json" {
                        packages.push(path);
                    }
                }
            }
        }
    }

    let package_count = packages.len();

    // Parse each package's package.json to find whitelisted files
    let whitelisted_files: Arc<Mutex<std::collections::HashSet<PathBuf>>> =
        Arc::new(Mutex::new(std::collections::HashSet::new()));

    // Process packages in parallel
    packages.par_iter().for_each(|pkg_path| {
        let pkg_name = extract_package_name(pkg_path, node_modules_path);

        // Parse package.json for entry points
        let pkg_json_path = pkg_path.join("package.json");
        if pkg_json_path.exists() {
            if let Ok(content) = fs::read_to_string(&pkg_json_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    let entry_files = extract_entry_points(&json, pkg_path);
                    let mut wl = whitelisted_files.lock().unwrap();
                    for f in entry_files {
                        wl.insert(f);
                    }
                }
            }
        }

        // Walk all files in this package
        let walker = WalkBuilder::new(pkg_path)
            .hidden(false)
            .git_ignore(false)
            .follow_links(false)
            .build();

        for entry in walker.flatten() {
            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                continue;
            }

            let file_path = entry.into_path();
            let file_size = fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);

            total_files.fetch_add(1, Ordering::Relaxed);
            total_size.fetch_add(file_size, Ordering::Relaxed);

            let current_files = total_files.load(Ordering::Relaxed);
            let current_size = total_size.load(Ordering::Relaxed);
            if current_files % 500 == 0 {
                pb.set_message(format!(
                    "Scanning... {} files found | {} indexed",
                    format_number(current_files),
                    format_size(current_size)
                ));
            }

            // Get relative path within package
            if let Ok(rel_path) = file_path.strip_prefix(pkg_path) {
                if let Some(category) = rules.classify(rel_path) {
                    // Check if this file is whitelisted (runtime-required)
                    let wl = whitelisted_files.lock().unwrap();
                    if wl.contains(&file_path) {
                        whitelisted.fetch_add(1, Ordering::Relaxed);
                    } else {
                        drop(wl);
                        let candidate = PruneCandidate {
                            path: file_path,
                            size: file_size,
                            category,
                            package_name: pkg_name.clone(),
                        };
                        candidates.lock().unwrap().push(candidate);
                    }
                }
            }
        }
    });

    pb.finish_and_clear();

    let candidates = Arc::try_unwrap(candidates)
        .map_err(|_| anyhow::anyhow!("Failed to unwrap candidates"))
        .context("Failed to collect candidates")?
        .into_inner()
        .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))?;

    Ok(ScanResult {
        root: node_modules_path.to_path_buf(),
        total_files: total_files.load(Ordering::Relaxed),
        total_size: total_size.load(Ordering::Relaxed),
        candidates,
        total_packages: package_count,
        whitelisted_count: whitelisted.load(Ordering::Relaxed),
    })
}

/// Extract the package name from its path.
fn extract_package_name(pkg_path: &Path, node_modules_path: &Path) -> String {
    pkg_path
        .strip_prefix(node_modules_path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| {
            pkg_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        })
}

/// Extract runtime entry points from package.json.
///
/// Parses `main`, `module`, `browser`, `bin`, and `exports` fields.
fn extract_entry_points(json: &serde_json::Value, pkg_root: &Path) -> Vec<PathBuf> {
    let mut entries = Vec::new();

    // "main" field
    if let Some(main) = json.get("main").and_then(|v| v.as_str()) {
        entries.push(pkg_root.join(main));
    }

    // "module" field
    if let Some(module) = json.get("module").and_then(|v| v.as_str()) {
        entries.push(pkg_root.join(module));
    }

    // "browser" field (can be string or object)
    if let Some(browser) = json.get("browser") {
        if let Some(s) = browser.as_str() {
            entries.push(pkg_root.join(s));
        } else if let Some(obj) = browser.as_object() {
            for value in obj.values() {
                if let Some(s) = value.as_str() {
                    entries.push(pkg_root.join(s));
                }
            }
        }
    }

    // "bin" field (can be string or object)
    if let Some(bin) = json.get("bin") {
        if let Some(s) = bin.as_str() {
            entries.push(pkg_root.join(s));
        } else if let Some(obj) = bin.as_object() {
            for value in obj.values() {
                if let Some(s) = value.as_str() {
                    entries.push(pkg_root.join(s));
                }
            }
        }
    }

    // "exports" field (can be string, object, or nested)
    if let Some(exports) = json.get("exports") {
        collect_export_paths(exports, pkg_root, &mut entries);
    }

    // "types" / "typings" field — keep type declaration entry points
    if let Some(types) = json
        .get("types")
        .or_else(|| json.get("typings"))
        .and_then(|v| v.as_str())
    {
        entries.push(pkg_root.join(types));
    }

    entries
}

/// Recursively collect file paths from the `exports` field.
fn collect_export_paths(value: &serde_json::Value, root: &Path, out: &mut Vec<PathBuf>) {
    match value {
        serde_json::Value::String(s) => {
            // Skip wildcard patterns
            if !s.contains('*') {
                out.push(root.join(s));
            }
        }
        serde_json::Value::Object(map) => {
            for v in map.values() {
                collect_export_paths(v, root, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_export_paths(v, root, out);
            }
        }
        _ => {}
    }
}

/// Format a number with comma separators.
pub fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

/// Format a size in bytes as a human-readable string.
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}
