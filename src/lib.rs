//! jatin-lean — library crate for programmatic access.
//!
//! This module re-exports all public APIs so that integration tests
//! and downstream Rust code can use jatin-lean as a library.

pub mod allocator;
pub mod analytics;
pub mod benchmark;
pub mod cache;
pub mod compress;
pub mod config;
pub mod dedup;
pub mod deleter;
pub mod display;
pub mod health;
pub mod lockfile;
pub mod mmap;
pub mod network;
pub mod plugin;
pub mod policy;
pub mod profiler;
pub mod rules;
pub mod scanner;
pub mod simd;
pub mod snapshot;
pub mod syscall;
pub mod tracer;
pub mod treeshake;
pub mod visualizer;
pub mod watcher;
pub mod ringbuffer;
pub mod strategy;
pub mod distributed_cache;
pub mod analyzer;
pub mod xdp_middleware;
pub mod shared_memory_ipc;
pub mod zero_copy_serde;
pub mod request_coalescing;
pub mod adaptive_engine;
pub mod unified_gateway;
pub mod simd_json;
pub mod memory_pool;
pub mod maglev;
pub mod io_uring;
pub mod cpu_cache;
pub mod hardware_tuning;
pub mod bpf_verifier;
pub mod pcie_bottleneck;
pub mod hedging;
pub mod mmap_ipc;
