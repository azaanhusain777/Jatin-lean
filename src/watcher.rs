//! File system watcher: monitor node_modules for auto-pruning.
//!
//! Watches for changes to node_modules (e.g., after `npm install`)
//! and automatically triggers a scan + prune cycle. Uses polling
//! for broad platform compatibility.

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

/// Configuration for the watcher.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Polling interval in seconds
    pub poll_interval_secs: u64,
    /// Whether to auto-prune on detected changes
    pub auto_prune: bool,
    /// Whether to create snapshots before auto-pruning
    pub snapshot_before_prune: bool,
    /// Maximum number of auto-prune cycles (0 = unlimited)
    pub max_cycles: u64,
    /// Grace period after change detection before scanning (seconds)
    pub grace_period_secs: u64,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 5,
            auto_prune: false,
            snapshot_before_prune: true,
            max_cycles: 0,
            grace_period_secs: 3,
        }
    }
}

/// State of a watched directory.
#[derive(Debug, Clone)]
struct DirectoryState {
    /// Hash of the directory state (file count + mod times)
    hash: u64,
    /// When this state was captured
    captured_at: Instant,
}

/// A watcher that monitors node_modules for changes.
pub struct NodeModulesWatcher {
    /// Path to the node_modules directory
    node_modules_path: PathBuf,
    /// Watcher configuration
    config: WatcherConfig,
    /// Running flag (shared with signal handler)
    running: Arc<AtomicBool>,
    /// Previous directory state for change detection
    last_state: Option<DirectoryState>,
    /// Number of prune cycles completed
    cycle_count: u64,
}

impl NodeModulesWatcher {
    /// Create a new watcher.
    pub fn new(node_modules_path: PathBuf, config: WatcherConfig) -> Self {
        Self {
            node_modules_path,
            config,
            running: Arc::new(AtomicBool::new(false)),
            last_state: None,
            cycle_count: 0,
        }
    }

    /// Get a clone of the running flag for use in signal handlers.
    pub fn running_flag(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    /// Start watching (blocking call).
    pub fn watch<F>(&mut self, on_change: F) -> Result<()>
    where
        F: Fn(&Path) -> Result<()>,
    {
        use console::style;

        self.running.store(true, Ordering::SeqCst);

        println!();
        println!(
            "  {} {}",
            style("Watch Mode").cyan().bold(),
            style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
        );
        println!(
            "  {} Watching: {}",
            style("◉").cyan(),
            style(self.node_modules_path.display()).white().bold()
        );
        println!(
            "  {} Poll interval: {}s",
            style("◉").cyan(),
            self.config.poll_interval_secs
        );
        println!(
            "  {} Auto-prune: {}",
            style("◉").cyan(),
            if self.config.auto_prune { "ON" } else { "OFF" }
        );
        println!(
            "  {} Press {} to stop watching.",
            style("ℹ").blue(),
            style("Ctrl+C").yellow().bold()
        );
        println!();

        // Capture initial state
        self.last_state = Some(self.capture_state()?);

        while self.running.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_secs(self.config.poll_interval_secs));

            if !self.running.load(Ordering::SeqCst) {
                break;
            }

            // Check for changes
            let current_state = match self.capture_state() {
                Ok(state) => state,
                Err(e) => {
                    eprintln!("  {} Error scanning: {}", style("⚠").yellow(), e);
                    continue;
                }
            };

            if let Some(ref last) = self.last_state {
                if current_state.hash != last.hash {
                    println!(
                        "  {} Changes detected in node_modules!",
                        style("⚡").yellow().bold()
                    );

                    // Grace period
                    if self.config.grace_period_secs > 0 {
                        println!(
                            "  {} Waiting {}s for install to complete...",
                            style("◉").dim(),
                            self.config.grace_period_secs
                        );
                        std::thread::sleep(Duration::from_secs(self.config.grace_period_secs));
                    }

                    // Trigger callback
                    match on_change(&self.node_modules_path) {
                        Ok(()) => {
                            self.cycle_count += 1;
                            println!(
                                "  {} Prune cycle #{} complete.",
                                style("✓").green().bold(),
                                self.cycle_count
                            );
                        }
                        Err(e) => {
                            eprintln!("  {} Prune failed: {}", style("✗").red(), e);
                        }
                    }

                    // Check max cycles
                    if self.config.max_cycles > 0 && self.cycle_count >= self.config.max_cycles {
                        println!(
                            "  {} Max cycles ({}) reached. Stopping.",
                            style("ℹ").blue(),
                            self.config.max_cycles
                        );
                        break;
                    }
                }
            }

            self.last_state = Some(current_state);
        }

        println!(
            "\n  {} Watcher stopped. {} prune cycles completed.",
            style("◉").cyan(),
            self.cycle_count
        );
        println!();

        Ok(())
    }

    /// Stop the watcher.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Capture the current state of the node_modules directory.
    fn capture_state(&self) -> Result<DirectoryState> {
        let hash = compute_directory_hash(&self.node_modules_path)?;
        Ok(DirectoryState {
            hash,
            captured_at: Instant::now(),
        })
    }
}

/// Compute a fast hash of a directory's state.
/// Uses file count at the top level + total sizes as a fingerprint.
fn compute_directory_hash(path: &Path) -> Result<u64> {
    let mut hash: u64 = 0;
    let mut count: u64 = 0;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            count += 1;
            if let Ok(metadata) = entry.metadata() {
                hash = hash.wrapping_add(metadata.len());
                if let Ok(modified) = metadata.modified() {
                    let mtime = modified
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    hash = hash.wrapping_add(mtime);
                }
            }
        }
    }

    hash = hash.wrapping_mul(31).wrapping_add(count);
    Ok(hash)
}

/// Post-install hook checker: detect if npm/yarn/pnpm just ran.
pub fn detect_recent_install(project_dir: &Path) -> Option<InstallInfo> {
    let lock_files = [
        ("package-lock.json", "npm"),
        ("yarn.lock", "yarn"),
        ("pnpm-lock.yaml", "pnpm"),
    ];

    for (filename, manager) in &lock_files {
        let path = project_dir.join(filename);
        if let Ok(metadata) = fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                let age = SystemTime::now()
                    .duration_since(modified)
                    .unwrap_or_default();

                // If lock file was modified in the last 30 seconds
                if age.as_secs() < 30 {
                    return Some(InstallInfo {
                        package_manager: manager.to_string(),
                        lock_file: filename.to_string(),
                        age_seconds: age.as_secs(),
                    });
                }
            }
        }
    }

    None
}

/// Information about a recent package install.
#[derive(Debug)]
pub struct InstallInfo {
    pub package_manager: String,
    pub lock_file: String,
    pub age_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_watcher_config_default() {
        let config = WatcherConfig::default();
        assert_eq!(config.poll_interval_secs, 5);
        assert!(!config.auto_prune);
        assert!(config.snapshot_before_prune);
    }

    #[test]
    fn test_compute_directory_hash() -> Result<()> {
        let temp = TempDir::new()?;
        let hash1 = compute_directory_hash(temp.path())?;

        // Add a file
        fs::write(temp.path().join("test.txt"), "hello")?;
        let hash2 = compute_directory_hash(temp.path())?;

        assert_ne!(hash1, hash2);
        Ok(())
    }

    #[test]
    fn test_watcher_creation() {
        let watcher =
            NodeModulesWatcher::new(PathBuf::from("/tmp/node_modules"), WatcherConfig::default());
        assert_eq!(watcher.cycle_count, 0);
        assert!(watcher.last_state.is_none());
    }

    #[test]
    fn test_detect_recent_install_none() {
        let temp = TempDir::new().unwrap();
        assert!(detect_recent_install(temp.path()).is_none());
    }
}
