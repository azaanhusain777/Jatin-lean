//! Legacy command support with deprecation warnings.
//! Maps old flat commands to the new hierarchical structure.

use clap::Subcommand;
use std::path::PathBuf;
use anyhow::Result;
use console::style;
use crate::output::OutputContext;

#[derive(Subcommand, Debug)]
pub enum LegacyCommands {
    /// [DEPRECATED] Use: jatin-lean node health
    #[command(hide = true)]
    Health { #[arg(default_value = ".")] path: PathBuf },
    /// [DEPRECATED] Use: jatin-lean node dedup
    #[command(hide = true)]
    Dedup { #[arg(default_value = ".")] path: PathBuf },
    /// [DEPRECATED] Use: jatin-lean node deps
    #[command(hide = true)]
    Deps { #[arg(default_value = ".")] path: PathBuf },
    /// [DEPRECATED] Use: jatin-lean node compress
    #[command(hide = true)]
    Compress { #[arg(default_value = ".")] path: PathBuf },
    /// [DEPRECATED] Use: jatin-lean node treeshake
    #[command(hide = true)]
    Treeshake { #[arg(default_value = ".")] path: PathBuf },
    /// [DEPRECATED] Use: jatin-lean node audit
    #[command(hide = true)]
    Audit { #[arg(default_value = ".")] path: PathBuf },
    /// [DEPRECATED] Use: jatin-lean node analyze
    #[command(hide = true)]
    Analyze { #[arg(default_value = ".")] path: PathBuf },
    /// [DEPRECATED] Use: jatin-lean analyze undo
    #[command(hide = true)]
    Undo,
    /// [DEPRECATED] Use: jatin-lean analyze snapshots --restore
    #[command(hide = true)]
    Restore { snapshot_id: String },
    /// [DEPRECATED] Use: jatin-lean analyze analytics
    #[command(hide = true)]
    Analytics { #[arg(long)] clear: bool },
    /// [DEPRECATED] Use: jatin-lean analyze snapshots
    #[command(hide = true)]
    Snapshots {
        #[arg(long)] list: bool,
        #[arg(long, value_name = "SNAPSHOT_ID")] restore: Option<String>,
        #[arg(long, value_name = "SNAPSHOT_ID")] delete: Option<String>,
        #[arg(long, value_name = "DAYS")] cleanup: Option<u64>,
    },
    /// [DEPRECATED] Use: jatin-lean node watch
    #[command(hide = true)]
    Watch {
        #[arg(default_value = ".")] path: PathBuf,
        #[arg(long, default_value = "5")] interval: u64,
        #[arg(long)] auto_prune: bool,
        #[arg(long, default_value = "0")] max_cycles: u64,
    },
    /// [DEPRECATED] Use: jatin-lean analyze cache
    #[command(hide = true)]
    Cache {
        #[arg(long)] clear: bool,
        #[arg(long)] stats: bool,
        #[arg(default_value = ".")] path: PathBuf,
    },
    /// [DEPRECATED] Use: jatin-lean node policy
    #[command(hide = true)]
    Policy {
        #[arg(long, value_name = "FILE")] file: Option<PathBuf>,
        #[arg(long, value_name = "FILE")] init: Option<PathBuf>,
        #[arg(default_value = ".")] path: PathBuf,
    },
    /// [DEPRECATED] Use: jatin-lean analyze plugins
    #[command(hide = true)]
    Plugins { #[arg(long)] list: bool },
    /// [DEPRECATED] Use: jatin-lean bench all
    #[command(hide = true)]
    Bench { #[arg(long)] all: bool, #[arg(long)] timer: bool },
    /// [DEPRECATED] Use: jatin-lean system io
    #[command(hide = true)]
    Io {
        #[arg(default_value = ".")] path: PathBuf,
        #[arg(long)] fs_info: bool,
        #[arg(long)] process: bool,
    },
    /// [DEPRECATED] Use: jatin-lean node visualize
    #[command(hide = true)]
    Visualize {
        #[arg(default_value = ".")] path: PathBuf,
        #[arg(long)] treemap: bool,
        #[arg(long)] sparklines: bool,
    },
    /// [DEPRECATED] Use: jatin-lean analyze dist-cache
    #[command(hide = true)]
    DistCache { #[arg(long)] stats: bool, #[arg(long)] clear: bool, #[arg(long)] evict: bool },
    /// [DEPRECATED] Use: jatin-lean network xdp
    #[command(hide = true)]
    Xdp {
        #[arg(long)] compare: bool,
        #[arg(long)] bench: bool,
        #[arg(long, default_value = "1000000")] packets: u64,
        #[arg(long, default_value = "none")] obfuscation: String,
    },
    /// [DEPRECATED] Use: jatin-lean memory ipc
    #[command(hide = true)]
    Ipc {
        #[arg(long)] bench: bool,
        #[arg(long, default_value = "1024")] capacity: usize,
        #[arg(long, default_value = "100000")] messages: u64,
        #[arg(long)] layout: bool,
    },
    /// [DEPRECATED] Use: jatin-lean bench serde
    #[command(hide = true)]
    Serde {
        #[arg(long)] bench: bool,
        #[arg(long, default_value = "100")] entities: usize,
        #[arg(long, default_value = "1000")] iterations: u64,
        #[arg(long)] compare: bool,
    },
    /// [DEPRECATED] Use: jatin-lean bench coalesce
    #[command(hide = true)]
    Coalesce {
        #[arg(long)] demo: bool,
        #[arg(long, default_value = "1000")] requests: u64,
        #[arg(long, default_value = "10")] keys: u64,
        #[arg(long)] cache_stats: bool,
    },
    /// [DEPRECATED] Use: jatin-lean analyze engine
    #[command(hide = true)]
    Engine {
        #[arg(long)] analyze: bool,
        #[arg(long)] grace_hopper: bool,
        #[arg(long)] compare: bool,
        #[arg(long)] bench: bool,
    },
    /// [DEPRECATED] Use: jatin-lean network gateway
    #[command(hide = true)]
    Gateway { #[arg(long)] bench: bool, #[arg(long, default_value = "10000")] requests: u64, #[arg(long, default_value = "1024")] payload_size: usize },
    /// [DEPRECATED] Use: jatin-lean bench json
    #[command(name = "simd-json", hide = true)]
    SimdJson { #[arg(long, value_name = "FILE")] file: Option<PathBuf>, #[arg(long)] input: Option<String>, #[arg(long)] keys: bool, #[arg(long)] merge_patch: bool },
    /// [DEPRECATED] Use: jatin-lean memory arena
    #[command(hide = true)]
    Arena { #[arg(long)] bench: bool, #[arg(long, default_value = "1024")] capacity_kb: usize, #[arg(long, default_value = "100000")] allocations: u64 },
    /// [DEPRECATED] Use: jatin-lean bench maglev
    #[command(hide = true)]
    Maglev { #[arg(long, default_value = "server-1,server-2,server-3,server-4,server-5")] backends: String, #[arg(long, default_value = "65537")] table_size: usize, #[arg(long)] analyze: bool, #[arg(long)] disruption: Option<String> },
    /// [DEPRECATED] Use: jatin-lean bench io-uring
    #[command(name = "io-uring", hide = true)]
    IoUring { #[arg(long)] bench: bool, #[arg(long, default_value = "10000")] files: u64, #[arg(long)] compare: bool, #[arg(long)] nvme: bool },
    /// [DEPRECATED] Use: jatin-lean system cpu-cache
    #[command(name = "cpu-cache", hide = true)]
    CpuCache { #[arg(long)] bench: bool, #[arg(long, default_value = "8192")] working_set_kb: usize, #[arg(long)] info: bool, #[arg(long)] tlb: bool },
    /// [DEPRECATED] Use: jatin-lean system optimize
    #[command(hide = true)]
    Optimize { #[arg(long)] assess: bool, #[arg(long)] numa: bool, #[arg(long)] network: bool, #[arg(long)] kernel: bool, #[arg(long)] generate: bool },
    /// [DEPRECATED] Use: jatin-lean network bpf
    #[command(hide = true)]
    Bpf { #[arg(long)] verify: bool, #[arg(long)] dpi: bool, #[arg(long, value_name = "PACKETS")] skbuff: Option<u64> },
    /// [DEPRECATED] Use: jatin-lean memory pcie
    #[command(hide = true)]
    Pcie { #[arg(long)] compare: bool, #[arg(long, default_value = "1")] size_gb: u64, #[arg(long, value_name = "LAYERS")] offload: Option<usize>, #[arg(long)] grace_hopper: bool },
    /// [DEPRECATED] Use: jatin-lean bench hedge
    #[command(hide = true)]
    Hedge { #[arg(long)] bench: bool, #[arg(long, default_value = "10000")] requests: u64, #[arg(long)] cache_demo: bool },
    /// [DEPRECATED] Use: jatin-lean memory mmap
    #[command(name = "mmap-ipc", hide = true)]
    MmapIpc { #[arg(long)] bench: bool, #[arg(long, default_value = "4096")] capacity: usize, #[arg(long, default_value = "256")] msg_size: usize, #[arg(long)] compare: bool },
    /// [DEPRECATED] Use: jatin-lean bench static-dispatch
    #[command(name = "static-dispatch", hide = true)]
    StaticDispatch { #[arg(long)] bench: bool },
}

fn show_deprecation(old_cmd: &str, new_cmd: &str) {
    eprintln!("  {} {}", style("⚠").yellow().bold(), style("This command syntax is deprecated.").yellow());
    eprintln!("  {} Use: {}", style("→").dim(), style(new_cmd).green().bold());
    eprintln!("  {} '{}' will be removed in v3.0.0", style("ℹ").blue(), style(old_cmd).dim());
    eprintln!();
}

pub fn handle_command(command: LegacyCommands, ctx: &OutputContext) -> Result<()> {
    use crate::cli::{node, system, network, memory, bench, analyze};
    match command {
        LegacyCommands::Health { path } => { if !ctx.json { show_deprecation("health", "jatin-lean node health"); } node::handle_command(node::NodeCommands::Health { path }, ctx) }
        LegacyCommands::Dedup { path } => { if !ctx.json { show_deprecation("dedup", "jatin-lean node dedup"); } node::handle_command(node::NodeCommands::Dedup { path }, ctx) }
        LegacyCommands::Deps { path } => { if !ctx.json { show_deprecation("deps", "jatin-lean node deps"); } node::handle_command(node::NodeCommands::Deps { path }, ctx) }
        LegacyCommands::Compress { path } => { if !ctx.json { show_deprecation("compress", "jatin-lean node compress"); } node::handle_command(node::NodeCommands::Compress { path }, ctx) }
        LegacyCommands::Treeshake { path } => { if !ctx.json { show_deprecation("treeshake", "jatin-lean node treeshake"); } node::handle_command(node::NodeCommands::Treeshake { path }, ctx) }
        LegacyCommands::Audit { path } => { if !ctx.json { show_deprecation("audit", "jatin-lean node audit"); } node::handle_command(node::NodeCommands::Audit { path }, ctx) }
        LegacyCommands::Analyze { path } => { if !ctx.json { show_deprecation("analyze", "jatin-lean node analyze"); } node::handle_command(node::NodeCommands::Analyze { path }, ctx) }
        LegacyCommands::Undo => { if !ctx.json { show_deprecation("undo", "jatin-lean analyze undo"); } analyze::handle_command(analyze::AnalyzeCommands::Undo, ctx) }
        LegacyCommands::Restore { snapshot_id } => { if !ctx.json { show_deprecation("restore", "jatin-lean analyze snapshots --restore"); } analyze::handle_command(analyze::AnalyzeCommands::Snapshots { list: false, restore: Some(snapshot_id), delete: None, cleanup: None }, ctx) }
        LegacyCommands::Analytics { clear } => { if !ctx.json { show_deprecation("analytics", "jatin-lean analyze analytics"); } analyze::handle_command(analyze::AnalyzeCommands::Analytics { clear }, ctx) }
        LegacyCommands::Snapshots { list, restore, delete, cleanup } => { if !ctx.json { show_deprecation("snapshots", "jatin-lean analyze snapshots"); } analyze::handle_command(analyze::AnalyzeCommands::Snapshots { list, restore, delete, cleanup }, ctx) }
        LegacyCommands::Watch { path, interval, auto_prune, max_cycles } => { if !ctx.json { show_deprecation("watch", "jatin-lean node watch"); } node::handle_command(node::NodeCommands::Watch { path, interval, auto_prune, max_cycles }, ctx) }
        LegacyCommands::Cache { clear, stats, path } => { if !ctx.json { show_deprecation("cache", "jatin-lean analyze cache"); } analyze::handle_command(analyze::AnalyzeCommands::Cache { clear, stats, path }, ctx) }
        LegacyCommands::Policy { file, init, path } => { if !ctx.json { show_deprecation("policy", "jatin-lean node policy"); } node::handle_command(node::NodeCommands::Policy { file, init, path }, ctx) }
        LegacyCommands::Plugins { list } => { if !ctx.json { show_deprecation("plugins", "jatin-lean analyze plugins"); } analyze::handle_command(analyze::AnalyzeCommands::Plugins { list }, ctx) }
        LegacyCommands::Bench { all: _, timer } => { if !ctx.json { show_deprecation("bench", "jatin-lean bench all"); } bench::handle_command(bench::BenchCommands::All { timer }, ctx) }
        LegacyCommands::Io { path, fs_info, process } => { if !ctx.json { show_deprecation("io", "jatin-lean system io"); } system::handle_command(system::SystemCommands::Io { path, fs_info, process }, ctx) }
        LegacyCommands::Visualize { path, treemap, sparklines } => { if !ctx.json { show_deprecation("visualize", "jatin-lean node visualize"); } node::handle_command(node::NodeCommands::Visualize { path, treemap, sparklines }, ctx) }
        LegacyCommands::DistCache { stats, clear, evict } => { if !ctx.json { show_deprecation("dist-cache", "jatin-lean analyze dist-cache"); } analyze::handle_command(analyze::AnalyzeCommands::DistCache { stats, clear, evict }, ctx) }
        LegacyCommands::Xdp { compare, bench, packets, obfuscation } => { if !ctx.json { show_deprecation("xdp", "jatin-lean network xdp"); } network::handle_command(network::NetworkCommands::Xdp { compare, bench, packets, obfuscation }, ctx) }
        LegacyCommands::Ipc { bench, capacity, messages, layout } => { if !ctx.json { show_deprecation("ipc", "jatin-lean memory ipc"); } memory::handle_command(memory::MemoryCommands::Ipc { bench, capacity, messages, layout }, ctx) }
        LegacyCommands::Serde { bench, entities, iterations, compare } => { if !ctx.json { show_deprecation("serde", "jatin-lean bench serde"); } bench::handle_command(bench::BenchCommands::Serde { bench, entities, iterations, compare }, ctx) }
        LegacyCommands::Coalesce { demo, requests, keys, cache_stats } => { if !ctx.json { show_deprecation("coalesce", "jatin-lean bench coalesce"); } bench::handle_command(bench::BenchCommands::Coalesce { demo, requests, keys, cache_stats }, ctx) }
        LegacyCommands::Engine { analyze: a, grace_hopper, compare, bench: b } => { if !ctx.json { show_deprecation("engine", "jatin-lean analyze engine"); } analyze::handle_command(analyze::AnalyzeCommands::Engine { analyze: a, grace_hopper, compare, bench: b }, ctx) }
        LegacyCommands::Gateway { bench, requests, payload_size } => { if !ctx.json { show_deprecation("gateway", "jatin-lean network gateway"); } network::handle_command(network::NetworkCommands::Gateway { bench, requests, payload_size }, ctx) }
        LegacyCommands::SimdJson { file, input, keys, merge_patch } => { if !ctx.json { show_deprecation("simd-json", "jatin-lean bench json"); } bench::handle_command(bench::BenchCommands::Json { file, input, keys, merge_patch }, ctx) }
        LegacyCommands::Arena { bench, capacity_kb, allocations } => { if !ctx.json { show_deprecation("arena", "jatin-lean memory arena"); } memory::handle_command(memory::MemoryCommands::Arena { bench, capacity_kb, allocations }, ctx) }
        LegacyCommands::Maglev { backends, table_size, analyze: a, disruption } => { if !ctx.json { show_deprecation("maglev", "jatin-lean bench maglev"); } bench::handle_command(bench::BenchCommands::Maglev { backends, table_size, analyze: a, disruption }, ctx) }
        LegacyCommands::IoUring { bench, files, compare, nvme } => { if !ctx.json { show_deprecation("io-uring", "jatin-lean bench io-uring"); } bench::handle_command(bench::BenchCommands::IoUring { bench, files, compare, nvme }, ctx) }
        LegacyCommands::CpuCache { bench, working_set_kb, info, tlb } => { if !ctx.json { show_deprecation("cpu-cache", "jatin-lean system cpu-cache"); } system::handle_command(system::SystemCommands::CpuCache { bench, working_set_kb, info, tlb }, ctx) }
        LegacyCommands::Optimize { assess, numa, network: n, kernel, generate } => { if !ctx.json { show_deprecation("optimize", "jatin-lean system optimize"); } system::handle_command(system::SystemCommands::Optimize { assess, numa, network: n, kernel, generate }, ctx) }
        LegacyCommands::Bpf { verify, dpi, skbuff } => { if !ctx.json { show_deprecation("bpf", "jatin-lean network bpf"); } network::handle_command(network::NetworkCommands::Bpf { verify, dpi, skbuff }, ctx) }
        LegacyCommands::Pcie { compare, size_gb, offload, grace_hopper } => { if !ctx.json { show_deprecation("pcie", "jatin-lean memory pcie"); } memory::handle_command(memory::MemoryCommands::Pcie { compare, size_gb, offload, grace_hopper }, ctx) }
        LegacyCommands::Hedge { bench, requests, cache_demo } => { if !ctx.json { show_deprecation("hedge", "jatin-lean bench hedge"); } bench::handle_command(bench::BenchCommands::Hedge { bench, requests, cache_demo }, ctx) }
        LegacyCommands::MmapIpc { bench, capacity, msg_size, compare } => { if !ctx.json { show_deprecation("mmap-ipc", "jatin-lean memory mmap"); } memory::handle_command(memory::MemoryCommands::Mmap { bench, capacity, msg_size, compare }, ctx) }
        LegacyCommands::StaticDispatch { bench } => { if !ctx.json { show_deprecation("static-dispatch", "jatin-lean bench static-dispatch"); } bench::handle_command(bench::BenchCommands::StaticDispatch { bench }, ctx) }
    }
}
