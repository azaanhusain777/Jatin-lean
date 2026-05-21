//! Benchmarking suite commands.

use crate::output::OutputContext;
use anyhow::Result;
use clap::Subcommand;
use console::style;

#[derive(Subcommand, Debug)]
pub enum BenchCommands {
    /// Run all built-in performance benchmarks
    All {
        #[arg(long)]
        timer: bool,
    },
    /// Zero-copy serialization benchmark (rkyv vs JSON)
    Serde {
        #[arg(long)]
        bench: bool,
        #[arg(long, default_value = "100")]
        entities: usize,
        #[arg(long, default_value = "1000")]
        iterations: u64,
        #[arg(long)]
        compare: bool,
    },
    /// SIMD-accelerated JSON structural scanner
    Json {
        #[arg(long, value_name = "FILE")]
        file: Option<std::path::PathBuf>,
        #[arg(long)]
        input: Option<String>,
        #[arg(long)]
        keys: bool,
        #[arg(long)]
        merge_patch: bool,
    },
    /// io_uring async I/O engine benchmark
    IoUring {
        #[arg(long)]
        bench: bool,
        #[arg(long, default_value = "10000")]
        files: u64,
        #[arg(long)]
        compare: bool,
        #[arg(long)]
        nvme: bool,
    },
    /// Request coalescing and cache stampede prevention
    Coalesce {
        #[arg(long)]
        demo: bool,
        #[arg(long, default_value = "1000")]
        requests: u64,
        #[arg(long, default_value = "10")]
        keys: u64,
        #[arg(long)]
        cache_stats: bool,
    },
    /// Request hedging & fragmented cache engine
    Hedge {
        #[arg(long)]
        bench: bool,
        #[arg(long, default_value = "10000")]
        requests: u64,
        #[arg(long)]
        cache_demo: bool,
    },
    /// Maglev consistent hashing analysis
    Maglev {
        #[arg(long, default_value = "server-1,server-2,server-3,server-4,server-5")]
        backends: String,
        #[arg(long, default_value = "65537")]
        table_size: usize,
        #[arg(long)]
        analyze: bool,
        #[arg(long)]
        disruption: Option<String>,
    },
    /// Monomorphic static dispatch benchmark
    StaticDispatch {
        #[arg(long)]
        bench: bool,
    },
}

pub fn handle_command(command: BenchCommands, ctx: &OutputContext) -> Result<()> {
    if !ctx.json {
        crate::display::print_banner();
    }
    let mut json_out = serde_json::Map::new();

    match command {
        BenchCommands::All { timer } => {
            if timer {
                if !ctx.json {
                    crate::benchmark::print_timer_info();
                }
            } else {
                if !ctx.json {
                    println!(
                        "  {} Running built-in benchmarks...\n",
                        style("⚡").yellow().bold()
                    );
                }
                let caps = crate::simd::CpuCapabilities::detect();
                if ctx.json {
                    json_out.insert(
                        "cpu_capabilities".to_string(),
                        serde_json::json!({
                            "architecture": caps.arch,
                            "simd_tier": caps.tier_name(),
                        }),
                    );
                } else {
                    println!(
                        "  {} {}",
                        style("CPU Features").cyan().bold(),
                        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
                    );
                    println!(
                        "  {} Architecture: {}",
                        style("◉").cyan(),
                        style(&caps.arch).white().bold()
                    );
                    println!(
                        "  {} SIMD tier: {}",
                        style("◉").cyan(),
                        style(caps.tier_name()).green().bold()
                    );
                    println!();
                }
                let suite = crate::benchmark::run_builtin_benchmarks();
                if ctx.json {
                    // Just a simple placeholder since we can't easily serialize the entire suite
                    json_out.insert("suite_executed".to_string(), serde_json::json!(true));
                } else {
                    suite.print_results();
                }
            }
            if ctx.json {
                crate::output::output_result(
                    "bench all",
                    &serde_json::Value::Object(json_out),
                    ctx,
                )?;
            }
            Ok(())
        }
        BenchCommands::Serde {
            bench,
            entities,
            iterations,
            compare,
        } => {
            use crate::zero_copy_serde::*;
            if !ctx.json {
                println!(
                    "  {} {}",
                    style("Zero-Copy Serialization Engine").cyan().bold(),
                    style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
                );
            }
            if compare {
                let table = benchmark_table();
                if ctx.json {
                    let mut arr = Vec::new();
                    for b in &table {
                        arr.push(serde_json::json!({
                            "framework": b.framework,
                            "access_latency": b.access_latency,
                            "deser_speed": b.deser_speed,
                            "supports_mutation": b.supports_mutation
                        }));
                    }
                    json_out.insert("comparison".to_string(), serde_json::Value::Array(arr));
                } else {
                    println!();
                    for b in &table {
                        let mutation = if b.supports_mutation {
                            style("✓").green()
                        } else {
                            style("✗").red()
                        };
                        println!(
                            "  {} {} | {} | {} | Mutation: {}",
                            style("▸").dim(),
                            style(b.framework).yellow().bold(),
                            style(b.access_latency).white(),
                            style(b.deser_speed).dim(),
                            mutation
                        );
                    }
                    println!();
                }
            }
            if bench {
                if !ctx.json {
                    println!(
                        "  {} Benchmark: {} entities × {} iterations",
                        style("⚡").yellow(),
                        style(entities).green().bold(),
                        style(iterations).white()
                    );
                }
                let mut engine = ZeroCopyEngine::new();
                let response = ZeroCopyEngine::sample_response(entities);
                let bytes = engine.serialize(&response);
                let _ = engine.deserialize_from(&bytes);
                engine.stats = SerdeStats::default();
                let start = std::time::Instant::now();
                let mut last_bytes = Vec::new();
                for _ in 0..iterations {
                    last_bytes = engine.serialize(&response);
                }
                let ser_time = start.elapsed();
                let start = std::time::Instant::now();
                for _ in 0..iterations {
                    let _ = engine.deserialize_from(&last_bytes);
                }
                let deser_time = start.elapsed();
                let start = std::time::Instant::now();
                let mut json_str = String::new();
                for _ in 0..iterations {
                    json_str = ZeroCopyEngine::to_json(&response);
                }
                let json_ser_time = start.elapsed();
                let start = std::time::Instant::now();
                for _ in 0..iterations {
                    let _ = engine.parse_json(&json_str);
                }
                let json_parse_time = start.elapsed();
                let start = std::time::Instant::now();
                for _ in 0..iterations {
                    let _ = ZeroCopyEngine::access_archived(&last_bytes);
                }
                let access_time = start.elapsed();

                let ser_speedup =
                    json_ser_time.as_nanos() as f64 / ser_time.as_nanos().max(1) as f64;
                let deser_speedup =
                    json_parse_time.as_nanos() as f64 / deser_time.as_nanos().max(1) as f64;

                if ctx.json {
                    json_out.insert("benchmark".to_string(), serde_json::json!({
                        "rkyv_serialize_ns_op": ser_time.as_nanos() as f64 / iterations as f64,
                        "rkyv_serialize_bytes": last_bytes.len(),
                        "rkyv_deserialize_ns_op": deser_time.as_nanos() as f64 / iterations as f64,
                        "rkyv_zero_copy_ns_op": access_time.as_nanos() as f64 / iterations as f64,
                        "json_serialize_ns_op": json_ser_time.as_nanos() as f64 / iterations as f64,
                        "json_serialize_bytes": json_str.len(),
                        "json_parse_ns_op": json_parse_time.as_nanos() as f64 / iterations as f64,
                        "serialize_speedup": ser_speedup,
                        "deserialize_speedup": deser_speedup,
                        "size_savings_pct": (1.0 - last_bytes.len() as f64 / json_str.len() as f64) * 100.0,
                    }));
                } else {
                    println!();
                    println!(
                        "  {} rkyv serialize:    {:.0} ns/op ({} bytes)",
                        style("⚡").yellow(),
                        ser_time.as_nanos() as f64 / iterations as f64,
                        last_bytes.len()
                    );
                    println!(
                        "  {} rkyv deserialize:  {:.0} ns/op",
                        style("⚡").yellow(),
                        deser_time.as_nanos() as f64 / iterations as f64
                    );
                    println!(
                        "  {} rkyv zero-copy:    {:.0} ns/op",
                        style("🚀").green(),
                        access_time.as_nanos() as f64 / iterations as f64
                    );
                    println!(
                        "  {} JSON serialize:    {:.0} ns/op ({} bytes)",
                        style("▸").dim(),
                        json_ser_time.as_nanos() as f64 / iterations as f64,
                        json_str.len()
                    );
                    println!(
                        "  {} JSON parse:        {:.0} ns/op",
                        style("▸").dim(),
                        json_parse_time.as_nanos() as f64 / iterations as f64
                    );
                    println!();
                    println!(
                        "  {} Serialize speedup:  {}x faster than JSON",
                        style("🚀").yellow(),
                        style(format!("{:.1}", ser_speedup)).green().bold()
                    );
                    println!(
                        "  {} Deserialize speedup: {}x faster than JSON",
                        style("🚀").yellow(),
                        style(format!("{:.1}", deser_speedup)).green().bold()
                    );
                    println!(
                        "  {} Size savings:        {} bytes vs {} bytes ({:.0}% smaller)",
                        style("📦").yellow(),
                        style(last_bytes.len()).green().bold(),
                        style(json_str.len()).dim(),
                        (1.0 - last_bytes.len() as f64 / json_str.len() as f64) * 100.0
                    );
                    println!();
                }
            }
            if !bench && !compare {
                if !ctx.json {
                    println!();
                    println!(
                        "  {} Use {} for framework comparison",
                        style("→").dim(),
                        style("jatin-lean bench serde --compare").yellow()
                    );
                    println!(
                        "  {} Use {} for serialization benchmark",
                        style("→").dim(),
                        style("jatin-lean bench serde --bench").yellow()
                    );
                    println!();
                }
            }
            if ctx.json {
                crate::output::output_result(
                    "bench serde",
                    &serde_json::Value::Object(json_out),
                    ctx,
                )?;
            }
            Ok(())
        }
        BenchCommands::Json {
            file,
            input,
            keys,
            merge_patch,
        } => {
            use crate::simd_json::*;
            if !ctx.json {
                println!(
                    "  {} {}",
                    style("SIMD JSON Structural Scanner").cyan().bold(),
                    style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
                );
                let scanner = SimdJsonScanner::new();
                println!(
                    "  {} SIMD width: {} bytes/chunk",
                    style("▸").dim(),
                    scanner.chunk_size
                );
            }
            let scanner = SimdJsonScanner::new();

            let json_bytes = if let Some(ref path) = file {
                std::fs::read(path).unwrap_or_default()
            } else if let Some(ref s) = input {
                s.as_bytes().to_vec()
            } else {
                serde_json::to_vec_pretty(&serde_json::json!({"project":"jatin-lean","version":"0.5.1","features":["xdp","ipc","rkyv"],"stats":{"modules":40,"tests":230,"loc":15000}})).unwrap()
            };
            let scan = scanner.scan(&json_bytes);

            if ctx.json {
                json_out.insert(
                    "scan".to_string(),
                    serde_json::json!({
                        "bytes": json_bytes.len(),
                        "structural_chars_found": scan.indices.len(),
                        "max_nesting_depth": scan.max_depth,
                    }),
                );
            } else {
                print_simd_report(&scan);
            }

            if keys {
                let extracted = scanner.extract_keys(&json_bytes, &scan);
                if ctx.json {
                    json_out.insert("extracted_keys".to_string(), serde_json::json!(extracted));
                } else {
                    println!(
                        "  {} Extracted {} keys:",
                        style("🔑").yellow(),
                        extracted.len()
                    );
                    for k in &extracted {
                        println!("    {} {}", style("→").dim(), style(k).yellow());
                    }
                    println!();
                }
            }

            if merge_patch {
                let mut original = serde_json::json!({"name":"Alice","age":30,"city":"NYC"});
                let patch = serde_json::json!({"age":31,"city":null,"role":"admin"});
                if !ctx.json {
                    println!(
                        "  {} JSON Merge Patch (RFC 7396) Demo:",
                        style("🔀").yellow()
                    );
                    println!(
                        "    Original: {}",
                        serde_json::to_string(&original).unwrap()
                    );
                    println!("    Patch:    {}", serde_json::to_string(&patch).unwrap());
                }

                json_merge_patch(&mut original, &patch);

                if ctx.json {
                    json_out.insert(
                        "merge_patch".to_string(),
                        serde_json::json!({
                            "result": original
                        }),
                    );
                } else {
                    println!(
                        "    Result:   {}",
                        style(serde_json::to_string(&original).unwrap()).green()
                    );
                    println!();
                }
            }
            if ctx.json {
                crate::output::output_result(
                    "bench json",
                    &serde_json::Value::Object(json_out),
                    ctx,
                )?;
            }
            Ok(())
        }
        BenchCommands::IoUring {
            bench,
            files,
            compare,
            nvme,
        } => {
            use crate::io_uring::*;
            if !ctx.json {
                println!(
                    "  {} {}",
                    style("io_uring Async I/O Engine").cyan().bold(),
                    style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
                );
            }
            let config = if nvme {
                if !ctx.json {
                    println!("  {} NVMe-optimized mode", style("⚡").yellow());
                }
                IoUringConfig::nvme_optimized()
            } else {
                IoUringConfig::scan_optimized()
            };

            if compare {
                let table = io_api_comparison();
                if ctx.json {
                    let mut arr = Vec::new();
                    for entry in &table {
                        arr.push(serde_json::json!({
                            "api": entry.api,
                            "syscalls_per_op": entry.syscalls_per_op,
                            "kernel_bypass": entry.kernel_bypass,
                            "zero_copy": entry.zero_copy,
                            "best_for": entry.best_for,
                        }));
                    }
                    json_out.insert("comparison".to_string(), serde_json::Value::Array(arr));
                } else {
                    println!();
                    for entry in &table {
                        let bypass = if entry.kernel_bypass {
                            style("✓").green()
                        } else {
                            style("✗").red()
                        };
                        let zc = if entry.zero_copy {
                            style("✓").green()
                        } else {
                            style("✗").red()
                        };
                        println!(
                            "  {} {} | Syscalls: {} | Bypass: {} | ZeroCopy: {}",
                            style("▸").dim(),
                            style(entry.api).yellow(),
                            entry.syscalls_per_op,
                            bypass,
                            zc
                        );
                        println!("    Best for: {}", style(entry.best_for).dim());
                    }
                    println!();
                }
            }
            if bench {
                let mut engine = IoUringEngine::new(config.clone());
                let paths: Vec<std::path::PathBuf> = (0..files)
                    .map(|i| std::path::PathBuf::from(format!("node_modules/pkg-{}/index.js", i)))
                    .collect();

                if !ctx.json {
                    println!(
                        "  {} Benchmark: {} batched stat operations",
                        style("⚡").yellow(),
                        style(files).green().bold()
                    );
                }

                let start = std::time::Instant::now();
                let batch_size = config.sq_depth as usize;
                for chunk in paths.chunks(batch_size) {
                    engine.submit_stat_batch(chunk);
                    engine.flush();
                }
                let elapsed = start.elapsed();

                let traditional_syscalls = files;
                let uring_syscalls = engine
                    .stats
                    .batches
                    .load(std::sync::atomic::Ordering::Relaxed);

                if ctx.json {
                    json_out.insert("benchmark".to_string(), serde_json::json!({
                        "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
                        "traditional_syscalls": traditional_syscalls,
                        "io_uring_syscalls": uring_syscalls,
                        "syscall_reduction_factor": traditional_syscalls as f64 / uring_syscalls.max(1) as f64,
                    }));
                } else {
                    print_iouring_report(&engine.stats, &config, elapsed);
                    println!(
                        "  {} Traditional: {} syscalls | io_uring: {} syscalls ({}x reduction)",
                        style("🚀").yellow(),
                        style(traditional_syscalls).red(),
                        style(uring_syscalls).green().bold(),
                        style(format!(
                            "{:.0}",
                            traditional_syscalls as f64 / uring_syscalls.max(1) as f64
                        ))
                        .green()
                        .bold()
                    );
                    println!();
                }
            }
            if !bench && !compare {
                if !ctx.json {
                    println!();
                    println!(
                        "  {} Use {} for benchmark",
                        style("→").dim(),
                        style("jatin-lean bench io-uring --bench").yellow()
                    );
                    println!(
                        "  {} Use {} for API comparison",
                        style("→").dim(),
                        style("jatin-lean bench io-uring --compare").yellow()
                    );
                    println!();
                }
            }
            if ctx.json {
                crate::output::output_result(
                    "bench io-uring",
                    &serde_json::Value::Object(json_out),
                    ctx,
                )?;
            }
            Ok(())
        }
        BenchCommands::Coalesce {
            demo,
            requests,
            keys,
            cache_stats,
        } => {
            use crate::request_coalescing::*;
            if !ctx.json {
                println!(
                    "  {} {}",
                    style("Request Coalescing Engine").cyan().bold(),
                    style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
                );
            }
            if demo || !cache_stats {
                if !ctx.json {
                    println!(
                        "  {} Singleflight demo: {} requests across {} unique keys",
                        style("⚡").yellow(),
                        style(requests).green().bold(),
                        style(keys).white()
                    );
                }
                let sf = SingleflightGroup::<String>::new();
                let start = std::time::Instant::now();
                for i in 0..requests {
                    let key = format!("/api/resource/{}", i % keys);
                    sf.do_once(&key, || format!("response-for-key-{}", i % keys));
                }
                let elapsed = start.elapsed();

                if ctx.json {
                    json_out.insert(
                        "singleflight_demo".to_string(),
                        serde_json::json!({
                            "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
                            "requests_per_sec": requests as f64 / elapsed.as_secs_f64(),
                        }),
                    );
                } else {
                    print_coalescing_report(&sf.stats);
                    println!(
                        "  {} Elapsed:           {:.2} ms",
                        style("▸").dim(),
                        elapsed.as_secs_f64() * 1000.0
                    );
                    println!(
                        "  {} Requests/sec:      {:.0}",
                        style("🚀").yellow(),
                        requests as f64 / elapsed.as_secs_f64()
                    );
                    println!();
                    println!("  {} JSONPath Query Demo:", style("🔍").yellow());
                    let json: serde_json::Value = serde_json::json!({"data":{"users":[{"name":"Alice","email":"alice@example.com","role":"admin"},{"name":"Bob","email":"bob@example.com","role":"user"}],"meta":{"total":2,"page":1}}});
                    let paths = vec![
                        "$.data.users[0].name",
                        "$.data.users[1].email",
                        "$.data.meta.total",
                    ];
                    for p in &paths {
                        let expr = JsonPathExpr::parse(p);
                        if let Some(val) = expr.extract(&json) {
                            println!("    {} → {}", style(p).yellow(), style(val).green());
                        }
                    }
                    println!();
                    println!("  {} Request Merger Demo:", style("🔀").yellow());
                    let merger = RequestMerger::new(std::time::Duration::from_millis(10));
                    merger.submit(MergeableRequest {
                        client_id: "Client-A".into(),
                        resource_path: "/api/users/1".into(),
                        requested_fields: vec!["name".into(), "email".into()],
                        arrived_at: std::time::Instant::now(),
                    });
                    merger.submit(MergeableRequest {
                        client_id: "Client-B".into(),
                        resource_path: "/api/users/1".into(),
                        requested_fields: vec!["email".into(), "role".into()],
                        arrived_at: std::time::Instant::now(),
                    });
                    let merged = merger.flush();
                    for q in &merged {
                        println!(
                            "    {} clients → superset: {:?}",
                            style(q.client_count).green().bold(),
                            q.superset_fields
                        );
                    }
                    println!();
                }
            }
            if cache_stats {
                let cache = StructuralCache::new(std::time::Duration::from_secs(60));
                let mut fields = std::collections::HashMap::new();
                fields.insert("name".to_string(), serde_json::json!("Alice"));
                fields.insert("email".to_string(), serde_json::json!("alice@example.com"));
                fields.insert("role".to_string(), serde_json::json!("admin"));
                cache.store_fields("/api/users/1", fields);
                let (found, missing) = cache.get_fields(
                    "/api/users/1",
                    &["name".to_string(), "email".to_string(), "phone".to_string()],
                );

                if ctx.json {
                    json_out.insert(
                        "cache_stats".to_string(),
                        serde_json::json!({
                            "requested": ["name", "email", "phone"],
                            "found_count": found.len(),
                            "missing": missing,
                            "hit_rate": cache.stats.hit_rate(),
                            "total_fields": cache.total_fields(),
                        }),
                    );
                } else {
                    println!();
                    println!("  {} Structural Cache Demo:", style("📊").yellow());
                    println!("    Requested: [name, email, phone]");
                    println!(
                        "    Found:     {} fields (partial hit)",
                        style(found.len()).green().bold()
                    );
                    println!("    Missing:   {:?}", missing);
                    println!("    Hit rate:  {:.1}%", cache.stats.hit_rate());
                    println!("    Stored:    {} total fields", cache.total_fields());
                    println!();
                }
            }
            if ctx.json {
                crate::output::output_result(
                    "bench coalesce",
                    &serde_json::Value::Object(json_out),
                    ctx,
                )?;
            }
            Ok(())
        }
        BenchCommands::Hedge {
            bench,
            requests,
            cache_demo,
        } => {
            use crate::hedging::*;
            if !ctx.json {
                println!(
                    "  {} {}",
                    style("Request Hedging & Fragmented Cache").cyan().bold(),
                    style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
                );
            }
            if bench {
                let engine = HedgingEngine::new(
                    vec![
                        "replica-us-east".into(),
                        "replica-us-west".into(),
                        "replica-eu".into(),
                    ],
                    HedgingStrategy::Immediate,
                );
                if !ctx.json {
                    println!(
                        "  {} Hedging {} requests across {} replicas",
                        style("⚡").yellow(),
                        style(requests).green().bold(),
                        engine.replicas.len()
                    );
                }
                let start = std::time::Instant::now();
                for i in 0..requests {
                    engine.execute(i, &format!("/api/resource/{}", i % 100));
                }
                let elapsed = start.elapsed();

                if ctx.json {
                    json_out.insert(
                        "benchmark".to_string(),
                        serde_json::json!({
                            "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
                            "requests_per_sec": requests as f64 / elapsed.as_secs_f64(),
                        }),
                    );
                } else {
                    println!(
                        "  {} Elapsed: {:.2} ms | RPS: {:.0}",
                        style("▸").dim(),
                        elapsed.as_secs_f64() * 1000.0,
                        requests as f64 / elapsed.as_secs_f64()
                    );
                    print_hedging_report(&engine.stats);
                }
            }
            if cache_demo {
                let mut cache = FragmentedCache::new();
                if !ctx.json {
                    println!("  {} Fragmented Cache Demo:", style("🗂️").yellow());
                }
                let mut fields = std::collections::HashMap::new();
                fields.insert("name".into(), serde_json::json!("Jatin"));
                fields.insert("email".into(), serde_json::json!("jatin@dev.com"));
                fields.insert("role".into(), serde_json::json!("engineer"));
                fields.insert("projects".into(), serde_json::json!(42));
                cache.store("/user/1", fields, std::time::Duration::from_secs(60));

                if !ctx.json {
                    if let FragmentResult::FullHit(f) =
                        cache.fetch_fragment("/user/1", &["name", "email"])
                    {
                        println!(
                            "    Client A [name,email]: {} ← from cache",
                            style(serde_json::to_string(&f).unwrap()).green()
                        );
                    }
                    if let FragmentResult::FullHit(f) =
                        cache.fetch_fragment("/user/1", &["role", "projects"])
                    {
                        println!(
                            "    Client B [role,projects]: {} ← from cache",
                            style(serde_json::to_string(&f).unwrap()).green()
                        );
                    }
                    if let FragmentResult::PartialHit { found, missing } =
                        cache.fetch_fragment("/user/1", &["name", "phone"])
                    {
                        println!(
                            "    Client C [name,phone]: found {} / missing {} ← partial",
                            style(serde_json::to_string(&found).unwrap()).green(),
                            style(format!("{:?}", missing)).red()
                        );
                    }
                }

                let mut patch = std::collections::HashMap::new();
                patch.insert("projects".into(), serde_json::json!(43));
                patch.insert("team".into(), serde_json::json!("platform"));
                cache.apply_delta("/user/1", &patch);

                if ctx.json {
                    json_out.insert(
                        "cache_demo".to_string(),
                        serde_json::json!({
                            "applied_delta": ["projects", "team"]
                        }),
                    );
                } else {
                    println!("    Delta applied: projects→43, +team=platform");
                    print_frag_cache_report(&cache.stats);
                }
            }
            if !bench && !cache_demo {
                if !ctx.json {
                    println!();
                    println!(
                        "  {} Use {} for hedging benchmark",
                        style("→").dim(),
                        style("jatin-lean bench hedge --bench").yellow()
                    );
                    println!(
                        "  {} Use {} for cache demo",
                        style("→").dim(),
                        style("jatin-lean bench hedge --cache-demo").yellow()
                    );
                    println!();
                }
            }
            if ctx.json {
                crate::output::output_result(
                    "bench hedge",
                    &serde_json::Value::Object(json_out),
                    ctx,
                )?;
            }
            Ok(())
        }
        BenchCommands::Maglev {
            backends,
            table_size,
            analyze,
            disruption,
        } => {
            use crate::maglev::*;
            if !ctx.json {
                println!(
                    "  {} {}",
                    style("Maglev Consistent Hash Ring").cyan().bold(),
                    style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
                );
            }
            let backend_list: Vec<String> =
                backends.split(',').map(|s| s.trim().to_string()).collect();
            let mut ring = MaglevHashRing::new(backend_list, table_size);
            if analyze {
                let start = std::time::Instant::now();
                for i in 0..100_000 {
                    ring.lookup(&format!("key-{}", i));
                }
                let elapsed = start.elapsed();

                if ctx.json {
                    json_out.insert(
                        "analyze".to_string(),
                        serde_json::json!({
                            "lookups": 100_000,
                            "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
                            "ns_per_lookup": elapsed.as_nanos() as f64 / 100_000.0,
                        }),
                    );
                } else {
                    println!(
                        "  {} 100K lookups: {:.2} ms ({:.0} ns/lookup)",
                        style("⚡").yellow(),
                        elapsed.as_secs_f64() * 1000.0,
                        elapsed.as_nanos() as f64 / 100_000.0
                    );
                }
            }
            if let Some(ref removed) = disruption {
                let rate = ring.disruption_rate(removed);
                let ideal = 100.0 / ring.backends.len() as f64;

                if ctx.json {
                    json_out.insert(
                        "disruption".to_string(),
                        serde_json::json!({
                            "removed_backend": removed,
                            "disruption_rate_pct": rate,
                            "ideal_disruption_pct": ideal,
                            "overhead_pct": rate - ideal,
                        }),
                    );
                } else {
                    println!(
                        "  {} Disruption when removing '{}': {:.1}%",
                        style("⚠").yellow(),
                        style(removed).red(),
                        rate
                    );
                    println!(
                        "  {} Ideal disruption: {:.1}% | Overhead: {:.1}%",
                        style("▸").dim(),
                        ideal,
                        rate - ideal
                    );
                }
            }
            if ctx.json {
                crate::output::output_result(
                    "bench maglev",
                    &serde_json::Value::Object(json_out),
                    ctx,
                )?;
            } else {
                print_maglev_report(&ring);
            }
            Ok(())
        }
        BenchCommands::StaticDispatch { bench } => {
            use crate::static_plugins::*;
            if bench {
                let runner = MonomorphicPluginRunner::new();
                let start = std::time::Instant::now();
                for _ in 0..1_000_000 {
                    runner.run_all_on_scan();
                }
                let elapsed = start.elapsed();

                if ctx.json {
                    json_out.insert(
                        "benchmark".to_string(),
                        serde_json::json!({
                            "executions": 1_000_000,
                            "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
                        }),
                    );
                } else {
                    println!(
                        "  {} Executed 1,000,000 static plugin dispatches in {:.2} ms",
                        style("⚡").yellow(),
                        elapsed.as_secs_f64() * 1000.0
                    );
                    print_static_dispatch_report();
                }
            } else {
                if !ctx.json {
                    print_static_dispatch_report();
                    println!(
                        "  {} Use {} to run benchmark",
                        style("→").dim(),
                        style("jatin-lean bench static-dispatch --bench").yellow()
                    );
                }
            }
            if ctx.json {
                crate::output::output_result(
                    "bench static-dispatch",
                    &serde_json::Value::Object(json_out),
                    ctx,
                )?;
            }
            Ok(())
        }
    }
}
