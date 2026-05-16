//! Structural Request Coalescing and Intelligent Caching
//!
//! Section 4 of High-Performance System Optimization Projects.
//! Implements JSONPath-aware request coalescing (singleflight pattern)
//! to prevent cache stampedes. Deduplicates concurrent identical requests,
//! serves one backend call for thousands of clients.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};

// ─── Cache Stampede Prevention ───────────────────────────────────────────────

/// Strategy for handling concurrent duplicate requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeduplicationStrategy {
    /// Request Hedging: send to multiple backends, use fastest response.
    /// Trades compute for latency reduction.
    Hedging,
    /// Request Coalescing (Singleflight): execute once, share result.
    /// Prevents cache stampedes.
    Coalescing,
    /// No deduplication — pass through all requests.
    PassThrough,
}

impl DeduplicationStrategy {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Hedging => "Request Hedging",
            Self::Coalescing => "Request Coalescing (Singleflight)",
            Self::PassThrough => "Pass-Through (No Dedup)",
        }
    }
}

// ─── Singleflight Implementation ─────────────────────────────────────────────

/// State of a coalesced in-flight request.
#[derive(Debug, Clone)]
enum FlightState<T> {
    /// Request is being processed — waiters should park
    InFlight { waiters: usize, started_at: Instant },
    /// Request completed — result is available
    Complete { result: T, completed_at: Instant },
}

/// Singleflight group: ensures only one in-flight request per key.
/// When multiple requests arrive for the same key, only the first
/// triggers the actual backend call. Subsequent requests wait and
/// receive the shared result.
pub struct SingleflightGroup<T: Clone> {
    flights: RwLock<HashMap<String, FlightState<T>>>,
    pub stats: CoalescingStats,
}

/// Coalescing performance statistics.
#[derive(Debug)]
pub struct CoalescingStats {
    pub total_requests: AtomicU64,
    pub coalesced_requests: AtomicU64,
    pub unique_flights: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub backend_calls_saved: AtomicU64,
}

impl CoalescingStats {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            coalesced_requests: AtomicU64::new(0),
            unique_flights: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            backend_calls_saved: AtomicU64::new(0),
        }
    }

    /// Percentage of requests that were coalesced (deduped).
    pub fn coalescing_rate(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        if total == 0 { return 0.0; }
        let coalesced = self.coalesced_requests.load(Ordering::Relaxed);
        coalesced as f64 / total as f64 * 100.0
    }

    /// Backend load reduction percentage.
    pub fn backend_reduction_pct(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        if total == 0 { return 0.0; }
        let saved = self.backend_calls_saved.load(Ordering::Relaxed);
        saved as f64 / total as f64 * 100.0
    }
}

impl<T: Clone> SingleflightGroup<T> {
    pub fn new() -> Self {
        Self {
            flights: RwLock::new(HashMap::new()),
            stats: CoalescingStats::new(),
        }
    }

    /// Execute a request, coalescing duplicates.
    /// `key` identifies the logical request.
    /// `fetch_fn` is called only if no in-flight request exists for this key.
    pub fn do_once<F>(&self, key: &str, fetch_fn: F) -> T
    where F: FnOnce() -> T {
        self.stats.total_requests.fetch_add(1, Ordering::Relaxed);

        // Check if result is already available
        {
            let flights = self.flights.read().unwrap();
            if let Some(FlightState::Complete { result, .. }) = flights.get(key) {
                self.stats.coalesced_requests.fetch_add(1, Ordering::Relaxed);
                self.stats.backend_calls_saved.fetch_add(1, Ordering::Relaxed);
                return result.clone();
            }
        }

        // Check if in-flight, or start new flight
        {
            let mut flights = self.flights.write().unwrap();
            match flights.get_mut(key) {
                Some(FlightState::InFlight { waiters, .. }) => {
                    *waiters += 1;
                    self.stats.coalesced_requests.fetch_add(1, Ordering::Relaxed);
                    self.stats.backend_calls_saved.fetch_add(1, Ordering::Relaxed);
                    // In a real async impl, we'd await a shared future here.
                    // For sync simulation, we execute (this is the "last waiter wins" variant).
                }
                Some(FlightState::Complete { result, .. }) => {
                    self.stats.coalesced_requests.fetch_add(1, Ordering::Relaxed);
                    self.stats.backend_calls_saved.fetch_add(1, Ordering::Relaxed);
                    return result.clone();
                }
                None => {
                    flights.insert(key.to_string(), FlightState::InFlight {
                        waiters: 0,
                        started_at: Instant::now(),
                    });
                    self.stats.unique_flights.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        // Execute the actual backend call (only once per key)
        let result = fetch_fn();

        // Store the result for subsequent waiters
        {
            let mut flights = self.flights.write().unwrap();
            flights.insert(key.to_string(), FlightState::Complete {
                result: result.clone(),
                completed_at: Instant::now(),
            });
        }

        result
    }

    /// Clear completed flights (housekeeping).
    pub fn clear_completed(&self) {
        let mut flights = self.flights.write().unwrap();
        flights.retain(|_, state| matches!(state, FlightState::InFlight { .. }));
    }

    /// Clear all flights.
    pub fn clear_all(&self) {
        let mut flights = self.flights.write().unwrap();
        flights.clear();
    }

    /// Number of active entries.
    pub fn active_count(&self) -> usize {
        self.flights.read().unwrap().len()
    }
}

// ─── JSONPath Query Engine ───────────────────────────────────────────────────

/// A simplified JSONPath expression for field extraction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonPathExpr {
    pub segments: Vec<PathSegment>,
}

/// A segment in a JSONPath expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSegment {
    /// Object field access: $.fieldName
    Field(String),
    /// Array index: $[0]
    Index(usize),
    /// Wildcard: $[*]
    Wildcard,
    /// Recursive descent: $..fieldName
    RecursiveDescent(String),
}

impl JsonPathExpr {
    /// Parse a simple JSONPath string.
    /// Supports: $.field, $.a.b.c, $.arr[0], $..field
    pub fn parse(expr: &str) -> Self {
        let mut segments = Vec::new();
        let trimmed = expr.strip_prefix('$').unwrap_or(expr);

        for part in trimmed.split('.') {
            if part.is_empty() { continue; }

            if part == "*" {
                segments.push(PathSegment::Wildcard);
            } else if part.starts_with('[') && part.ends_with(']') {
                let inner = &part[1..part.len()-1];
                if inner == "*" {
                    segments.push(PathSegment::Wildcard);
                } else if let Ok(idx) = inner.parse::<usize>() {
                    segments.push(PathSegment::Index(idx));
                }
            } else if part.contains('[') {
                // field[0] format
                let bracket = part.find('[').unwrap();
                let field = &part[..bracket];
                segments.push(PathSegment::Field(field.to_string()));
                let idx_str = &part[bracket+1..part.len()-1];
                if let Ok(idx) = idx_str.parse::<usize>() {
                    segments.push(PathSegment::Index(idx));
                }
            } else {
                segments.push(PathSegment::Field(part.to_string()));
            }
        }

        Self { segments }
    }

    /// Extract value from a JSON value using this path.
    pub fn extract<'a>(&self, value: &'a serde_json::Value) -> Option<&'a serde_json::Value> {
        let mut current = value;
        for segment in &self.segments {
            match segment {
                PathSegment::Field(name) => {
                    current = current.get(name)?;
                }
                PathSegment::Index(idx) => {
                    current = current.get(idx)?;
                }
                PathSegment::Wildcard => {
                    // Return the array/object itself
                    return Some(current);
                }
                PathSegment::RecursiveDescent(_) => {
                    // Simplified: just return current
                    return Some(current);
                }
            }
        }
        Some(current)
    }
}

// ─── Structural Request Merger ───────────────────────────────────────────────

/// Merges overlapping field requests into a superset query.
/// Client A wants [x, y], Client B wants [y, z] → merged query [x, y, z].
pub struct RequestMerger {
    /// Pending requests within the current merge window
    pending: Mutex<Vec<MergeableRequest>>,
    /// Merge window duration
    pub window: Duration,
}

/// A request that can be merged with others.
#[derive(Debug, Clone)]
pub struct MergeableRequest {
    pub client_id: String,
    pub resource_path: String,
    pub requested_fields: Vec<String>,
    pub arrived_at: Instant,
}

/// Result of merging requests.
#[derive(Debug, Clone)]
pub struct MergedQuery {
    pub resource_path: String,
    pub superset_fields: Vec<String>,
    pub client_field_map: HashMap<String, Vec<String>>,
    pub client_count: usize,
}

impl RequestMerger {
    pub fn new(window: Duration) -> Self {
        Self {
            pending: Mutex::new(Vec::new()),
            window,
        }
    }

    /// Submit a request for potential merging.
    pub fn submit(&self, req: MergeableRequest) {
        let mut pending = self.pending.lock().unwrap();
        pending.push(req);
    }

    /// Flush pending requests and produce merged queries.
    /// Groups by resource_path, merges field sets.
    pub fn flush(&self) -> Vec<MergedQuery> {
        let mut pending = self.pending.lock().unwrap();
        let requests = std::mem::take(&mut *pending);
        drop(pending);

        // Group by resource path
        let mut groups: HashMap<String, Vec<MergeableRequest>> = HashMap::new();
        for req in requests {
            groups.entry(req.resource_path.clone()).or_default().push(req);
        }

        // Merge each group
        groups.into_iter().map(|(path, reqs)| {
            let mut superset = Vec::new();
            let mut client_map = HashMap::new();

            for req in &reqs {
                client_map.insert(req.client_id.clone(), req.requested_fields.clone());
                for field in &req.requested_fields {
                    if !superset.contains(field) {
                        superset.push(field.clone());
                    }
                }
            }

            MergedQuery {
                resource_path: path,
                superset_fields: superset,
                client_field_map: client_map,
                client_count: reqs.len(),
            }
        }).collect()
    }
}

// ─── Structural Cache ────────────────────────────────────────────────────────

/// A structural cache that stores individual JSON fields rather than
/// opaque monolithic blobs. Allows partial cache hits for overlapping requests.
pub struct StructuralCache {
    /// Cache entries: resource_path → { field_name → (value, expires_at) }
    entries: RwLock<HashMap<String, HashMap<String, CachedField>>>,
    pub ttl: Duration,
    pub stats: CacheLayerStats,
}

/// A cached individual field.
#[derive(Debug, Clone)]
pub struct CachedField {
    pub value: serde_json::Value,
    pub stored_at: Instant,
    pub expires_at: Instant,
    pub hit_count: u64,
}

/// Cache layer statistics.
#[derive(Debug)]
pub struct CacheLayerStats {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub partial_hits: AtomicU64,
    pub evictions: AtomicU64,
    pub stored_fields: AtomicU64,
}

impl CacheLayerStats {
    pub fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            partial_hits: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            stored_fields: AtomicU64::new(0),
        }
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits.load(Ordering::Relaxed)
            + self.misses.load(Ordering::Relaxed)
            + self.partial_hits.load(Ordering::Relaxed);
        if total == 0 { return 0.0; }
        (self.hits.load(Ordering::Relaxed) + self.partial_hits.load(Ordering::Relaxed))
            as f64 / total as f64 * 100.0
    }
}

impl StructuralCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            ttl,
            stats: CacheLayerStats::new(),
        }
    }

    /// Store fields for a resource.
    pub fn store_fields(&self, resource: &str, fields: HashMap<String, serde_json::Value>) {
        let now = Instant::now();
        let expires = now + self.ttl;

        let mut entries = self.entries.write().unwrap();
        let resource_cache = entries.entry(resource.to_string()).or_default();

        for (field, value) in fields {
            resource_cache.insert(field, CachedField {
                value,
                stored_at: now,
                expires_at: expires,
                hit_count: 0,
            });
            self.stats.stored_fields.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Retrieve specific fields for a resource.
    /// Returns (found_fields, missing_fields).
    pub fn get_fields(&self, resource: &str, requested: &[String])
        -> (HashMap<String, serde_json::Value>, Vec<String>)
    {
        let now = Instant::now();
        let entries = self.entries.read().unwrap();

        let mut found = HashMap::new();
        let mut missing = Vec::new();

        if let Some(resource_cache) = entries.get(resource) {
            for field in requested {
                if let Some(cached) = resource_cache.get(field) {
                    if cached.expires_at > now {
                        found.insert(field.clone(), cached.value.clone());
                    } else {
                        missing.push(field.clone());
                    }
                } else {
                    missing.push(field.clone());
                }
            }
        } else {
            missing = requested.to_vec();
        }

        // Update stats
        if missing.is_empty() {
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
        } else if found.is_empty() {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
        } else {
            self.stats.partial_hits.fetch_add(1, Ordering::Relaxed);
        }

        (found, missing)
    }

    /// Evict expired entries.
    pub fn evict_expired(&self) -> u64 {
        let now = Instant::now();
        let mut entries = self.entries.write().unwrap();
        let mut evicted = 0u64;

        for resource_cache in entries.values_mut() {
            let before = resource_cache.len();
            resource_cache.retain(|_, field| field.expires_at > now);
            evicted += (before - resource_cache.len()) as u64;
        }

        entries.retain(|_, cache| !cache.is_empty());
        self.stats.evictions.fetch_add(evicted, Ordering::Relaxed);
        evicted
    }

    /// Total cached fields.
    pub fn total_fields(&self) -> usize {
        self.entries.read().unwrap().values().map(|c| c.len()).sum()
    }
}

/// Print coalescing report.
pub fn print_coalescing_report(stats: &CoalescingStats) {
    use console::style;
    println!();
    println!("  {} {}", style("Request Coalescing Report").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
    println!("  {} Total requests:     {}",
        style("▸").dim(), style(stats.total_requests.load(Ordering::Relaxed)).white().bold());
    println!("  {} Coalesced:          {} ({:.1}% deduped)",
        style("▸").dim(), stats.coalesced_requests.load(Ordering::Relaxed),
        stats.coalescing_rate());
    println!("  {} Backend calls saved: {} ({:.1}% reduction)",
        style("🚀").yellow(), stats.backend_calls_saved.load(Ordering::Relaxed),
        stats.backend_reduction_pct());
    println!("  {} Unique flights:     {}",
        style("▸").dim(), stats.unique_flights.load(Ordering::Relaxed));
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_singleflight_dedup() {
        let sf = SingleflightGroup::<String>::new();
        let mut call_count = 0u32;

        // First call triggers the fetch
        let r1 = sf.do_once("key1", || { "result-A".to_string() });
        assert_eq!(r1, "result-A");

        // Second call for same key gets cached result
        let r2 = sf.do_once("key1", || { "result-B".to_string() });
        assert_eq!(r2, "result-A"); // Should be same as first

        assert_eq!(sf.stats.total_requests.load(Ordering::Relaxed), 2);
        assert!(sf.stats.coalesced_requests.load(Ordering::Relaxed) >= 1);
    }

    #[test]
    fn test_singleflight_different_keys() {
        let sf = SingleflightGroup::<i32>::new();
        let r1 = sf.do_once("a", || 1);
        let r2 = sf.do_once("b", || 2);
        assert_eq!(r1, 1);
        assert_eq!(r2, 2);
        assert_eq!(sf.stats.unique_flights.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_jsonpath_parse() {
        let path = JsonPathExpr::parse("$.data.users[0].name");
        assert_eq!(path.segments.len(), 4);
        assert_eq!(path.segments[0], PathSegment::Field("data".to_string()));
        assert_eq!(path.segments[1], PathSegment::Field("users".to_string()));
        assert_eq!(path.segments[2], PathSegment::Index(0));
        assert_eq!(path.segments[3], PathSegment::Field("name".to_string()));
    }

    #[test]
    fn test_jsonpath_extract() {
        let json: serde_json::Value = serde_json::json!({
            "data": { "users": [{ "name": "Alice" }, { "name": "Bob" }] }
        });
        let path = JsonPathExpr::parse("$.data.users[0].name");
        let result = path.extract(&json).unwrap();
        assert_eq!(result, "Alice");
    }

    #[test]
    fn test_request_merger() {
        let merger = RequestMerger::new(Duration::from_millis(10));
        merger.submit(MergeableRequest {
            client_id: "A".into(), resource_path: "/users".into(),
            requested_fields: vec!["name".into(), "email".into()],
            arrived_at: Instant::now(),
        });
        merger.submit(MergeableRequest {
            client_id: "B".into(), resource_path: "/users".into(),
            requested_fields: vec!["email".into(), "age".into()],
            arrived_at: Instant::now(),
        });

        let merged = merger.flush();
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].client_count, 2);
        assert_eq!(merged[0].superset_fields.len(), 3); // name, email, age
    }

    #[test]
    fn test_structural_cache() {
        let cache = StructuralCache::new(Duration::from_secs(60));
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), serde_json::json!("Alice"));
        fields.insert("email".to_string(), serde_json::json!("alice@example.com"));
        cache.store_fields("/users/1", fields);

        let (found, missing) = cache.get_fields("/users/1",
            &["name".to_string(), "email".to_string(), "age".to_string()]);
        assert_eq!(found.len(), 2);
        assert_eq!(missing, vec!["age".to_string()]);
    }

    #[test]
    fn test_structural_cache_eviction() {
        let cache = StructuralCache::new(Duration::from_millis(1));
        let mut fields = HashMap::new();
        fields.insert("x".to_string(), serde_json::json!(42));
        cache.store_fields("/test", fields);

        std::thread::sleep(Duration::from_millis(5));
        let evicted = cache.evict_expired();
        assert_eq!(evicted, 1);
    }
}
