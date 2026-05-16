//! Snapshot engine: pre-deletion snapshots for undo/restore.
//!
//! Before any destructive operation, takes a snapshot of the files
//! that are about to be deleted. This allows users to restore files
//! if something goes wrong.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::scanner::{format_size, PruneCandidate};

/// Metadata for a single snapshotted file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFileEntry {
    /// Original absolute path of the file
    pub original_path: String,
    /// Relative path within the snapshot archive
    pub archive_path: String,
    /// Size in bytes
    pub size: u64,
    /// Category label
    pub category: String,
    /// Package name
    pub package_name: String,
}

/// Metadata for a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifest {
    /// Unique snapshot ID
    pub id: String,
    /// When the snapshot was created (Unix epoch seconds)
    pub created_at: u64,
    /// The node_modules path this snapshot is for
    pub node_modules_path: String,
    /// Number of files in the snapshot
    pub file_count: u64,
    /// Total size of files in bytes
    pub total_size: u64,
    /// Individual file entries
    pub files: Vec<SnapshotFileEntry>,
}

/// Manager for creating and restoring snapshots.
pub struct SnapshotManager {
    /// Base directory for storing snapshots
    pub snapshots_dir: PathBuf,
}

impl SnapshotManager {
    /// Create a new SnapshotManager.
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        let snapshots_dir = home.join(".config").join("jatin-lean").join("snapshots");
        if !snapshots_dir.exists() {
            fs::create_dir_all(&snapshots_dir)?;
        }
        Ok(Self { snapshots_dir })
    }

    /// Create a snapshot of the given candidates before deletion.
    pub fn create_snapshot(
        &self,
        candidates: &[PruneCandidate],
        node_modules_path: &Path,
    ) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let snapshot_id = format!("snap_{:x}", now);
        let snapshot_dir = self.snapshots_dir.join(&snapshot_id);
        fs::create_dir_all(&snapshot_dir)?;

        let files_dir = snapshot_dir.join("files");
        fs::create_dir_all(&files_dir)?;

        let mut entries = Vec::new();
        let mut total_size: u64 = 0;

        for (i, candidate) in candidates.iter().enumerate() {
            // Create a safe archive path
            let archive_name = format!(
                "{:06}_{}",
                i,
                candidate
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
            );

            let dest = files_dir.join(&archive_name);

            // Copy the file to the snapshot
            match fs::copy(&candidate.path, &dest) {
                Ok(_) => {
                    total_size += candidate.size;
                    entries.push(SnapshotFileEntry {
                        original_path: candidate.path.display().to_string(),
                        archive_path: archive_name,
                        size: candidate.size,
                        category: candidate.category.label().to_string(),
                        package_name: candidate.package_name.clone(),
                    });
                }
                Err(e) => {
                    eprintln!(
                        "  Warning: Could not snapshot {}: {}",
                        candidate.path.display(),
                        e
                    );
                }
            }
        }

        let manifest = SnapshotManifest {
            id: snapshot_id.clone(),
            created_at: now,
            node_modules_path: node_modules_path.display().to_string(),
            file_count: entries.len() as u64,
            total_size,
            files: entries,
        };

        let manifest_path = snapshot_dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        fs::write(&manifest_path, manifest_json)?;

        Ok(snapshot_id)
    }

    /// List all available snapshots.
    pub fn list_snapshots(&self) -> Result<Vec<SnapshotManifest>> {
        let mut snapshots = Vec::new();

        if !self.snapshots_dir.exists() {
            return Ok(snapshots);
        }

        for entry in fs::read_dir(&self.snapshots_dir)?.flatten() {
            if entry.path().is_dir() {
                let manifest_path = entry.path().join("manifest.json");
                if manifest_path.exists() {
                    if let Ok(content) = fs::read_to_string(&manifest_path) {
                        if let Ok(manifest) = serde_json::from_str::<SnapshotManifest>(&content) {
                            snapshots.push(manifest);
                        }
                    }
                }
            }
        }

        snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(snapshots)
    }

    /// Restore files from a snapshot.
    pub fn restore_snapshot(&self, snapshot_id: &str) -> Result<RestoreResult> {
        let snapshot_dir = self.snapshots_dir.join(snapshot_id);
        let manifest_path = snapshot_dir.join("manifest.json");

        if !manifest_path.exists() {
            anyhow::bail!("Snapshot not found: {}", snapshot_id);
        }

        let content = fs::read_to_string(&manifest_path)?;
        let manifest: SnapshotManifest = serde_json::from_str(&content)?;

        let files_dir = snapshot_dir.join("files");
        let mut restored = 0u64;
        let mut restored_size = 0u64;
        let mut failures = Vec::new();

        for entry in &manifest.files {
            let source = files_dir.join(&entry.archive_path);
            let dest = PathBuf::from(&entry.original_path);

            // Create parent directories if needed
            if let Some(parent) = dest.parent() {
                if !parent.exists() {
                    let _ = fs::create_dir_all(parent);
                }
            }

            match fs::copy(&source, &dest) {
                Ok(_) => {
                    restored += 1;
                    restored_size += entry.size;
                }
                Err(e) => {
                    failures.push((entry.original_path.clone(), e.to_string()));
                }
            }
        }

        Ok(RestoreResult {
            snapshot_id: snapshot_id.to_string(),
            restored_count: restored,
            restored_size,
            failures,
        })
    }

    /// Delete a snapshot to free disk space.
    pub fn delete_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let snapshot_dir = self.snapshots_dir.join(snapshot_id);
        if snapshot_dir.exists() {
            fs::remove_dir_all(&snapshot_dir)
                .with_context(|| format!("Failed to delete snapshot: {}", snapshot_id))?;
        }
        Ok(())
    }

    /// Delete all snapshots older than the given number of days.
    pub fn cleanup_old_snapshots(&self, max_age_days: u64) -> Result<u64> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let max_age_secs = max_age_days * 86400;
        let mut deleted = 0u64;

        let snapshots = self.list_snapshots()?;
        for snap in snapshots {
            if now.saturating_sub(snap.created_at) > max_age_secs {
                self.delete_snapshot(&snap.id)?;
                deleted += 1;
            }
        }

        Ok(deleted)
    }

    /// Get total disk space used by all snapshots.
    pub fn total_snapshot_size(&self) -> Result<u64> {
        let snapshots = self.list_snapshots()?;
        Ok(snapshots.iter().map(|s| s.total_size).sum())
    }
}

/// Result of a restore operation.
#[derive(Debug)]
pub struct RestoreResult {
    pub snapshot_id: String,
    pub restored_count: u64,
    pub restored_size: u64,
    pub failures: Vec<(String, String)>,
}

/// Print snapshot list to the terminal.
pub fn print_snapshot_list(snapshots: &[SnapshotManifest]) {
    use console::style;

    println!();
    println!(
        "  {} {}",
        style("Snapshots").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );

    if snapshots.is_empty() {
        println!("  {} No snapshots found.", style("ℹ").blue());
        println!(
            "  {} Snapshots are created with {}",
            style("→").dim(),
            style("--snapshot").yellow()
        );
        println!();
        return;
    }

    for snap in snapshots {
        let age = {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let diff = now.saturating_sub(snap.created_at);
            if diff < 3600 {
                format!("{}m ago", diff / 60)
            } else if diff < 86400 {
                format!("{}h ago", diff / 3600)
            } else {
                format!("{}d ago", diff / 86400)
            }
        };

        println!(
            "  {} {} — {} files ({}) — {}",
            style("▸").dim(),
            style(&snap.id).cyan().bold(),
            snap.file_count,
            format_size(snap.total_size),
            style(&age).dim(),
        );
        println!(
            "    {} {}",
            style("→").dim(),
            style(&snap.node_modules_path).dim(),
        );
    }
    println!();
}

/// Print restore result to the terminal.
pub fn print_restore_result(result: &RestoreResult) {
    use console::style;

    println!();
    if result.restored_count > 0 {
        println!(
            "  {} Restored {} files ({})",
            style("✓").green().bold(),
            result.restored_count,
            format_size(result.restored_size),
        );
    }
    if !result.failures.is_empty() {
        println!(
            "  {} {} files could not be restored:",
            style("⚠").yellow(),
            result.failures.len(),
        );
        for (path, err) in result.failures.iter().take(5) {
            println!("    {} {} — {}", style("→").dim(), path, err);
        }
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_snapshot_manifest_serialization() {
        let manifest = SnapshotManifest {
            id: "test_snap".to_string(),
            created_at: 1700000000,
            node_modules_path: "/test/node_modules".to_string(),
            file_count: 5,
            total_size: 1024,
            files: vec![],
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: SnapshotManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "test_snap");
    }
}
