//! Unified High-Performance Gateway Architecture
//!
//! From Section "Synthesis" of the HPC document.
//! Chains all 5 optimization layers into a single pipeline:
//! 1. Ingestion (XDP) → 2. Deserialization (rkyv) → 3. Deduplication (Coalescing)
//! → 4. Cross-Boundary IPC → 5. Adaptive Execution → 6. Response & Cache

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

// ─── Pipeline Stages ─────────────────────────────────────────────────────────

/// The 6 stages of the unified gateway pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    /// Stage 1: Packet ingestion & DPI filtering (XDP fast path)
    Ingestion,
    /// Stage 2: Zero-copy deserialization (rkyv byte cast)
    Deserialization,
    /// Stage 3: Request deduplication (singleflight coalescing)
    Deduplication,
    /// Stage 4: Cross-boundary IPC (shared memory ring buffer)
    CrossBoundaryIpc,
    /// Stage 5: Adaptive compute routing (CPU/GPU/Hybrid)
    AdaptiveExecution,
    /// Stage 6: Response caching & fan-out
    ResponseCache,
}

impl PipelineStage {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Ingestion => "Ingestion (XDP)",
            Self::Deserialization => "Deserialization (rkyv)",
            Self::Deduplication => "Deduplication (Singleflight)",
            Self::CrossBoundaryIpc => "Cross-Boundary IPC (SPSC)",
            Self::AdaptiveExecution => "Adaptive Execution (CPU/GPU)",
            Self::ResponseCache => "Response & Cache (Structural)",
        }
    }

    pub fn all() -> &'static [PipelineStage] {
        &[
            Self::Ingestion, Self::Deserialization, Self::Deduplication,
            Self::CrossBoundaryIpc, Self::AdaptiveExecution, Self::ResponseCache,
        ]
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Ingestion => "🔷",
            Self::Deserialization => "⚡",
            Self::Deduplication => "🔀",
            Self::CrossBoundaryIpc => "📡",
            Self::AdaptiveExecution => "🧠",
            Self::ResponseCache => "💾",
        }
    }
}

// ─── Stage Metrics ───────────────────────────────────────────────────────────

/// Performance metrics for a single pipeline stage.
#[derive(Debug)]
pub struct StageMetrics {
    pub stage: PipelineStage,
    pub invocations: AtomicU64,
    pub total_ns: AtomicU64,
    pub errors: AtomicU64,
    pub bytes_processed: AtomicU64,
}

impl StageMetrics {
    pub fn new(stage: PipelineStage) -> Self {
        Self {
            stage,
            invocations: AtomicU64::new(0),
            total_ns: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            bytes_processed: AtomicU64::new(0),
        }
    }

    pub fn record(&self, elapsed_ns: u64, bytes: u64) {
        self.invocations.fetch_add(1, Ordering::Relaxed);
        self.total_ns.fetch_add(elapsed_ns, Ordering::Relaxed);
        self.bytes_processed.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn avg_latency_ns(&self) -> f64 {
        let inv = self.invocations.load(Ordering::Relaxed);
        if inv == 0 { return 0.0; }
        self.total_ns.load(Ordering::Relaxed) as f64 / inv as f64
    }

    pub fn throughput_gbps(&self, wall_time: Duration) -> f64 {
        let secs = wall_time.as_secs_f64();
        if secs < 0.001 { return 0.0; }
        let bytes = self.bytes_processed.load(Ordering::Relaxed) as f64;
        (bytes * 8.0) / (secs * 1e9)
    }
}

// ─── Pipeline Request ────────────────────────────────────────────────────────

/// A request flowing through the unified gateway pipeline.
#[derive(Debug, Clone)]
pub struct PipelineRequest {
    pub id: u64,
    pub payload: Vec<u8>,
    pub resource_key: String,
    pub client_id: String,
    pub created_at: Instant,
    pub stage_timings: Vec<(PipelineStage, Duration)>,
}

impl PipelineRequest {
    pub fn new(id: u64, payload: Vec<u8>, resource_key: String, client_id: String) -> Self {
        Self {
            id, payload, resource_key, client_id,
            created_at: Instant::now(),
            stage_timings: Vec::with_capacity(6),
        }
    }

    pub fn total_latency(&self) -> Duration {
        self.created_at.elapsed()
    }

    pub fn record_stage(&mut self, stage: PipelineStage, duration: Duration) {
        self.stage_timings.push((stage, duration));
    }
}

/// Pipeline response.
#[derive(Debug, Clone)]
pub struct PipelineResponse {
    pub request_id: u64,
    pub payload: Vec<u8>,
    pub cached: bool,
    pub coalesced: bool,
    pub compute_target: String,
    pub total_latency: Duration,
}

// ─── Unified Gateway ─────────────────────────────────────────────────────────

/// The unified gateway that chains all optimization layers.
pub struct UnifiedGateway {
    pub metrics: Vec<StageMetrics>,
    pub pipeline_stats: GatewayStats,
    started_at: Instant,
}

/// Gateway-level statistics.
#[derive(Debug)]
pub struct GatewayStats {
    pub requests_processed: AtomicU64,
    pub requests_coalesced: AtomicU64,
    pub cache_hits: AtomicU64,
    pub total_bytes_in: AtomicU64,
    pub total_bytes_out: AtomicU64,
}

impl GatewayStats {
    pub fn new() -> Self {
        Self {
            requests_processed: AtomicU64::new(0),
            requests_coalesced: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            total_bytes_in: AtomicU64::new(0),
            total_bytes_out: AtomicU64::new(0),
        }
    }
}

impl UnifiedGateway {
    pub fn new() -> Self {
        let metrics = PipelineStage::all().iter()
            .map(|s| StageMetrics::new(*s))
            .collect();
        Self {
            metrics,
            pipeline_stats: GatewayStats::new(),
            started_at: Instant::now(),
        }
    }

    /// Process a request through the full 6-stage pipeline.
    pub fn process(&self, mut req: PipelineRequest) -> PipelineResponse {
        self.pipeline_stats.requests_processed.fetch_add(1, Ordering::Relaxed);
        self.pipeline_stats.total_bytes_in.fetch_add(req.payload.len() as u64, Ordering::Relaxed);
        let payload_len = req.payload.len() as u64;

        // Stage 1: Ingestion (XDP packet validation)
        let s1 = Instant::now();
        let _valid = self.stage_ingestion(&req);
        let s1_dur = s1.elapsed();
        self.metrics[0].record(s1_dur.as_nanos() as u64, payload_len);
        req.record_stage(PipelineStage::Ingestion, s1_dur);

        // Stage 2: Deserialization (zero-copy byte cast)
        let s2 = Instant::now();
        let _data = self.stage_deserialize(&req);
        let s2_dur = s2.elapsed();
        self.metrics[1].record(s2_dur.as_nanos() as u64, payload_len);
        req.record_stage(PipelineStage::Deserialization, s2_dur);

        // Stage 3: Deduplication check
        let s3 = Instant::now();
        let coalesced = self.stage_dedup(&req);
        let s3_dur = s3.elapsed();
        self.metrics[2].record(s3_dur.as_nanos() as u64, 0);
        req.record_stage(PipelineStage::Deduplication, s3_dur);
        if coalesced {
            self.pipeline_stats.requests_coalesced.fetch_add(1, Ordering::Relaxed);
        }

        // Stage 4: IPC transfer
        let s4 = Instant::now();
        self.stage_ipc(&req);
        let s4_dur = s4.elapsed();
        self.metrics[3].record(s4_dur.as_nanos() as u64, payload_len);
        req.record_stage(PipelineStage::CrossBoundaryIpc, s4_dur);

        // Stage 5: Adaptive execution routing
        let s5 = Instant::now();
        let target = self.stage_execute(&req);
        let s5_dur = s5.elapsed();
        self.metrics[4].record(s5_dur.as_nanos() as u64, payload_len);
        req.record_stage(PipelineStage::AdaptiveExecution, s5_dur);

        // Stage 6: Cache & respond
        let s6 = Instant::now();
        let response_payload = self.stage_cache(&req);
        let s6_dur = s6.elapsed();
        self.metrics[5].record(s6_dur.as_nanos() as u64, response_payload.len() as u64);
        req.record_stage(PipelineStage::ResponseCache, s6_dur);

        self.pipeline_stats.total_bytes_out.fetch_add(response_payload.len() as u64, Ordering::Relaxed);

        PipelineResponse {
            request_id: req.id,
            payload: response_payload,
            cached: false,
            coalesced,
            compute_target: target,
            total_latency: req.total_latency(),
        }
    }

    fn stage_ingestion(&self, _req: &PipelineRequest) -> bool {
        // Simulate XDP packet validation (header check, DPI filter)
        true
    }

    fn stage_deserialize(&self, _req: &PipelineRequest) -> bool {
        // Simulate rkyv zero-copy byte cast (~1.4ns)
        true
    }

    fn stage_dedup(&self, req: &PipelineRequest) -> bool {
        // Simulate singleflight check
        // In production: check if req.resource_key is already in-flight
        req.id % 10 == 0 // Simulate 10% coalescing
    }

    fn stage_ipc(&self, _req: &PipelineRequest) {
        // Simulate SPSC ring buffer transfer (~102ns)
    }

    fn stage_execute(&self, req: &PipelineRequest) -> String {
        // Simulate adaptive routing decision
        if req.payload.len() > 1024 * 1024 {
            "GPU (Parallel)".to_string()
        } else {
            "CPU (Sequential)".to_string()
        }
    }

    fn stage_cache(&self, req: &PipelineRequest) -> Vec<u8> {
        // Simulate structural cache storage + response generation
        let response = format!("OK:{}", req.resource_key);
        response.into_bytes()
    }

    /// Run a full pipeline benchmark.
    pub fn benchmark(&self, request_count: u64, payload_size: usize) -> BenchmarkResult {
        let start = Instant::now();
        let payload = vec![0xABu8; payload_size];

        for i in 0..request_count {
            let req = PipelineRequest::new(
                i,
                payload.clone(),
                format!("/api/resource/{}", i % 100),
                format!("client-{}", i % 50),
            );
            self.process(req);
        }

        let elapsed = start.elapsed();
        BenchmarkResult {
            total_requests: request_count,
            elapsed,
            rps: request_count as f64 / elapsed.as_secs_f64(),
            avg_latency_ns: elapsed.as_nanos() as f64 / request_count as f64,
            stage_latencies: self.metrics.iter().map(|m| {
                (m.stage, m.avg_latency_ns())
            }).collect(),
        }
    }
}

/// Benchmark result.
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub total_requests: u64,
    pub elapsed: Duration,
    pub rps: f64,
    pub avg_latency_ns: f64,
    pub stage_latencies: Vec<(PipelineStage, f64)>,
}

/// Print gateway pipeline report.
pub fn print_gateway_report(gw: &UnifiedGateway) {
    use console::style;
    let wall = gw.started_at.elapsed();
    println!();
    println!("  {} {}", style("Unified Gateway Pipeline").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());

    for m in &gw.metrics {
        let inv = m.invocations.load(Ordering::Relaxed);
        if inv > 0 {
            println!("  {} {} | {:.0} ns avg | {} invocations | {:.2} Gbps",
                m.stage.icon(), style(m.stage.label()).yellow(),
                m.avg_latency_ns(), inv, m.throughput_gbps(wall));
        }
    }

    let stats = &gw.pipeline_stats;
    println!();
    println!("  {} Processed:  {} requests",
        style("▸").dim(), stats.requests_processed.load(Ordering::Relaxed));
    println!("  {} Coalesced:  {} requests",
        style("▸").dim(), stats.requests_coalesced.load(Ordering::Relaxed));
    println!("  {} Bytes in:   {} | Bytes out: {}",
        style("▸").dim(),
        stats.total_bytes_in.load(Ordering::Relaxed),
        stats.total_bytes_out.load(Ordering::Relaxed));
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_stages() {
        assert_eq!(PipelineStage::all().len(), 6);
    }

    #[test]
    fn test_gateway_process() {
        let gw = UnifiedGateway::new();
        let req = PipelineRequest::new(1, vec![1,2,3], "/test".into(), "c1".into());
        let resp = gw.process(req);
        assert_eq!(resp.request_id, 1);
        assert!(!resp.payload.is_empty());
    }

    #[test]
    fn test_gateway_benchmark() {
        let gw = UnifiedGateway::new();
        let result = gw.benchmark(100, 256);
        assert_eq!(result.total_requests, 100);
        assert!(result.rps > 0.0);
        assert_eq!(result.stage_latencies.len(), 6);
    }

    #[test]
    fn test_stage_metrics() {
        let m = StageMetrics::new(PipelineStage::Ingestion);
        m.record(1000, 64);
        m.record(2000, 128);
        assert_eq!(m.invocations.load(Ordering::Relaxed), 2);
        assert!((m.avg_latency_ns() - 1500.0).abs() < 0.1);
    }

    #[test]
    fn test_pipeline_request() {
        let mut req = PipelineRequest::new(42, vec![0; 100], "/api".into(), "c".into());
        req.record_stage(PipelineStage::Ingestion, Duration::from_nanos(50));
        assert_eq!(req.stage_timings.len(), 1);
    }
}
