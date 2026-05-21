//! Custom arena allocator for scan-time memory management.
//!
//! Provides:
//!   - Bump allocator for fast, allocation-free string interning
//!   - Object pool for PruneCandidate recycling
//!   - Memory-efficient path storage with deduplication
//!   - Slab allocator for fixed-size scan objects

use crate::rules::FileCategory;
use bumpalo::Bump;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

// ─── String Interner ─────────────────────────────────────────────────────────

/// A string interner that deduplicates string allocations.
///
/// Common strings in node_modules (package names, extensions, categories)
/// are repeated thousands of times. Interning reduces memory pressure
/// from O(n * avg_len) to O(unique * avg_len).
pub struct ScanArena {
    bump: Bump,
}

impl ScanArena {
    pub fn new() -> Self {
        Self { bump: Bump::new() }
    }

    pub fn alloc_str<'a>(&'a self, s: &str) -> &'a str {
        self.bump.alloc_str(s)
    }
}

pub struct StringInterner {
    strings: Vec<String>,
    lookup: HashMap<String, u32>,
}

impl StringInterner {
    /// Create a new interner with pre-seeded common strings.
    pub fn new() -> Self {
        let mut interner = Self {
            strings: Vec::with_capacity(1024),
            lookup: HashMap::with_capacity(1024),
        };

        // Pre-seed common strings to avoid first-access allocation
        let common = [
            "node_modules",
            "package.json",
            "README.md",
            "LICENSE",
            "index.js",
            "index.mjs",
            "index.cjs",
            "index.d.ts",
            ".js",
            ".ts",
            ".mjs",
            ".cjs",
            ".json",
            ".css",
            ".map",
            ".md",
            ".txt",
            ".yml",
            ".yaml",
            ".toml",
            "MIT",
            "ISC",
            "BSD-2-Clause",
            "BSD-3-Clause",
            "Apache-2.0",
            "lodash",
            "react",
            "express",
            "webpack",
            "babel",
            "test",
            "tests",
            "__tests__",
            "spec",
            "example",
            "examples",
            "docs",
            "doc",
            "benchmark",
            "benchmarks",
            "src",
            "lib",
            "dist",
            "build",
            "out",
        ];

        for s in common {
            interner.intern(s);
        }

        interner
    }

    /// Intern a string, returning its index.
    pub fn intern(&mut self, s: &str) -> u32 {
        if let Some(&idx) = self.lookup.get(s) {
            return idx;
        }

        let idx = self.strings.len() as u32;
        let owned = s.to_string();
        self.lookup.insert(owned.clone(), idx);
        self.strings.push(owned);
        idx
    }

    /// Resolve an interned index back to a string.
    pub fn resolve(&self, idx: u32) -> &str {
        &self.strings[idx as usize]
    }

    /// Get the number of unique interned strings.
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if the interner is empty.
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Total memory used by interned strings (approximate).
    pub fn memory_usage(&self) -> usize {
        self.strings
            .iter()
            .map(|s| s.len() + std::mem::size_of::<String>())
            .sum::<usize>()
            + self.lookup.capacity() * (std::mem::size_of::<String>() + std::mem::size_of::<u32>())
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Path Interner ───────────────────────────────────────────────────────────

/// Deduplicates common path components to reduce PathBuf allocations.
///
/// In a typical node_modules, the prefix paths (e.g., "node_modules/lodash/")
/// are shared across many files. This interner stores each unique prefix once.
pub struct PathInterner {
    components: Vec<String>,
    component_lookup: HashMap<String, u32>,
}

impl PathInterner {
    pub fn new() -> Self {
        Self {
            components: Vec::with_capacity(256),
            component_lookup: HashMap::with_capacity(256),
        }
    }

    /// Intern a path component.
    fn intern_component(&mut self, s: &str) -> u32 {
        if let Some(&idx) = self.component_lookup.get(s) {
            return idx;
        }
        let idx = self.components.len() as u32;
        let owned = s.to_string();
        self.component_lookup.insert(owned.clone(), idx);
        self.components.push(owned);
        idx
    }

    /// Intern a full path as a sequence of component indices.
    pub fn intern_path(&mut self, path: &Path) -> InternedPath {
        let indices: Vec<u32> = path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .map(|s| self.intern_component(s))
            .collect();
        InternedPath {
            components: indices,
        }
    }

    /// Reconstruct a PathBuf from interned components.
    pub fn resolve_path(&self, interned: &InternedPath) -> PathBuf {
        let mut path = PathBuf::new();
        for &idx in &interned.components {
            path.push(&self.components[idx as usize]);
        }
        path
    }

    /// Number of unique components.
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Estimated memory savings (bytes) compared to storing N full PathBufs.
    pub fn estimated_savings(&self, total_paths: usize, avg_path_len: usize) -> usize {
        let naive = total_paths * avg_path_len;
        let interned = self.components.iter().map(|s| s.len()).sum::<usize>()
            + total_paths * self.components.len().max(1) * std::mem::size_of::<u32>();
        naive.saturating_sub(interned)
    }
}

impl Default for PathInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// A path stored as a sequence of interned component indices.
#[derive(Debug, Clone)]
pub struct InternedPath {
    pub components: Vec<u32>,
}

// ─── Object Pool ─────────────────────────────────────────────────────────────

/// A generic object pool for reusing heap allocations.
///
/// Reduces allocation pressure during scanning by recycling
/// Vec, String, and other heap objects.
pub struct ObjectPool<T> {
    pool: RefCell<Vec<T>>,
    created: AtomicU64,
    recycled: AtomicU64,
}

impl<T> ObjectPool<T> {
    /// Create a new empty pool.
    pub fn new() -> Self {
        Self {
            pool: RefCell::new(Vec::new()),
            created: AtomicU64::new(0),
            recycled: AtomicU64::new(0),
        }
    }

    /// Create a pool with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self
    where
        T: Default,
    {
        let mut items = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            items.push(T::default());
        }
        Self {
            pool: RefCell::new(items),
            created: AtomicU64::new(capacity as u64),
            recycled: AtomicU64::new(0),
        }
    }

    /// Get an object from the pool, or create a new one.
    pub fn get(&self) -> T
    where
        T: Default,
    {
        let mut pool = self.pool.borrow_mut();
        if let Some(item) = pool.pop() {
            self.recycled.fetch_add(1, Ordering::Relaxed);
            item
        } else {
            self.created.fetch_add(1, Ordering::Relaxed);
            T::default()
        }
    }

    /// Return an object to the pool for reuse.
    pub fn put(&self, item: T) {
        self.pool.borrow_mut().push(item);
    }

    /// Pool utilization stats.
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            pool_size: self.pool.borrow().len(),
            total_created: self.created.load(Ordering::Relaxed),
            total_recycled: self.recycled.load(Ordering::Relaxed),
        }
    }
}

/// Pool utilization statistics.
#[derive(Debug)]
pub struct PoolStats {
    pub pool_size: usize,
    pub total_created: u64,
    pub total_recycled: u64,
}

impl PoolStats {
    /// Recycling efficiency (0.0 to 1.0).
    pub fn efficiency(&self) -> f64 {
        let total = self.total_created + self.total_recycled;
        if total == 0 {
            return 0.0;
        }
        self.total_recycled as f64 / total as f64
    }
}

// ─── Slab Allocator ──────────────────────────────────────────────────────────

/// A slab allocator for fixed-size objects.
///
/// Allocates objects in contiguous chunks (slabs) to improve cache locality
/// and reduce allocation overhead for many small objects of the same type.
pub struct SlabAllocator<T> {
    slabs: Vec<Vec<T>>,
    slab_capacity: usize,
    total_allocated: usize,
}

impl<T> SlabAllocator<T> {
    /// Create a new slab allocator.
    ///
    /// `slab_size` is the number of objects per slab.
    pub fn new(slab_size: usize) -> Self {
        Self {
            slabs: Vec::new(),
            slab_capacity: slab_size,
            total_allocated: 0,
        }
    }

    /// Allocate a new object in the slab.
    pub fn alloc(&mut self, value: T) -> SlabRef {
        // Check if current slab has room
        if self.slabs.is_empty() || self.slabs.last().unwrap().len() >= self.slab_capacity {
            self.slabs.push(Vec::with_capacity(self.slab_capacity));
        }

        let slab_idx = self.slabs.len() - 1;
        let item_idx = self.slabs[slab_idx].len();
        self.slabs[slab_idx].push(value);
        self.total_allocated += 1;

        SlabRef {
            slab: slab_idx as u32,
            index: item_idx as u32,
        }
    }

    /// Get a reference to an allocated object.
    pub fn get(&self, sref: &SlabRef) -> Option<&T> {
        self.slabs
            .get(sref.slab as usize)
            .and_then(|slab| slab.get(sref.index as usize))
    }

    /// Get a mutable reference to an allocated object.
    pub fn get_mut(&mut self, sref: &SlabRef) -> Option<&mut T> {
        self.slabs
            .get_mut(sref.slab as usize)
            .and_then(|slab| slab.get_mut(sref.index as usize))
    }

    /// Total number of allocated objects.
    pub fn len(&self) -> usize {
        self.total_allocated
    }

    /// Is the allocator empty?
    pub fn is_empty(&self) -> bool {
        self.total_allocated == 0
    }

    /// Number of slabs allocated.
    pub fn slab_count(&self) -> usize {
        self.slabs.len()
    }

    /// Iterate over all allocated objects.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.slabs.iter().flat_map(|slab| slab.iter())
    }

    /// Memory overhead of the slab structure itself (bytes).
    pub fn overhead_bytes(&self) -> usize {
        self.slabs.capacity() * std::mem::size_of::<Vec<T>>()
            + self
                .slabs
                .iter()
                .map(|s| (s.capacity() - s.len()) * std::mem::size_of::<T>())
                .sum::<usize>()
    }
}

/// Reference to an object in a slab.
#[derive(Debug, Clone, Copy)]
pub struct SlabRef {
    pub slab: u32,
    pub index: u32,
}

// ─── Memory Stats ────────────────────────────────────────────────────────────

/// Report memory usage statistics.
pub fn print_memory_stats(interner: &StringInterner, path_interner: &PathInterner) {
    use crate::scanner::format_size;
    use console::style;

    println!();
    println!(
        "  {} {}",
        style("Memory Optimization").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );
    println!(
        "  {} Unique strings interned: {}",
        style("◉").cyan(),
        style(interner.len()).white().bold()
    );
    println!(
        "  {} String memory: {}",
        style("◉").dim(),
        format_size(interner.memory_usage() as u64)
    );
    println!(
        "  {} Unique path components: {}",
        style("◉").cyan(),
        style(path_interner.component_count()).white().bold()
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_interner_basic() {
        let mut interner = StringInterner::new();
        let idx1 = interner.intern("hello");
        let idx2 = interner.intern("world");
        let idx3 = interner.intern("hello"); // duplicate

        assert_eq!(idx1, idx3); // Same string = same index
        assert_ne!(idx1, idx2);
        assert_eq!(interner.resolve(idx1), "hello");
        assert_eq!(interner.resolve(idx2), "world");
    }

    #[test]
    fn test_string_interner_preseeded() {
        let interner = StringInterner::new();
        // assert!(interner.len() > 0);
        assert!(!interner.is_empty()); // Pre-seeded common strings
    }

    #[test]
    fn test_path_interner() {
        let mut interner = PathInterner::new();
        let path = Path::new("node_modules/lodash/index.js");
        let interned = interner.intern_path(path);
        let resolved = interner.resolve_path(&interned);
        assert_eq!(resolved, path);
    }

    #[test]
    fn test_path_interner_dedup() {
        let mut interner = PathInterner::new();
        interner.intern_path(Path::new("node_modules/lodash/index.js"));
        interner.intern_path(Path::new("node_modules/lodash/package.json"));
        interner.intern_path(Path::new("node_modules/express/index.js"));

        // "node_modules", "lodash", "index.js", "package.json", "express"
        assert!(interner.component_count() <= 5);
    }

    #[test]
    fn test_object_pool_basic() {
        let pool: ObjectPool<Vec<u8>> = ObjectPool::new();
        let item = pool.get();
        assert!(item.is_empty());

        let mut item2 = pool.get();
        item2.extend_from_slice(b"hello");
        pool.put(item2);

        // Next get should recycle
        let recycled = pool.get();
        // Note: recycled may or may not have content (depends on clear logic)

        let stats = pool.stats();
        assert!(stats.total_created > 0);
    }

    #[test]
    fn test_object_pool_efficiency() {
        let pool: ObjectPool<String> = ObjectPool::new();

        // Create and return objects
        for _ in 0..10 {
            let s = pool.get();
            pool.put(s);
        }

        let stats = pool.stats();
        assert!(stats.efficiency() > 0.0);
    }

    #[test]
    fn test_slab_allocator() {
        let mut slab: SlabAllocator<String> = SlabAllocator::new(4);

        let r1 = slab.alloc("hello".to_string());
        let r2 = slab.alloc("world".to_string());
        let r3 = slab.alloc("foo".to_string());
        let r4 = slab.alloc("bar".to_string());
        let r5 = slab.alloc("baz".to_string()); // triggers new slab

        assert_eq!(slab.len(), 5);
        assert_eq!(slab.slab_count(), 2);
        assert_eq!(slab.get(&r1), Some(&"hello".to_string()));
        assert_eq!(slab.get(&r5), Some(&"baz".to_string()));
    }

    #[test]
    fn test_slab_allocator_iter() {
        let mut slab: SlabAllocator<i32> = SlabAllocator::new(3);
        slab.alloc(1);
        slab.alloc(2);
        slab.alloc(3);
        slab.alloc(4);

        let items: Vec<&i32> = slab.iter().collect();
        assert_eq!(items, vec![&1, &2, &3, &4]);
    }

    #[test]
    fn test_slab_get_mut() {
        let mut slab: SlabAllocator<String> = SlabAllocator::new(4);
        let r = slab.alloc("hello".to_string());

        if let Some(s) = slab.get_mut(&r) {
            s.push_str(" world");
        }

        assert_eq!(slab.get(&r), Some(&"hello world".to_string()));
    }
    pub struct ArenaPruneCandidate<'a> {
        pub path: &'a str,
        pub size: u64,
        pub category: FileCategory,
        pub package_name: &'a str,
    }

    #[test]
    fn test_pool_stats() {
        let pool: ObjectPool<Vec<u8>> = ObjectPool::with_capacity(5);
        let stats = pool.stats();
        assert_eq!(stats.pool_size, 5);
        assert_eq!(stats.total_created, 5);
    }
}
