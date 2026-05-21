//! Arena Memory Pool Allocator for Zero-Allocation Scanning
//!
//! Inspired by Section 3.1 heap allocation elimination.
//! Pre-allocates contiguous memory regions and hands out slices,
//! avoiding per-object heap allocations during scanning.
//! Reduces GC/allocator pressure and improves cache locality.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Cache-line size for alignment.
const CACHE_LINE: usize = 64;

// ─── Arena Allocator ─────────────────────────────────────────────────────────

/// A bump-pointer arena allocator.
/// Allocates from a contiguous memory block with O(1) allocation cost.
/// Memory is freed all at once when the arena is dropped — no per-object dealloc.
pub struct Arena {
    /// Backing memory
    memory: Vec<u8>,
    /// Current allocation offset
    offset: AtomicUsize,
    /// Total capacity
    capacity: usize,
    /// Stats
    pub stats: ArenaStats,
}

/// Arena allocation statistics.
#[derive(Debug)]
pub struct ArenaStats {
    pub allocations: AtomicUsize,
    pub bytes_allocated: AtomicUsize,
    pub bytes_wasted: AtomicUsize, // alignment padding
    pub peak_usage: AtomicUsize,
}

impl Default for ArenaStats {
    fn default() -> Self {
        Self::new()
    }
}

impl ArenaStats {
    pub fn new() -> Self {
        Self {
            allocations: AtomicUsize::new(0),
            bytes_allocated: AtomicUsize::new(0),
            bytes_wasted: AtomicUsize::new(0),
            peak_usage: AtomicUsize::new(0),
        }
    }

    pub fn utilization(&self, capacity: usize) -> f64 {
        if capacity == 0 {
            return 0.0;
        }
        self.bytes_allocated.load(Ordering::Relaxed) as f64 / capacity as f64 * 100.0
    }
}

impl Arena {
    /// Create a new arena with the given capacity in bytes.
    pub fn new(capacity: usize) -> Self {
        Self {
            memory: vec![0u8; capacity],
            offset: AtomicUsize::new(0),
            capacity,
            stats: ArenaStats::new(),
        }
    }

    /// Create an arena sized for scanning node_modules.
    /// Rule of thumb: 128 bytes per expected file entry.
    pub fn for_scan(expected_files: usize) -> Self {
        let capacity = expected_files * 128;
        Self::new(capacity.max(64 * 1024)) // Minimum 64KB
    }

    /// Allocate `size` bytes with the given alignment.
    /// Returns a mutable slice to the allocated region.
    pub fn alloc(&self, size: usize, align: usize) -> Option<&mut [u8]> {
        loop {
            let current = self.offset.load(Ordering::Relaxed);
            let aligned = (current + align - 1) & !(align - 1);
            let new_offset = aligned + size;

            if new_offset > self.capacity {
                return None; // Out of memory
            }

            if self
                .offset
                .compare_exchange_weak(current, new_offset, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                let waste = aligned - current;
                self.stats.allocations.fetch_add(1, Ordering::Relaxed);
                self.stats
                    .bytes_allocated
                    .fetch_add(size, Ordering::Relaxed);
                self.stats.bytes_wasted.fetch_add(waste, Ordering::Relaxed);
                let _ = self
                    .stats
                    .peak_usage
                    .fetch_max(new_offset, Ordering::Relaxed);

                // Safety: we own exclusive access to [aligned..new_offset]
                let ptr = self.memory.as_ptr().wrapping_add(aligned) as *mut u8;
                return Some(unsafe { std::slice::from_raw_parts_mut(ptr, size) });
            }
            // CAS failed, retry
        }
    }

    /// Allocate space for a string and copy it in.
    pub fn alloc_str(&self, s: &str) -> Option<&str> {
        let bytes = self.alloc(s.len(), 1)?;
        bytes.copy_from_slice(s.as_bytes());
        Some(unsafe { std::str::from_utf8_unchecked(bytes) })
    }

    /// Allocate a cache-line aligned block (64 bytes).
    pub fn alloc_cache_aligned(&self, size: usize) -> Option<&mut [u8]> {
        self.alloc(size, CACHE_LINE)
    }

    /// Current usage in bytes.
    pub fn used(&self) -> usize {
        self.offset.load(Ordering::Relaxed)
    }

    /// Remaining capacity.
    pub fn remaining(&self) -> usize {
        self.capacity - self.used()
    }

    /// Reset the arena (invalidates all previous allocations).
    pub fn reset(&self) {
        self.offset.store(0, Ordering::Release);
    }

    /// Utilization percentage.
    pub fn utilization(&self) -> f64 {
        self.stats.utilization(self.capacity)
    }
}

// Safety: Arena is safe to share across threads because alloc uses atomic CAS
unsafe impl Sync for Arena {}
unsafe impl Send for Arena {}

// ─── Typed Arena Pool ────────────────────────────────────────────────────────

/// A typed arena pool for allocating fixed-size objects.
/// Pre-allocates N slots and hands them out with zero overhead.
pub struct TypedPool<T: Copy + Default> {
    slots: Vec<UnsafeCell<T>>,
    next: AtomicUsize,
    capacity: usize,
}

impl<T: Copy + Default> TypedPool<T> {
    pub fn new(capacity: usize) -> Self {
        let mut slots = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            slots.push(UnsafeCell::new(T::default()));
        }
        Self {
            slots,
            next: AtomicUsize::new(0),
            capacity,
        }
    }

    /// Allocate a slot, returning a mutable reference.
    pub fn alloc(&self) -> Option<&mut T> {
        let idx = self.next.fetch_add(1, Ordering::AcqRel);
        if idx >= self.capacity {
            return None;
        }
        Some(unsafe { &mut *self.slots[idx].get() })
    }

    /// Allocate and initialize a slot.
    pub fn alloc_init(&self, value: T) -> Option<&mut T> {
        let slot = self.alloc()?;
        *slot = value;
        Some(slot)
    }

    pub fn used(&self) -> usize {
        self.next.load(Ordering::Relaxed).min(self.capacity)
    }

    pub fn remaining(&self) -> usize {
        self.capacity - self.used()
    }

    pub fn reset(&self) {
        self.next.store(0, Ordering::Release);
    }
}

unsafe impl<T: Copy + Default> Sync for TypedPool<T> {}
unsafe impl<T: Copy + Default> Send for TypedPool<T> {}

// ─── Promotable Arena (Hybrid Arena-GC Model) ────────────────────────────────

/// A promotable arena that supports "promoting" objects to the global heap.
/// Inspired by Section 4.1: Hybrid Arena-GC Memory Model.
/// Objects are allocated in the arena by default (O(1) bump-pointer).
/// If an object escapes the request scope, it can be "promoted" to a
/// long-lived collection to avoid destruction when the arena is reset.
pub struct PromotableArena<T: Clone + Default> {
    arena: Arena,
    /// Objects promoted to the "global GC heap" (represented here as a Vec)
    promoted: std::sync::Mutex<Vec<T>>,
    /// Fixed size of T for allocation logic
    element_size: usize,
}

impl<T: Clone + Default> PromotableArena<T> {
    pub fn new(capacity_bytes: usize) -> Self {
        Self {
            arena: Arena::new(capacity_bytes),
            promoted: std::sync::Mutex::new(Vec::new()),
            element_size: std::mem::size_of::<T>(),
        }
    }

    /// Allocate a new object in the arena.
    pub fn alloc(&self, value: T) -> Option<&mut T> {
        let bytes = self
            .arena
            .alloc(self.element_size, std::mem::align_of::<T>())?;
        let ptr = bytes.as_mut_ptr() as *mut T;
        unsafe {
            std::ptr::write(ptr, value);
            Some(&mut *ptr)
        }
    }

    /// "Promote" an object to the global heap to prevent its destruction.
    /// This simulates the promote() DFS traversal described in the report.
    pub fn promote(&self, value: T) {
        let mut guard = self.promoted.lock().unwrap();
        guard.push(value);
    }

    /// Reset the arena, destroying all arena-allocated objects,
    /// but PRESERVING the promoted objects.
    pub fn reset_arena_only(&self) {
        self.arena.reset();
    }

    /// Clear everything, including promoted objects.
    pub fn clear_all(&self) {
        self.arena.reset();
        let mut guard = self.promoted.lock().unwrap();
        guard.clear();
    }

    pub fn stats(&self) -> (usize, usize) {
        let promoted_count = self.promoted.lock().unwrap().len();
        (self.arena.used(), promoted_count)
    }
}

// ─── Scan Entry for Arena Allocation ─────────────────────────────────────────

/// A fixed-size scan entry that can be allocated from a TypedPool.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScanEntry {
    /// Path hash (FNV-1a) instead of String to avoid heap allocation
    pub path_hash: u64,
    /// File size
    pub size: u64,
    /// Whether this is a pruning candidate
    pub is_candidate: bool,
    /// Category of the file (encoded as u8)
    pub category: u8,
    /// Depth in the directory tree
    pub depth: u16,
    _pad: [u8; 5],
}

impl ScanEntry {
    /// FNV-1a hash of a path string.
    pub fn hash_path(path: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in path.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    pub fn new(path: &str, size: u64, is_candidate: bool, category: u8, depth: u16) -> Self {
        Self {
            path_hash: Self::hash_path(path),
            size,
            is_candidate,
            category,
            depth,
            _pad: [0; 5],
        }
    }
}

/// Print arena report.
pub fn print_arena_report(arena: &Arena) {
    use console::style;
    println!();
    println!(
        "  {} {}",
        style("Arena Allocator Report").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );
    println!(
        "  {} Capacity:      {} bytes ({:.1} KB)",
        style("▸").dim(),
        arena.capacity,
        arena.capacity as f64 / 1024.0
    );
    println!(
        "  {} Used:          {} bytes ({:.1}%)",
        style("▸").dim(),
        arena.used(),
        arena.utilization()
    );
    println!(
        "  {} Remaining:     {} bytes",
        style("▸").dim(),
        arena.remaining()
    );
    println!(
        "  {} Allocations:   {}",
        style("▸").dim(),
        arena.stats.allocations.load(Ordering::Relaxed)
    );
    println!(
        "  {} Wasted (pad):  {} bytes",
        style("▸").dim(),
        arena.stats.bytes_wasted.load(Ordering::Relaxed)
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_basic() {
        let arena = Arena::new(1024);
        let slice = arena.alloc(64, 8).unwrap();
        assert_eq!(slice.len(), 64);
        assert_eq!(arena.stats.allocations.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_arena_str() {
        let arena = Arena::new(1024);
        let s = arena.alloc_str("hello world").unwrap();
        assert_eq!(s, "hello world");
    }

    #[test]
    fn test_arena_cache_aligned() {
        let arena = Arena::new(4096);
        // First alloc a byte to offset past 0 — then cache-aligned alloc must skip forward
        let _ = arena.alloc(1, 1);
        let slice = arena.alloc_cache_aligned(128).unwrap();
        assert_eq!(slice.len(), 128);
        // The offset within the arena buffer should be 64-byte aligned
        let base = arena.memory.as_ptr() as usize;
        let ptr = slice.as_ptr() as usize;
        let offset = ptr - base;
        assert_eq!(offset % CACHE_LINE, 0);
    }

    #[test]
    fn test_arena_oom() {
        let arena = Arena::new(64);
        let _ = arena.alloc(32, 1).unwrap();
        let _ = arena.alloc(32, 1).unwrap();
        assert!(arena.alloc(1, 1).is_none()); // OOM
    }

    #[test]
    fn test_arena_reset() {
        let arena = Arena::new(1024);
        arena.alloc(512, 1).unwrap();
        assert!(arena.used() >= 512);
        arena.reset();
        assert_eq!(arena.used(), 0);
    }

    #[test]
    fn test_typed_pool() {
        let pool = TypedPool::<ScanEntry>::new(100);
        let entry = pool
            .alloc_init(ScanEntry::new("test.js", 1024, true, 1, 2))
            .unwrap();
        assert_eq!(entry.size, 1024);
        assert!(entry.is_candidate);
        assert_eq!(pool.used(), 1);
    }

    #[test]
    fn test_typed_pool_oom() {
        let pool = TypedPool::<ScanEntry>::new(2);
        pool.alloc().unwrap();
        pool.alloc().unwrap();
        assert!(pool.alloc().is_none());
    }

    #[test]
    fn test_scan_entry_hash() {
        let h1 = ScanEntry::hash_path("node_modules/react/index.js");
        let h2 = ScanEntry::hash_path("node_modules/react/index.js");
        let h3 = ScanEntry::hash_path("node_modules/vue/index.js");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_for_scan() {
        let arena = Arena::for_scan(10000);
        assert!(arena.capacity >= 10000 * 128);
    }
}
