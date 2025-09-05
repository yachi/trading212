#![allow(unused_crate_dependencies)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::uninlined_format_args)]

//! Trading212 API Integration Tests
//!
//! Tests tool execution with the Trading212 API using mocked responses.

use rust_mcp_sdk::schema::CallToolRequestParams;
use serde_json::{json, Map};
use tempfile::TempDir;
use trading212_mcp_server::{
    config::{EnvProvider, Trading212Config},
    handler::Trading212Handler,
    tools::Trading212Tools,
};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

/// Mock environment provider for testing
struct MockEnvProvider {
    vars: std::collections::HashMap<String, String>,
}

impl MockEnvProvider {
    fn new() -> Self {
        Self {
            vars: std::collections::HashMap::new(),
        }
    }

    fn set(&mut self, key: &str, value: &str) {
        self.vars.insert(key.to_string(), value.to_string());
    }
}

impl EnvProvider for MockEnvProvider {
    fn var(&self, key: &str) -> Result<String, std::env::VarError> {
        self.vars
            .get(key)
            .cloned()
            .ok_or(std::env::VarError::NotPresent)
    }
}

/// Create a test handler with mocked Trading212 API
async fn create_test_handler_with_mock_api() -> (Trading212Handler, MockServer) {
    let mock_server = MockServer::start().await;
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let api_key_path = temp_dir.path().join(".trading212-api-key");
    std::fs::write(&api_key_path, "test_api_key_integration").expect("Failed to write API key");

    let mut mock_env = MockEnvProvider::new();
    mock_env.set("HOME", temp_dir.path().to_str().unwrap());
    mock_env.set("TRADING212_BASE_URL", &mock_server.uri());

    let config =
        Trading212Config::with_env_provider(&mock_env).expect("Failed to create test config");

    // Create handler
    let client = reqwest::Client::builder()
        .user_agent("Trading212-MCP-Server-Test/0.1.0")
        .build()
        .expect("Failed to create HTTP client");

    let cache = trading212_mcp_server::Trading212Cache::new().expect("Failed to create cache");
    let handler = Trading212Handler {
        client,
        config,
        cache,
    };

    // Keep temp_dir alive for the duration of the test
    std::mem::forget(temp_dir);

    (handler, mock_server)
}

#[tokio::test]
async fn test_get_instruments_tool_execution() {
    let (handler, mock_server) = create_test_handler_with_mock_api().await;

    // Mock successful instruments response (no query parameters expected)
    Mock::given(method("GET"))
        .and(path("/equity/metadata/instruments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "isin": "US0378331005",
                "ticker": "AAPL",
                "name": "Apple Inc.",
                "type": "STOCK",
                "currencyCode": "USD",
                "workingScheduleId": 1,
                "shortName": "Apple",
                "maxOpenQuantity": 1000.0,
                "addedOn": "2020-01-01"
            },
            {
                "isin": "US5949181045",
                "ticker": "MSFT",
                "name": "Microsoft Corporation",
                "type": "STOCK",
                "currencyCode": "USD",
                "workingScheduleId": 1,
                "shortName": "Microsoft",
                "maxOpenQuantity": 2000.0,
                "addedOn": "2020-01-01"
            }
        ])))
        .mount(&mock_server)
        .await;

    // Test via handler's call_tool_request method
    let mut arguments = Map::new();
    arguments.insert("search".to_string(), json!("AAPL"));

    let call_params = CallToolRequestParams {
        name: "get_instruments".to_string(),
        arguments: Some(arguments),
    };

    let result = Trading212Tools::try_from(call_params).expect("Tool conversion should succeed");

    match result {
        Trading212Tools::GetInstrumentsTool(tool) => {
            let tool_result = tool
                .call_tool(&handler.client, &handler.config, &handler.cache)
                .await;
            assert!(tool_result.is_ok());
            let content = tool_result.unwrap();
            assert!(!content.content.is_empty());
        }
        _ => panic!("Expected GetInstrumentsTool"),
    }
}

#[tokio::test]
async fn test_get_pies_tool_execution() {
    let (handler, mock_server) = create_test_handler_with_mock_api().await;

    // Mock successful pies response
    Mock::given(method("GET"))
        .and(path("/equity/pies"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": 12345,
                "cash": 100.0,
                "dividendDetails": {
                    "gained": 5.0,
                    "reinvested": 4.0,
                    "inCash": 1.0
                },
                "result": {
                    "priceAvgInvestedValue": 1000.0,
                    "priceAvgValue": 1100.0,
                    "priceAvgResult": 100.0,
                    "priceAvgResultCoef": 0.1
                },
                "progress": 0.75,
                "status": "AHEAD"
            }
        ])))
        .mount(&mock_server)
        .await;

    // Test via handler's call_tool_request method
    let call_params = CallToolRequestParams {
        name: "get_pies".to_string(),
        arguments: Some(Map::new()),
    };

    let result = Trading212Tools::try_from(call_params).expect("Tool conversion should succeed");

    match result {
        Trading212Tools::GetPiesTool(tool) => {
            let tool_result = tool
                .call_tool(&handler.client, &handler.config, &handler.cache)
                .await;
            assert!(tool_result.is_ok());
            let content = tool_result.unwrap();
            assert!(!content.content.is_empty());
        }
        _ => panic!("Expected GetPiesTool"),
    }
}

#[tokio::test]
async fn test_api_error_handling() {
    let (handler, mock_server) = create_test_handler_with_mock_api().await;

    // Mock API error response
    Mock::given(method("GET"))
        .and(path("/equity/metadata/instruments"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": "Unauthorized"
        })))
        .mount(&mock_server)
        .await;

    // Test error handling via handler
    let call_params = CallToolRequestParams {
        name: "get_instruments".to_string(),
        arguments: Some(Map::new()),
    };

    let result = Trading212Tools::try_from(call_params).expect("Tool conversion should succeed");

    match result {
        Trading212Tools::GetInstrumentsTool(tool) => {
            let tool_result = tool
                .call_tool(&handler.client, &handler.config, &handler.cache)
                .await;
            assert!(tool_result.is_err());
            let error = tool_result.unwrap_err();
            assert!(error.to_string().contains("API error") || error.to_string().contains("401"));
        }
        _ => panic!("Expected GetInstrumentsTool"),
    }
}

#[tokio::test]
async fn test_sequential_tool_requests() {
    let (handler, mock_server) = create_test_handler_with_mock_api().await;

    // Mock response for sequential requests (only expect 1 due to caching)
    Mock::given(method("GET"))
        .and(path("/equity/metadata/instruments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "isin": "US0378331005",
                "ticker": "AAPL",
                "name": "Apple Inc.",
                "type": "STOCK",
                "currencyCode": "USD",
                "workingScheduleId": 1,
                "shortName": "Apple",
                "maxOpenQuantity": 1000.0,
                "addedOn": "2020-01-01"
            }
        ])))
        .expect(1) // Only first request hits the server, rest served from cache
        .mount(&mock_server)
        .await;

    // Test multiple sequential calls to verify tool execution works reliably with caching
    for i in 0..3 {
        let call_params = CallToolRequestParams {
            name: "get_instruments".to_string(),
            arguments: Some(Map::new()),
        };

        let result =
            Trading212Tools::try_from(call_params).expect("Tool conversion should succeed");

        let tool_result = match result {
            Trading212Tools::GetInstrumentsTool(tool) => {
                tool.call_tool(&handler.client, &handler.config, &handler.cache)
                    .await
            }
            _ => panic!("Expected GetInstrumentsTool"),
        };

        assert!(tool_result.is_ok(), "Tool request {} should succeed", i);
    }
}

#[tokio::test]
async fn test_update_pie_tool_execution() {
    let (handler, mock_server) = create_test_handler_with_mock_api().await;

    // Mock successful pie update response
    Mock::given(method("POST"))
        .and(path("/equity/pies/12345"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "instruments": [
                {
                    "ticker": "AAPL_US_EQ",
                    "result": {
                        "priceAvgInvestedValue": 100.0,
                        "priceAvgValue": 105.0,
                        "priceAvgResult": 5.0,
                        "priceAvgResultCoef": 0.05
                    },
                    "expectedShare": 0.60,
                    "currentShare": 0.58,
                    "ownedQuantity": 1.0,
                    "issues": []
                }
            ],
            "settings": {
                "id": 12345,
                "instrumentShares": null,
                "name": "Updated Tech Portfolio",
                "icon": "tech",
                "goal": 15000.0,
                "creationDate": 1_640_995_200.0,
                "endDate": "2025-12-31T23:59:59.999+00:00",
                "initialInvestment": 1000.0,
                "dividendCashAction": "REINVEST",
                "publicUrl": null
            }
        })))
        .mount(&mock_server)
        .await;

    // Test via handler's call_tool_request method
    let mut arguments = Map::new();
    arguments.insert("pie_id".to_string(), json!(12345));
    arguments.insert("name".to_string(), json!("Updated Tech Portfolio"));
    arguments.insert("goal".to_string(), json!(15000.0));
    arguments.insert("dividend_cash_action".to_string(), json!("REINVEST"));
    arguments.insert(
        "instrument_shares".to_string(),
        json!([
            {"ticker": "AAPL_US_EQ", "weight": 0.60}
        ]),
    );

    let call_params = CallToolRequestParams {
        name: "update_pie".to_string(),
        arguments: Some(arguments),
    };

    let result = Trading212Tools::try_from(call_params).expect("Tool conversion should succeed");

    match result {
        Trading212Tools::UpdatePieTool(tool) => {
            let tool_result = tool
                .call_tool(&handler.client, &handler.config, &handler.cache)
                .await;
            assert!(tool_result.is_ok());
            let content = tool_result.unwrap();
            assert!(!content.content.is_empty());
        }
        _ => panic!("Expected UpdatePieTool"),
    }
}
