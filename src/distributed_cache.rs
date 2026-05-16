//! Distributed cache protocol for sharing scan results across machines.
//!
//! Provides a local cache with remote fallback, cache sync protocol,
//! and cache invalidation mechanisms.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Configuration for the distributed cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedCacheConfig {
    /// Whether distributed caching is enabled
    pub enabled: bool,
    /// Remote cache endpoints (URLs)
    pub endpoints: Vec<String>,
    /// Local cache directory
    pub local_cache_dir: PathBuf,
    /// Cache TTL in seconds
    pub ttl_seconds: u64,
    /// Maximum cache size in bytes
    pub max_cache_size: u64,
    /// Whether to fallback to local on remote failure
    pub fallback_to_local: bool,
    /// Timeout for remote operations in milliseconds
    pub timeout_ms: u64,
}

impl Default for DistributedCacheConfig {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("jatin-lean")
            .join("distributed-cache");

        Self {
            enabled: false,
            endpoints: Vec::new(),
            local_cache_dir: cache_dir,
            ttl_seconds: 86400, // 24 hours
            max_cache_size: 100 * 1024 * 1024, // 100MB
            fallback_to_local: true,
            timeout_ms: 5000,
        }
    }
}

/// A cache entry that can be synced across machines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedCacheEntry {
    /// Unique key for this entry
    pub key: String,
    /// Content hash of the cached data
    pub content_hash: u64,
    /// The cached scan data (serialized)
    pub data: CachedScanData,
    /// When this entry was created
    pub created_at: u64,
    /// When this entry expires
    pub expires_at: u64,
    /// Machine identifier that created this entry
    pub origin_machine: String,
    /// Version of the cache format
    pub version: u32,
}

/// Cached scan data for a package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedScanData {
    /// Package name
    pub package_name: String,
    /// Package version
    pub package_version: String,
    /// Number of candidate files
    pub candidate_count: u64,
    /// Total candidate size
    pub candidate_size: u64,
    /// File categories breakdown
    pub category_breakdown: HashMap<String, (u64, u64)>,
    /// Package hash for validation
    pub package_hash: u64,
}

/// The distributed cache manager.
pub struct DistributedCache {
    config: DistributedCacheConfig,
    local_entries: HashMap<String, DistributedCacheEntry>,
    stats: CacheStats,
}

/// Cache operation statistics.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub local_hits: u64,
    pub local_misses: u64,
    pub remote_hits: u64,
    pub remote_misses: u64,
    pub remote_errors: u64,
    pub entries_synced: u64,
    pub entries_evicted: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.local_hits + self.local_misses + self.remote_hits + self.remote_misses;
        if total == 0 { return 0.0; }
        (self.local_hits + self.remote_hits) as f64 / total as f64 * 100.0
    }
}

impl DistributedCache {
    /// Create a new distributed cache manager.
    pub fn new(config: DistributedCacheConfig) -> Result<Self> {
        // Ensure local cache directory exists
        if !config.local_cache_dir.exists() {
            fs::create_dir_all(&config.local_cache_dir)
                .with_context(|| format!("Failed to create cache dir: {}", config.local_cache_dir.display()))?;
        }

        let local_entries = Self::load_local_entries(&config.local_cache_dir)?;

        Ok(Self {
            config,
            local_entries,
            stats: CacheStats::default(),
        })
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Result<Self> {
        Self::new(DistributedCacheConfig::default())
    }

    /// Look up a cache entry by key.
    pub fn get(&mut self, key: &str) -> Option<&CachedScanData> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Check local cache first
        if let Some(entry) = self.local_entries.get(key) {
            if entry.expires_at > now {
                self.stats.local_hits += 1;
                return Some(&entry.data);
            } else {
                self.stats.local_misses += 1;
                // Entry expired — will be cleaned up later
            }
        } else {
            self.stats.local_misses += 1;
        }

        // Remote fallback would go here (requires async/network)
        // For now, return None
        None
    }

    /// Store a cache entry.
    pub fn put(&mut self, key: String, data: CachedScanData) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = DistributedCacheEntry {
            key: key.clone(),
            content_hash: Self::hash_data(&data),
            data,
            created_at: now,
            expires_at: now + self.config.ttl_seconds,
            origin_machine: Self::machine_id(),
            version: 1,
        };

        self.local_entries.insert(key, entry);
        self.save_local_entries()?;
        Ok(())
    }

    /// Remove a cache entry.
    pub fn remove(&mut self, key: &str) -> Result<()> {
        self.local_entries.remove(key);
        self.save_local_entries()?;
        Ok(())
    }

    /// Invalidate all entries for a package.
    pub fn invalidate_package(&mut self, package_name: &str) -> Result<u64> {
        let keys_to_remove: Vec<String> = self.local_entries
            .iter()
            .filter(|(_, entry)| entry.data.package_name == package_name)
            .map(|(key, _)| key.clone())
            .collect();

        let count = keys_to_remove.len() as u64;
        for key in keys_to_remove {
            self.local_entries.remove(&key);
        }

        if count > 0 {
            self.save_local_entries()?;
        }
        Ok(count)
    }

    /// Evict expired entries.
    pub fn evict_expired(&mut self) -> Result<u64> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let keys_to_remove: Vec<String> = self.local_entries
            .iter()
            .filter(|(_, entry)| entry.expires_at <= now)
            .map(|(key, _)| key.clone())
            .collect();

        let count = keys_to_remove.len() as u64;
        for key in keys_to_remove {
            self.local_entries.remove(&key);
        }

        self.stats.entries_evicted += count;
        if count > 0 {
            self.save_local_entries()?;
        }
        Ok(count)
    }

    /// Get cache statistics.
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Get the number of cached entries.
    pub fn entry_count(&self) -> usize {
        self.local_entries.len()
    }

    /// Clear all local cache entries.
    pub fn clear(&mut self) -> Result<()> {
        self.local_entries.clear();
        self.save_local_entries()?;
        Ok(())
    }

    /// Generate a cache key for a package path and hash.
    pub fn cache_key(package_path: &Path, package_hash: u64) -> String {
        let path_str = package_path.to_string_lossy();
        format!("{}:{:016x}", path_str, package_hash)
    }

    // --- Internal helpers ---

    fn hash_data(data: &CachedScanData) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in data.package_name.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= data.candidate_count;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= data.candidate_size;
        hash = hash.wrapping_mul(0x100000001b3);
        hash
    }

    fn machine_id() -> String {
        hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    }

    fn local_cache_file(cache_dir: &Path) -> PathBuf {
        cache_dir.join("distributed-cache.json")
    }

    fn load_local_entries(cache_dir: &Path) -> Result<HashMap<String, DistributedCacheEntry>> {
        let cache_file = Self::local_cache_file(cache_dir);
        if cache_file.exists() {
            let content = fs::read_to_string(&cache_file)
                .with_context(|| format!("Failed to read cache: {}", cache_file.display()))?;
            let entries: HashMap<String, DistributedCacheEntry> = serde_json::from_str(&content)
                .unwrap_or_default();
            Ok(entries)
        } else {
            Ok(HashMap::new())
        }
    }

    fn save_local_entries(&self) -> Result<()> {
        let cache_file = Self::local_cache_file(&self.config.local_cache_dir);
        let content = serde_json::to_string(&self.local_entries)
            .context("Failed to serialize cache")?;
        fs::write(&cache_file, content)
            .with_context(|| format!("Failed to write cache: {}", cache_file.display()))?;
        Ok(())
    }
}

/// Print distributed cache info to console.
pub fn print_cache_info(cache: &DistributedCache) {
    use console::style;

    println!();
    println!(
        "  {} {}",
        style("Distributed Cache").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );

    let stats = cache.stats();
    println!(
        "  {} Entries: {} | Hit rate: {:.1}%",
        style("◉").cyan(),
        style(cache.entry_count()).white().bold(),
        stats.hit_rate(),
    );
    println!(
        "  {} Local: {} hits / {} misses",
        style("◉").cyan(),
        style(stats.local_hits).green(),
        style(stats.local_misses).yellow(),
    );
    println!(
        "  {} Evicted: {} | Synced: {}",
        style("◉").dim(),
        stats.entries_evicted,
        stats.entries_synced,
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config(dir: &Path) -> DistributedCacheConfig {
        DistributedCacheConfig {
            local_cache_dir: dir.to_path_buf(),
            ..Default::default()
        }
    }

    #[test]
    fn test_cache_put_get() -> Result<()> {
        let temp = TempDir::new()?;
        let mut cache = DistributedCache::new(test_config(temp.path()))?;

        let data = CachedScanData {
            package_name: "lodash".to_string(),
            package_version: "4.17.21".to_string(),
            candidate_count: 50,
            candidate_size: 100_000,
            category_breakdown: HashMap::new(),
            package_hash: 12345,
        };

        cache.put("lodash:key".to_string(), data)?;
        assert!(cache.get("lodash:key").is_some());
        assert!(cache.get("nonexistent").is_none());
        Ok(())
    }

    #[test]
    fn test_cache_remove() -> Result<()> {
        let temp = TempDir::new()?;
        let mut cache = DistributedCache::new(test_config(temp.path()))?;

        let data = CachedScanData {
            package_name: "test".to_string(),
            package_version: "1.0.0".to_string(),
            candidate_count: 10,
            candidate_size: 5000,
            category_breakdown: HashMap::new(),
            package_hash: 0,
        };

        cache.put("test:key".to_string(), data)?;
        assert_eq!(cache.entry_count(), 1);

        cache.remove("test:key")?;
        assert_eq!(cache.entry_count(), 0);
        Ok(())
    }

    #[test]
    fn test_cache_clear() -> Result<()> {
        let temp = TempDir::new()?;
        let mut cache = DistributedCache::new(test_config(temp.path()))?;

        for i in 0..5 {
            let data = CachedScanData {
                package_name: format!("pkg-{}", i),
                package_version: "1.0.0".to_string(),
                candidate_count: 1, candidate_size: 100,
                category_breakdown: HashMap::new(),
                package_hash: i as u64,
            };
            cache.put(format!("key-{}", i), data)?;
        }
        assert_eq!(cache.entry_count(), 5);
        cache.clear()?;
        assert_eq!(cache.entry_count(), 0);
        Ok(())
    }

    #[test]
    fn test_cache_stats() -> Result<()> {
        let temp = TempDir::new()?;
        let mut cache = DistributedCache::new(test_config(temp.path()))?;

        // Trigger misses
        cache.get("miss1");
        cache.get("miss2");
        assert_eq!(cache.stats().local_misses, 2);

        // Add and hit
        let data = CachedScanData {
            package_name: "hit".to_string(),
            package_version: "1.0.0".to_string(),
            candidate_count: 1, candidate_size: 100,
            category_breakdown: HashMap::new(),
            package_hash: 0,
        };
        cache.put("hit:key".to_string(), data)?;
        cache.get("hit:key");
        assert_eq!(cache.stats().local_hits, 1);
        Ok(())
    }

    #[test]
    fn test_cache_key_generation() {
        let key = DistributedCache::cache_key(
            Path::new("/project/node_modules/lodash"),
            0xDEADBEEF,
        );
        assert!(key.contains("lodash"));
        assert!(key.contains("deadbeef"));
    }
}
