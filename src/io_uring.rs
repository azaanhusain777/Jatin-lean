//! io_uring Async I/O Engine
//!
//! Linux's io_uring eliminates syscall overhead by using shared memory
//! ring buffers between user-space and kernel. This achieves up to 10x
//! faster I/O than epoll for file operations.
//! For node_modules scanning: submit thousands of readdir/stat operations
//! in a single batch, kernel processes them asynchronously.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

// ─── io_uring Operation Types ────────────────────────────────────────────────

/// Supported io_uring operations for file scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOp {
    /// Batched readdir (IORING_OP_GETDENTS)
    ReadDir,
    /// Batched stat (IORING_OP_STATX)
    Stat,
    /// Batched read (IORING_OP_READ)
    Read,
    /// Batched unlink (IORING_OP_UNLINKAT)
    Unlink,
    /// Batched openat (IORING_OP_OPENAT)
    Open,
    /// No-op (for benchmarking ring overhead)
    Nop,
}

impl IoOp {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ReadDir => "IORING_OP_GETDENTS",
            Self::Stat => "IORING_OP_STATX",
            Self::Read => "IORING_OP_READ",
            Self::Unlink => "IORING_OP_UNLINKAT",
            Self::Open => "IORING_OP_OPENAT",
            Self::Nop => "IORING_OP_NOP",
        }
    }

    pub fn syscall_equivalent(&self) -> &'static str {
        match self {
            Self::ReadDir => "getdents64()",
            Self::Stat => "statx()",
            Self::Read => "read()",
            Self::Unlink => "unlinkat()",
            Self::Open => "openat()",
            Self::Nop => "(none)",
        }
    }
}

// ─── Submission Queue Entry ──────────────────────────────────────────────────

/// A submission queue entry (SQE) — represents one I/O request.
#[derive(Debug, Clone)]
pub struct SubmissionEntry {
    pub op: IoOp,
    pub path: PathBuf,
    pub flags: u32,
    pub user_data: u64,
    pub submitted_at: Instant,
}

/// A completion queue entry (CQE) — result of one I/O operation.
#[derive(Debug, Clone)]
pub struct CompletionEntry {
    pub user_data: u64,
    pub result: i64,
    pub flags: u32,
    pub latency: Duration,
}

// ─── io_uring Ring Descriptor ────────────────────────────────────────────────

/// io_uring ring configuration.
#[derive(Debug, Clone)]
pub struct IoUringConfig {
    /// Submission queue depth (power of 2)
    pub sq_depth: u32,
    /// Completion queue depth (usually 2x SQ depth)
    pub cq_depth: u32,
    /// Enable SQPOLL (kernel-side polling, zero syscalls)
    pub sqpoll: bool,
    /// SQPOLL idle timeout (ms)
    pub sqpoll_idle_ms: u32,
    /// Enable IOPOLL (busy-polling for NVMe)
    pub iopoll: bool,
    /// Enable fixed buffers (registered I/O buffers)
    pub fixed_buffers: bool,
    /// Number of fixed buffers
    pub num_fixed_buffers: u32,
    /// Fixed buffer size
    pub fixed_buffer_size: usize,
    /// Enable direct descriptors (registered file descriptors)
    pub direct_descriptors: bool,
}

impl Default for IoUringConfig {
    fn default() -> Self {
        Self {
            sq_depth: 256,
            cq_depth: 512,
            sqpoll: false,
            sqpoll_idle_ms: 1000,
            iopoll: false,
            fixed_buffers: true,
            num_fixed_buffers: 64,
            fixed_buffer_size: 4096,
            direct_descriptors: true,
        }
    }
}

impl IoUringConfig {
    /// High-performance config for NVMe SSDs.
    pub fn nvme_optimized() -> Self {
        Self {
            sq_depth: 1024,
            cq_depth: 4096,
            sqpoll: true,
            sqpoll_idle_ms: 2000,
            iopoll: true,
            fixed_buffers: true,
            num_fixed_buffers: 256,
            fixed_buffer_size: 8192,
            direct_descriptors: true,
        }
    }

    /// Config optimized for node_modules scanning.
    pub fn scan_optimized() -> Self {
        Self {
            sq_depth: 512,
            cq_depth: 1024,
            sqpoll: true,
            sqpoll_idle_ms: 500,
            iopoll: false,
            fixed_buffers: true,
            num_fixed_buffers: 128,
            fixed_buffer_size: 4096,
            direct_descriptors: true,
        }
    }
}

// ─── Simulated io_uring Engine ───────────────────────────────────────────────

/// Simulated io_uring engine for cross-platform development.
/// On Linux, this would use the actual io_uring syscalls.
pub struct IoUringEngine {
    pub config: IoUringConfig,
    submission_queue: VecDeque<SubmissionEntry>,
    completion_queue: VecDeque<CompletionEntry>,
    pub stats: IoUringStats,
    next_user_data: u64,
}

/// io_uring performance statistics.
#[derive(Debug)]
pub struct IoUringStats {
    pub submissions: AtomicU64,
    pub completions: AtomicU64,
    pub batches: AtomicU64,
    pub total_submit_ns: AtomicU64,
    pub total_complete_ns: AtomicU64,
    pub syscalls_saved: AtomicU64,
    pub bytes_processed: AtomicU64,
}

impl IoUringStats {
    pub fn new() -> Self {
        Self {
            submissions: AtomicU64::new(0),
            completions: AtomicU64::new(0),
            batches: AtomicU64::new(0),
            total_submit_ns: AtomicU64::new(0),
            total_complete_ns: AtomicU64::new(0),
            syscalls_saved: AtomicU64::new(0),
            bytes_processed: AtomicU64::new(0),
        }
    }

    pub fn avg_submit_ns(&self) -> f64 {
        let s = self.submissions.load(Ordering::Relaxed);
        if s == 0 { return 0.0; }
        self.total_submit_ns.load(Ordering::Relaxed) as f64 / s as f64
    }

    pub fn avg_completion_ns(&self) -> f64 {
        let c = self.completions.load(Ordering::Relaxed);
        if c == 0 { return 0.0; }
        self.total_complete_ns.load(Ordering::Relaxed) as f64 / c as f64
    }

    pub fn iops(&self, wall_time: Duration) -> f64 {
        let secs = wall_time.as_secs_f64();
        if secs < 1e-9 { return 0.0; }
        self.completions.load(Ordering::Relaxed) as f64 / secs
    }
}

impl IoUringEngine {
    pub fn new(config: IoUringConfig) -> Self {
        Self {
            config,
            submission_queue: VecDeque::with_capacity(256),
            completion_queue: VecDeque::with_capacity(512),
            stats: IoUringStats::new(),
            next_user_data: 0,
        }
    }

    /// Submit a single I/O operation to the submission queue.
    pub fn submit(&mut self, op: IoOp, path: PathBuf, flags: u32) -> u64 {
        let user_data = self.next_user_data;
        self.next_user_data += 1;

        self.submission_queue.push_back(SubmissionEntry {
            op, path, flags, user_data,
            submitted_at: Instant::now(),
        });

        self.stats.submissions.fetch_add(1, Ordering::Relaxed);
        user_data
    }

    /// Submit a batch of stat operations (the core scanning optimization).
    pub fn submit_stat_batch(&mut self, paths: &[PathBuf]) -> Vec<u64> {
        let start = Instant::now();
        let ids: Vec<u64> = paths.iter()
            .map(|p| self.submit(IoOp::Stat, p.clone(), 0))
            .collect();
        let elapsed = start.elapsed();
        self.stats.batches.fetch_add(1, Ordering::Relaxed);
        self.stats.total_submit_ns.fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);
        // Each batched op saves one syscall vs individual stat() calls
        self.stats.syscalls_saved.fetch_add(ids.len().saturating_sub(1) as u64, Ordering::Relaxed);
        ids
    }

    /// Submit a batch of unlink operations (deletion optimization).
    pub fn submit_unlink_batch(&mut self, paths: &[PathBuf]) -> Vec<u64> {
        let start = Instant::now();
        let ids: Vec<u64> = paths.iter()
            .map(|p| self.submit(IoOp::Unlink, p.clone(), 0))
            .collect();
        let elapsed = start.elapsed();
        self.stats.batches.fetch_add(1, Ordering::Relaxed);
        self.stats.total_submit_ns.fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);
        self.stats.syscalls_saved.fetch_add(ids.len().saturating_sub(1) as u64, Ordering::Relaxed);
        ids
    }

    /// Process the submission queue (simulate kernel processing).
    pub fn flush(&mut self) -> Vec<CompletionEntry> {
        let start = Instant::now();
        let mut results = Vec::with_capacity(self.submission_queue.len());

        while let Some(sqe) = self.submission_queue.pop_front() {
            let latency = sqe.submitted_at.elapsed();
            results.push(CompletionEntry {
                user_data: sqe.user_data,
                result: 0, // success
                flags: 0,
                latency,
            });
            self.stats.completions.fetch_add(1, Ordering::Relaxed);
        }

        let elapsed = start.elapsed();
        self.stats.total_complete_ns.fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);
        results
    }

    /// Get the pending submission count.
    pub fn pending(&self) -> usize {
        self.submission_queue.len()
    }
}

// ─── I/O Comparison Table ────────────────────────────────────────────────────

/// I/O API comparison data.
#[derive(Debug, Clone)]
pub struct IoApiComparison {
    pub api: &'static str,
    pub syscalls_per_op: &'static str,
    pub batching: &'static str,
    pub kernel_bypass: bool,
    pub zero_copy: bool,
    pub best_for: &'static str,
}

pub fn io_api_comparison() -> Vec<IoApiComparison> {
    vec![
        IoApiComparison {
            api: "read()/write() (Traditional)",
            syscalls_per_op: "1 per operation",
            batching: "None",
            kernel_bypass: false, zero_copy: false,
            best_for: "Simple sequential I/O",
        },
        IoApiComparison {
            api: "epoll (Event-driven)",
            syscalls_per_op: "1 per batch (epoll_wait)",
            batching: "Event batching only",
            kernel_bypass: false, zero_copy: false,
            best_for: "Network servers (nginx, Node.js)",
        },
        IoApiComparison {
            api: "aio (Linux AIO)",
            syscalls_per_op: "1 per batch (io_submit)",
            batching: "Submission batching",
            kernel_bypass: false, zero_copy: true,
            best_for: "Database engines (O_DIRECT)",
        },
        IoApiComparison {
            api: "io_uring (Modern)",
            syscalls_per_op: "0 (SQPOLL mode)",
            batching: "Full SQ/CQ ring batching",
            kernel_bypass: true, zero_copy: true,
            best_for: "Everything (file, network, timers)",
        },
    ]
}

/// Print io_uring report.
pub fn print_iouring_report(stats: &IoUringStats, config: &IoUringConfig, wall_time: Duration) {
    use console::style;
    println!();
    println!("  {} {}", style("io_uring Async I/O Engine").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
    println!("  {} SQ depth: {} | CQ depth: {} | SQPOLL: {} | IOPOLL: {}",
        style("▸").dim(), config.sq_depth, config.cq_depth,
        if config.sqpoll { "✓" } else { "✗" },
        if config.iopoll { "✓" } else { "✗" });
    println!("  {} Submissions:    {}", style("▸").dim(),
        stats.submissions.load(Ordering::Relaxed));
    println!("  {} Completions:    {}", style("▸").dim(),
        stats.completions.load(Ordering::Relaxed));
    println!("  {} Batches:        {}", style("▸").dim(),
        stats.batches.load(Ordering::Relaxed));
    println!("  {} Syscalls saved: {}", style("⚡").yellow(),
        style(stats.syscalls_saved.load(Ordering::Relaxed)).green().bold());
    println!("  {} Avg submit:     {:.0} ns", style("▸").dim(), stats.avg_submit_ns());
    println!("  {} IOPS:           {:.0}", style("🚀").yellow(),
        style(format!("{:.0}", stats.iops(wall_time))).green().bold());
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submit_and_flush() {
        let mut engine = IoUringEngine::new(IoUringConfig::default());
        engine.submit(IoOp::Stat, PathBuf::from("/tmp/test"), 0);
        assert_eq!(engine.pending(), 1);
        let results = engine.flush();
        assert_eq!(results.len(), 1);
        assert_eq!(engine.pending(), 0);
    }

    #[test]
    fn test_stat_batch() {
        let mut engine = IoUringEngine::new(IoUringConfig::default());
        let paths: Vec<PathBuf> = (0..100).map(|i| PathBuf::from(format!("/file-{}", i))).collect();
        let ids = engine.submit_stat_batch(&paths);
        assert_eq!(ids.len(), 100);
        assert_eq!(engine.stats.syscalls_saved.load(Ordering::Relaxed), 99);
    }

    #[test]
    fn test_nvme_config() {
        let cfg = IoUringConfig::nvme_optimized();
        assert!(cfg.sqpoll);
        assert!(cfg.iopoll);
        assert_eq!(cfg.sq_depth, 1024);
    }

    #[test]
    fn test_io_comparison() {
        let table = io_api_comparison();
        assert_eq!(table.len(), 4);
        assert!(table[3].kernel_bypass); // io_uring
    }

    #[test]
    fn test_unlink_batch() {
        let mut engine = IoUringEngine::new(IoUringConfig::default());
        let paths: Vec<PathBuf> = (0..50).map(|i| PathBuf::from(format!("/del-{}", i))).collect();
        let ids = engine.submit_unlink_batch(&paths);
        assert_eq!(ids.len(), 50);
        let results = engine.flush();
        assert_eq!(results.len(), 50);
    }
}
