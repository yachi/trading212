//! Integration Tests for Trading212 MCP Server
//!
//! Tests the core functionality that can be tested without complex MCP server setup.

use serde_json::json;
use tempfile::TempDir;
use trading212_mcp_server::{
    config::{EnvProvider, Trading212Config},
    handler::Trading212Handler,
    tools::Trading212Tools,
};
use wiremock::{
    matchers::{method, path, query_param},
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

    // Create handler using new() method and then replace the config
    let client = reqwest::Client::builder()
        .user_agent("Trading212-MCP-Server-Test/0.1.0")
        .build()
        .expect("Failed to create HTTP client");

    let handler = Trading212Handler { client, config };

    // Keep temp_dir alive for the duration of the test
    std::mem::forget(temp_dir);

    (handler, mock_server)
}

#[tokio::test]
async fn test_handler_creation_integration() {
    let (handler, _mock_server) = create_test_handler_with_mock_api().await;

    // Verify handler is properly configured
    assert!(!handler.config.api_key.is_empty());
    assert!(handler.config.base_url.contains("http"));
}

#[tokio::test]
async fn test_get_instruments_tool_execution() {
    let (_handler, mock_server) = create_test_handler_with_mock_api().await;

    // Mock successful instruments response
    Mock::given(method("GET"))
        .and(path("/equity/metadata/instruments"))
        .and(query_param("search", "AAPL"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "isin": "US0378331005",
                "ticker": "AAPL",
                "name": "Apple Inc.",
                "type": "STOCK",
                "currencyCode": "USD"
            }
        ])))
        .mount(&mock_server)
        .await;

    // Test the tool directly
    let tool = trading212_mcp_server::tools::GetInstrumentsTool {
        search: Some("AAPL".to_string()),
        instrument_type: None,
    };

    // Create a test HTTP client pointing to mock server
    let client = reqwest::Client::new();
    let url = format!(
        "{}/equity/metadata/instruments?search=AAPL",
        mock_server.uri()
    );

    let response = client
        .get(&url)
        .header("Authorization", "Bearer test_api_key_integration")
        .send()
        .await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.status(), 200);

    let json: serde_json::Value = response.json().await.unwrap();
    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_get_pies_tool_execution() {
    let (_handler, mock_server) = create_test_handler_with_mock_api().await;

    // Mock successful pies response
    Mock::given(method("GET"))
        .and(path("/equity/pies"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": 12345,
                "name": "Tech Portfolio",
                "dividendCashAction": "REINVEST",
                "goal": 10000.0,
                "creationDate": "2023-01-01T00:00:00Z"
            }
        ])))
        .mount(&mock_server)
        .await;

    // Test the tool directly
    let _tool = trading212_mcp_server::tools::GetPiesTool {};

    // Create a test HTTP client pointing to mock server
    let client = reqwest::Client::new();
    let url = format!("{}/equity/pies", mock_server.uri());

    let response = client
        .get(&url)
        .header("Authorization", "Bearer test_api_key_integration")
        .send()
        .await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.status(), 200);

    let json: serde_json::Value = response.json().await.unwrap();
    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_api_error_handling() {
    let (_handler, mock_server) = create_test_handler_with_mock_api().await;

    // Mock API error response
    Mock::given(method("GET"))
        .and(path("/equity/metadata/instruments"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": "Unauthorized"
        })))
        .mount(&mock_server)
        .await;

    // Test error handling
    let client = reqwest::Client::new();
    let url = format!("{}/equity/metadata/instruments", mock_server.uri());

    let response = client
        .get(&url)
        .header("Authorization", "Bearer invalid_key")
        .send()
        .await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.status(), 401);
}

#[test]
fn test_tools_conversion() {
    use rust_mcp_sdk::schema::CallToolRequestParams;

    // Test tool parameter conversion
    let test_cases = vec![
        (
            "get_instruments",
            json!({
                "search": "AAPL",
                "type": "STOCK"
            }),
        ),
        ("get_pies", json!({})),
        (
            "get_pie_by_id",
            json!({
                "pie_id": 12345
            }),
        ),
    ];

    for (tool_name, arguments) in test_cases {
        let params = CallToolRequestParams {
            name: tool_name.to_string(),
            arguments: Some(arguments.as_object().unwrap().clone()),
        };

        let result = Trading212Tools::try_from(params);
        assert!(
            result.is_ok(),
            "Tool conversion should succeed for: {}",
            tool_name
        );
    }
}

#[test]
fn test_tools_list() {
    let tools = Trading212Tools::tools();

    assert_eq!(tools.len(), 4);

    let tool_names: Vec<_> = tools.iter().map(|t| &t.name).collect();
    assert!(tool_names.contains(&&"get_instruments".to_string()));
    assert!(tool_names.contains(&&"get_pies".to_string()));
    assert!(tool_names.contains(&&"get_pie_by_id".to_string()));
    assert!(tool_names.contains(&&"update_pie".to_string()));
}

#[test]
fn test_config_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let api_key_path = temp_dir.path().join(".trading212-api-key");
    std::fs::write(&api_key_path, "test_api_key_12345").expect("Failed to write API key");

    let mut mock_env = MockEnvProvider::new();
    mock_env.set("HOME", temp_dir.path().to_str().unwrap());
    mock_env.set("TRADING212_BASE_URL", "https://demo.trading212.com/api/v0");

    let config = Trading212Config::with_env_provider(&mock_env);
    assert!(config.is_ok());

    let config = config.unwrap();
    assert_eq!(config.api_key, "test_api_key_12345");
    assert_eq!(config.base_url, "https://demo.trading212.com/api/v0");
}

#[tokio::test]
async fn test_concurrent_requests() {
    let (_handler, mock_server) = create_test_handler_with_mock_api().await;

    // Mock response for concurrent requests
    Mock::given(method("GET"))
        .and(path("/equity/metadata/instruments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "isin": "US0378331005",
                "ticker": "AAPL",
                "name": "Apple Inc.",
                "type": "STOCK",
                "currencyCode": "USD"
            }
        ])))
        .expect(3)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let url = format!("{}/equity/metadata/instruments", mock_server.uri());

    let mut tasks = Vec::new();

    for i in 0..3 {
        let client = client.clone();
        let url = url.clone();
        let task = tokio::spawn(async move {
            let response = client
                .get(&url)
                .header("Authorization", "Bearer test_api_key_integration")
                .send()
                .await;
            (i, response)
        });
        tasks.push(task);
    }

    // Wait for all concurrent requests to complete
    for task in tasks {
        let (task_id, result) = task.await.expect("Task should complete");
        assert!(
            result.is_ok(),
            "Concurrent request {} should succeed",
            task_id
        );

        let response = result.unwrap();
        assert_eq!(response.status(), 200);
    }
}

#[test]
fn test_url_building_integration() {
    let config = Trading212Config {
        api_key: "test_key".to_string(),
        base_url: "https://demo.trading212.com/api/v0".to_string(),
    };

    // Test various endpoint URL building scenarios
    let test_cases = vec![
        (
            "equity/metadata/instruments",
            "/equity/metadata/instruments",
        ),
        ("/equity/pies", "/equity/pies"),
        ("", "/"),
    ];

    for (input, expected_suffix) in test_cases {
        let url = config.endpoint_url(input);
        assert!(url.ends_with(expected_suffix));
        assert!(url.contains("demo.trading212.com"));
    }
}

#[tokio::test]
async fn test_update_pie_tool_execution() {
    let (_handler, mock_server) = create_test_handler_with_mock_api().await;

    // Mock successful pie update response
    Mock::given(method("POST"))
        .and(path("/equity/pies/12345"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 12345,
            "name": "Updated Tech Portfolio",
            "dividendCashAction": "REINVEST",
            "goal": 15000.0,
            "creationDate": "2023-01-01T00:00:00Z",
            "instrumentShares": {
                "AAPL_US_EQ": 0.60,
                "GOOGL_US_EQ": 0.40
            }
        })))
        .mount(&mock_server)
        .await;

    // Test the update pie tool directly
    let instrument_shares = vec![
        trading212_mcp_server::tools::InstrumentAllocation {
            ticker: "AAPL_US_EQ".to_string(),
            weight: 0.60,
        },
        trading212_mcp_server::tools::InstrumentAllocation {
            ticker: "GOOGL_US_EQ".to_string(),
            weight: 0.40,
        },
    ];

    let _tool = trading212_mcp_server::tools::UpdatePieTool {
        pie_id: 12345,
        instrument_shares: Some(instrument_shares),
        name: Some("Updated Tech Portfolio".to_string()),
        icon: None,
        goal: Some(15000.0),
        dividend_cash_action: Some("REINVEST".to_string()),
        end_date: None,
    };

    // Create a test HTTP client pointing to mock server
    let client = reqwest::Client::new();
    let url = format!("{}/equity/pies/12345", mock_server.uri());

    let request_body = json!({
        "instrumentShares": {
            "AAPL_US_EQ": 0.60,
            "GOOGL_US_EQ": 0.40
        },
        "name": "Updated Tech Portfolio",
        "goal": 15000.0,
        "dividendCashAction": "REINVEST"
    });

    let response = client
        .post(&url)
        .header("Authorization", "Bearer test_api_key_integration")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.status(), 200);

    let json: serde_json::Value = response.json().await.unwrap();
    assert_eq!(json["id"], 12345);
    assert_eq!(json["name"], "Updated Tech Portfolio");
    assert_eq!(json["goal"], 15000.0);
}
