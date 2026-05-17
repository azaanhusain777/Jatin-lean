//! Memory & IPC optimization commands.

use clap::Subcommand;
use anyhow::Result;
use console::style;
use crate::output::OutputContext;

#[derive(Subcommand, Debug)]
pub enum MemoryCommands {
    /// Shared memory IPC ring buffer benchmark
    Ipc {
        /// Run the SPSC ring buffer throughput benchmark
        #[arg(long)]
        bench: bool,

        /// Ring buffer capacity (slots)
        #[arg(long, default_value = "1024")]
        capacity: usize,

        /// Number of messages to send
        #[arg(long, default_value = "100000")]
        messages: u64,

        /// Show memory layout info
        #[arg(long)]
        layout: bool,
    },

    /// mmap-backed SPSC ring buffer IPC benchmark
    Mmap {
        /// Run IPC throughput benchmark
        #[arg(long)]
        bench: bool,

        /// Ring buffer capacity (slots)
        #[arg(long, default_value = "4096")]
        capacity: usize,

        /// Message size in bytes
        #[arg(long, default_value = "256")]
        msg_size: usize,

        /// Show FFI comparison table
        #[arg(long)]
        compare: bool,
    },

    /// Arena memory pool allocator benchmark
    Arena {
        /// Run allocation benchmark
        #[arg(long)]
        bench: bool,

        /// Arena capacity in KB
        #[arg(long, default_value = "1024")]
        capacity_kb: usize,

        /// Number of allocations
        #[arg(long, default_value = "100000")]
        allocations: u64,
    },

    /// PCIe bottleneck quantifier & CUDA memory analysis
    Pcie {
        /// Compare all memory paradigms
        #[arg(long)]
        compare: bool,

        /// Data transfer size in GB
        #[arg(long, default_value = "1")]
        size_gb: u64,

        /// Simulate LLM layer offloading
        #[arg(long, value_name = "LAYERS")]
        offload: Option<usize>,

        /// Use Grace Hopper (NVLink-C2C) profile
        #[arg(long)]
        grace_hopper: bool,
    },
}

pub fn handle_command(command: MemoryCommands, ctx: &OutputContext) -> Result<()> {
    if !ctx.json { crate::display::print_banner(); }

    match command {
        MemoryCommands::Ipc { bench, capacity, messages, layout } => {
            use crate::shared_memory_ipc::*;
            
            if !ctx.json {
                println!("  {} {}", style("Lock-Free Shared Memory IPC").cyan().bold(),
                    style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
            }

            let mut json_out = serde_json::Map::new();

            if layout {
                if ctx.json {
                    let region_size = SharedMemoryRegion::required_size(capacity, MessageSlot::SIZE);
                    json_out.insert("layout".to_string(), serde_json::json!({
                        "cache_line_size": CACHE_LINE_SIZE,
                        "aligned_atomic_index_size": std::mem::size_of::<AlignedAtomicIndex>(),
                        "spsc_ring_header_size": std::mem::size_of::<SpscRingHeader>(),
                        "message_slot_size": MessageSlot::SIZE,
                        "max_payload_per_slot": MessageSlot::MAX_PAYLOAD,
                        "shared_memory_needed_bytes": region_size,
                        "capacity_slots": capacity,
                    }));
                } else {
                    println!();
                    println!("  {} Memory Layout:", style("📐").yellow());
                    println!("    Cache line size:       {} bytes", CACHE_LINE_SIZE);
                    println!("    AlignedAtomicIndex:    {} bytes (aligned to {})",
                        std::mem::size_of::<AlignedAtomicIndex>(), std::mem::align_of::<AlignedAtomicIndex>());
                    println!("    SpscRingHeader:        {} bytes", std::mem::size_of::<SpscRingHeader>());
                    println!("    MessageSlot:           {} bytes ({}KB)", MessageSlot::SIZE, MessageSlot::SIZE / 1024);
                    println!("    Max payload per slot:  {} bytes", MessageSlot::MAX_PAYLOAD);
                    let region_size = SharedMemoryRegion::required_size(capacity, MessageSlot::SIZE);
                    println!("    Shared memory needed:  {} bytes ({:.1} MB) for {} slots",
                        region_size, region_size as f64 / (1024.0 * 1024.0), capacity);
                    println!();
                }
            }

            if bench {
                if !ctx.json {
                    println!("  {} SPSC Ring Buffer Benchmark: {} msgs, {} slots",
                        style("⚡").yellow(), style(messages).green().bold(), style(capacity).white());
                }

                let ring = SpscIpcRing::new(capacity);
                let payload = b"benchmark-payload-data-for-ipc-testing-12345678";
                let start = std::time::Instant::now();

                for i in 0..messages {
                    while ring.push(1, payload).is_err() { ring.pop(); }
                    if i % 2 == 0 { ring.pop(); }
                }
                while ring.pop().is_some() {}

                let elapsed = start.elapsed();
                
                if ctx.json {
                    json_out.insert("benchmark".to_string(), serde_json::json!({
                        "messages": messages,
                        "capacity": capacity,
                        "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
                        "ops_per_sec": messages as f64 / elapsed.as_secs_f64(),
                    }));
                } else {
                    print_ipc_report(&ring.stats, elapsed);
                    println!("  {} Total elapsed: {:.2} ms",
                        style("▸").dim(), elapsed.as_secs_f64() * 1000.0);
                    println!("  {} Ops/sec: {:.0}",
                        style("🚀").yellow(), messages as f64 / elapsed.as_secs_f64());
                    println!();
                }
            }

            if !bench && !layout {
                if !ctx.json {
                    println!();
                    println!("  {} Use {} for memory layout info",
                        style("→").dim(), style("jatin-lean memory ipc --layout").yellow());
                    println!("  {} Use {} for throughput benchmark",
                        style("→").dim(), style("jatin-lean memory ipc --bench").yellow());
                    println!();
                }
            }
            
            if ctx.json {
                crate::output::output_result("memory ipc", &serde_json::Value::Object(json_out), ctx)?;
            }
            Ok(())
        }

        MemoryCommands::Mmap { bench, capacity, msg_size, compare } => {
            use crate::mmap_ipc::*;
            
            if !ctx.json {
                println!("  {} {}", style("mmap Ring Buffer IPC Engine").cyan().bold(),
                    style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
            }

            let mut json_out = serde_json::Map::new();

            if compare { 
                if !ctx.json { print_ffi_comparison(); }
            }

            if bench {
                let ring = MmapRingBuffer::new(capacity, msg_size);
                let msg = vec![42u8; msg_size];

                if !ctx.json {
                    println!("  {} Benchmark: {} slots × {} byte messages",
                        style("⚡").yellow(), style(capacity).green().bold(), style(msg_size).white());
                }

                let start = std::time::Instant::now();
                let mut writes = 0u64;
                for _ in 0..capacity { if ring.write(&msg) { writes += 1; } }
                let batch = ring.read_batch(capacity);
                let reads = batch.len() as u64;
                let config = BatchProcessorConfig::default();
                let result = process_batch_parallel(&batch, &config);
                let elapsed = start.elapsed();
                let ipc_latency = if writes > 0 { elapsed.as_nanos() as f64 / writes as f64 } else { 0.0 };

                if ctx.json {
                    json_out.insert("benchmark".to_string(), serde_json::json!({
                        "written": writes,
                        "read": reads,
                        "batch_throughput_msg_per_sec": result.throughput_msg_per_sec,
                        "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
                        "ipc_latency_ns_per_msg": ipc_latency,
                    }));
                } else {
                    println!("  {} Written: {} | Read: {} (batch)", style("▸").dim(), writes, reads);
                    println!("  {} Batch throughput: {:.0} msg/s",
                        style("🚀").yellow(), style(format!("{:.0}", result.throughput_msg_per_sec)).green().bold());
                    println!("  {} Total elapsed: {:.2} ms",
                        style("▸").dim(), elapsed.as_secs_f64() * 1000.0);
                    println!("  {} IPC latency: {:.0} ns/msg (vs 50,000 ns JSON-over-HTTP)",
                        style("⚡").yellow(), style(format!("{:.0}", ipc_latency)).green().bold());
                    print_mmap_report(&ring.stats);
                }
            }

            if !bench && !compare {
                if !ctx.json {
                    println!();
                    println!("  {} Use {} for throughput benchmark", style("→").dim(),
                        style("jatin-lean memory mmap --bench").yellow());
                    println!("  {} Use {} for FFI comparison", style("→").dim(),
                        style("jatin-lean memory mmap --compare").yellow());
                    println!();
                }
            }
            
            if ctx.json {
                crate::output::output_result("memory mmap", &serde_json::Value::Object(json_out), ctx)?;
            }
            Ok(())
        }

        MemoryCommands::Arena { bench, capacity_kb, allocations } => {
            use crate::memory_pool::*;
            
            if !ctx.json {
                println!("  {} {}", style("Arena Memory Pool Allocator").cyan().bold(),
                    style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
            }

            let mut json_out = serde_json::Map::new();

            let capacity = capacity_kb * 1024;
            let arena = crate::memory_pool::Arena::new(capacity);

            if bench {
                if !ctx.json {
                    println!("  {} Benchmark: {} allocations in {} KB arena",
                        style("⚡").yellow(), style(allocations).green().bold(), style(capacity_kb).white());
                }

                let start = std::time::Instant::now();
                let mut success = 0u64;
                for _ in 0..allocations {
                    if arena.alloc(32, 8).is_some() { success += 1; }
                }
                let elapsed = start.elapsed();

                let pool = TypedPool::<ScanEntry>::new(allocations as usize);
                let start2 = std::time::Instant::now();
                for i in 0..allocations {
                    pool.alloc_init(ScanEntry::new(&format!("file-{}.js", i), 1024, true, 1, 2));
                }
                let elapsed2 = start2.elapsed();
                
                if ctx.json {
                    json_out.insert("benchmark".to_string(), serde_json::json!({
                        "allocations_requested": allocations,
                        "allocations_successful": success,
                        "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
                        "allocations_per_sec": success as f64 / elapsed.as_secs_f64(),
                        "avg_alloc_ns": elapsed.as_nanos() as f64 / success.max(1) as f64,
                        "typed_pool_ns_per_alloc": elapsed2.as_nanos() as f64 / allocations as f64,
                    }));
                } else {
                    println!("  {} Successful: {}/{}", style("▸").dim(), success, allocations);
                    println!("  {} Elapsed:    {:.2} ms", style("▸").dim(), elapsed.as_secs_f64() * 1000.0);
                    println!("  {} Allocs/sec: {:.0}", style("🚀").yellow(), success as f64 / elapsed.as_secs_f64());
                    println!("  {} Avg alloc:  {:.0} ns", style("⚡").yellow(),
                        elapsed.as_nanos() as f64 / success.max(1) as f64);
                    println!("  {} TypedPool:  {:.0} ns/alloc",
                        style("⚡").yellow(), elapsed2.as_nanos() as f64 / allocations as f64);
                }
            }

            if ctx.json {
                crate::output::output_result("memory arena", &serde_json::Value::Object(json_out), ctx)?;
            } else {
                println!();
                print_arena_report(&arena);
            }
            Ok(())
        }

        MemoryCommands::Pcie { compare, size_gb, offload, grace_hopper } => {
            use crate::pcie_bottleneck::*;
            
            if !ctx.json {
                println!("  {} {}", style("PCIe & CUDA Memory Analysis").cyan().bold(),
                    style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
            }

            let mut json_out = serde_json::Map::new();
            let interconnect = if grace_hopper { PcieGen::NvLinkC2C } else { PcieGen::Gen5 };
            let data_bytes = size_gb * 1024 * 1024 * 1024;

            if compare {
                let mem_types = [CudaMemoryType::Pageable, CudaMemoryType::Pinned,
                    CudaMemoryType::UnifiedManaged, CudaMemoryType::HardwareCoherent];
                let sims: Vec<TransferSimulation> = mem_types.iter()
                    .map(|mt| simulate_transfer(*mt, interconnect, data_bytes)).collect();
                
                if ctx.json {
                    let mut sims_json = Vec::new();
                    for sim in &sims {
                        sims_json.push(serde_json::json!({
                            "memory_type": sim.mem_type.label(),
                            "interconnect": sim.interconnect.label(),
                            "transfer_time_us": sim.transfer_time_us,
                            "effective_bandwidth_gbps": sim.effective_bandwidth_gbps,
                            "first_access_latency_ns": sim.first_access_latency_ns,
                        }));
                    }
                    json_out.insert("compare".to_string(), serde_json::Value::Array(sims_json));
                } else {
                    print_pcie_report(&sims);
                }
            }

            if let Some(num_layers) = offload {
                let mut ctrl = if grace_hopper {
                    VramOffloadController::grace_hopper()
                } else {
                    VramOffloadController::discrete_gpu()
                };
                let layers: Vec<(String, u64)> = (0..num_layers)
                    .map(|i| (format!("transformer.layer.{}", i), 2u64 * 1024 * 1024 * 1024)).collect();
                let plan = ctrl.place_layers(&layers);
                
                if ctx.json {
                    let mut plan_json = Vec::new();
                    for layer in &plan {
                        plan_json.push(serde_json::json!({
                            "layer": layer.layer_name,
                            "location": match layer.placement {
                                LayerPlacement::Vram => "vram",
                                LayerPlacement::SystemRam => "system_ram",
                                LayerPlacement::NvLinkUnified => "nvlink_unified",
                            },
                        }));
                    }
                    json_out.insert("offload".to_string(), serde_json::Value::Array(plan_json));
                } else {
                    print_offload_report(&plan);
                }
            }

            if !compare && offload.is_none() {
                let sim = simulate_transfer(
                    if grace_hopper { CudaMemoryType::HardwareCoherent } else { CudaMemoryType::Pinned },
                    interconnect, data_bytes);
                    
                if ctx.json {
                    json_out.insert("simulation".to_string(), serde_json::json!({
                        "memory_type": sim.mem_type.label(),
                        "interconnect": sim.interconnect.label(),
                        "size_gb": size_gb,
                        "transfer_time_us": sim.transfer_time_us,
                        "effective_bandwidth_gbps": sim.effective_bandwidth_gbps,
                        "first_access_latency_ns": sim.first_access_latency_ns,
                    }));
                } else {
                    println!();
                    println!("  {} {} via {} | {:.1} GB transfer",
                        style("▸").dim(), style(sim.mem_type.label()).yellow(),
                        sim.interconnect.label(), size_gb);
                    println!("  {} Time: {:.1} µs | BW: {:.1} GB/s | Latency: {:.0} ns",
                        style("⚡").yellow(), sim.transfer_time_us,
                        sim.effective_bandwidth_gbps, sim.first_access_latency_ns);
                    println!();
                }
            }
            
            if ctx.json {
                crate::output::output_result("memory pcie", &serde_json::Value::Object(json_out), ctx)?;
            }
            Ok(())
        }
    }
}
