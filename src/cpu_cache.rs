//! CPU Cache Prefetching & Branch Prediction Optimization
//!
//! Hardware-level optimizations that exploit CPU microarchitecture:
//! - Software prefetch hints (PREFETCHNTA/T0/T1/T2) to pre-load
//!   data into L1/L2/L3 cache before it's needed
//! - Branch prediction hints via likely/unlikely annotations
//! - Cache-oblivious scanning algorithms that maximize spatial locality
//! - TLB optimization via huge page awareness

use std::sync::atomic::{AtomicU64, Ordering};

// ─── Cache Hierarchy Model ───────────────────────────────────────────────────

/// CPU cache hierarchy parameters (modern x86-64).
#[derive(Debug, Clone)]
pub struct CacheHierarchy {
    pub l1d_size: usize,       // L1 data cache (per core)
    pub l1d_latency_ns: f64,   // ~1ns / 4 cycles
    pub l1i_size: usize,       // L1 instruction cache
    pub l2_size: usize,        // L2 unified (per core)
    pub l2_latency_ns: f64,    // ~4ns / 12 cycles
    pub l3_size: usize,        // L3 shared (per socket)
    pub l3_latency_ns: f64,    // ~12ns / 36 cycles
    pub main_memory_ns: f64,   // ~100ns / 300 cycles
    pub cache_line_size: usize, // 64 bytes
    pub tlb_entries: usize,     // L1 dTLB entries
    pub page_size: usize,       // 4KB standard
    pub huge_page_size: usize,  // 2MB huge pages
}

impl Default for CacheHierarchy {
    fn default() -> Self {
        Self {
            l1d_size: 48 * 1024,          // 48KB (Intel 12th+ gen)
            l1d_latency_ns: 1.0,
            l1i_size: 32 * 1024,          // 32KB
            l2_size: 1280 * 1024,         // 1.25MB
            l2_latency_ns: 4.0,
            l3_size: 30 * 1024 * 1024,    // 30MB
            l3_latency_ns: 12.0,
            main_memory_ns: 100.0,
            cache_line_size: 64,
            tlb_entries: 64,
            page_size: 4096,
            huge_page_size: 2 * 1024 * 1024,
        }
    }
}

impl CacheHierarchy {
    /// Detect actual cache sizes from the OS.
    pub fn detect() -> Self {
        let mut h = Self::default();
        // Try to read from sysfs on Linux
        #[cfg(target_os = "linux")]
        {
            if let Ok(l1) = std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cache/index0/size") {
                if let Some(kb) = l1.trim().strip_suffix('K') {
                    if let Ok(n) = kb.parse::<usize>() { h.l1d_size = n * 1024; }
                }
            }
            if let Ok(l2) = std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cache/index2/size") {
                if let Some(kb) = l2.trim().strip_suffix('K') {
                    if let Ok(n) = kb.parse::<usize>() { h.l2_size = n * 1024; }
                } else if let Some(mb) = l2.trim().strip_suffix('M') {
                    if let Ok(n) = mb.parse::<usize>() { h.l2_size = n * 1024 * 1024; }
                }
            }
        }
        h
    }

    /// Calculate working set size that fits in each cache level.
    pub fn l1_items(&self, item_size: usize) -> usize {
        self.l1d_size / item_size
    }

    pub fn l2_items(&self, item_size: usize) -> usize {
        self.l2_size / item_size
    }

    pub fn l3_items(&self, item_size: usize) -> usize {
        self.l3_size / item_size
    }
}

// ─── Prefetch Operations ─────────────────────────────────────────────────────

/// Prefetch temporal locality hint.
#[derive(Debug, Clone, Copy)]
pub enum PrefetchHint {
    /// Non-temporal: read once, don't pollute cache (streaming)
    NonTemporal,
    /// T0: Prefetch into L1 (will be read very soon)
    L1,
    /// T1: Prefetch into L2 (will be read soon)
    L2,
    /// T2: Prefetch into L3 (will be read eventually)
    L3,
}

/// Software prefetch a memory address.
/// On x86-64, this maps to PREFETCHNTA/T0/T1/T2 instructions.
#[inline(always)]
pub fn prefetch<T>(ptr: *const T, hint: PrefetchHint) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        match hint {
            PrefetchHint::NonTemporal => {
                std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_NTA);
            }
            PrefetchHint::L1 => {
                std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T0);
            }
            PrefetchHint::L2 => {
                std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T1);
            }
            PrefetchHint::L3 => {
                std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T2);
            }
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (ptr, hint); // Prefetch is a no-op on unsupported architectures
    }
}

/// Prefetch the next N cache lines ahead of the current pointer.
#[inline(always)]
pub fn prefetch_ahead<T>(slice: &[T], current_idx: usize, ahead: usize) {
    let target_idx = current_idx + ahead;
    if target_idx < slice.len() {
        prefetch(&slice[target_idx] as *const T, PrefetchHint::L1);
    }
}

// ─── Branch Prediction Hints ─────────────────────────────────────────────────

/// Hint to the compiler/CPU that a condition is likely true.
#[inline(always)]
pub fn likely(b: bool) -> bool {
    if !b { std::hint::black_box(false) } else { true }
}

/// Hint to the compiler/CPU that a condition is unlikely true.
#[inline(always)]
pub fn unlikely(b: bool) -> bool {
    if b { std::hint::black_box(true) } else { false }
}

// ─── Cache-Oblivious Scanner ─────────────────────────────────────────────────

/// Performance stats for cache-optimized operations.
#[derive(Debug)]
pub struct CacheOptStats {
    pub items_processed: AtomicU64,
    pub cache_line_accesses: AtomicU64,
    pub prefetches_issued: AtomicU64,
    pub estimated_l1_hits: AtomicU64,
    pub estimated_l2_hits: AtomicU64,
    pub estimated_l3_hits: AtomicU64,
    pub estimated_main_mem: AtomicU64,
}

impl CacheOptStats {
    pub fn new() -> Self {
        Self {
            items_processed: AtomicU64::new(0),
            cache_line_accesses: AtomicU64::new(0),
            prefetches_issued: AtomicU64::new(0),
            estimated_l1_hits: AtomicU64::new(0),
            estimated_l2_hits: AtomicU64::new(0),
            estimated_l3_hits: AtomicU64::new(0),
            estimated_main_mem: AtomicU64::new(0),
        }
    }

    /// Estimate total access latency in nanoseconds.
    pub fn estimated_latency_ns(&self, hierarchy: &CacheHierarchy) -> f64 {
        let l1 = self.estimated_l1_hits.load(Ordering::Relaxed) as f64 * hierarchy.l1d_latency_ns;
        let l2 = self.estimated_l2_hits.load(Ordering::Relaxed) as f64 * hierarchy.l2_latency_ns;
        let l3 = self.estimated_l3_hits.load(Ordering::Relaxed) as f64 * hierarchy.l3_latency_ns;
        let mm = self.estimated_main_mem.load(Ordering::Relaxed) as f64 * hierarchy.main_memory_ns;
        l1 + l2 + l3 + mm
    }
}

/// Scan a slice with cache-optimized prefetching.
/// Prefetches PREFETCH_DISTANCE items ahead to hide memory latency.
pub fn scan_with_prefetch<T, F>(data: &[T], prefetch_distance: usize, mut process: F) -> CacheOptStats
where
    F: FnMut(&T) -> bool,
{
    let stats = CacheOptStats::new();
    let hierarchy = CacheHierarchy::default();
    let item_size = std::mem::size_of::<T>();
    let items_per_line = (hierarchy.cache_line_size / item_size).max(1);

    for i in 0..data.len() {
        // Prefetch ahead
        if i + prefetch_distance < data.len() {
            prefetch(&data[i + prefetch_distance] as *const T, PrefetchHint::L1);
            stats.prefetches_issued.fetch_add(1, Ordering::Relaxed);
        }

        // Process current item
        let _result = process(&data[i]);
        stats.items_processed.fetch_add(1, Ordering::Relaxed);

        // Track cache line access
        if i % items_per_line == 0 {
            stats.cache_line_accesses.fetch_add(1, Ordering::Relaxed);
        }

        // Estimate cache level (heuristic: sequential access = mostly L1 hits)
        if i < hierarchy.l1_items(item_size) {
            stats.estimated_l1_hits.fetch_add(1, Ordering::Relaxed);
        } else if i < hierarchy.l2_items(item_size) {
            stats.estimated_l2_hits.fetch_add(1, Ordering::Relaxed);
        } else if i < hierarchy.l3_items(item_size) {
            stats.estimated_l3_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            stats.estimated_main_mem.fetch_add(1, Ordering::Relaxed);
        }
    }

    stats
}

// ─── TLB Optimization ────────────────────────────────────────────────────────

/// TLB (Translation Lookaside Buffer) optimization info.
#[derive(Debug, Clone)]
pub struct TlbInfo {
    /// Number of 4KB pages in working set
    pub pages_4k: usize,
    /// Number of 2MB huge pages needed
    pub pages_2m: usize,
    /// Whether huge pages would help
    pub recommend_huge_pages: bool,
    /// TLB coverage with standard pages
    pub tlb_coverage_standard: f64,
    /// TLB coverage with huge pages
    pub tlb_coverage_huge: f64,
}

/// Calculate TLB optimization recommendations for a working set size.
pub fn analyze_tlb(working_set_bytes: usize) -> TlbInfo {
    let hierarchy = CacheHierarchy::default();
    let pages_4k = (working_set_bytes + hierarchy.page_size - 1) / hierarchy.page_size;
    let pages_2m = (working_set_bytes + hierarchy.huge_page_size - 1) / hierarchy.huge_page_size;

    let standard_coverage = (hierarchy.tlb_entries * hierarchy.page_size) as f64 / working_set_bytes as f64;
    let huge_coverage = (hierarchy.tlb_entries * hierarchy.huge_page_size) as f64 / working_set_bytes as f64;

    TlbInfo {
        pages_4k,
        pages_2m,
        recommend_huge_pages: pages_4k > hierarchy.tlb_entries,
        tlb_coverage_standard: standard_coverage.min(1.0) * 100.0,
        tlb_coverage_huge: huge_coverage.min(1.0) * 100.0,
    }
}

/// Print cache optimization report.
pub fn print_cache_report(hierarchy: &CacheHierarchy, stats: &CacheOptStats) {
    use console::style;
    println!();
    println!("  {} {}", style("CPU Cache Optimization Report").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
    println!("  {} L1D: {}KB | L2: {}KB | L3: {}MB | Line: {}B",
        style("🖥️").dim(),
        hierarchy.l1d_size / 1024,
        hierarchy.l2_size / 1024,
        hierarchy.l3_size / 1024 / 1024,
        hierarchy.cache_line_size);
    println!("  {} Items processed:   {}",
        style("▸").dim(), stats.items_processed.load(Ordering::Relaxed));
    println!("  {} Prefetches issued: {}",
        style("⚡").yellow(), stats.prefetches_issued.load(Ordering::Relaxed));
    println!("  {} Cache line reads:  {}",
        style("▸").dim(), stats.cache_line_accesses.load(Ordering::Relaxed));
    println!("  {} Est. L1 hits: {} | L2: {} | L3: {} | Main mem: {}",
        style("▸").dim(),
        stats.estimated_l1_hits.load(Ordering::Relaxed),
        stats.estimated_l2_hits.load(Ordering::Relaxed),
        stats.estimated_l3_hits.load(Ordering::Relaxed),
        stats.estimated_main_mem.load(Ordering::Relaxed));
    println!("  {} Est. total latency: {:.1} µs",
        style("⚡").yellow(), stats.estimated_latency_ns(hierarchy) / 1000.0);
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hierarchy_default() {
        let h = CacheHierarchy::default();
        assert_eq!(h.cache_line_size, 64);
        assert!(h.l1d_size > 0);
        assert!(h.l2_size > h.l1d_size);
        assert!(h.l3_size > h.l2_size);
    }

    #[test]
    fn test_scan_with_prefetch() {
        let data: Vec<u64> = (0..10000).collect();
        let stats = scan_with_prefetch(&data, 8, |&x| x % 2 == 0);
        assert_eq!(stats.items_processed.load(Ordering::Relaxed), 10000);
        assert!(stats.prefetches_issued.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn test_tlb_analysis() {
        let info = analyze_tlb(100 * 1024 * 1024); // 100MB
        assert!(info.recommend_huge_pages);
        assert!(info.tlb_coverage_huge > info.tlb_coverage_standard);
    }

    #[test]
    fn test_cache_items() {
        let h = CacheHierarchy::default();
        assert!(h.l1_items(64) > 0);
        assert!(h.l2_items(64) > h.l1_items(64));
    }
}
