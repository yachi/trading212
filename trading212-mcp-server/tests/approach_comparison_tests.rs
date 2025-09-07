//! Integration tests comparing standard vs streaming approaches for correctness and performance

#![allow(
    unused_crate_dependencies,
    clippy::unwrap_used,
    unused_variables,
    clippy::single_char_pattern
)]

use reqwest::Client;
use std::collections::HashSet;
use trading212_mcp_server::cache::Trading212Cache;
use trading212_mcp_server::config::Trading212Config;
use trading212_mcp_server::tools::GetInstrumentsTool;

/// Mock small JSON response for testing both approaches
const MOCK_INSTRUMENTS_JSON: &str = r#"[
    {
        "ticker": "AAPL_US_EQ",
        "type": "STOCK",
        "workingScheduleId": 1,
        "isin": "US0378331005",
        "currencyCode": "USD",
        "name": "Apple Inc.",
        "shortName": "Apple",
        "maxOpenQuantity": 1000.0,
        "addedOn": "2020-01-01"
    },
    {
        "ticker": "MSFT_US_EQ", 
        "type": "STOCK",
        "workingScheduleId": 1,
        "isin": "US5949181045",
        "currencyCode": "USD",
        "name": "Microsoft Corporation",
        "shortName": "Microsoft",
        "maxOpenQuantity": 2000.0,
        "addedOn": "2020-01-01"
    },
    {
        "ticker": "QQQ_US_EQ",
        "type": "ETF", 
        "workingScheduleId": 1,
        "isin": "US46090E1038",
        "currencyCode": "USD",
        "name": "Invesco QQQ Trust",
        "shortName": "QQQ",
        "maxOpenQuantity": 5000.0,
        "addedOn": "2020-01-01"
    },
    {
        "ticker": "GOOGL_US_EQ",
        "type": "STOCK",
        "workingScheduleId": 1, 
        "isin": "US02079K3059",
        "currencyCode": "USD",
        "name": "Alphabet Inc Class A",
        "shortName": "Alphabet",
        "maxOpenQuantity": 1500.0,
        "addedOn": "2020-01-01"
    }
]"#;

async fn setup_test_environment() -> (Client, Trading212Config, Trading212Cache) {
    let client = Client::new();

    // Set up test environment variables for config
    std::env::set_var("TRADING212_API_KEY", "test_key");

    // Create config without reading from file for testing
    let config = Trading212Config::new_with_api_key("test_key".to_string());
    let cache = Trading212Cache::new().unwrap();

    // Pre-populate cache with mock data
    let cache_key = trading212_mcp_server::cache::CacheKey::new("equity/metadata/instruments", "");
    let instruments_cache = cache.get_cache("equity/metadata/instruments");
    instruments_cache
        .insert(cache_key, MOCK_INSTRUMENTS_JSON.to_string())
        .await;

    (client, config, cache)
}

fn extract_tickers_from_response(response: &str) -> HashSet<String> {
    // Simple extraction of ticker symbols from JSON response
    // In a real test, you'd parse the full JSON response
    let mut tickers = HashSet::new();

    for line in response.lines() {
        if line.contains("\"ticker\":") {
            if let Some(start) = line.find("\"ticker\": \"") {
                if let Some(end) = line[start + 11..].find("\"") {
                    let ticker = line[start + 11..start + 11 + end].to_string();
                    tickers.insert(ticker);
                }
            }
        }
    }

    tickers
}

#[tokio::test]
async fn test_both_approaches_return_same_results() {
    // Ensure clean environment for this test
    std::env::remove_var("TRADING212_USE_STREAMING");
    std::env::remove_var("TRADING212_USE_STANDARD");

    let (client, config, cache) = setup_test_environment().await;

    let tool = GetInstrumentsTool {
        search: None,
        instrument_type: None,
        limit: Some(10),
        page: Some(1),
    };

    // Test standard approach (force it since streaming is now default)
    std::env::set_var("TRADING212_USE_STANDARD", "1");
    let standard_result = tool.call_tool(&client, &config, &cache).await;
    assert!(standard_result.is_ok(), "Standard approach should succeed");

    // Test streaming approach
    std::env::set_var("TRADING212_USE_STREAMING", "1");
    let streaming_result = tool.call_tool(&client, &config, &cache).await;
    assert!(
        streaming_result.is_ok(),
        "Streaming approach should succeed"
    );

    // Both should return successful results
    let standard_response = format!("{:?}", standard_result.unwrap());
    let streaming_response = format!("{:?}", streaming_result.unwrap());

    // Extract and compare the core data (ignoring formatting differences)
    let standard_tickers = extract_tickers_from_response(&standard_response);
    let streaming_tickers = extract_tickers_from_response(&streaming_response);

    assert_eq!(
        standard_tickers, streaming_tickers,
        "Both approaches should return the same instruments"
    );

    // Cleanup
    std::env::remove_var("TRADING212_USE_STREAMING");
    std::env::remove_var("TRADING212_USE_STANDARD");
}

#[tokio::test]
async fn test_search_filtering_consistency() {
    // Ensure clean environment for this test
    std::env::remove_var("TRADING212_USE_STREAMING");
    std::env::remove_var("TRADING212_USE_STANDARD");

    let (client, config, cache) = setup_test_environment().await;

    let tool = GetInstrumentsTool {
        search: Some("Apple".to_string()),
        instrument_type: None,
        limit: Some(5),
        page: Some(1),
    };

    // Test standard approach (force it since streaming is now default)
    std::env::set_var("TRADING212_USE_STANDARD", "1");
    let standard_result = tool.call_tool(&client, &config, &cache).await;

    // Test streaming approach
    std::env::set_var("TRADING212_USE_STREAMING", "1");
    let streaming_result = tool.call_tool(&client, &config, &cache).await;

    // Both should succeed
    assert!(standard_result.is_ok(), "Standard search should succeed");
    assert!(streaming_result.is_ok(), "Streaming search should succeed");

    // Both should find Apple
    let standard_response = format!("{:?}", standard_result.unwrap());
    let streaming_response = format!("{:?}", streaming_result.unwrap());

    assert!(
        standard_response.contains("Apple"),
        "Standard should find Apple"
    );
    assert!(
        streaming_response.contains("Apple"),
        "Streaming should find Apple"
    );

    // Cleanup
    std::env::remove_var("TRADING212_USE_STREAMING");
    std::env::remove_var("TRADING212_USE_STANDARD");
}

#[tokio::test]
async fn test_type_filtering_consistency() {
    // Ensure clean environment for this test
    std::env::remove_var("TRADING212_USE_STREAMING");
    std::env::remove_var("TRADING212_USE_STANDARD");

    let (client, config, cache) = setup_test_environment().await;

    let tool = GetInstrumentsTool {
        search: None,
        instrument_type: Some("ETF".to_string()),
        limit: Some(5),
        page: Some(1),
    };

    // Test standard approach (force it since streaming is now default)
    std::env::set_var("TRADING212_USE_STANDARD", "1");
    let standard_result = tool.call_tool(&client, &config, &cache).await;

    // Test streaming approach
    std::env::set_var("TRADING212_USE_STREAMING", "1");
    let streaming_result = tool.call_tool(&client, &config, &cache).await;

    // Both should succeed
    assert!(
        standard_result.is_ok(),
        "Standard type filter should succeed"
    );
    assert!(
        streaming_result.is_ok(),
        "Streaming type filter should succeed"
    );

    // Both should find ETF
    let standard_response = format!("{:?}", standard_result.unwrap());
    let streaming_response = format!("{:?}", streaming_result.unwrap());

    assert!(
        standard_response.contains("QQQ"),
        "Standard should find QQQ ETF"
    );
    assert!(
        streaming_response.contains("QQQ"),
        "Streaming should find QQQ ETF"
    );

    // Cleanup
    std::env::remove_var("TRADING212_USE_STREAMING");
    std::env::remove_var("TRADING212_USE_STANDARD");
}

#[tokio::test]
async fn test_pagination_consistency() {
    // Ensure clean environment for this test
    std::env::remove_var("TRADING212_USE_STREAMING");
    std::env::remove_var("TRADING212_USE_STANDARD");

    let (client, config, cache) = setup_test_environment().await;

    let tool = GetInstrumentsTool {
        search: None,
        instrument_type: None,
        limit: Some(2),
        page: Some(1),
    };

    // Test standard approach (force it since streaming is now default)
    std::env::set_var("TRADING212_USE_STANDARD", "1");
    let standard_result = tool.call_tool(&client, &config, &cache).await;

    // Test streaming approach
    std::env::set_var("TRADING212_USE_STREAMING", "1");
    let streaming_result = tool.call_tool(&client, &config, &cache).await;

    // Both should succeed
    assert!(
        standard_result.is_ok(),
        "Standard pagination should succeed"
    );
    assert!(
        streaming_result.is_ok(),
        "Streaming pagination should succeed"
    );

    // Both should return limited results
    let standard_response = format!("{:?}", standard_result.unwrap());
    let streaming_response = format!("{:?}", streaming_result.unwrap());

    // Should contain pagination info
    assert!(
        standard_response.contains("Page 1"),
        "Standard should show page 1"
    );
    assert!(
        streaming_response.contains("Page 1"),
        "Streaming should show page 1"
    );

    // Cleanup
    std::env::remove_var("TRADING212_USE_STREAMING");
    std::env::remove_var("TRADING212_USE_STANDARD");
}

#[tokio::test]
async fn test_approach_selection_logic() {
    // Store original environment state
    let original_streaming = std::env::var("TRADING212_USE_STREAMING").ok();
    let original_standard = std::env::var("TRADING212_USE_STANDARD").ok();

    // Ensure clean environment for this test
    std::env::remove_var("TRADING212_USE_STREAMING");
    std::env::remove_var("TRADING212_USE_STANDARD");

    // Verify environment is actually clean before proceeding
    assert!(
        std::env::var("TRADING212_USE_STREAMING").is_err(),
        "TRADING212_USE_STREAMING should be unset"
    );
    assert!(
        std::env::var("TRADING212_USE_STANDARD").is_err(),
        "TRADING212_USE_STANDARD should be unset"
    );

    let (client, config, cache) = setup_test_environment().await;

    // Test that selective queries trigger streaming
    let selective_tool = GetInstrumentsTool {
        search: Some("Apple".to_string()),
        instrument_type: None,
        limit: Some(5), // Small limit with search should trigger streaming
        page: Some(1),
    };

    // Retry logic to handle race conditions with environment variables in parallel tests
    let mut should_use_streaming = false;
    let mut attempts = 0;
    let max_attempts = 10;

    while attempts < max_attempts {
        // Aggressively clean environment
        std::env::remove_var("TRADING212_USE_STREAMING");
        std::env::remove_var("TRADING212_USE_STANDARD");

        // Verify environment is clean
        let env_streaming = std::env::var("TRADING212_USE_STREAMING").ok();
        let env_standard = std::env::var("TRADING212_USE_STANDARD").ok();

        if env_streaming.is_none() && env_standard.is_none() {
            should_use_streaming = selective_tool.should_use_streaming();
            if should_use_streaming {
                break; // Test passed, exit retry loop
            }
        }

        attempts += 1;
        if attempts < max_attempts {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    }

    assert!(
        should_use_streaming,
        "Selective queries should use streaming after {attempts} attempts"
    );

    // Test that type-only filters trigger streaming
    let type_only_tool = GetInstrumentsTool {
        search: None,
        instrument_type: Some("STOCK".to_string()),
        limit: Some(100),
        page: Some(1),
    };

    assert!(
        type_only_tool.should_use_streaming(),
        "Type-only filters should use streaming"
    );

    // Test that large queries now use streaming approach (new default)
    let large_tool = GetInstrumentsTool {
        search: None,
        instrument_type: None,
        limit: Some(100),
        page: Some(1),
    };

    assert!(
        large_tool.should_use_streaming(),
        "Large queries should use streaming approach (new default)"
    );

    // Test forced streaming via environment
    std::env::set_var("TRADING212_USE_STREAMING", "1");
    assert!(
        large_tool.should_use_streaming(),
        "Environment variable should force streaming"
    );

    // Restore original environment state
    match original_streaming {
        Some(value) => std::env::set_var("TRADING212_USE_STREAMING", value),
        None => std::env::remove_var("TRADING212_USE_STREAMING"),
    }
    match original_standard {
        Some(value) => std::env::set_var("TRADING212_USE_STANDARD", value),
        None => std::env::remove_var("TRADING212_USE_STANDARD"),
    }
}
