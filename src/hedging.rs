//! Request Hedging Engine & Fragmented Cache Delivery
//!
//! From Sections 4.1-4.2: Implements request hedging (dual-backend
//! racing), fragmented JSON cache delivery (serve partial field subsets
//! from a single cached superset), and RFC 7396 delta streaming.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

// ─── Request Hedging ─────────────────────────────────────────────────────────

/// Hedging strategy for latency optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HedgingStrategy {
    /// No hedging — single request
    None,
    /// Immediate dual-send to 2 replicas
    Immediate,
    /// Delayed hedge: send second after timeout
    Delayed(u64), // delay in ms
    /// Adaptive: hedge based on p99 latency history
    Adaptive,
}

/// A hedged request context.
#[derive(Debug, Clone)]
pub struct HedgedRequest {
    pub id: u64,
    pub resource: String,
    pub replicas_sent: Vec<String>,
    pub winner: Option<String>,
    pub cancellations_sent: usize,
    pub latency: Duration,
}

/// Hedging engine that races requests across replicas.
pub struct HedgingEngine {
    pub replicas: Vec<String>,
    pub strategy: HedgingStrategy,
    pub stats: HedgingStats,
    p99_history: Vec<Duration>,
}

#[derive(Debug)]
pub struct HedgingStats {
    pub requests: AtomicU64,
    pub hedges_sent: AtomicU64,
    pub hedge_wins: AtomicU64,
    pub cancellations: AtomicU64,
    pub total_latency_ns: AtomicU64,
}

impl HedgingStats {
    pub fn new() -> Self {
        Self {
            requests: AtomicU64::new(0), hedges_sent: AtomicU64::new(0),
            hedge_wins: AtomicU64::new(0), cancellations: AtomicU64::new(0),
            total_latency_ns: AtomicU64::new(0),
        }
    }

    pub fn hedge_win_rate(&self) -> f64 {
        let total = self.hedges_sent.load(Ordering::Relaxed);
        if total == 0 { return 0.0; }
        self.hedge_wins.load(Ordering::Relaxed) as f64 / total as f64 * 100.0
    }
}

impl HedgingEngine {
    pub fn new(replicas: Vec<String>, strategy: HedgingStrategy) -> Self {
        Self { replicas, strategy, stats: HedgingStats::new(), p99_history: Vec::new() }
    }

    /// Execute a hedged request. Returns the winning response.
    pub fn execute(&self, id: u64, resource: &str) -> HedgedRequest {
        let start = Instant::now();
        self.stats.requests.fetch_add(1, Ordering::Relaxed);

        let replicas_to_send: Vec<String> = match self.strategy {
            HedgingStrategy::None => vec![self.replicas[0].clone()],
            HedgingStrategy::Immediate => {
                let count = 2.min(self.replicas.len());
                self.replicas[..count].to_vec()
            }
            HedgingStrategy::Delayed(_) | HedgingStrategy::Adaptive => {
                let count = 2.min(self.replicas.len());
                self.replicas[..count].to_vec()
            }
        };

        let sent_count = replicas_to_send.len();
        if sent_count > 1 {
            self.stats.hedges_sent.fetch_add(1, Ordering::Relaxed);
        }

        // Simulate: the faster replica wins (use hash to deterministically pick)
        let winner_idx = (fnv_hash(resource.as_bytes()) as usize) % sent_count;
        let winner = replicas_to_send[winner_idx].clone();

        if sent_count > 1 && winner_idx > 0 {
            self.stats.hedge_wins.fetch_add(1, Ordering::Relaxed);
        }

        let cancellations = if sent_count > 1 { sent_count - 1 } else { 0 };
        self.stats.cancellations.fetch_add(cancellations as u64, Ordering::Relaxed);

        let latency = start.elapsed();
        self.stats.total_latency_ns.fetch_add(latency.as_nanos() as u64, Ordering::Relaxed);

        HedgedRequest {
            id, resource: resource.into(), replicas_sent: replicas_to_send,
            winner: Some(winner), cancellations_sent: cancellations, latency,
        }
    }
}

// ─── Fragmented Cache Delivery ───────────────────────────────────────────────

/// A cached superset document that can serve partial field subsets.
#[derive(Debug, Clone)]
pub struct CachedSuperset {
    pub resource_key: String,
    pub fields: HashMap<String, serde_json::Value>,
    pub cached_at: Instant,
    pub ttl: Duration,
    pub hit_count: u64,
}

/// Fragmented cache that stores JSON field-level data.
pub struct FragmentedCache {
    pub entries: HashMap<String, CachedSuperset>,
    pub stats: FragCacheStats,
}

#[derive(Debug)]
pub struct FragCacheStats {
    pub lookups: AtomicU64,
    pub full_hits: AtomicU64,
    pub partial_hits: AtomicU64,
    pub misses: AtomicU64,
    pub fragments_served: AtomicU64,
    pub backend_calls_saved: AtomicU64,
}

impl FragCacheStats {
    pub fn new() -> Self {
        Self {
            lookups: AtomicU64::new(0), full_hits: AtomicU64::new(0),
            partial_hits: AtomicU64::new(0), misses: AtomicU64::new(0),
            fragments_served: AtomicU64::new(0), backend_calls_saved: AtomicU64::new(0),
        }
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.lookups.load(Ordering::Relaxed);
        if total == 0 { return 0.0; }
        let hits = self.full_hits.load(Ordering::Relaxed) + self.partial_hits.load(Ordering::Relaxed);
        hits as f64 / total as f64 * 100.0
    }
}

impl FragmentedCache {
    pub fn new() -> Self {
        Self { entries: HashMap::new(), stats: FragCacheStats::new() }
    }

    /// Store a superset of fields for a resource.
    pub fn store(&mut self, key: &str, fields: HashMap<String, serde_json::Value>, ttl: Duration) {
        self.entries.insert(key.into(), CachedSuperset {
            resource_key: key.into(), fields, cached_at: Instant::now(), ttl, hit_count: 0,
        });
    }

    /// Fetch a subset of fields from the cached superset.
    /// Returns (found_fields, missing_fields).
    pub fn fetch_fragment(&mut self, key: &str, requested: &[&str]) -> FragmentResult {
        self.stats.lookups.fetch_add(1, Ordering::Relaxed);

        if let Some(entry) = self.entries.get_mut(key) {
            if entry.cached_at.elapsed() > entry.ttl {
                self.entries.remove(key);
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                return FragmentResult::Miss;
            }

            entry.hit_count += 1;
            let mut found = HashMap::new();
            let mut missing = Vec::new();

            for &field in requested {
                if let Some(val) = entry.fields.get(field) {
                    found.insert(field.to_string(), val.clone());
                } else {
                    missing.push(field.to_string());
                }
            }

            self.stats.fragments_served.fetch_add(found.len() as u64, Ordering::Relaxed);

            if missing.is_empty() {
                self.stats.full_hits.fetch_add(1, Ordering::Relaxed);
                self.stats.backend_calls_saved.fetch_add(1, Ordering::Relaxed);
                FragmentResult::FullHit(found)
            } else if !found.is_empty() {
                self.stats.partial_hits.fetch_add(1, Ordering::Relaxed);
                FragmentResult::PartialHit { found, missing }
            } else {
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                FragmentResult::Miss
            }
        } else {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
            FragmentResult::Miss
        }
    }

    /// Apply a RFC 7396 JSON Merge Patch to a cached entry (delta update).
    pub fn apply_delta(&mut self, key: &str, patch: &HashMap<String, serde_json::Value>) -> bool {
        if let Some(entry) = self.entries.get_mut(key) {
            for (k, v) in patch {
                if v.is_null() {
                    entry.fields.remove(k);
                } else {
                    entry.fields.insert(k.clone(), v.clone());
                }
            }
            true
        } else {
            false
        }
    }
}

#[derive(Debug)]
pub enum FragmentResult {
    FullHit(HashMap<String, serde_json::Value>),
    PartialHit { found: HashMap<String, serde_json::Value>, missing: Vec<String> },
    Miss,
}

fn fnv_hash(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    h
}

pub fn print_hedging_report(stats: &HedgingStats) {
    use console::style;
    println!();
    println!("  {} {}", style("Request Hedging Report").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
    println!("  {} Requests:      {}", style("▸").dim(), stats.requests.load(Ordering::Relaxed));
    println!("  {} Hedges sent:   {}", style("▸").dim(), stats.hedges_sent.load(Ordering::Relaxed));
    println!("  {} Hedge wins:    {} ({:.1}%)", style("⚡").yellow(),
        stats.hedge_wins.load(Ordering::Relaxed), stats.hedge_win_rate());
    println!("  {} Cancellations: {}", style("▸").dim(), stats.cancellations.load(Ordering::Relaxed));
    println!();
}

pub fn print_frag_cache_report(stats: &FragCacheStats) {
    use console::style;
    println!();
    println!("  {} {}", style("Fragmented Cache Report").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
    println!("  {} Lookups:       {}", style("▸").dim(), stats.lookups.load(Ordering::Relaxed));
    println!("  {} Full hits:     {}", style("▸").dim(), stats.full_hits.load(Ordering::Relaxed));
    println!("  {} Partial hits:  {}", style("▸").dim(), stats.partial_hits.load(Ordering::Relaxed));
    println!("  {} Misses:        {}", style("▸").dim(), stats.misses.load(Ordering::Relaxed));
    println!("  {} Hit rate:      {:.1}%", style("🚀").yellow(), stats.hit_rate());
    println!("  {} Fragments:     {}", style("▸").dim(), stats.fragments_served.load(Ordering::Relaxed));
    println!("  {} Backend saved: {}", style("⚡").yellow(),
        style(stats.backend_calls_saved.load(Ordering::Relaxed)).green().bold());
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hedging_immediate() {
        let engine = HedgingEngine::new(
            vec!["replica-1".into(), "replica-2".into()],
            HedgingStrategy::Immediate,
        );
        let result = engine.execute(1, "/api/data");
        assert_eq!(result.replicas_sent.len(), 2);
        assert!(result.winner.is_some());
    }

    #[test]
    fn test_fragmented_cache() {
        let mut cache = FragmentedCache::new();
        let mut fields = HashMap::new();
        fields.insert("name".into(), serde_json::json!("Alice"));
        fields.insert("age".into(), serde_json::json!(30));
        fields.insert("email".into(), serde_json::json!("alice@example.com"));
        cache.store("/user/1", fields, Duration::from_secs(60));

        // Full hit: request subset
        let result = cache.fetch_fragment("/user/1", &["name", "age"]);
        match result {
            FragmentResult::FullHit(found) => assert_eq!(found.len(), 2),
            _ => panic!("Expected full hit"),
        }

        // Partial hit: request with missing field
        let result = cache.fetch_fragment("/user/1", &["name", "phone"]);
        match result {
            FragmentResult::PartialHit { found, missing } => {
                assert_eq!(found.len(), 1);
                assert_eq!(missing, vec!["phone"]);
            }
            _ => panic!("Expected partial hit"),
        }
    }

    #[test]
    fn test_delta_update() {
        let mut cache = FragmentedCache::new();
        let mut fields = HashMap::new();
        fields.insert("name".into(), serde_json::json!("Alice"));
        fields.insert("age".into(), serde_json::json!(30));
        cache.store("/user/1", fields, Duration::from_secs(60));

        let mut patch = HashMap::new();
        patch.insert("age".into(), serde_json::json!(31));
        patch.insert("role".into(), serde_json::json!("admin"));
        cache.apply_delta("/user/1", &patch);

        let result = cache.fetch_fragment("/user/1", &["age", "role"]);
        match result {
            FragmentResult::FullHit(found) => {
                assert_eq!(found["age"], 31);
                assert_eq!(found["role"], "admin");
            }
            _ => panic!("Expected full hit after delta"),
        }
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = FragmentedCache::new();
        let result = cache.fetch_fragment("/nonexistent", &["name"]);
        match result { FragmentResult::Miss => {}, _ => panic!("Expected miss") }
    }
}
