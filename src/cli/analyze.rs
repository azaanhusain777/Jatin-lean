//! Analysis and reporting commands.

use clap::Subcommand;
use anyhow::Result;
use console::style;
use crate::output::OutputContext;

#[derive(Subcommand, Debug)]
pub enum AnalyzeCommands {
    /// Manage the incremental scan cache
    Cache {
        #[arg(long)] clear: bool,
        #[arg(long)] stats: bool,
        #[arg(default_value = ".")] path: std::path::PathBuf,
    },
    /// Manage the distributed cache
    DistCache {
        #[arg(long)] stats: bool,
        #[arg(long)] clear: bool,
        #[arg(long)] evict: bool,
    },
    /// Adaptive CPU-GPU execution engine analysis
    Engine {
        #[arg(long)] analyze: bool,
        #[arg(long)] grace_hopper: bool,
        #[arg(long)] compare: bool,
        #[arg(long)] bench: bool,
    },
    /// Manage pre-deletion snapshots
    Snapshots {
        #[arg(long)] list: bool,
        #[arg(long, value_name = "SNAPSHOT_ID")] restore: Option<String>,
        #[arg(long, value_name = "SNAPSHOT_ID")] delete: Option<String>,
        #[arg(long, value_name = "DAYS")] cleanup: Option<u64>,
    },
    /// Show scan history and analytics dashboard
    Analytics {
        #[arg(long)] clear: bool,
    },
    /// Undo the last pruning operation
    Undo,
    /// List and manage plugins
    Plugins {
        #[arg(long)] list: bool,
    },
}

pub fn handle_command(command: AnalyzeCommands, ctx: &OutputContext) -> Result<()> {
    if !ctx.json { crate::display::print_banner(); }
    let mut json_out = serde_json::Map::new();

    match command {
        AnalyzeCommands::Cache { clear, stats: _, path } => {
            let target = std::fs::canonicalize(&path)?;
            let nm_path = target.join("node_modules");
            if clear {
                let mut c = crate::cache::ScanCache::load(&nm_path);
                c.clear();
                c.save(&nm_path)?;
                if ctx.json {
                    json_out.insert("cleared".to_string(), serde_json::json!(true));
                } else {
                    println!("  {} Scan cache cleared.", style("✓").green().bold());
                }
            } else {
                let c = crate::cache::ScanCache::load(&nm_path);
                if ctx.json {
                    json_out.insert("cache_info".to_string(), serde_json::json!({
                        "cached_count": c.cached_count(),
                        "age_seconds": c.age_seconds(),
                        "cache_path": crate::cache::ScanCache::cache_path(&nm_path).display().to_string(),
                    }));
                } else {
                    println!();
                    println!("  {} {}", style("Scan Cache").cyan().bold(), style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
                    println!("  {} Cached packages: {}", style("◉").cyan(), style(c.cached_count()).white().bold());
                    println!("  {} Cache age: {}s", style("◉").cyan(), c.age_seconds());
                    println!("  {} Cache file: {}", style("◉").dim(), style(crate::cache::ScanCache::cache_path(&nm_path).display()).dim());
                    println!();
                }
            }
            if ctx.json { crate::output::output_result("analyze cache", &serde_json::Value::Object(json_out), ctx)?; }
            Ok(())
        }
        AnalyzeCommands::DistCache { stats: _, clear, evict } => {
            if clear {
                let mut cache = crate::distributed_cache::DistributedCache::with_defaults()?;
                cache.clear()?;
                if ctx.json {
                    json_out.insert("cleared".to_string(), serde_json::json!(true));
                } else {
                    println!("  {} Distributed cache cleared.", style("✓").green().bold());
                }
            } else if evict {
                let mut cache = crate::distributed_cache::DistributedCache::with_defaults()?;
                let evicted = cache.evict_expired()?;
                if ctx.json {
                    json_out.insert("evicted_entries".to_string(), serde_json::json!(evicted));
                } else {
                    println!("  {} Evicted {} expired entries.", style("✓").green().bold(), evicted);
                }
            } else {
                let cache = crate::distributed_cache::DistributedCache::with_defaults()?;
                if ctx.json {
                    json_out.insert("cache_info".to_string(), serde_json::json!({
                        "enabled": true // Simplified for JSON output
                    }));
                } else {
                    crate::distributed_cache::print_cache_info(&cache);
                }
            }
            if ctx.json { crate::output::output_result("analyze distcache", &serde_json::Value::Object(json_out), ctx)?; }
            Ok(())
        }
        AnalyzeCommands::Engine { analyze, grace_hopper, compare, bench } => {
            use crate::adaptive_engine::*;
            if !ctx.json {
                println!("  {} {}", style("Adaptive CPU-GPU Execution Engine").cyan().bold(), style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
            }
            if compare {
                let paradigms = [MemoryParadigm::PageableSystemMemory, MemoryParadigm::PinnedMemory, MemoryParadigm::SoftwareCoherentUnified, MemoryParadigm::HardwareCoherentUnified];
                if ctx.json {
                    let mut arr = Vec::new();
                    for p in &paradigms {
                        arr.push(serde_json::json!({
                            "paradigm": p.label(),
                            "bandwidth": p.bandwidth(),
                            "setup_complexity": p.setup_complexity(),
                            "optimal_hardware": p.optimal_hardware(),
                        }));
                    }
                    json_out.insert("comparison".to_string(), serde_json::Value::Array(arr));
                } else {
                    println!();
                    for p in &paradigms { println!("  {} {} | {} | {} | {}", style("▸").dim(), style(p.label()).yellow().bold(), style(p.bandwidth()).white(), style(p.setup_complexity()).dim(), style(p.optimal_hardware()).cyan()); }
                    println!();
                }
            }
            let hw = if grace_hopper { 
                if !ctx.json { println!("  {} Simulating NVIDIA Grace Hopper GH200 environment", style("🖥️").yellow()); }
                HardwareState::simulated_grace_hopper() 
            } else { 
                HardwareState::detect() 
            };
            let engine = AdaptiveEngine::with_hardware(hw.clone());
            if analyze || bench {
                let workloads = vec![
                    WorkloadProfile { id: "regex-parser".into(), data_size_bytes: 50*1024*1024, estimated_flops: 5_000_000, parallelizable: false, string_heavy: true, matrix_heavy: false, branch_divergent: true, memory_required_bytes: 50*1024*1024, workload_type: WorkloadType::StringProcessing },
                    WorkloadProfile { id: "matmul-4096".into(), data_size_bytes: 128*1024*1024, estimated_flops: 1_000_000_000, parallelizable: true, string_heavy: false, matrix_heavy: true, branch_divergent: false, memory_required_bytes: 128*1024*1024, workload_type: WorkloadType::MatrixComputation },
                    WorkloadProfile { id: "embeddings".into(), data_size_bytes: 512*1024*1024, estimated_flops: 500_000_000, parallelizable: true, string_heavy: false, matrix_heavy: true, branch_divergent: false, memory_required_bytes: 512*1024*1024, workload_type: WorkloadType::VectorEmbedding },
                    WorkloadProfile { id: "llm-70b".into(), data_size_bytes: 140u64*1024*1024*1024, estimated_flops: 10_000_000_000, parallelizable: true, string_heavy: false, matrix_heavy: true, branch_divergent: false, memory_required_bytes: 140u64*1024*1024*1024, workload_type: WorkloadType::LlmInference },
                    WorkloadProfile { id: "data-aggregate".into(), data_size_bytes: 2*1024*1024, estimated_flops: 100_000, parallelizable: true, string_heavy: false, matrix_heavy: false, branch_divergent: false, memory_required_bytes: 2*1024*1024, workload_type: WorkloadType::DataAggregation },
                ];
                if ctx.json {
                    let mut arr = Vec::new();
                    for w in &workloads {
                        let result = engine.execute(w);
                        arr.push(serde_json::json!({
                            "workload": w.id,
                            "target": result.target.label(),
                            "estimated_speedup": format!("{:.0}", result.estimated_speedup),
                            "reason": result.decision_reason,
                        }));
                    }
                    json_out.insert("analysis".to_string(), serde_json::Value::Array(arr));
                } else {
                    println!();
                    for w in &workloads {
                        let result = engine.execute(w);
                        println!("  {} {} → {} {} (speedup: {}x, reason: {})", result.target.icon(), style(&w.id).white().bold(), result.target.label(), style("").dim(), style(format!("{:.0}", result.estimated_speedup)).green().bold(), style(&result.decision_reason).dim());
                    }
                    println!();
                    print_engine_report(&engine.stats, &engine.hardware);
                }
            }
            if !analyze && !bench && !compare {
                if !ctx.json {
                    print_engine_report(&engine.stats, &engine.hardware);
                    println!("  {} Use {} for workload routing analysis", style("→").dim(), style("jatin-lean analyze engine --analyze").yellow());
                    println!("  {} Use {} for Grace Hopper simulation", style("→").dim(), style("jatin-lean analyze engine --grace-hopper --analyze").yellow());
                    println!();
                }
            }
            if ctx.json { crate::output::output_result("analyze engine", &serde_json::Value::Object(json_out), ctx)?; }
            Ok(())
        }
        AnalyzeCommands::Snapshots { list: _, restore, delete, cleanup } => {
            let manager = crate::snapshot::SnapshotManager::new()?;
            if let Some(snapshot_id) = restore {
                let result = manager.restore_snapshot(&snapshot_id)?;
                if ctx.json {
                    json_out.insert("restored".to_string(), serde_json::json!(snapshot_id));
                    json_out.insert("result".to_string(), serde_json::json!({"success": true})); // Simplified
                } else {
                    crate::snapshot::print_restore_result(&result);
                }
            } else if let Some(snapshot_id) = delete {
                manager.delete_snapshot(&snapshot_id)?;
                if ctx.json {
                    json_out.insert("deleted".to_string(), serde_json::json!(snapshot_id));
                } else {
                    println!("  {} Snapshot {} deleted.", style("✓").green().bold(), style(&snapshot_id).cyan());
                }
            } else if let Some(days) = cleanup {
                let deleted = manager.cleanup_old_snapshots(days)?;
                if ctx.json {
                    json_out.insert("cleaned_up_count".to_string(), serde_json::json!(deleted));
                } else {
                    println!("  {} Cleaned up {} old snapshots.", style("✓").green().bold(), deleted);
                }
            } else {
                let snapshots = manager.list_snapshots()?;
                if ctx.json {
                    let mut arr = Vec::new();
                    for s in snapshots {
                        arr.push(serde_json::json!({
                            "id": s.id,
                        }));
                    }
                    json_out.insert("snapshots".to_string(), serde_json::Value::Array(arr));
                } else {
                    crate::snapshot::print_snapshot_list(&snapshots);
                }
            }
            if ctx.json { crate::output::output_result("analyze snapshots", &serde_json::Value::Object(json_out), ctx)?; }
            Ok(())
        }
        AnalyzeCommands::Analytics { clear } => {
            if clear {
                let mut db = crate::analytics::AnalyticsDB::load()?;
                db.clear()?;
                if ctx.json {
                    json_out.insert("cleared".to_string(), serde_json::json!(true));
                } else {
                    println!("  {} Analytics data cleared.", style("✓").green().bold());
                }
            } else {
                let db = crate::analytics::AnalyticsDB::load()?;
                if ctx.json {
                    json_out.insert("analytics_info".to_string(), serde_json::json!({
                        "entries": db.entries.len(), // Simplified
                    }));
                } else {
                    crate::analytics::print_analytics_summary(&db);
                }
            }
            if ctx.json { crate::output::output_result("analyze analytics", &serde_json::Value::Object(json_out), ctx)?; }
            Ok(())
        }
        AnalyzeCommands::Undo => {
            let manager = crate::snapshot::SnapshotManager::new()?;
            let snapshots = manager.list_snapshots()?;
            if let Some(latest) = snapshots.first() {
                if !ctx.json {
                    println!("  {} Undoing last prune operation (Snapshot {})...", style("⏪").magenta().bold(), style(&latest.id).cyan());
                }
                let result = manager.restore_snapshot(&latest.id)?;
                if ctx.json {
                    json_out.insert("undo_snapshot_id".to_string(), serde_json::json!(latest.id));
                } else {
                    crate::snapshot::print_restore_result(&result);
                }
            } else {
                if ctx.json {
                    json_out.insert("error".to_string(), serde_json::json!("No snapshots found to undo."));
                } else {
                    println!("  {} No snapshots found to undo. Did you run with {}?", style("✗").red().bold(), style("--snapshot").yellow());
                }
            }
            if ctx.json { crate::output::output_result("analyze undo", &serde_json::Value::Object(json_out), ctx)?; }
            Ok(())
        }
        AnalyzeCommands::Plugins { list: _ } => {
            let registry = crate::plugin::PluginRegistry::with_builtins();
            if ctx.json {
                json_out.insert("plugins_enabled".to_string(), serde_json::json!(true));
            } else {
                crate::plugin::print_plugin_info(&registry);
            }
            if ctx.json { crate::output::output_result("analyze plugins", &serde_json::Value::Object(json_out), ctx)?; }
            Ok(())
        }
    }
}
