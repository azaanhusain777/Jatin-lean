//! BPF Verifier Simulation & Deep Packet Inspection Engine
//!
//! From Sections 1.1-1.2: Simulates the eBPF verifier's instruction count
//! limits, loop unrolling enforcement, and DPI (Deep Packet Inspection)
//! evasion detection. Also models sk_buff elimination savings.

/// Maximum eBPF instruction count allowed by the verifier.
pub const BPF_MAX_INSNS: usize = 1_000_000;
/// Maximum nested calls depth.
pub const BPF_MAX_CALL_DEPTH: usize = 8;
/// Maximum loops (bounded, since Linux 5.3).
pub const BPF_MAX_LOOP_ITERATIONS: u32 = 8192;

// ─── BPF Verifier Simulation ─────────────────────────────────────────────────

/// BPF program type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfProgType {
    XdpIngress,
    TcIngress,
    TcEgress,
    CgroupSkb,
    SocketFilter,
    Tracepoint,
}

impl BpfProgType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::XdpIngress => "XDP (Ingress)",
            Self::TcIngress => "TC (Ingress)",
            Self::TcEgress => "TC (Egress)",
            Self::CgroupSkb => "Cgroup SKB",
            Self::SocketFilter => "Socket Filter",
            Self::Tracepoint => "Tracepoint",
        }
    }
}

/// A simulated BPF program for verifier analysis.
#[derive(Debug, Clone)]
pub struct BpfProgram {
    pub name: String,
    pub prog_type: BpfProgType,
    pub instruction_count: usize,
    pub max_loop_depth: u32,
    pub call_depth: usize,
    pub map_accesses: usize,
    pub packet_accesses: usize,
    pub tail_calls: usize,
    pub helper_calls: Vec<String>,
}

/// Verifier result.
#[derive(Debug, Clone)]
pub struct VerifierResult {
    pub accepted: bool,
    pub reason: String,
    pub insn_utilization: f64,
    pub complexity_score: u32,
    pub warnings: Vec<String>,
}

impl BpfProgram {
    /// Run the simulated BPF verifier.
    pub fn verify(&self) -> VerifierResult {
        let mut warnings = Vec::new();

        if self.instruction_count > BPF_MAX_INSNS {
            return VerifierResult {
                accepted: false,
                reason: format!(
                    "Program exceeds {} instruction limit ({} insns)",
                    BPF_MAX_INSNS, self.instruction_count
                ),
                insn_utilization: self.instruction_count as f64 / BPF_MAX_INSNS as f64 * 100.0,
                complexity_score: 100,
                warnings,
            };
        }

        if self.call_depth > BPF_MAX_CALL_DEPTH {
            return VerifierResult {
                accepted: false,
                reason: format!(
                    "Call depth {} exceeds max {}",
                    self.call_depth, BPF_MAX_CALL_DEPTH
                ),
                insn_utilization: 0.0,
                complexity_score: 100,
                warnings,
            };
        }

        if self.max_loop_depth > BPF_MAX_LOOP_ITERATIONS {
            return VerifierResult {
                accepted: false,
                reason: format!(
                    "Unbounded loop detected ({} iterations)",
                    self.max_loop_depth
                ),
                insn_utilization: 0.0,
                complexity_score: 100,
                warnings,
            };
        }

        if self.instruction_count > BPF_MAX_INSNS / 2 {
            warnings.push("Program uses >50% of instruction budget".into());
        }
        if self.tail_calls > 32 {
            warnings.push("Excessive tail calls may cause chain depth issues".into());
        }

        let complexity = ((self.instruction_count as f64 / BPF_MAX_INSNS as f64) * 40.0
            + (self.call_depth as f64 / BPF_MAX_CALL_DEPTH as f64) * 30.0
            + (self.map_accesses as f64 / 100.0).min(1.0) * 30.0) as u32;

        VerifierResult {
            accepted: true,
            reason: "Program accepted by BPF verifier".into(),
            insn_utilization: self.instruction_count as f64 / BPF_MAX_INSNS as f64 * 100.0,
            complexity_score: complexity,
            warnings,
        }
    }
}

// ─── DPI Detection Engine ────────────────────────────────────────────────────

/// DPI evasion technique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpiEvasion {
    /// Standard — no evasion (plaintext protocols)
    None,
    /// UDP→TCP header rewrite (12-byte extension)
    UdpToTcpRewrite,
    /// TLS 1.3 record layer mimicry
    TlsRecordMimicry,
    /// HTTP chunked encoding injection
    HttpChunkedInject,
    /// DNS-over-HTTPS tunneling
    DohTunnel,
    /// QUIC IETF header camouflage
    QuicCamouflage,
}

impl DpiEvasion {
    pub fn overhead_bytes(&self) -> usize {
        match self {
            Self::None => 0,
            Self::UdpToTcpRewrite => 12,
            Self::TlsRecordMimicry => 37,
            Self::HttpChunkedInject => 64,
            Self::DohTunnel => 48,
            Self::QuicCamouflage => 28,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::UdpToTcpRewrite => "UDP→TCP Rewrite",
            Self::TlsRecordMimicry => "TLS 1.3 Mimicry",
            Self::HttpChunkedInject => "HTTP Chunked Inject",
            Self::DohTunnel => "DNS-over-HTTPS Tunnel",
            Self::QuicCamouflage => "QUIC Camouflage",
        }
    }

    pub fn all() -> &'static [DpiEvasion] {
        &[
            Self::None,
            Self::UdpToTcpRewrite,
            Self::TlsRecordMimicry,
            Self::HttpChunkedInject,
            Self::DohTunnel,
            Self::QuicCamouflage,
        ]
    }

    /// Can this evasion bypass the given DPI method?
    pub fn bypasses(&self, dpi: &DpiMethod) -> bool {
        match (self, dpi) {
            (Self::None, _) => false,
            (Self::UdpToTcpRewrite, DpiMethod::ProtocolWhitelist) => true,
            (Self::TlsRecordMimicry, DpiMethod::SniInspection) => true,
            (Self::TlsRecordMimicry, DpiMethod::PayloadSignature) => true,
            (Self::HttpChunkedInject, DpiMethod::PayloadSignature) => true,
            (Self::DohTunnel, DpiMethod::DnsFilter) => true,
            (Self::QuicCamouflage, DpiMethod::ProtocolWhitelist) => true,
            _ => false,
        }
    }
}

/// DPI method used by firewalls.
#[derive(Debug, Clone, Copy)]
pub enum DpiMethod {
    ProtocolWhitelist,
    SniInspection,
    PayloadSignature,
    DnsFilter,
    StatisticalAnalysis,
}

// ─── sk_buff Elimination Model ───────────────────────────────────────────────

/// Model the cost savings of bypassing sk_buff allocation.
#[derive(Debug, Clone)]
pub struct SkbuffModel {
    /// Size of sk_buff struct (bytes)
    pub skb_size: usize,
    /// Cost of allocating one sk_buff (ns)
    pub alloc_cost_ns: f64,
    /// Cost of freeing one sk_buff (ns)
    pub free_cost_ns: f64,
    /// L2 cache pollution per sk_buff (bytes)
    pub cache_pollution_bytes: usize,
}

impl Default for SkbuffModel {
    fn default() -> Self {
        Self {
            skb_size: 256,       // sizeof(sk_buff) on Linux 6.x
            alloc_cost_ns: 80.0, // SLAB allocator overhead
            free_cost_ns: 60.0,
            cache_pollution_bytes: 320, // sk_buff + data pointers
        }
    }
}

impl SkbuffModel {
    /// Calculate savings for N packets bypassing sk_buff via XDP.
    pub fn savings(&self, packet_count: u64) -> SkbuffSavings {
        let alloc_ns_saved = packet_count as f64 * self.alloc_cost_ns;
        let free_ns_saved = packet_count as f64 * self.free_cost_ns;
        let memory_saved = packet_count * self.skb_size as u64;
        let cache_saved = packet_count * self.cache_pollution_bytes as u64;

        SkbuffSavings {
            total_ns_saved: alloc_ns_saved + free_ns_saved,
            memory_bytes_saved: memory_saved,
            cache_bytes_saved: cache_saved,
            equivalent_throughput_gain_pct: (alloc_ns_saved + free_ns_saved)
                / (packet_count as f64 * 500.0)
                * 100.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkbuffSavings {
    pub total_ns_saved: f64,
    pub memory_bytes_saved: u64,
    pub cache_bytes_saved: u64,
    pub equivalent_throughput_gain_pct: f64,
}

pub fn print_verifier_report(prog: &BpfProgram, result: &VerifierResult) {
    use console::style;
    println!();
    println!(
        "  {} {}",
        style("BPF Verifier Report").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );
    let status = if result.accepted {
        style("ACCEPTED ✓").green().bold()
    } else {
        style("REJECTED ✗").red().bold()
    };
    println!(
        "  {} Program '{}' [{}]: {}",
        style("▸").dim(),
        style(&prog.name).yellow(),
        prog.prog_type.label(),
        status
    );
    println!(
        "  {} Instructions: {}/{} ({:.1}% budget)",
        style("▸").dim(),
        prog.instruction_count,
        BPF_MAX_INSNS,
        result.insn_utilization
    );
    println!(
        "  {} Call depth: {}/{} | Maps: {} | Tail calls: {}",
        style("▸").dim(),
        prog.call_depth,
        BPF_MAX_CALL_DEPTH,
        prog.map_accesses,
        prog.tail_calls
    );
    println!(
        "  {} Complexity score: {}/100",
        style("▸").dim(),
        result.complexity_score
    );
    if !result.warnings.is_empty() {
        for w in &result.warnings {
            println!("  {} {}", style("⚠").yellow(), w);
        }
    }
    println!("  {} {}", style("▸").dim(), result.reason);
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_accept() {
        let prog = BpfProgram {
            name: "xdp_pass".into(),
            prog_type: BpfProgType::XdpIngress,
            instruction_count: 500,
            max_loop_depth: 100,
            call_depth: 2,
            map_accesses: 5,
            packet_accesses: 10,
            tail_calls: 0,
            helper_calls: vec!["bpf_xdp_adjust_head".into()],
        };
        assert!(prog.verify().accepted);
    }

    #[test]
    fn test_verifier_reject_insns() {
        let prog = BpfProgram {
            name: "too_big".into(),
            prog_type: BpfProgType::XdpIngress,
            instruction_count: BPF_MAX_INSNS + 1,
            max_loop_depth: 0,
            call_depth: 0,
            map_accesses: 0,
            packet_accesses: 0,
            tail_calls: 0,
            helper_calls: vec![],
        };
        assert!(!prog.verify().accepted);
    }

    #[test]
    fn test_dpi_bypass() {
        assert!(DpiEvasion::UdpToTcpRewrite.bypasses(&DpiMethod::ProtocolWhitelist));
        assert!(!DpiEvasion::None.bypasses(&DpiMethod::ProtocolWhitelist));
    }

    #[test]
    fn test_skbuff_savings() {
        let model = SkbuffModel::default();
        let savings = model.savings(1_000_000);
        assert!(savings.total_ns_saved > 0.0);
        assert!(savings.memory_bytes_saved > 0);
    }
}
