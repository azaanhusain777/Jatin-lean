//! System-level optimization commands.

use clap::Subcommand;
use std::path::PathBuf;
use anyhow::Result;
use console::style;
use crate::output::OutputContext;

#[derive(Subcommand, Debug)]
pub enum SystemCommands {
    /// System hardware optimization assessment
    Optimize {
        /// Run full system assessment
        #[arg(long)]
        assess: bool,

        /// Show NUMA topology
        #[arg(long)]
        numa: bool,

        /// Show network tuning recommendations
        #[arg(long)]
        network: bool,

        /// Show kernel parameter tuning
        #[arg(long)]
        kernel: bool,

        /// Generate all sysctl commands
        #[arg(long)]
        generate: bool,
    },

    /// CPU cache optimization and prefetch benchmark
    CpuCache {
        /// Run cache-optimized scan benchmark
        #[arg(long)]
        bench: bool,

        /// Working set size in KB
        #[arg(long, default_value = "8192")]
        working_set_kb: usize,

        /// Show cache hierarchy info
        #[arg(long)]
        info: bool,

        /// Analyze TLB pressure
        #[arg(long)]
        tlb: bool,
    },

    /// Show I/O statistics and filesystem info
    Io {
        /// Path to the project directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Show filesystem info
        #[arg(long)]
        fs_info: bool,

        /// Show process resource usage
        #[arg(long)]
        process: bool,
    },
}

pub fn handle_command(command: SystemCommands, ctx: &OutputContext) -> Result<()> {
    if !ctx.json { crate::display::print_banner(); }

    match command {
        SystemCommands::Optimize { assess, numa, network, kernel, generate } => {
            use crate::hardware_tuning::*;
            
            if !ctx.json {
                println!("  {} {}", style("System Hardware Optimization").cyan().bold(),
                    style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
            }

            let mut json_out = serde_json::Map::new();

            if numa {
                let topo = NumaTopology::detect();
                if ctx.json {
                    json_out.insert("numa".to_string(), serde_json::json!({
                        "nodes": topo.nodes.len(),
                        "cores": topo.total_cores,
                        "is_numa": topo.is_numa,
                    }));
                } else {
                    println!();
                    println!("  {} NUMA Topology:", style("🖥️").yellow());
                    println!("    Nodes: {} | Cores: {} | NUMA: {}",
                        topo.nodes.len(), topo.total_cores,
                        if topo.is_numa { style("YES").green().bold() } else { style("no").dim() });
                    for node in &topo.nodes {
                        println!("    Node {}: {} CPUs, {} MB total ({} MB free)",
                            node.id, node.cpu_list.len(), node.memory_total_mb, node.memory_free_mb);
                    }
                    println!();
                }
            }

            if network {
                let tuning = NetworkTuning::default();
                if ctx.json {
                    json_out.insert("network".to_string(), serde_json::json!({
                        "tcp_nodelay": tuning.tcp_nodelay,
                        "tcp_quickack": tuning.tcp_quickack,
                        "so_reuseport": tuning.so_reuseport,
                        "tcp_fastopen": tuning.tcp_fastopen,
                        "recv_buffer_size": tuning.recv_buffer_size,
                        "send_buffer_size": tuning.send_buffer_size,
                        "somaxconn": tuning.somaxconn,
                        "tcp_fin_timeout": tuning.tcp_fin_timeout,
                    }));
                } else {
                    println!();
                    println!("  {} Recommended Network Settings:", style("🌐").yellow());
                    println!("    TCP_NODELAY:      {}", if tuning.tcp_nodelay { "✓" } else { "✗" });
                    println!("    TCP_QUICKACK:     {}", if tuning.tcp_quickack { "✓" } else { "✗" });
                    println!("    SO_REUSEPORT:     {}", if tuning.so_reuseport { "✓" } else { "✗" });
                    println!("    TCP_FASTOPEN:     {}", if tuning.tcp_fastopen { "✓" } else { "✗" });
                    println!("    Recv buffer:      {} MB", tuning.recv_buffer_size / 1024 / 1024);
                    println!("    Send buffer:      {} MB", tuning.send_buffer_size / 1024 / 1024);
                    println!("    somaxconn:        {}", tuning.somaxconn);
                    println!("    FIN timeout:      {}s", tuning.tcp_fin_timeout);
                    println!();
                }
            }

            if kernel {
                let tuning = KernelTuning::scan_optimized();
                if ctx.json {
                    json_out.insert("kernel".to_string(), serde_json::json!({
                        "fs_file_max": tuning.fs_file_max,
                        "fs_inotify_max_user_watches": tuning.fs_inotify_max_user_watches,
                        "vm_dirty_ratio": tuning.vm_dirty_ratio,
                        "vm_swappiness": tuning.vm_swappiness,
                        "vfs_cache_pressure": tuning.vfs_cache_pressure,
                        "transparent_hugepages": tuning.transparent_hugepages,
                    }));
                } else {
                    println!();
                    println!("  {} Kernel Parameters (scan-optimized):", style("🔧").yellow());
                    println!("    fs.file-max:        {}", tuning.fs_file_max);
                    println!("    inotify watches:    {}", tuning.fs_inotify_max_user_watches);
                    println!("    vm.dirty_ratio:     {}%", tuning.vm_dirty_ratio);
                    println!("    vm.swappiness:      {}", tuning.vm_swappiness);
                    println!("    vfs_cache_pressure: {}", tuning.vfs_cache_pressure);
                    println!("    THP:                {}", if tuning.transparent_hugepages { "enabled" } else { "disabled" });
                    println!();
                }
            }

            if generate {
                let net = NetworkTuning::default();
                let kern = KernelTuning::scan_optimized();
                if ctx.json {
                    json_out.insert("generate".to_string(), serde_json::json!({
                        "network": net.to_sysctl_commands(),
                        "kernel": kern.to_sysctl_commands(),
                    }));
                } else {
                    println!();
                    println!("  {} Generated sysctl commands:", style("📜").yellow());
                    println!("  {} Copy and run with sudo:", style("▸").dim());
                    println!();
                    for cmd in net.to_sysctl_commands() { println!("    {}", style(&cmd).yellow()); }
                    for cmd in kern.to_sysctl_commands() { println!("    {}", style(&cmd).yellow()); }
                    println!();
                }
            }

            if assess || (!numa && !network && !kernel && !generate) {
                let assessment = assess_system();
                if ctx.json {
                    json_out.insert("assessment".to_string(), serde_json::json!({
                        "current_score": assessment.current_score,
                        "optimized_score": assessment.optimized_score,
                    }));
                } else {
                    print_system_report(&assessment);
                }
            }
            
            if ctx.json {
                crate::output::output_result("system optimize", &serde_json::Value::Object(json_out), ctx)?;
            }
            Ok(())
        }

        SystemCommands::CpuCache { bench, working_set_kb, info, tlb } => {
            use crate::cpu_cache::*;
            if !ctx.json {
                println!("  {} {}", style("CPU Cache Optimization Engine").cyan().bold(),
                    style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
            }

            let hierarchy = CacheHierarchy::detect();
            let mut json_out = serde_json::Map::new();

            if info {
                if ctx.json {
                    json_out.insert("hierarchy".to_string(), serde_json::json!({
                        "l1d_size": hierarchy.l1d_size,
                        "l1d_latency_ns": hierarchy.l1d_latency_ns,
                        "l1i_size": hierarchy.l1i_size,
                        "l2_size": hierarchy.l2_size,
                        "l2_latency_ns": hierarchy.l2_latency_ns,
                        "l3_size": hierarchy.l3_size,
                        "l3_latency_ns": hierarchy.l3_latency_ns,
                        "main_memory_ns": hierarchy.main_memory_ns,
                        "cache_line_size": hierarchy.cache_line_size,
                        "tlb_entries": hierarchy.tlb_entries,
                    }));
                } else {
                    println!();
                    println!("  {} Detected Cache Hierarchy:", style("🖥️").yellow());
                    println!("    L1D:     {}KB ({:.0}ns)", hierarchy.l1d_size / 1024, hierarchy.l1d_latency_ns);
                    println!("    L1I:     {}KB", hierarchy.l1i_size / 1024);
                    println!("    L2:      {}KB ({:.0}ns)", hierarchy.l2_size / 1024, hierarchy.l2_latency_ns);
                    println!("    L3:      {}MB ({:.0}ns)", hierarchy.l3_size / 1024 / 1024, hierarchy.l3_latency_ns);
                    println!("    Main:    {:.0}ns", hierarchy.main_memory_ns);
                    println!("    Line:    {}B | TLB: {} entries", hierarchy.cache_line_size, hierarchy.tlb_entries);
                    println!("    Items/L1: {} (64B) | Items/L2: {} | Items/L3: {}",
                        hierarchy.l1_items(64), hierarchy.l2_items(64), hierarchy.l3_items(64));
                    println!();
                }
            }

            if bench {
                let item_count = working_set_kb * 1024 / 8;
                if !ctx.json {
                    println!("  {} Prefetch benchmark: {} items ({} KB working set)",
                        style("⚡").yellow(), style(item_count).green().bold(), style(working_set_kb).white());
                }

                let data: Vec<u64> = (0..item_count as u64).collect();

                let start = std::time::Instant::now();
                let mut sum = 0u64;
                for &v in &data { sum = sum.wrapping_add(v); }
                let no_prefetch = start.elapsed();
                std::hint::black_box(sum);

                let start = std::time::Instant::now();
                let stats = scan_with_prefetch(&data, 16, |&x| {
                    sum = sum.wrapping_add(x);
                    true
                });
                let with_prefetch = start.elapsed();
                std::hint::black_box(sum);

                if ctx.json {
                    let speedup = no_prefetch.as_nanos() as f64 / with_prefetch.as_nanos().max(1) as f64;
                    json_out.insert("benchmark".to_string(), serde_json::json!({
                        "items": item_count,
                        "working_set_kb": working_set_kb,
                        "no_prefetch_ms": no_prefetch.as_secs_f64() * 1000.0,
                        "with_prefetch_ms": with_prefetch.as_secs_f64() * 1000.0,
                        "speedup": speedup,
                    }));
                } else {
                    println!();
                    println!("  {} Without prefetch: {:.2} ms ({:.0} ns/item)",
                        style("▸").dim(), no_prefetch.as_secs_f64() * 1000.0,
                        no_prefetch.as_nanos() as f64 / item_count as f64);
                    println!("  {} With prefetch:    {:.2} ms ({:.0} ns/item)",
                        style("⚡").yellow(), with_prefetch.as_secs_f64() * 1000.0,
                        with_prefetch.as_nanos() as f64 / item_count as f64);
                    let speedup = no_prefetch.as_nanos() as f64 / with_prefetch.as_nanos().max(1) as f64;
                    println!("  {} Speedup: {}x",
                        style("🚀").yellow(), style(format!("{:.2}", speedup)).green().bold());
                    print_cache_report(&hierarchy, &stats);
                }
            }

            if tlb {
                let ws_bytes = working_set_kb * 1024;
                let info = analyze_tlb(ws_bytes);
                if ctx.json {
                    json_out.insert("tlb".to_string(), serde_json::json!({
                        "working_set_kb": working_set_kb,
                        "pages_4k": info.pages_4k,
                        "pages_2m": info.pages_2m,
                        "tlb_coverage_standard_pct": info.tlb_coverage_standard,
                        "tlb_coverage_huge_pct": info.tlb_coverage_huge,
                        "recommend_huge_pages": info.recommend_huge_pages,
                    }));
                } else {
                    println!();
                    println!("  {} TLB Analysis for {} KB working set:", style("📊").yellow(), working_set_kb);
                    println!("    4KB pages needed:    {}", info.pages_4k);
                    println!("    2MB hugepages needed:{}", info.pages_2m);
                    println!("    TLB coverage (4KB):  {:.1}%", info.tlb_coverage_standard);
                    println!("    TLB coverage (2MB):  {:.1}%", info.tlb_coverage_huge);
                    println!("    Recommend hugepages: {}",
                        if info.recommend_huge_pages { style("YES").green().bold() } else { style("no").dim() });
                    println!();
                }
            }

            if !bench && !info && !tlb {
                if !ctx.json {
                    println!();
                    println!("  {} Use {} for cache hierarchy info", style("→").dim(),
                        style("jatin-lean system cpu-cache --info").yellow());
                    println!("  {} Use {} for prefetch benchmark", style("→").dim(),
                        style("jatin-lean system cpu-cache --bench").yellow());
                    println!();
                }
            }
            
            if ctx.json {
                crate::output::output_result("system cpu-cache", &serde_json::Value::Object(json_out), ctx)?;
            }
            Ok(())
        }

        SystemCommands::Io { path, fs_info, process } => {
            let target = std::fs::canonicalize(&path)?;
            let nm_path = target.join("node_modules");
            
            let mut json_out = serde_json::Map::new();

            if nm_path.exists() {
                let stats = crate::mmap::io_stats(&nm_path)?;
                if ctx.json {
                    json_out.insert("io_stats".to_string(), serde_json::json!({
                        "path": nm_path.display().to_string(),
                    }));
                } else {
                    stats.print_info();
                }
            } else {
                if !ctx.json {
                    println!("  {} No node_modules found. Analyzing project directory...", style("ℹ").blue());
                }
                let stats = crate::mmap::io_stats(&target)?;
                if ctx.json {
                    json_out.insert("io_stats".to_string(), serde_json::json!({
                        "path": target.display().to_string(),
                    }));
                } else {
                    stats.print_info();
                }
            }
            
            if fs_info {
                let info = crate::syscall::FsInfo::query(&target)?;
                if ctx.json {
                    json_out.insert("fs_info".to_string(), serde_json::json!({
                        "block_size_bytes": info.block_size,
                        "filesystem_type": info.fs_type,
                    }));
                } else {
                    info.print_info();
                }
            }
            
            if process {
                let proc = crate::syscall::ProcessStats::current();
                if ctx.json {
                    json_out.insert("process".to_string(), serde_json::json!({
                        "peak_rss_bytes": proc.peak_rss_bytes,
                        "user_time_ms": proc.user_time_ms,
                    }));
                } else {
                    proc.print_info();
                }
            }
            
            if ctx.json {
                crate::output::output_result("system io", &serde_json::Value::Object(json_out), ctx)?;
            }
            Ok(())
        }
    }
}
