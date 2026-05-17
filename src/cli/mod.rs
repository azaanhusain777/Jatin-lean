//! CLI command definitions and handlers for jatin-lean v2.0.
//!
//! Organizes 32+ commands into 6 logical categories:
//! - `node` — Node.js ecosystem optimization
//! - `system` — System-level optimization
//! - `network` — Network & eBPF tools
//! - `memory` — Memory & IPC optimization
//! - `bench` — Benchmarking suite
//! - `analyze` — Analysis and reporting tools
//!
//! Also provides legacy command support with deprecation warnings.

pub mod node;
pub mod system;
pub mod network;
pub mod memory;
pub mod bench;
pub mod analyze;
pub mod legacy;

pub use node::NodeCommands;
pub use system::SystemCommands;
pub use network::NetworkCommands;
pub use memory::MemoryCommands;
pub use bench::BenchCommands;
pub use analyze::AnalyzeCommands;
pub use legacy::LegacyCommands;

use crate::output::OutputContext;
use anyhow::Result;

pub fn handle_node_command(command: NodeCommands, ctx: &OutputContext) -> Result<()> {
    node::handle_command(command, ctx)
}

pub fn handle_system_command(command: SystemCommands, ctx: &OutputContext) -> Result<()> {
    system::handle_command(command, ctx)
}

pub fn handle_network_command(command: NetworkCommands, ctx: &OutputContext) -> Result<()> {
    network::handle_command(command, ctx)
}

pub fn handle_memory_command(command: MemoryCommands, ctx: &OutputContext) -> Result<()> {
    memory::handle_command(command, ctx)
}

pub fn handle_bench_command(command: BenchCommands, ctx: &OutputContext) -> Result<()> {
    bench::handle_command(command, ctx)
}

pub fn handle_analyze_command(command: AnalyzeCommands, ctx: &OutputContext) -> Result<()> {
    analyze::handle_command(command, ctx)
}

pub fn handle_legacy_command(command: LegacyCommands, ctx: &OutputContext) -> Result<()> {
    legacy::handle_command(command, ctx)
}
