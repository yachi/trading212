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
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::single_component_path_imports,
    missing_docs
)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use reqwest::Client;
use std::hint::black_box;
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

// Helper function to calculate average instrument size from sample data
fn calculate_avg_instrument_size(sample_json: &str) -> usize {
    // Parse a small sample to get realistic size estimates
    let sample_result: Result<Vec<serde_json::Value>, _> = serde_json::from_str(sample_json);
    match sample_result {
        Ok(instruments) if !instruments.is_empty() => {
            // Take first 10 instruments as sample
            let sample_size = instruments.len().min(10);
            let sample = &instruments[0..sample_size];

            // Calculate average JSON size per instrument
            let total_size: usize = sample
                .iter()
                .map(|inst| serde_json::to_string(inst).map(|s| s.len()).unwrap_or(200))
                .sum();

            let avg_size = total_size / sample_size;
            // Add overhead for deserialized struct (estimated 1.5x JSON size)
            (avg_size as f64 * 1.5) as usize
        }
        _ => 300, // Fallback default size
    }
}

// Memory tracking helper using size estimation with actual sampling
// Since we can't use unsafe code, we estimate memory usage based on data structures
struct MemoryTracker {
    json_size: usize,
    result_count: usize,
    approach: String,
    avg_instrument_size: usize,
}

impl MemoryTracker {
    fn new(json_size: usize, approach: &str, sample_json: &str) -> Self {
        let avg_instrument_size = calculate_avg_instrument_size(sample_json);
        Self {
            json_size,
            result_count: 0,
            approach: approach.to_string(),
            avg_instrument_size,
        }
    }

    fn set_result_count(&mut self, count: usize) {
        self.result_count = count;
    }

    fn estimate_peak_memory(&self) -> usize {
        // Estimate memory usage based on approach using calculated sizes
        if self.approach == "standard" {
            // Standard approach loads entire JSON into memory
            // Plus parsed array of serde_json::Value objects
            // Plus final Instrument structs
            // Estimate: JSON + serde_json::Values + Instrument structs
            let json_mem = self.json_size;
            let values_mem = (self.json_size as f64 * 1.2) as usize; // serde_json::Value overhead
            let structs_mem = self.result_count * self.avg_instrument_size;
            json_mem + values_mem + structs_mem
        } else {
            // Streaming approach processes one instrument at a time
            // Only keeps filtered results in memory + small working buffer
            // No full JSON array in memory, just the input string
            let working_buffer = self.avg_instrument_size * 2; // Buffer for current parsing
            let results_mem = self.result_count * self.avg_instrument_size;
            working_buffer + results_mem
        }
    }

    fn get_memory_efficiency_ratio(&self) -> f64 {
        // Calculate memory efficiency compared to standard approach
        let standard_estimate = self.json_size * 3; // Simple standard estimate
        let current_estimate = self.estimate_peak_memory();
        standard_estimate as f64 / current_estimate as f64
    }
}

async fn benchmark_standard_approach(
    tool: &GetInstrumentsTool,
    client: &Client,
    config: &Trading212Config,
    cache: &Trading212Cache,
    json_size: usize,
    sample_json: &str,
) -> Result<(u128, usize, f64), Box<dyn std::error::Error>> {
    let mut memory_tracker = MemoryTracker::new(json_size, "standard", sample_json);
    let start = Instant::now();

    // Save current env var state and force standard approach
    let original_value = std::env::var("TRADING212_USE_STREAMING").ok();
    std::env::remove_var("TRADING212_USE_STREAMING");

    let result = tool.call_tool(client, config, cache).await?;
    let duration = start.elapsed().as_millis();

    // Count results for memory estimation
    // Convert the tool result to JSON to extract the result count
    if let Ok(result_json) = serde_json::to_value(&result) {
        if let Some(content) = result_json["content"].as_array() {
            if let Some(text) = content.first().and_then(|c| c["text"].as_str()) {
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(text) {
                    if let Some(array) = json_value.as_array() {
                        memory_tracker.set_result_count(array.len());
                    }
                }
            }
        }
    }

    let peak_memory = memory_tracker.estimate_peak_memory();
    let efficiency_ratio = memory_tracker.get_memory_efficiency_ratio();

    // Restore original env var state
    if let Some(value) = original_value {
        std::env::set_var("TRADING212_USE_STREAMING", value);
    }

    Ok((duration, peak_memory, efficiency_ratio))
}

async fn benchmark_streaming_approach(
    tool: &GetInstrumentsTool,
    client: &Client,
    config: &Trading212Config,
    cache: &Trading212Cache,
    json_size: usize,
    sample_json: &str,
) -> Result<(u128, usize, f64), Box<dyn std::error::Error>> {
    let mut memory_tracker = MemoryTracker::new(json_size, "streaming", sample_json);
    let start = Instant::now();

    // Save current env var state and force streaming approach
    let original_value = std::env::var("TRADING212_USE_STREAMING").ok();
    std::env::set_var("TRADING212_USE_STREAMING", "1");

    let result = tool.call_tool(client, config, cache).await?;
    let duration = start.elapsed().as_millis();

    // Count results for memory estimation
    // Convert the tool result to JSON to extract the result count
    if let Ok(result_json) = serde_json::to_value(&result) {
        if let Some(content) = result_json["content"].as_array() {
            if let Some(text) = content.first().and_then(|c| c["text"].as_str()) {
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(text) {
                    if let Some(array) = json_value.as_array() {
                        memory_tracker.set_result_count(array.len());
                    }
                }
            }
        }
    }

    let peak_memory = memory_tracker.estimate_peak_memory();
    let efficiency_ratio = memory_tracker.get_memory_efficiency_ratio();

    // Restore original env var state
    if let Some(value) = original_value {
        std::env::set_var("TRADING212_USE_STREAMING", value);
    } else {
        std::env::remove_var("TRADING212_USE_STREAMING");
    }

    Ok((duration, peak_memory, efficiency_ratio))
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
                    let result = benchmark_standard_approach(
                        &tool,
                        &client,
                        &config,
                        &cache,
                        mock_json.len(),
                        &mock_json,
                    )
                    .await;
                    black_box(result)
                });
            });

            // Benchmark streaming approach
            group.bench_function("streaming", |b| {
                b.to_async(&rt).iter(|| async {
                    let result = benchmark_streaming_approach(
                        &tool,
                        &client,
                        &config,
                        &cache,
                        mock_json.len(),
                        &mock_json,
                    )
                    .await;
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
