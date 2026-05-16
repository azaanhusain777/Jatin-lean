//! Profiler module: performance timing breakdowns for scan operations.
//!
//! Tracks the duration of each phase of the scanning pipeline and
//! provides detailed timing reports for performance optimization.

use std::time::{Duration, Instant};

use crate::scanner::format_number;

/// A named timing span within the profiler.
#[derive(Debug, Clone)]
pub struct TimingSpan {
    pub name: String,
    pub duration: Duration,
    pub items_processed: u64,
}

impl TimingSpan {
    /// Throughput in items/second.
    pub fn throughput(&self) -> f64 {
        let secs = self.duration.as_secs_f64();
        if secs > 0.0 {
            self.items_processed as f64 / secs
        } else {
            0.0
        }
    }
}

/// Performance profiler for tracking operation timings.
#[derive(Debug)]
pub struct Profiler {
    /// Start time of the entire profiling session
    start: Instant,
    /// All completed timing spans
    spans: Vec<TimingSpan>,
    /// Currently active span
    active_span: Option<(String, Instant, u64)>,
    /// Whether profiling is enabled
    enabled: bool,
}

impl Profiler {
    /// Create a new profiler.
    pub fn new(enabled: bool) -> Self {
        Self {
            start: Instant::now(),
            spans: Vec::new(),
            active_span: None,
            enabled,
        }
    }

    /// Start a new timing span.
    pub fn start_span(&mut self, name: &str) {
        if !self.enabled {
            return;
        }
        // End any active span first
        self.end_span(0);
        self.active_span = Some((name.to_string(), Instant::now(), 0));
    }

    /// End the current timing span.
    pub fn end_span(&mut self, items_processed: u64) {
        if !self.enabled {
            return;
        }
        if let Some((name, start, _)) = self.active_span.take() {
            self.spans.push(TimingSpan {
                name,
                duration: start.elapsed(),
                items_processed,
            });
        }
    }

    /// Record a completed span with a known duration.
    pub fn record_span(&mut self, name: &str, duration: Duration, items_processed: u64) {
        if !self.enabled {
            return;
        }
        self.spans.push(TimingSpan {
            name: name.to_string(),
            duration,
            items_processed,
        });
    }

    /// Get total elapsed time.
    pub fn total_elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Get all spans.
    pub fn spans(&self) -> &[TimingSpan] {
        &self.spans
    }

    /// Get the slowest span.
    pub fn slowest_span(&self) -> Option<&TimingSpan> {
        self.spans.iter().max_by_key(|s| s.duration)
    }

    /// Get timing breakdown as percentages.
    pub fn percentage_breakdown(&self) -> Vec<(&str, f64)> {
        let total = self.total_elapsed().as_secs_f64();
        if total <= 0.0 {
            return Vec::new();
        }
        self.spans
            .iter()
            .map(|s| (s.name.as_str(), s.duration.as_secs_f64() / total * 100.0))
            .collect()
    }

    /// Whether profiling is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Print profiling results to the terminal.
pub fn print_profiling_report(profiler: &Profiler) {
    use console::style;

    if !profiler.is_enabled() || profiler.spans().is_empty() {
        return;
    }

    println!();
    println!(
        "  {} {}",
        style("Performance Profile").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );
    println!();

    let total = profiler.total_elapsed();
    println!(
        "  {} Total time: {:.2}s",
        style("⏱").bold(),
        total.as_secs_f64()
    );
    println!();

    // Phase breakdown
    println!(
        "  {} {}",
        style("Phase Breakdown").white().bold(),
        style("───────────────────────────────").dim()
    );

    let breakdown = profiler.percentage_breakdown();
    for (name, pct) in &breakdown {
        let bar_len = (*pct / 2.0) as usize;
        let bar: String = "█".repeat(bar_len);
        let empty: String = "░".repeat(50usize.saturating_sub(bar_len));

        println!(
            "  {} {:20} {:>6.1}% {}{}",
            style("▸").dim(),
            name,
            pct,
            style(&bar).cyan(),
            style(&empty).dim(),
        );
    }

    // Throughput
    println!();
    println!(
        "  {} {}",
        style("Throughput").white().bold(),
        style("────────────────────────────────").dim()
    );

    for span in profiler.spans() {
        if span.items_processed > 0 {
            println!(
                "  {} {} — {} items in {:.2}s ({:.0} items/sec)",
                style("▸").dim(),
                span.name,
                format_number(span.items_processed),
                span.duration.as_secs_f64(),
                span.throughput(),
            );
        }
    }

    // Bottleneck
    if let Some(slowest) = profiler.slowest_span() {
        println!();
        println!(
            "  {} Bottleneck: {} ({:.2}s, {:.1}% of total)",
            style("⚠").yellow(),
            style(&slowest.name).yellow().bold(),
            slowest.duration.as_secs_f64(),
            slowest.duration.as_secs_f64() / total.as_secs_f64() * 100.0,
        );
    }

    println!();
}

/// Macro for easy span measurement (use in calling code).
/// Usage: measure_span!(profiler, "span_name", { ... code ... })
#[macro_export]
macro_rules! measure_span {
    ($profiler:expr, $name:expr, $items:expr, $body:block) => {{
        let _start = std::time::Instant::now();
        let _result = $body;
        $profiler.record_span($name, _start.elapsed(), $items);
        _result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_profiler_disabled() {
        let mut profiler = Profiler::new(false);
        profiler.start_span("test");
        profiler.end_span(100);
        assert!(profiler.spans().is_empty());
    }

    #[test]
    fn test_profiler_enabled() {
        let mut profiler = Profiler::new(true);
        profiler.start_span("test");
        thread::sleep(Duration::from_millis(10));
        profiler.end_span(50);
        assert_eq!(profiler.spans().len(), 1);
        assert!(profiler.spans()[0].duration.as_millis() >= 10);
        assert_eq!(profiler.spans()[0].items_processed, 50);
    }

    #[test]
    fn test_profiler_multiple_spans() {
        let mut profiler = Profiler::new(true);

        profiler.start_span("phase1");
        thread::sleep(Duration::from_millis(10));
        profiler.end_span(100);

        profiler.start_span("phase2");
        thread::sleep(Duration::from_millis(10));
        profiler.end_span(200);

        assert_eq!(profiler.spans().len(), 2);
    }

    #[test]
    fn test_profiler_record_span() {
        let mut profiler = Profiler::new(true);
        profiler.record_span("manual", Duration::from_millis(500), 1000);

        assert_eq!(profiler.spans().len(), 1);
        assert_eq!(profiler.spans()[0].name, "manual");
        assert_eq!(profiler.spans()[0].items_processed, 1000);
    }

    #[test]
    fn test_timing_span_throughput() {
        let span = TimingSpan {
            name: "test".to_string(),
            duration: Duration::from_secs(2),
            items_processed: 1000,
        };
        assert!((span.throughput() - 500.0).abs() < 1.0);
    }

    #[test]
    fn test_timing_span_zero_duration() {
        let span = TimingSpan {
            name: "instant".to_string(),
            duration: Duration::from_secs(0),
            items_processed: 100,
        };
        assert_eq!(span.throughput(), 0.0);
    }

    #[test]
    fn test_slowest_span() {
        let mut profiler = Profiler::new(true);
        profiler.record_span("fast", Duration::from_millis(100), 0);
        profiler.record_span("slow", Duration::from_millis(500), 0);
        profiler.record_span("medium", Duration::from_millis(300), 0);

        let slowest = profiler.slowest_span().unwrap();
        assert_eq!(slowest.name, "slow");
    }

    #[test]
    fn test_percentage_breakdown() {
        let mut profiler = Profiler::new(true);
        profiler.record_span("a", Duration::from_millis(100), 0);
        profiler.record_span("b", Duration::from_millis(100), 0);

        let breakdown = profiler.percentage_breakdown();
        assert_eq!(breakdown.len(), 2);
        // Each should be roughly 50% but depends on total elapsed
    }
}
