//! Maglev Consistent Hashing Algorithm
//!
//! From Section 1.2 of the HPC document.
//! Google's Maglev uses a fixed-size lookup table with O(1) lookups
//! and minimal disruption when backends are added/removed.
//! Table generation is O(M*N) where M = table size, N = backends.

use std::collections::HashMap;
use std::time::Instant;

/// Default Maglev table size (must be prime for uniform distribution).
pub const DEFAULT_TABLE_SIZE: usize = 65537;

// ─── Maglev Hasher ───────────────────────────────────────────────────────────

/// Maglev consistent hash ring.
pub struct MaglevHashRing {
    /// Lookup table: index → backend ID
    pub table: Vec<usize>,
    /// Table size (prime number)
    pub table_size: usize,
    /// Backend names
    pub backends: Vec<String>,
    /// Per-backend permutation offsets
    permutation_offset: Vec<usize>,
    /// Per-backend permutation skip values
    permutation_skip: Vec<usize>,
    /// Stats
    pub stats: MaglevStats,
}

#[derive(Debug, Clone)]
pub struct MaglevStats {
    pub lookups: u64,
    pub table_build_ns: u64,
    pub backend_count: usize,
    pub table_size: usize,
}

impl MaglevHashRing {
    /// Build a new Maglev hash ring with the given backends.
    pub fn new(backends: Vec<String>, table_size: usize) -> Self {
        let start = Instant::now();
        let n = backends.len();
        assert!(n > 0, "At least one backend required");
        assert!(table_size > 0, "Table size must be positive");

        // Compute permutation parameters for each backend
        let mut offset = Vec::with_capacity(n);
        let mut skip = Vec::with_capacity(n);
        for name in &backends {
            let h1 = fnv1a(name.as_bytes());
            let h2 = fnv1a_seed(name.as_bytes(), 0x12345678);
            offset.push((h1 as usize) % table_size);
            skip.push(((h2 as usize) % (table_size - 1)) + 1);
        }

        // Populate lookup table using Maglev's round-robin permutation
        let mut table = vec![usize::MAX; table_size];
        let mut next = vec![0usize; n];
        let mut filled = 0;

        'outer: loop {
            for i in 0..n {
                // Find next empty slot for backend i
                let mut slot = (offset[i] + next[i] * skip[i]) % table_size;
                while table[slot] != usize::MAX {
                    next[i] += 1;
                    slot = (offset[i] + next[i] * skip[i]) % table_size;
                }
                table[slot] = i;
                next[i] += 1;
                filled += 1;
                if filled >= table_size {
                    break 'outer;
                }
            }
        }

        let build_ns = start.elapsed().as_nanos() as u64;

        Self {
            table,
            table_size,
            backends,
            permutation_offset: offset,
            permutation_skip: skip,
            stats: MaglevStats {
                lookups: 0,
                table_build_ns: build_ns,
                backend_count: n,
                table_size,
            },
        }
    }

    /// Lookup which backend serves the given key. O(1).
    pub fn lookup(&mut self, key: &str) -> &str {
        let hash = fnv1a(key.as_bytes()) as usize;
        let idx = hash % self.table_size;
        let backend_idx = self.table[idx];
        self.stats.lookups += 1;
        &self.backends[backend_idx]
    }

    /// Lookup without mutable stats (for read-only).
    pub fn lookup_immutable(&self, key: &str) -> &str {
        let hash = fnv1a(key.as_bytes()) as usize;
        let idx = hash % self.table_size;
        &self.backends[self.table[idx]]
    }

    /// Distribution analysis: how evenly are keys distributed?
    pub fn distribution(&self) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for &backend_idx in &self.table {
            *counts
                .entry(self.backends[backend_idx].clone())
                .or_insert(0) += 1;
        }
        counts
    }

    /// Standard deviation of distribution (lower = more even).
    pub fn distribution_stddev(&self) -> f64 {
        let dist = self.distribution();
        let mean = self.table_size as f64 / self.backends.len() as f64;
        let variance: f64 = dist
            .values()
            .map(|&count| (count as f64 - mean).powi(2))
            .sum::<f64>()
            / dist.len() as f64;
        variance.sqrt()
    }

    /// Disruption rate when removing one backend.
    /// Returns percentage of keys that would move.
    pub fn disruption_rate(&self, removed_backend: &str) -> f64 {
        let remaining: Vec<String> = self
            .backends
            .iter()
            .filter(|b| b.as_str() != removed_backend)
            .cloned()
            .collect();
        if remaining.is_empty() {
            return 100.0;
        }

        let new_ring = MaglevHashRing::new(remaining, self.table_size);
        let mut moved = 0;
        for i in 0..self.table_size {
            let old_backend = &self.backends[self.table[i]];
            if old_backend == removed_backend {
                moved += 1; // This key must move
            } else {
                let new_backend = &new_ring.backends[new_ring.table[i]];
                if old_backend != new_backend {
                    moved += 1;
                }
            }
        }
        moved as f64 / self.table_size as f64 * 100.0
    }
}

// ─── FNV-1a Hash ─────────────────────────────────────────────────────────────

/// FNV-1a 64-bit hash.
pub fn fnv1a(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// FNV-1a with seed.
pub fn fnv1a_seed(data: &[u8], seed: u64) -> u64 {
    let mut hash = seed;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Print Maglev report.
pub fn print_maglev_report(ring: &MaglevHashRing) {
    use console::style;
    println!();
    println!(
        "  {} {}",
        style("Maglev Consistent Hash Ring").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );
    println!(
        "  {} Table size:    {} (prime)",
        style("▸").dim(),
        ring.table_size
    );
    println!(
        "  {} Backends:      {}",
        style("▸").dim(),
        ring.backends.len()
    );
    println!(
        "  {} Build time:    {:.2} µs",
        style("▸").dim(),
        ring.stats.table_build_ns as f64 / 1000.0
    );
    println!(
        "  {} Lookups:       {}",
        style("▸").dim(),
        ring.stats.lookups
    );
    println!(
        "  {} Std deviation: {:.2}",
        style("▸").dim(),
        ring.distribution_stddev()
    );

    let dist = ring.distribution();
    let ideal = ring.table_size / ring.backends.len();
    for (backend, count) in &dist {
        let pct = *count as f64 / ring.table_size as f64 * 100.0;
        let skew = (*count as f64 / ideal as f64 - 1.0) * 100.0;
        println!(
            "    {} → {} slots ({:.1}%, skew: {:+.1}%)",
            style(backend).yellow(),
            count,
            pct,
            skew
        );
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maglev_basic() {
        let backends = vec!["server-1".into(), "server-2".into(), "server-3".into()];
        let mut ring = MaglevHashRing::new(backends, 101);
        let result = ring.lookup("test-key");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_maglev_consistency() {
        let backends = vec!["a".into(), "b".into(), "c".into()];
        let ring = MaglevHashRing::new(backends, 101);
        let r1 = ring.lookup_immutable("key1");
        let r2 = ring.lookup_immutable("key1");
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_maglev_distribution() {
        let backends = vec!["a".into(), "b".into(), "c".into()];
        let ring = MaglevHashRing::new(backends, 997);
        let dist = ring.distribution();
        assert_eq!(dist.len(), 3);
        for count in dist.values() {
            assert!(*count > 200); // roughly 997/3 ≈ 332
        }
    }

    #[test]
    fn test_maglev_low_disruption() {
        let backends = vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()];
        let ring = MaglevHashRing::new(backends, 997);
        let disruption = ring.disruption_rate("c");
        // Ideal disruption for Maglev: ~1/N, so ~20% for 5 backends
        assert!(disruption < 40.0, "Disruption too high: {:.1}%", disruption);
    }

    #[test]
    fn test_fnv1a() {
        let h1 = fnv1a(b"hello");
        let h2 = fnv1a(b"hello");
        let h3 = fnv1a(b"world");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }
}
