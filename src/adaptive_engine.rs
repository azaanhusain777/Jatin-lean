//! Adaptive Execution Engine and Unified CPU-GPU Memory Pipeline
//!
//! Section 5 of High-Performance System Optimization Projects.
//! Implements a workload routing middleware that dynamically decides
//! whether to execute tasks on CPU or GPU based on workload profiling,
//! with unified memory management inspired by Grace Hopper NVLink-C2C.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

// ─── Memory Management Paradigms ─────────────────────────────────────────────

/// Memory management paradigm (from Table in Section 5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryParadigm {
    /// Standard OS malloc — low setup, low bandwidth
    PageableSystemMemory,
    /// cudaHostAlloc — high setup, saturates PCIe via DMA
    PinnedMemory,
    /// cudaMallocManaged — low setup, variable bandwidth (page faults)
    SoftwareCoherentUnified,
    /// Grace Hopper NVLink-C2C — low setup, 900 GB/s
    HardwareCoherentUnified,
}

impl MemoryParadigm {
    pub fn label(&self) -> &'static str {
        match self {
            Self::PageableSystemMemory => "Pageable System Memory",
            Self::PinnedMemory => "Pinned (Page-Locked) Memory",
            Self::SoftwareCoherentUnified => "Software-Coherent Unified Memory",
            Self::HardwareCoherentUnified => "Hardware-Coherent Unified Memory",
        }
    }

    pub fn bandwidth(&self) -> &'static str {
        match self {
            Self::PageableSystemMemory => "Low (CPU-staged transfers)",
            Self::PinnedMemory => "High (saturates PCIe limits)",
            Self::SoftwareCoherentUnified => "Variable (latency spikes on first access)",
            Self::HardwareCoherentUnified => "900 GB/s (NVLink-C2C)",
        }
    }

    pub fn setup_complexity(&self) -> &'static str {
        match self {
            Self::PageableSystemMemory => "Low (standard malloc)",
            Self::PinnedMemory => "High (explicit cudaHostAlloc)",
            Self::SoftwareCoherentUnified => "Low (cudaMallocManaged)",
            Self::HardwareCoherentUnified => "Low (shared virtual address)",
        }
    }

    pub fn optimal_hardware(&self) -> &'static str {
        match self {
            Self::PageableSystemMemory => "Legacy CPU Systems",
            Self::PinnedMemory => "x86 + PCIe Discrete GPU",
            Self::SoftwareCoherentUnified => "Pascal/Volta/Ampere GPUs",
            Self::HardwareCoherentUnified => "Grace Hopper Superchip (GH200)",
        }
    }
}

// ─── Compute Target ──────────────────────────────────────────────────────────

/// Target device for workload execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComputeTarget {
    /// CPU execution (good for sequential, branchy, string-heavy work)
    Cpu,
    /// GPU execution (good for parallel, SIMD, matrix-heavy work)
    Gpu,
    /// Hybrid: CPU orchestrates, GPU processes data
    Hybrid,
    /// CPU fallback when GPU VRAM is exhausted (graceful degradation)
    CpuFallback,
}

impl ComputeTarget {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Cpu => "CPU (Sequential)",
            Self::Gpu => "GPU (Parallel)",
            Self::Hybrid => "Hybrid (CPU+GPU)",
            Self::CpuFallback => "CPU Fallback (VRAM exhausted)",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Cpu => "🔵",
            Self::Gpu => "🟢",
            Self::Hybrid => "🟡",
            Self::CpuFallback => "🔴",
        }
    }
}

// ─── Workload Classification ─────────────────────────────────────────────────

/// Workload characteristics that determine execution target.
#[derive(Debug, Clone)]
pub struct WorkloadProfile {
    /// Unique workload identifier
    pub id: String,
    /// Estimated data size in bytes
    pub data_size_bytes: u64,
    /// Estimated computational complexity (FLOPs)
    pub estimated_flops: u64,
    /// Whether the workload is highly parallelizable
    pub parallelizable: bool,
    /// Whether the workload involves string/regex operations
    pub string_heavy: bool,
    /// Whether the workload involves matrix/vector operations
    pub matrix_heavy: bool,
    /// Whether the workload has high branch divergence
    pub branch_divergent: bool,
    /// Required memory for processing
    pub memory_required_bytes: u64,
    /// Workload type classification
    pub workload_type: WorkloadType,
}

/// High-level workload type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadType {
    /// String manipulation, regex parsing (CPU-preferred)
    StringProcessing,
    /// Matrix multiplication, linear algebra (GPU-preferred)
    MatrixComputation,
    /// Vector embeddings, similarity search (GPU-preferred)
    VectorEmbedding,
    /// Data aggregation, sorting (CPU or hybrid)
    DataAggregation,
    /// LLM inference (GPU with CPU fallback)
    LlmInference,
    /// Image/video processing (GPU-preferred)
    MediaProcessing,
    /// General-purpose computation
    General,
}

impl WorkloadType {
    pub fn default_target(&self) -> ComputeTarget {
        match self {
            Self::StringProcessing => ComputeTarget::Cpu,
            Self::MatrixComputation => ComputeTarget::Gpu,
            Self::VectorEmbedding => ComputeTarget::Gpu,
            Self::DataAggregation => ComputeTarget::Hybrid,
            Self::LlmInference => ComputeTarget::Gpu,
            Self::MediaProcessing => ComputeTarget::Gpu,
            Self::General => ComputeTarget::Cpu,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::StringProcessing => "String Processing",
            Self::MatrixComputation => "Matrix Computation",
            Self::VectorEmbedding => "Vector Embedding",
            Self::DataAggregation => "Data Aggregation",
            Self::LlmInference => "LLM Inference",
            Self::MediaProcessing => "Media Processing",
            Self::General => "General Purpose",
        }
    }
}

// ─── Hardware State ──────────────────────────────────────────────────────────

/// Current state of hardware resources.
#[derive(Debug, Clone)]
pub struct HardwareState {
    /// CPU utilization (0.0 - 1.0)
    pub cpu_utilization: f64,
    /// Available CPU cores
    pub cpu_cores_available: u32,
    /// Total system memory (bytes)
    pub system_memory_total: u64,
    /// Available system memory (bytes)
    pub system_memory_available: u64,
    /// Whether a GPU is present
    pub gpu_present: bool,
    /// GPU utilization (0.0 - 1.0)
    pub gpu_utilization: f64,
    /// Total GPU VRAM (bytes)
    pub gpu_vram_total: u64,
    /// Available GPU VRAM (bytes)
    pub gpu_vram_available: u64,
    /// Whether NVLink-C2C is available (Grace Hopper)
    pub nvlink_available: bool,
    /// NVLink bandwidth (GB/s)
    pub nvlink_bandwidth_gbs: f64,
    /// PCIe bandwidth (GB/s)
    pub pcie_bandwidth_gbs: f64,
    /// Active memory paradigm
    pub memory_paradigm: MemoryParadigm,
}

impl Default for HardwareState {
    fn default() -> Self {
        Self {
            cpu_utilization: 0.0,
            cpu_cores_available: num_cpus::get() as u32,
            system_memory_total: 16 * 1024 * 1024 * 1024, // 16GB default
            system_memory_available: 8 * 1024 * 1024 * 1024,
            gpu_present: false,
            gpu_utilization: 0.0,
            gpu_vram_total: 0,
            gpu_vram_available: 0,
            nvlink_available: false,
            nvlink_bandwidth_gbs: 0.0,
            pcie_bandwidth_gbs: 32.0, // PCIe Gen4 x16
            memory_paradigm: MemoryParadigm::PageableSystemMemory,
        }
    }
}

impl HardwareState {
    /// Detect hardware (simulated — real impl would use sysinfo/nvml).
    pub fn detect() -> Self {
        let cores = num_cpus::get() as u32;
        Self {
            cpu_cores_available: cores,
            ..Default::default()
        }
    }

    /// Simulate a Grace Hopper environment for testing.
    pub fn simulated_grace_hopper() -> Self {
        Self {
            cpu_utilization: 0.2,
            cpu_cores_available: 72, // Grace CPU: 72 Neoverse cores
            system_memory_total: 512 * 1024 * 1024 * 1024, // 512GB LPDDR5X
            system_memory_available: 400 * 1024 * 1024 * 1024,
            gpu_present: true,
            gpu_utilization: 0.1,
            gpu_vram_total: 96 * 1024 * 1024 * 1024, // 96GB HBM3
            gpu_vram_available: 80 * 1024 * 1024 * 1024,
            nvlink_available: true,
            nvlink_bandwidth_gbs: 900.0,
            pcie_bandwidth_gbs: 128.0, // PCIe Gen5 x16
            memory_paradigm: MemoryParadigm::HardwareCoherentUnified,
        }
    }
}

// ─── Routing Decision ────────────────────────────────────────────────────────

/// The result of the adaptive routing decision.
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub target: ComputeTarget,
    pub memory_paradigm: MemoryParadigm,
    pub reason: String,
    pub estimated_speedup: f64,
    pub confidence: f64,
}

// ─── Adaptive Execution Engine ───────────────────────────────────────────────

/// The adaptive execution engine that routes workloads dynamically.
/// Evaluates query complexity, dataset size, and hardware congestion
/// to determine optimal CPU/GPU/Hybrid execution.
pub struct AdaptiveEngine {
    pub hardware: HardwareState,
    pub stats: EngineStats,
    /// Thresholds for routing decisions
    pub config: EngineConfig,
}

/// Engine configuration thresholds.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Minimum data size (bytes) to consider GPU offloading
    pub gpu_min_data_size: u64,
    /// Minimum FLOP count to consider GPU offloading
    pub gpu_min_flops: u64,
    /// Maximum GPU utilization before fallback to CPU
    pub gpu_max_utilization: f64,
    /// Minimum available VRAM (bytes) for GPU execution
    pub gpu_min_vram_available: u64,
    /// CPU utilization threshold for hybrid routing
    pub cpu_hybrid_threshold: f64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            gpu_min_data_size: 1024 * 1024, // 1MB
            gpu_min_flops: 1_000_000,
            gpu_max_utilization: 0.9,
            gpu_min_vram_available: 512 * 1024 * 1024, // 512MB
            cpu_hybrid_threshold: 0.7,
        }
    }
}

/// Engine performance statistics.
#[derive(Debug)]
pub struct EngineStats {
    pub cpu_dispatches: AtomicU64,
    pub gpu_dispatches: AtomicU64,
    pub hybrid_dispatches: AtomicU64,
    pub fallback_dispatches: AtomicU64,
    pub total_compute_ns: AtomicU64,
    pub gpu_offload_bytes: AtomicU64,
}

impl EngineStats {
    pub fn new() -> Self {
        Self {
            cpu_dispatches: AtomicU64::new(0),
            gpu_dispatches: AtomicU64::new(0),
            hybrid_dispatches: AtomicU64::new(0),
            fallback_dispatches: AtomicU64::new(0),
            total_compute_ns: AtomicU64::new(0),
            gpu_offload_bytes: AtomicU64::new(0),
        }
    }

    pub fn total_dispatches(&self) -> u64 {
        self.cpu_dispatches.load(Ordering::Relaxed)
            + self.gpu_dispatches.load(Ordering::Relaxed)
            + self.hybrid_dispatches.load(Ordering::Relaxed)
            + self.fallback_dispatches.load(Ordering::Relaxed)
    }

    pub fn gpu_offload_rate(&self) -> f64 {
        let total = self.total_dispatches();
        if total == 0 { return 0.0; }
        let gpu = self.gpu_dispatches.load(Ordering::Relaxed)
            + self.hybrid_dispatches.load(Ordering::Relaxed);
        gpu as f64 / total as f64 * 100.0
    }
}

impl AdaptiveEngine {
    /// Create an engine with detected hardware.
    pub fn new() -> Self {
        Self {
            hardware: HardwareState::detect(),
            stats: EngineStats::new(),
            config: EngineConfig::default(),
        }
    }

    /// Create with specific hardware state.
    pub fn with_hardware(hw: HardwareState) -> Self {
        Self {
            hardware: hw,
            stats: EngineStats::new(),
            config: EngineConfig::default(),
        }
    }

    /// Route a workload to the optimal compute target.
    pub fn route(&self, profile: &WorkloadProfile) -> RoutingDecision {
        // Rule 1: String-heavy or branch-divergent → CPU
        if profile.string_heavy || profile.branch_divergent {
            return RoutingDecision {
                target: ComputeTarget::Cpu,
                memory_paradigm: self.hardware.memory_paradigm,
                reason: "String/branch-divergent workload → CPU preferred (SIMT warp divergence avoidance)".into(),
                estimated_speedup: 1.0,
                confidence: 0.95,
            };
        }

        // Rule 2: No GPU → CPU
        if !self.hardware.gpu_present {
            return RoutingDecision {
                target: ComputeTarget::Cpu,
                memory_paradigm: MemoryParadigm::PageableSystemMemory,
                reason: "No GPU detected → CPU-only execution".into(),
                estimated_speedup: 1.0,
                confidence: 1.0,
            };
        }

        // Rule 3: GPU overloaded → CPU fallback
        if self.hardware.gpu_utilization > self.config.gpu_max_utilization {
            return RoutingDecision {
                target: ComputeTarget::CpuFallback,
                memory_paradigm: self.hardware.memory_paradigm,
                reason: format!("GPU utilization {:.0}% exceeds threshold → CPU fallback", 
                    self.hardware.gpu_utilization * 100.0),
                estimated_speedup: 0.5,
                confidence: 0.8,
            };
        }

        // Rule 4: Insufficient VRAM → graceful degradation
        if profile.memory_required_bytes > self.hardware.gpu_vram_available {
            if self.hardware.nvlink_available {
                // Grace Hopper: use unified memory (no copy needed)
                return RoutingDecision {
                    target: ComputeTarget::Hybrid,
                    memory_paradigm: MemoryParadigm::HardwareCoherentUnified,
                    reason: "Data exceeds VRAM, NVLink available → hybrid with unified memory (900 GB/s)".into(),
                    estimated_speedup: 8.0,
                    confidence: 0.9,
                };
            } else {
                // Standard GPU: offload layers to CPU memory
                return RoutingDecision {
                    target: ComputeTarget::CpuFallback,
                    memory_paradigm: MemoryParadigm::SoftwareCoherentUnified,
                    reason: "Data exceeds VRAM, no NVLink → CPU fallback with cudaMallocManaged".into(),
                    estimated_speedup: 0.3,
                    confidence: 0.7,
                };
            }
        }

        // Rule 5: Matrix/vector work → GPU
        if profile.matrix_heavy || profile.workload_type == WorkloadType::VectorEmbedding {
            let speedup = if self.hardware.nvlink_available { 50.0 } else { 20.0 };
            return RoutingDecision {
                target: ComputeTarget::Gpu,
                memory_paradigm: self.hardware.memory_paradigm,
                reason: "Parallelizable matrix/vector workload → GPU offload".into(),
                estimated_speedup: speedup,
                confidence: 0.95,
            };
        }

        // Rule 6: Large parallelizable data → Hybrid
        if profile.parallelizable && profile.data_size_bytes > self.config.gpu_min_data_size {
            return RoutingDecision {
                target: ComputeTarget::Hybrid,
                memory_paradigm: self.hardware.memory_paradigm,
                reason: "Large parallelizable dataset → hybrid CPU+GPU execution".into(),
                estimated_speedup: 10.0,
                confidence: 0.85,
            };
        }

        // Rule 7: LLM inference
        if profile.workload_type == WorkloadType::LlmInference {
            return RoutingDecision {
                target: ComputeTarget::Gpu,
                memory_paradigm: self.hardware.memory_paradigm,
                reason: "LLM inference → GPU with dynamic layer offloading".into(),
                estimated_speedup: 100.0,
                confidence: 0.9,
            };
        }

        // Default: CPU
        RoutingDecision {
            target: ComputeTarget::Cpu,
            memory_paradigm: self.hardware.memory_paradigm,
            reason: "General workload below GPU thresholds → CPU execution".into(),
            estimated_speedup: 1.0,
            confidence: 0.7,
        }
    }

    /// Execute a workload (simulated — records stats).
    pub fn execute(&self, profile: &WorkloadProfile) -> ExecutionResult {
        let decision = self.route(profile);
        let start = Instant::now();

        // Record dispatch
        match decision.target {
            ComputeTarget::Cpu => self.stats.cpu_dispatches.fetch_add(1, Ordering::Relaxed),
            ComputeTarget::Gpu => {
                self.stats.gpu_dispatches.fetch_add(1, Ordering::Relaxed);
                self.stats.gpu_offload_bytes.fetch_add(profile.data_size_bytes, Ordering::Relaxed);
                0
            }
            ComputeTarget::Hybrid => self.stats.hybrid_dispatches.fetch_add(1, Ordering::Relaxed),
            ComputeTarget::CpuFallback => self.stats.fallback_dispatches.fetch_add(1, Ordering::Relaxed),
        };

        let elapsed = start.elapsed();
        self.stats.total_compute_ns.fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);

        ExecutionResult {
            workload_id: profile.id.clone(),
            target: decision.target,
            decision_reason: decision.reason,
            execution_time: elapsed,
            estimated_speedup: decision.estimated_speedup,
        }
    }
}

/// Result of workload execution.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub workload_id: String,
    pub target: ComputeTarget,
    pub decision_reason: String,
    pub execution_time: Duration,
    pub estimated_speedup: f64,
}

/// Print engine report.
pub fn print_engine_report(stats: &EngineStats, hw: &HardwareState) {
    use console::style;
    println!();
    println!("  {} {}", style("Adaptive Execution Engine").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
    println!("  {} Hardware: {} cores | {} | {}",
        style("🖥️").dim(), hw.cpu_cores_available,
        if hw.gpu_present { "GPU ✓" } else { "No GPU" },
        if hw.nvlink_available { "NVLink ✓" } else { "PCIe" });
    println!("  {} Memory paradigm: {}", style("▸").dim(), hw.memory_paradigm.label());
    println!("  {} CPU dispatches:      {}", style("🔵").dim(),
        stats.cpu_dispatches.load(Ordering::Relaxed));
    println!("  {} GPU dispatches:      {}", style("🟢").dim(),
        stats.gpu_dispatches.load(Ordering::Relaxed));
    println!("  {} Hybrid dispatches:   {}", style("🟡").dim(),
        stats.hybrid_dispatches.load(Ordering::Relaxed));
    println!("  {} Fallback dispatches: {}", style("🔴").dim(),
        stats.fallback_dispatches.load(Ordering::Relaxed));
    println!("  {} GPU offload rate:    {:.1}%", style("⚡").yellow(),
        stats.gpu_offload_rate());
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_only_engine() {
        let engine = AdaptiveEngine::new(); // No GPU detected
        let profile = WorkloadProfile {
            id: "test".into(), data_size_bytes: 1024,
            estimated_flops: 1000, parallelizable: false,
            string_heavy: false, matrix_heavy: false,
            branch_divergent: false, memory_required_bytes: 1024,
            workload_type: WorkloadType::General,
        };
        let decision = engine.route(&profile);
        assert_eq!(decision.target, ComputeTarget::Cpu);
    }

    #[test]
    fn test_string_routes_to_cpu() {
        let engine = AdaptiveEngine::with_hardware(HardwareState::simulated_grace_hopper());
        let profile = WorkloadProfile {
            id: "regex".into(), data_size_bytes: 100_000_000,
            estimated_flops: 10_000_000, parallelizable: true,
            string_heavy: true, matrix_heavy: false,
            branch_divergent: true, memory_required_bytes: 100_000_000,
            workload_type: WorkloadType::StringProcessing,
        };
        let decision = engine.route(&profile);
        assert_eq!(decision.target, ComputeTarget::Cpu);
    }

    #[test]
    fn test_matrix_routes_to_gpu() {
        let engine = AdaptiveEngine::with_hardware(HardwareState::simulated_grace_hopper());
        let profile = WorkloadProfile {
            id: "matmul".into(), data_size_bytes: 1024 * 1024 * 100,
            estimated_flops: 1_000_000_000, parallelizable: true,
            string_heavy: false, matrix_heavy: true,
            branch_divergent: false, memory_required_bytes: 1024 * 1024 * 100,
            workload_type: WorkloadType::MatrixComputation,
        };
        let decision = engine.route(&profile);
        assert_eq!(decision.target, ComputeTarget::Gpu);
        assert!(decision.estimated_speedup > 10.0);
    }

    #[test]
    fn test_nvlink_hybrid_fallback() {
        let engine = AdaptiveEngine::with_hardware(HardwareState::simulated_grace_hopper());
        let profile = WorkloadProfile {
            id: "huge-llm".into(),
            data_size_bytes: 200 * 1024 * 1024 * 1024, // 200GB
            estimated_flops: 10_000_000_000,
            parallelizable: true, string_heavy: false, matrix_heavy: true,
            branch_divergent: false,
            memory_required_bytes: 200 * 1024 * 1024 * 1024, // exceeds 80GB VRAM
            workload_type: WorkloadType::LlmInference,
        };
        let decision = engine.route(&profile);
        assert_eq!(decision.target, ComputeTarget::Hybrid);
        assert_eq!(decision.memory_paradigm, MemoryParadigm::HardwareCoherentUnified);
    }

    #[test]
    fn test_memory_paradigms() {
        let gh = MemoryParadigm::HardwareCoherentUnified;
        assert!(gh.bandwidth().contains("900"));
        assert!(gh.optimal_hardware().contains("Grace Hopper"));
    }

    #[test]
    fn test_engine_stats() {
        let stats = EngineStats::new();
        stats.cpu_dispatches.store(50, Ordering::Relaxed);
        stats.gpu_dispatches.store(30, Ordering::Relaxed);
        stats.hybrid_dispatches.store(20, Ordering::Relaxed);
        assert_eq!(stats.total_dispatches(), 100);
        assert!((stats.gpu_offload_rate() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_execute_records_stats() {
        let engine = AdaptiveEngine::new();
        let profile = WorkloadProfile {
            id: "exec-test".into(), data_size_bytes: 1024,
            estimated_flops: 100, parallelizable: false,
            string_heavy: false, matrix_heavy: false,
            branch_divergent: false, memory_required_bytes: 1024,
            workload_type: WorkloadType::General,
        };
        let result = engine.execute(&profile);
        assert_eq!(result.workload_id, "exec-test");
        assert_eq!(engine.stats.total_dispatches(), 1);
    }
}
