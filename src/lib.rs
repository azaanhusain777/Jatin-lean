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
