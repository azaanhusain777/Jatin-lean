//! Performance profiling and metrics collection for jatin-lean.
//!
//! This module provides comprehensive performance monitoring to identify
//! bottlenecks and optimization opportunities.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Performance metrics for a complete scan operation
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Total time spent scanning
    pub scan_duration: Duration,
    
    /// Total time spent tracing dependencies
    pub trace_duration: Duration,
    
    /// Total time spent deleting files
    pub delete_duration: Duration,
    
    /// Total time for entire operation
    pub total_duration: Duration,
    
    /// Number of files processed per second
    pub files_per_second: f64,
    
    /// Bytes processed per second
    pub bytes_per_second: f64,
    
    /// Number of packages scanned
    pub packages_scanned: usize,
    
    /// Average time per package
    pub avg_time_per_package: Duration,
    
    /// Identified bottlenecks
    pub bottlenecks: Vec<Bottleneck>,
    
    /// Per-package timing breakdown
    pub package_timings: HashMap<String, PackageTiming>,
    
    /// Phase breakdown
    pub phase_breakdown: PhaseBreakdown,
}

/// Represents a performance bottleneck
#[derive(Debug, Clone)]
pub struct Bottleneck {
    /// Package or operation name
    pub name: String,
    
    /// Type of operation (scan, trace, delete, etc.)
    pub operation: String,
    
    /// Time taken
    pub duration: Duration,
    
    /// Reason for slowness
    pub reason: String,
    
    /// Severity (1-10, 10 being worst)
    pub severity: u8,
}

/// Timing information for a single package
#[derive(Debug, Clone)]
pub struct PackageTiming {
    /// Package name
    pub name: String,
    
    /// Time to scan package
    pub scan_time: Duration,
    
    /// Time to trace dependencies
    pub trace_time: Duration,
    
    /// Number of files in package
    pub file_count: usize,
    
    /// Total size of package
    pub total_size: u64,
    
    /// Number of candidates found
    pub candidates_found: usize,
}

/// Breakdown of time spent in each phase
#[derive(Debug, Clone)]
pub struct PhaseBreakdown {
    /// Time spent discovering packages
    pub discovery: Duration,
    
    /// Time spent parsing package.json files
    pub parsing: Duration,
    
    /// Time spent walking file trees
    pub walking: Duration,
    
    /// Time spent classifying files
    pub classification: Duration,
    
    /// Time spent tracing dependencies
    pub tracing: Duration,
    
    /// Time spent deleting files
    pub deletion: Duration,
    
    /// Time spent on I/O operations
    pub io_time: Duration,
    
    /// Time spent on CPU operations
    pub cpu_time: Duration,
}

impl Default for PhaseBreakdown {
    fn default() -> Self {
        Self {
            discovery: Duration::ZERO,
            parsing: Duration::ZERO,
            walking: Duration::ZERO,
            classification: Duration::ZERO,
            tracing: Duration::ZERO,
            deletion: Duration::ZERO,
            io_time: Duration::ZERO,
            cpu_time: Duration::ZERO,
        }
    }
}

/// Timer utility for measuring operation duration
pub struct Timer {
    start: Instant,
    label: String,
}

impl Timer {
    /// Create a new timer with a label
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            label: label.into(),
        }
    }
    
    /// Get elapsed time without stopping the timer
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
    
    /// Stop the timer and return elapsed time
    pub fn stop(self) -> Duration {
        self.start.elapsed()
    }
    
    /// Stop the timer and print elapsed time
    pub fn stop_and_print(self) -> Duration {
        let elapsed = self.start.elapsed();
        println!("  [TIMER] {} took {:?}", self.label, elapsed);
        elapsed
    }
}

/// Profiler for collecting metrics during execution
pub struct Profiler {
    /// Start time of profiling
    start_time: Instant,
    
    /// Phase timings
    pub phase_breakdown: PhaseBreakdown,
    
    /// Per-package timings
    pub package_timings: HashMap<String, PackageTiming>,
    
    /// Detected bottlenecks
    pub bottlenecks: Vec<Bottleneck>,
    
    /// Total files processed
    pub total_files: u64,
    
    /// Total bytes processed
    pub total_bytes: u64,
    
    /// Total packages processed
    pub total_packages: usize,
}

impl Profiler {
    /// Create a new profiler
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            phase_breakdown: PhaseBreakdown::default(),
            package_timings: HashMap::new(),
            bottlenecks: Vec::new(),
            total_files: 0,
            total_bytes: 0,
            total_packages: 0,
        }
    }
    
    /// Create a new profiler with optional profiling enabled
    pub fn with_profiling(_enabled: bool) -> Self {
        // For now, always create a profiler
        // Can be enhanced later to conditionally enable/disable
        Self::new()
    }
    
    /// Start a named span (for compatibility with existing code)
    pub fn start_span(&mut self, _name: &str) {
        // No-op for now, can be enhanced later
    }
    
    /// End a named span (for compatibility with existing code)
    pub fn end_span(&mut self, _count: u64) {
        // No-op for now, can be enhanced later
    }
    
    /// Record discovery phase timing
    pub fn record_discovery(&mut self, duration: Duration) {
        self.phase_breakdown.discovery = duration;
    }
    
    /// Record parsing phase timing
    pub fn record_parsing(&mut self, duration: Duration) {
        self.phase_breakdown.parsing = duration;
    }
    
    /// Record walking phase timing
    pub fn record_walking(&mut self, duration: Duration) {
        self.phase_breakdown.walking = duration;
    }
    
    /// Record classification phase timing
    pub fn record_classification(&mut self, duration: Duration) {
        self.phase_breakdown.classification = duration;
    }
    
    /// Record tracing phase timing
    pub fn record_tracing(&mut self, duration: Duration) {
        self.phase_breakdown.tracing = duration;
    }
    
    /// Record deletion phase timing
    pub fn record_deletion(&mut self, duration: Duration) {
        self.phase_breakdown.deletion = duration;
    }
    
    /// Record package timing
    pub fn record_package(&mut self, timing: PackageTiming) {
        self.total_packages += 1;
        self.total_files += timing.file_count as u64;
        self.total_bytes += timing.total_size;
        
        // Detect bottlenecks
        if timing.scan_time > Duration::from_millis(100) {
            self.bottlenecks.push(Bottleneck {
                name: timing.name.clone(),
                operation: "scan".to_string(),
                duration: timing.scan_time,
                reason: format!("Large package with {} files", timing.file_count),
                severity: self.calculate_severity(timing.scan_time),
            });
        }
        
        self.package_timings.insert(timing.name.clone(), timing);
    }
    
    /// Add a custom bottleneck
    pub fn add_bottleneck(&mut self, bottleneck: Bottleneck) {
        self.bottlenecks.push(bottleneck);
    }
    
    /// Calculate severity score (1-10) based on duration
    fn calculate_severity(&self, duration: Duration) -> u8 {
        let millis = duration.as_millis();
        match millis {
            0..=50 => 1,
            51..=100 => 3,
            101..=200 => 5,
            201..=500 => 7,
            501..=1000 => 9,
            _ => 10,
        }
    }
    
    /// Finalize and generate performance metrics
    pub fn finalize(self) -> PerformanceMetrics {
        let total_duration = self.start_time.elapsed();
        
        let scan_duration = self.phase_breakdown.discovery
            + self.phase_breakdown.parsing
            + self.phase_breakdown.walking
            + self.phase_breakdown.classification;
        
        let files_per_second = if total_duration.as_secs_f64() > 0.0 {
            self.total_files as f64 / total_duration.as_secs_f64()
        } else {
            0.0
        };
        
        let bytes_per_second = if total_duration.as_secs_f64() > 0.0 {
            self.total_bytes as f64 / total_duration.as_secs_f64()
        } else {
            0.0
        };
        
        let avg_time_per_package = if self.total_packages > 0 {
            scan_duration / self.total_packages as u32
        } else {
            Duration::ZERO
        };
        
        PerformanceMetrics {
            scan_duration,
            trace_duration: self.phase_breakdown.tracing,
            delete_duration: self.phase_breakdown.deletion,
            total_duration,
            files_per_second,
            bytes_per_second,
            packages_scanned: self.total_packages,
            avg_time_per_package,
            bottlenecks: self.bottlenecks,
            package_timings: self.package_timings,
            phase_breakdown: self.phase_breakdown,
        }
    }
    
    /// Get top N slowest packages
    pub fn get_slowest_packages(&self, n: usize) -> Vec<&PackageTiming> {
        let mut timings: Vec<&PackageTiming> = self.package_timings.values().collect();
        timings.sort_by(|a, b| b.scan_time.cmp(&a.scan_time));
        timings.into_iter().take(n).collect()
    }
    
    /// Get top N bottlenecks by severity
    pub fn get_top_bottlenecks(&self, n: usize) -> Vec<&Bottleneck> {
        let mut bottlenecks = self.bottlenecks.iter().collect::<Vec<_>>();
        bottlenecks.sort_by(|a, b| b.severity.cmp(&a.severity));
        bottlenecks.into_iter().take(n).collect()
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro for easy timing of code blocks
#[macro_export]
macro_rules! time_operation {
    ($profiler:expr, $phase:ident, $code:block) => {{
        let timer = $crate::profiler::Timer::new(stringify!($phase));
        let result = $code;
        let duration = timer.stop();
        $profiler.$phase(duration);
        result
    }};
}

/// Format duration in human-readable format
pub fn format_duration(duration: Duration) -> String {
    let micros = duration.as_micros();
    if micros < 1_000 {
        format!("{}µs", micros)
    } else if micros < 1_000_000 {
        format!("{:.2}ms", micros as f64 / 1_000.0)
    } else {
        format!("{:.2}s", duration.as_secs_f64())
    }
}

/// Print performance summary
pub fn print_performance_summary(metrics: &PerformanceMetrics) {
    use console::style;
    
    println!("\n  {} {}", 
        style("Performance Summary").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );
    
    println!("  {} Total Duration: {}", 
        style("⏱").dim(),
        style(format_duration(metrics.total_duration)).yellow()
    );
    
    println!("  {} Scan: {} | Trace: {} | Delete: {}", 
        style("📊").dim(),
        style(format_duration(metrics.scan_duration)).cyan(),
        style(format_duration(metrics.trace_duration)).cyan(),
        style(format_duration(metrics.delete_duration)).cyan()
    );
    
    println!("  {} Throughput: {:.0} files/sec | {:.2} MB/sec", 
        style("⚡").dim(),
        metrics.files_per_second,
        metrics.bytes_per_second / 1_000_000.0
    );
    
    println!("  {} Packages: {} | Avg: {}/package", 
        style("📦").dim(),
        metrics.packages_scanned,
        format_duration(metrics.avg_time_per_package)
    );
    
    // Print bottlenecks if any
    if !metrics.bottlenecks.is_empty() {
        println!("\n  {} {}", 
            style("Bottlenecks Detected").yellow().bold(),
            style("━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
        );
        
        for (_i, bottleneck) in metrics.bottlenecks.iter().take(5).enumerate() {
            let severity_icon = match bottleneck.severity {
                1..=3 => "🟢",
                4..=6 => "🟡",
                7..=8 => "🟠",
                _ => "🔴",
            };
            
            println!("  {} {} - {} ({}): {}", 
                severity_icon,
                style(&bottleneck.name).yellow(),
                bottleneck.operation,
                format_duration(bottleneck.duration),
                style(&bottleneck.reason).dim()
            );
        }
    }
    
    println!();
}

/// Print profiling report (alias for compatibility)
pub fn print_profiling_report(profiler: &Profiler) {
    let metrics = profiler.clone().finalize();
    print_performance_summary(&metrics);
}

/// Clone implementation for Profiler (needed for finalize)
impl Clone for Profiler {
    fn clone(&self) -> Self {
        Self {
            start_time: self.start_time,
            phase_breakdown: self.phase_breakdown.clone(),
            package_timings: self.package_timings.clone(),
            bottlenecks: self.bottlenecks.clone(),
            total_files: self.total_files,
            total_bytes: self.total_bytes,
            total_packages: self.total_packages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_timer() {
        let timer = Timer::new("test");
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = timer.stop();
        assert!(elapsed >= Duration::from_millis(10));
    }
    
    #[test]
    fn test_profiler() {
        let mut profiler = Profiler::new();
        
        profiler.record_package(PackageTiming {
            name: "test-package".to_string(),
            scan_time: Duration::from_millis(50),
            trace_time: Duration::from_millis(10),
            file_count: 100,
            total_size: 1_000_000,
            candidates_found: 20,
        });
        
        assert_eq!(profiler.total_packages, 1);
        assert_eq!(profiler.total_files, 100);
        assert_eq!(profiler.total_bytes, 1_000_000);
    }
    
    #[test]
    fn test_severity_calculation() {
        let profiler = Profiler::new();
        
        assert_eq!(profiler.calculate_severity(Duration::from_millis(25)), 1);
        assert_eq!(profiler.calculate_severity(Duration::from_millis(75)), 3);
        assert_eq!(profiler.calculate_severity(Duration::from_millis(150)), 5);
        assert_eq!(profiler.calculate_severity(Duration::from_millis(300)), 7);
        assert_eq!(profiler.calculate_severity(Duration::from_millis(750)), 9);
        assert_eq!(profiler.calculate_severity(Duration::from_millis(1500)), 10);
    }
    
    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_micros(500)), "500µs");
        assert_eq!(format_duration(Duration::from_millis(5)), "5.00ms");
        assert_eq!(format_duration(Duration::from_secs(2)), "2.00s");
    }
}
