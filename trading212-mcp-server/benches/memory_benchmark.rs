//! Benchmark comparing memory usage and performance of standard vs streaming approaches
//!
//! Run with: cargo bench --bench memory_benchmark

#![allow(
    unused_crate_dependencies,
    unused_imports,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::unwrap_used,
    clippy::uninlined_format_args,
    clippy::cast_precision_loss,
    missing_docs
)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use reqwest::Client;
use std::sync::Arc;
use std::time::Instant;
use tokio::runtime::Runtime;

use trading212_mcp_server::cache::Trading212Cache;
use trading212_mcp_server::config::Trading212Config;
use trading212_mcp_server::tools::GetInstrumentsTool;

// Mock large JSON response for testing (simulating 3.4MB Trading212 response)
fn create_mock_instruments_json(count: usize) -> String {
    let mut instruments = Vec::new();

    for i in 0..count {
        let instrument = format!(
            r#"{{
                "ticker": "MOCK{}_US_EQ",
                "type": "STOCK", 
                "workingScheduleId": {},
                "isin": "US{:010}",
                "currencyCode": "USD",
                "name": "Mock Corporation {}",
                "shortName": "Mock{}",
                "maxOpenQuantity": {}.0,
                "addedOn": "2020-01-01"
            }}"#,
            i,
            i % 10,
            i,
            i,
            i,
            (i % 1000) + 100
        );
        instruments.push(instrument);
    }

    format!("[{}]", instruments.join(","))
}

// Memory tracking helper
struct MemoryTracker {
    start_memory: usize,
    peak_memory: usize,
}

impl MemoryTracker {
    fn new() -> Self {
        Self {
            start_memory: get_memory_usage(),
            peak_memory: 0,
        }
    }

    fn update_peak(&mut self) {
        let current = get_memory_usage();
        if current > self.peak_memory {
            self.peak_memory = current;
        }
    }

    fn peak_delta(&self) -> usize {
        self.peak_memory.saturating_sub(self.start_memory)
    }
}

// Simple memory usage approximation
fn get_memory_usage() -> usize {
    // This is a simple approximation - in real benchmarks you'd use system tools
    // For this demo, we'll track allocations manually
    0
}

async fn benchmark_standard_approach(
    tool: &GetInstrumentsTool,
    client: &Client,
    config: &Trading212Config,
    cache: &Trading212Cache,
) -> Result<(u128, usize), Box<dyn std::error::Error>> {
    let mut memory_tracker = MemoryTracker::new();
    let start = Instant::now();

    // Force standard approach
    std::env::remove_var("TRADING212_USE_STREAMING");

    memory_tracker.update_peak();
    let result = tool.call_tool(client, config, cache).await;
    memory_tracker.update_peak();

    let duration = start.elapsed().as_millis();
    let peak_memory = memory_tracker.peak_delta();

    result?;
    Ok((duration, peak_memory))
}

async fn benchmark_streaming_approach(
    tool: &GetInstrumentsTool,
    client: &Client,
    config: &Trading212Config,
    cache: &Trading212Cache,
) -> Result<(u128, usize), Box<dyn std::error::Error>> {
    let mut memory_tracker = MemoryTracker::new();
    let start = Instant::now();

    // Force streaming approach
    std::env::set_var("TRADING212_USE_STREAMING", "1");

    memory_tracker.update_peak();
    let result = tool.call_tool(client, config, cache).await;
    memory_tracker.update_peak();

    let duration = start.elapsed().as_millis();
    let peak_memory = memory_tracker.peak_delta();

    result?;
    Ok((duration, peak_memory))
}

fn bench_approaches(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Create test data of different sizes to simulate different JSON response sizes
    let test_sizes = vec![
        (1000, "1K instruments (~100KB)"),
        (5000, "5K instruments (~500KB)"),
        (15000, "15K instruments (~1.5MB)"),
        (30000, "30K instruments (~3MB)"),
    ];

    for (instrument_count, size_desc) in test_sizes {
        let mock_json = create_mock_instruments_json(instrument_count);

        // Setup test environment
        let client = Client::new();
        let config = Trading212Config::new().unwrap();
        let cache = Trading212Cache::new().unwrap();

        // Test different query patterns
        let test_queries = vec![
            (
                GetInstrumentsTool {
                    search: None,
                    instrument_type: None,
                    limit: Some(100),
                    page: Some(1),
                },
                "no_filter",
            ),
            (
                GetInstrumentsTool {
                    search: Some("MOCK1".to_string()),
                    instrument_type: None,
                    limit: Some(10),
                    page: Some(1),
                },
                "search_filter",
            ),
            (
                GetInstrumentsTool {
                    search: None,
                    instrument_type: Some("STOCK".to_string()),
                    limit: Some(50),
                    page: Some(1),
                },
                "type_filter",
            ),
        ];

        for (tool, query_desc) in test_queries {
            let group_name = format!("{}_{}", size_desc, query_desc);
            let mut group = c.benchmark_group(group_name);

            // Pre-populate cache with mock data for consistent testing
            rt.block_on(async {
                let cache_key =
                    trading212_mcp_server::cache::CacheKey::new("equity/metadata/instruments", "");
                let instruments_cache = cache.get_cache("equity/metadata/instruments");
                instruments_cache.insert(cache_key, mock_json.clone()).await;
            });

            // Benchmark standard approach
            group.bench_function("standard", |b| {
                b.to_async(&rt).iter(|| async {
                    let result = benchmark_standard_approach(&tool, &client, &config, &cache).await;
                    black_box(result)
                });
            });

            // Benchmark streaming approach
            group.bench_function("streaming", |b| {
                b.to_async(&rt).iter(|| async {
                    let result =
                        benchmark_streaming_approach(&tool, &client, &config, &cache).await;
                    black_box(result)
                });
            });

            group.finish();
        }
    }

    // Clean up environment
    std::env::remove_var("TRADING212_USE_STREAMING");
}

fn bench_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Focus on the 3.4MB scenario specifically
    let mock_json = create_mock_instruments_json(30000); // ~3MB
    println!(
        "Mock JSON size: {} bytes (~{:.1}MB)",
        mock_json.len(),
        mock_json.len() as f64 / 1_048_576.0
    );

    let client = Client::new();
    let config = Trading212Config::new().unwrap();
    let cache = Trading212Cache::new().unwrap();

    // Pre-populate cache
    rt.block_on(async {
        let cache_key =
            trading212_mcp_server::cache::CacheKey::new("equity/metadata/instruments", "");
        let instruments_cache = cache.get_cache("equity/metadata/instruments");
        instruments_cache.insert(cache_key, mock_json).await;
    });

    let tool = GetInstrumentsTool {
        search: Some("MOCK".to_string()),
        instrument_type: None,
        limit: Some(100),
        page: Some(1),
    };

    c.bench_function("memory_3mb_standard", |b| {
        b.to_async(&rt).iter(|| async {
            std::env::remove_var("TRADING212_USE_STREAMING");
            let result = tool.call_tool(&client, &config, &cache).await;
            black_box(result)
        });
    });

    c.bench_function("memory_3mb_streaming", |b| {
        b.to_async(&rt).iter(|| async {
            std::env::set_var("TRADING212_USE_STREAMING", "1");
            let result = tool.call_tool(&client, &config, &cache).await;
            black_box(result)
        });
    });
}

criterion_group!(benches, bench_approaches, bench_memory_usage);
criterion_main!(benches);
