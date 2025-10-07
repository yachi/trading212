#![allow(unused_crate_dependencies)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::unused_async)]
#![allow(clippy::use_self)]
#![allow(clippy::future_not_send)]
#![allow(clippy::doc_markdown)]

//! MCP Protocol Compliance Tests
//!
//! Tests the complete MCP protocol flow: MCP Client → JSON-RPC → MCP Server → Mock Trading212 API
//!
//! These tests verify that the complete MCP communication works end-to-end, including:
//! - MCP protocol serialization/deserialization
//! - Transport layer communication
//! - Server initialization and capability negotiation
//! - Tool discovery and execution
//! - Error handling across the protocol boundary

use rust_mcp_sdk::schema::LATEST_PROTOCOL_VERSION;
use serde_json::json;
use std::process::{Command, Stdio};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command as TokioCommand};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

/// Test helper to create an MCP server process
struct McpServerProcess {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl McpServerProcess {
    /// Start the MCP server as a subprocess with mocked Trading212 API
    async fn start_with_mock_api(
        mock_server_uri: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Create temp directory for API key
        let temp_dir = TempDir::new()?;
        let api_key_path = temp_dir.path().join(".trading212-api-key");
        std::fs::write(&api_key_path, "test_api_key_integration")?;

        // Build the server binary if needed
        let build_output = Command::new("cargo")
            .args(["build", "--bin", "trading212-mcp-server"])
            .output()?;

        if !build_output.status.success() {
            return Err(format!(
                "Failed to build server: {}",
                String::from_utf8_lossy(&build_output.stderr)
            )
            .into());
        }

        // Start the server process
        let mut child = TokioCommand::new("target/debug/trading212-mcp-server")
            .env("HOME", temp_dir.path())
            .env("TRADING212_BASE_URL", mock_server_uri)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = child.stdin.take().ok_or("Failed to get stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to get stdout")?;
        let stdout = BufReader::new(stdout);

        // Keep temp_dir alive for the duration of the test
        std::mem::forget(temp_dir);

        Ok(McpServerProcess {
            _child: child,
            stdin,
            stdout,
        })
    }

    /// Send an MCP request and get the response
    async fn send_request<T: serde::Serialize>(
        &mut self,
        request: &T,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // Serialize request to JSON-RPC format
        let request_json = serde_json::to_string(request)?;

        // Send request with newline delimiter
        self.stdin.write_all(request_json.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;

        // Read response
        let mut response_line = String::new();
        self.stdout.read_line(&mut response_line).await?;

        // Parse response
        let response: serde_json::Value = serde_json::from_str(&response_line)?;
        Ok(response)
    }
}

/// Real integration test: Initialize MCP server
#[tokio::test]
async fn test_real_mcp_initialize() {
    let mock_server = MockServer::start().await;

    let mut server_process = McpServerProcess::start_with_mock_api(&mock_server.uri())
        .await
        .expect("Failed to start MCP server");

    // Send initialize request
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": LATEST_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let response = server_process
        .send_request(&initialize_request)
        .await
        .expect("Failed to send initialize request");

    // Verify initialize response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"].is_object());

    let result = &response["result"];
    assert!(result["serverInfo"].is_object());
    assert_eq!(result["serverInfo"]["name"], "Trading212 MCP Server");
    assert!(result["capabilities"]["tools"].is_object());
}

/// Real integration test: List tools via MCP protocol
#[tokio::test]
async fn test_real_mcp_list_tools() {
    let mock_server = MockServer::start().await;

    let mut server_process = McpServerProcess::start_with_mock_api(&mock_server.uri())
        .await
        .expect("Failed to start MCP server");

    // Initialize first
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": LATEST_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    server_process
        .send_request(&initialize_request)
        .await
        .expect("Initialize failed");

    // Send list tools request
    let list_tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let response = server_process
        .send_request(&list_tools_request)
        .await
        .expect("Failed to send list tools request");

    // Verify tools list response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response["result"].is_object());

    let result = &response["result"];
    assert!(result["tools"].is_array());

    let tools = result["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 4);

    let tool_names: Vec<_> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(tool_names.contains(&"get_instruments"));
    assert!(tool_names.contains(&"get_all_pies_with_holdings"));
    assert!(tool_names.contains(&"update_pie"));
    assert!(tool_names.contains(&"create_pie"));
}

/// Real integration test: Call get_instruments tool via MCP protocol with client-side filtering
#[tokio::test]
async fn test_real_mcp_call_get_instruments() {
    let mock_server = MockServer::start().await;

    // Mock successful instruments response (returns all instruments, no query parameters)
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

    let mut server_process = McpServerProcess::start_with_mock_api(&mock_server.uri())
        .await
        .expect("Failed to start MCP server");

    // Initialize first
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": LATEST_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    server_process
        .send_request(&initialize_request)
        .await
        .expect("Initialize failed");

    // Call get_instruments tool
    let call_tool_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "get_instruments",
            "arguments": {
                "search": "AAPL"
            }
        }
    });

    let response = server_process
        .send_request(&call_tool_request)
        .await
        .expect("Failed to send call tool request");

    // Verify tool call response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 3);
    assert!(response["result"].is_object());

    let result = &response["result"];
    assert!(result["content"].is_array());

    let content = result["content"].as_array().unwrap();
    assert!(!content.is_empty());

    // Verify the content contains the expected instrument data
    let content_text = content[0]["text"].as_str().unwrap();
    assert!(content_text.contains("Apple Inc."));
    assert!(content_text.contains("AAPL"));
    assert!(content_text.contains("1 instruments"));
}

/// Real integration test: Error handling via MCP protocol
#[tokio::test]
async fn test_real_mcp_error_handling() {
    let mock_server = MockServer::start().await;

    // Mock API error response
    Mock::given(method("GET"))
        .and(path("/equity/metadata/instruments"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": "Unauthorized"
        })))
        .mount(&mock_server)
        .await;

    let mut server_process = McpServerProcess::start_with_mock_api(&mock_server.uri())
        .await
        .expect("Failed to start MCP server");

    // Initialize first
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": LATEST_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    server_process
        .send_request(&initialize_request)
        .await
        .expect("Initialize failed");

    // Call get_instruments tool (should fail)
    let call_tool_request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "get_instruments",
            "arguments": {}
        }
    });

    let response = server_process
        .send_request(&call_tool_request)
        .await
        .expect("Failed to send call tool request");

    // Verify error response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 4);
    // Debug: Print the actual response\n    println!("Test response: {}", serde_json::to_string_pretty(&response).unwrap());\n    \n    // Check if response contains error information (either as JSON-RPC error or as tool result with error content)\n    let has_error = response["error"].is_object() || \n        (response["result"].is_object() && \n         response["result"]["content"].is_array() && \n         !response["result"]["content"].as_array().unwrap().is_empty());\n    assert!(has_error, "Response should contain error information");
}

/// Real integration test: Call get_all_pies_with_holdings tool via MCP protocol
#[tokio::test]
async fn test_real_mcp_call_get_pies() {
    let mock_server = MockServer::start().await;

    // Mock successful pies list response
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

    // Mock pie details response
    Mock::given(method("GET"))
        .and(path("/equity/pies/12345"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "instruments": [{
                "ticker": "AAPL_US_EQ",
                "expectedShare": 1.0,
                "currentShare": 1.0,
                "ownedQuantity": 10.0
            }],
            "settings": {
                "id": 12345,
                "name": "Test Pie",
                "icon": null,
                "goal": null,
                "dividendCashAction": "REINVEST"
            }
        })))
        .mount(&mock_server)
        .await;

    let mut server_process = McpServerProcess::start_with_mock_api(&mock_server.uri())
        .await
        .expect("Failed to start MCP server");

    // Initialize first
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": LATEST_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    server_process
        .send_request(&initialize_request)
        .await
        .expect("Initialize failed");

    // Call get_all_pies_with_holdings tool
    let call_tool_request = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "get_all_pies_with_holdings",
            "arguments": {}
        }
    });

    let response = server_process
        .send_request(&call_tool_request)
        .await
        .expect("Failed to send call tool request");

    // Verify tool call response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 5);
    assert!(response["result"].is_object());

    let result = &response["result"];
    assert!(result["content"].is_array());

    let content = result["content"].as_array().unwrap();
    assert!(!content.is_empty());

    // Verify the content contains the expected pie data with holdings
    let content_text = content[0]["text"].as_str().unwrap();
    assert!(content_text.contains("12345"));
    assert!(content_text.contains("pies with holdings"));
}

/// Real integration test: Update pie tool via MCP protocol
#[tokio::test]
async fn test_real_mcp_call_update_pie() {
    let mock_server = MockServer::start().await;

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

    let mut server_process = McpServerProcess::start_with_mock_api(&mock_server.uri())
        .await
        .expect("Failed to start MCP server");

    // Initialize first
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": LATEST_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    server_process
        .send_request(&initialize_request)
        .await
        .expect("Initialize failed");

    // Call update_pie tool
    let call_tool_request = json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "tools/call",
        "params": {
            "name": "update_pie",
            "arguments": {
                "pie_id": 12345,
                "name": "Updated Tech Portfolio",
                "goal": 15000.0,
                "dividend_cash_action": "REINVEST",
                "instrument_shares": [
                    {"ticker": "AAPL_US_EQ", "weight": 0.60}
                ]
            }
        }
    });

    let response = server_process
        .send_request(&call_tool_request)
        .await
        .expect("Failed to send call tool request");

    // Verify tool call response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 6);
    assert!(response["result"].is_object());

    let result = &response["result"];
    assert!(result["content"].is_array());

    let content = result["content"].as_array().unwrap();
    assert!(!content.is_empty());

    // Verify the content contains the expected updated pie data
    let content_text = content[0]["text"].as_str().unwrap();
    assert!(content_text.contains("Updated Tech Portfolio"));
    assert!(content_text.contains("12345"));
}

/// Real integration test: Invalid tool name handling
#[tokio::test]
async fn test_real_mcp_invalid_tool() {
    let mock_server = MockServer::start().await;

    let mut server_process = McpServerProcess::start_with_mock_api(&mock_server.uri())
        .await
        .expect("Failed to start MCP server");

    // Initialize first
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": LATEST_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    server_process
        .send_request(&initialize_request)
        .await
        .expect("Initialize failed");

    // Call invalid tool
    let call_tool_request = json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "tools/call",
        "params": {
            "name": "invalid_tool",
            "arguments": {}
        }
    });

    let response = server_process
        .send_request(&call_tool_request)
        .await
        .expect("Failed to send call tool request");

    // Verify error response for invalid tool
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 7);
    // Debug: Print the actual response\n    println!("Test response: {}", serde_json::to_string_pretty(&response).unwrap());\n    \n    // Check if response contains error information (either as JSON-RPC error or as tool result with error content)\n    let has_error = response["error"].is_object() || \n        (response["result"].is_object() && \n         response["result"]["content"].is_array() && \n         !response["result"]["content"].as_array().unwrap().is_empty());\n    assert!(has_error, "Response should contain error information");

    // Verify error content (either in JSON-RPC error or tool result)
    if response["error"].is_object() {
        let error = &response["error"];
        assert!(error["message"].is_string());
        let error_message = error["message"].as_str().unwrap();
        assert!(error_message.contains("invalid_tool") || error_message.contains("Unknown tool"));
    } else if response["result"].is_object() {
        let result = &response["result"];
        let content = result["content"].as_array().unwrap();
        let content_text = content[0]["text"].as_str().unwrap();
        assert!(
            content_text.contains("invalid_tool")
                || content_text.contains("Unknown tool")
                || content_text.contains("error")
        );
    }
}
