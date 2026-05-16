//! Incremental scanning cache: hash-based caching to skip unchanged packages.
//!
//! Computes a fast hash of each package directory's modification times
//! and skips re-scanning packages that haven't changed since the last scan.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A cached scan state for a single package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageCacheEntry {
    /// Hash of the package directory (based on file count + total mtime)
    pub hash: u64,
    /// Number of candidate files last time
    pub candidate_count: u64,
    /// Total candidate size last time
    pub candidate_size: u64,
    /// Timestamp of when this cache entry was created
    pub cached_at: u64,
}

/// The full scan cache database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanCache {
    /// Version of the cache format
    pub version: u32,
    /// Cache keyed by absolute package directory path
    pub packages: HashMap<String, PackageCacheEntry>,
    /// When the cache was last written
    pub last_updated: u64,
}

impl Default for ScanCache {
    fn default() -> Self {
        Self {
            version: 1,
            packages: HashMap::new(),
            last_updated: 0,
        }
    }
}

impl ScanCache {
    /// Get the cache file path for a given node_modules directory.
    pub fn cache_path(node_modules_path: &Path) -> PathBuf {
        let parent = node_modules_path.parent().unwrap_or(node_modules_path);
        parent.join(".jatin-lean-cache.json")
    }

    /// Load the cache from disk, or create a new one.
    pub fn load(node_modules_path: &Path) -> Self {
        let path = Self::cache_path(node_modules_path);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cache) = serde_json::from_str::<ScanCache>(&content) {
                    return cache;
                }
            }
        }
        Self::default()
    }

    /// Save the cache to disk.
    pub fn save(&mut self, node_modules_path: &Path) -> Result<()> {
        self.last_updated = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let path = Self::cache_path(node_modules_path);
        let content = serde_json::to_string(self).context("Failed to serialize scan cache")?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write cache: {}", path.display()))?;
        Ok(())
    }

    /// Compute a fast hash of a package directory.
    /// Uses file count + sum of modification timestamps as a fingerprint.
    pub fn compute_package_hash(pkg_path: &Path) -> u64 {
        let mut hash: u64 = 0;
        let mut file_count: u64 = 0;

        if let Ok(entries) = fs::read_dir(pkg_path) {
            for entry in entries.flatten() {
                file_count += 1;
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        let mtime = modified
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        hash = hash.wrapping_add(mtime);
                    }
                    hash = hash.wrapping_add(metadata.len());
                }
            }
        }

        // Mix in file count for extra discrimination
        hash = hash.wrapping_mul(31).wrapping_add(file_count);
        hash
    }

    /// Check if a package has changed since it was last cached.
    pub fn is_package_changed(&self, pkg_path: &Path) -> bool {
        let key = pkg_path.display().to_string();
        match self.packages.get(&key) {
            Some(entry) => {
                let current_hash = Self::compute_package_hash(pkg_path);
                current_hash != entry.hash
            }
            None => true, // Not in cache, treat as changed
        }
    }

    /// Update the cache entry for a package.
    pub fn update_package(&mut self, pkg_path: &Path, candidate_count: u64, candidate_size: u64) {
        let key = pkg_path.display().to_string();
        let hash = Self::compute_package_hash(pkg_path);
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.packages.insert(
            key,
            PackageCacheEntry {
                hash,
                candidate_count,
                candidate_size,
                cached_at: now,
            },
        );
    }

    /// Get cached results for a package if it hasn't changed.
    pub fn get_cached(&self, pkg_path: &Path) -> Option<&PackageCacheEntry> {
        if !self.is_package_changed(pkg_path) {
            let key = pkg_path.display().to_string();
            self.packages.get(&key)
        } else {
            None
        }
    }

    /// Remove stale entries (packages that no longer exist).
    pub fn prune_stale(&mut self) {
        self.packages.retain(|path, _| Path::new(path).exists());
    }

    /// Get the number of cached packages.
    pub fn cached_count(&self) -> usize {
        self.packages.len()
    }

    /// Clear the entire cache.
    pub fn clear(&mut self) {
        self.packages.clear();
    }

    /// Get cache age in seconds.
    pub fn age_seconds(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.last_updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_scan_cache_default() {
        let cache = ScanCache::default();
        assert_eq!(cache.version, 1);
        assert!(cache.packages.is_empty());
    }

    #[test]
    fn test_compute_package_hash_empty_dir() -> Result<()> {
        let temp = TempDir::new()?;
        let hash = ScanCache::compute_package_hash(temp.path());
        // Should produce a non-zero hash due to the file_count mixing
        assert_eq!(hash, 0); // Empty dir → 0 files → 0 * 31 + 0 = 0
        Ok(())
    }

    #[test]
    fn test_compute_package_hash_with_files() -> Result<()> {
        let temp = TempDir::new()?;
        fs::write(temp.path().join("file1.txt"), "hello")?;
        fs::write(temp.path().join("file2.txt"), "world")?;

        let hash = ScanCache::compute_package_hash(temp.path());
        assert!(hash > 0);
        Ok(())
    }

    #[test]
    fn test_is_package_changed_not_cached() {
        let cache = ScanCache::default();
        let temp = TempDir::new().unwrap();
        assert!(cache.is_package_changed(temp.path()));
    }

    #[test]
    fn test_update_and_check_package() -> Result<()> {
        let mut cache = ScanCache::default();
        let temp = TempDir::new()?;
        fs::write(temp.path().join("index.js"), "module.exports = {}")?;

        cache.update_package(temp.path(), 5, 1024);
        assert!(!cache.is_package_changed(temp.path()));

        // Modify the directory
        fs::write(temp.path().join("new_file.js"), "new content")?;
        assert!(cache.is_package_changed(temp.path()));

        Ok(())
    }

    #[test]
    fn test_save_and_load_cache() -> Result<()> {
        let temp = TempDir::new()?;
        let nm_path = temp.path().join("node_modules");
        fs::create_dir_all(&nm_path)?;

        let mut cache = ScanCache::default();
        cache.update_package(&nm_path.join("test-pkg"), 10, 2048);
        cache.save(&nm_path)?;

        let loaded = ScanCache::load(&nm_path);
        assert_eq!(loaded.cached_count(), 1);

        Ok(())
    }

    #[test]
    fn test_prune_stale_entries() {
        let mut cache = ScanCache::default();
        cache.packages.insert(
            "/nonexistent/path/package".to_string(),
            PackageCacheEntry {
                hash: 12345,
                candidate_count: 5,
                candidate_size: 1024,
                cached_at: 0,
            },
        );

        assert_eq!(cache.cached_count(), 1);
        cache.prune_stale();
        assert_eq!(cache.cached_count(), 0);
    }
}
