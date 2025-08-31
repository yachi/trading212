#![allow(unused_crate_dependencies)]
#![allow(dead_code)]
#![allow(clippy::semicolon_if_nothing_returned)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::uninlined_format_args)]
#![allow(missing_docs)]

//! Performance benchmarks for Trading212 MCP Server

use criterion::{criterion_group, criterion_main, Criterion};
use serde_json::json;
use std::hint::black_box;
use std::time::Duration;

// Simple structs to simulate tool creation without importing the full crate
#[derive(Debug, Clone, Default)]
struct GetInstrumentsTool {
    search: Option<String>,
    instrument_type: Option<String>,
    limit: Option<u32>,
    page: Option<u32>,
}

#[derive(Debug, Clone)]
struct GetPiesTool {}

#[derive(Debug, Clone)]
struct GetPieByIdTool {
    pie_id: u64,
}

#[derive(Debug, Clone)]
struct Trading212Config {
    api_key: String,
    base_url: String,
}

impl Trading212Config {
    fn endpoint_url(&self, endpoint: &str) -> String {
        let endpoint = if endpoint.starts_with('/') {
            endpoint
        } else {
            &format!("/{endpoint}")
        };
        format!("{}{}", self.base_url, endpoint)
    }
}

fn benchmark_tools_creation(c: &mut Criterion) {
    c.bench_function("create_get_instruments_tool", |b| {
        b.iter(|| {
            black_box(GetInstrumentsTool {
                search: Some("AAPL".to_string()),
                instrument_type: Some("STOCK".to_string()),
                ..Default::default()
            })
        })
    });

    c.bench_function("create_get_pies_tool", |b| {
        b.iter(|| black_box(GetPiesTool {}))
    });

    c.bench_function("create_get_pie_by_id_tool", |b| {
        b.iter(|| black_box(GetPieByIdTool { pie_id: 12345 }))
    });
}

fn benchmark_config_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_operations");

    let config = Trading212Config {
        api_key: "test_api_key_for_benchmarking".to_string(),
        base_url: "https://live.trading212.com/api/v0".to_string(),
    };

    group.bench_function("endpoint_url_generation", |b| {
        b.iter(|| black_box(config.endpoint_url(black_box("/equity/metadata/instruments"))))
    });

    group.bench_function("endpoint_url_with_params", |b| {
        b.iter(|| {
            black_box(config.endpoint_url(black_box("/equity/metadata/instruments?search=AAPL")))
        })
    });

    group.finish();
}

fn benchmark_json_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_parsing");
    group.measurement_time(Duration::from_secs(10));

    // Sample JSON responses for benchmarking
    let instruments_json = json!([
        {
            "isin": "US0378331005",
            "ticker": "AAPL",
            "name": "Apple Inc.",
            "type": "STOCK",
            "currencyCode": "USD"
        },
        {
            "isin": "US5949181045",
            "ticker": "MSFT",
            "name": "Microsoft Corporation",
            "type": "STOCK",
            "currencyCode": "USD"
        }
    ]);

    let pies_json = json!([
        {
            "id": 12345,
            "name": "Tech Portfolio",
            "dividendCashAction": "REINVEST",
            "goal": 10000.0,
            "creationDate": "2023-01-01T00:00:00Z"
        }
    ]);

    let pie_details_json = json!({
        "id": 12345,
        "name": "Tech Portfolio",
        "dividendCashAction": "REINVEST",
        "goal": 10000.0,
        "cash": 500.0,
        "investedValue": 9500.0,
        "result": {
            "value": 9750.0,
            "unrealizedPnl": 250.0,
            "realizedPnl": 0.0
        },
        "creationDate": "2023-01-01T00:00:00Z",
        "instruments": [
            {
                "isin": "US0378331005",
                "ticker": "AAPL",
                "currentShare": 60.0,
                "targetShare": 60.0,
                "ownedQuantity": 10.0,
                "currentPrice": 150.0
            }
        ]
    });

    group.bench_function("parse_instruments_response", |b| {
        let json_str = instruments_json.to_string();
        b.iter(|| {
            black_box(serde_json::from_str::<serde_json::Value>(black_box(&json_str)).unwrap())
        })
    });

    group.bench_function("parse_pies_response", |b| {
        let json_str = pies_json.to_string();
        b.iter(|| {
            black_box(serde_json::from_str::<serde_json::Value>(black_box(&json_str)).unwrap())
        })
    });

    group.bench_function("parse_pie_details_response", |b| {
        let json_str = pie_details_json.to_string();
        b.iter(|| {
            black_box(serde_json::from_str::<serde_json::Value>(black_box(&json_str)).unwrap())
        })
    });

    group.finish();
}

fn benchmark_url_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("url_building");

    group.bench_function("build_instruments_url_no_params", |b| {
        b.iter(|| {
            black_box(format!(
                "{}{}",
                black_box("https://live.trading212.com/api/v0"),
                black_box("/equity/metadata/instruments")
            ))
        })
    });

    group.bench_function("build_instruments_url_with_search", |b| {
        b.iter(|| {
            black_box(format!(
                "{}{}?search={}",
                black_box("https://live.trading212.com/api/v0"),
                black_box("/equity/metadata/instruments"),
                black_box("AAPL")
            ))
        })
    });

    group.bench_function("build_instruments_url_with_both_params", |b| {
        b.iter(|| {
            black_box(format!(
                "{}{}?search={}&type={}",
                black_box("https://live.trading212.com/api/v0"),
                black_box("/equity/metadata/instruments"),
                black_box("AAPL"),
                black_box("STOCK")
            ))
        })
    });

    group.finish();
}

fn benchmark_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");

    group.bench_function("api_key_validation", |b| {
        let api_key = "test_api_key_1234567890_abcdefghijklmnopqrstuvwxyz";
        b.iter(|| black_box(!black_box(api_key).is_empty() && black_box(api_key).len() > 10))
    });

    group.bench_function("endpoint_path_normalization", |b| {
        let endpoint = "equity/metadata/instruments";
        b.iter(|| {
            black_box(if black_box(endpoint).starts_with('/') {
                endpoint.to_string()
            } else {
                format!("/{}", endpoint)
            })
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_tools_creation,
    benchmark_config_operations,
    benchmark_json_parsing,
    benchmark_url_building,
    benchmark_string_operations
);
criterion_main!(benches);
