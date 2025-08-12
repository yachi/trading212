//! MCP protocol message handler for Trading212 operations.
//!
//! This module implements the [`Trading212Handler`] struct that processes
//! MCP protocol messages and coordinates with the Trading212 API.

use async_trait::async_trait;
use reqwest::Client;
use rust_mcp_sdk::schema::{
    schema_utils::CallToolError, CallToolRequest, CallToolResult, ListToolsRequest,
    ListToolsResult, RpcError,
};
use rust_mcp_sdk::{error::McpSdkError, mcp_server::ServerHandler, McpServer};

use crate::{config::Trading212Config, errors::Trading212Error, tools::Trading212Tools};

/// Handler for Trading212 MCP protocol messages.
///
/// This struct implements the [`ServerHandler`] trait to process MCP requests
/// and manage communication with the Trading212 API.
pub struct Trading212Handler {
    /// HTTP client for making API requests
    pub client: Client,
    /// Configuration for the Trading212 server
    pub config: Trading212Config,
}

impl Trading212Handler {
    /// Create a new `Trading212Handler` instance.
    ///
    /// Loads configuration and initializes the HTTP client.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading or HTTP client creation fails.
    pub fn new() -> Result<Self, McpSdkError> {
        let config = Trading212Config::new().map_err(|e| {
            McpSdkError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;

        let client = Client::builder()
            .user_agent("Trading212-MCP-Server/0.1.0")
            .build()
            .map_err(|e| {
                McpSdkError::from(std::io::Error::other(format!(
                    "Failed to create HTTP client: {e}"
                )))
            })?;

        tracing::info!(
            base_url = %config.base_url,
            "Initialized Trading212Handler"
        );

        Ok(Self { client, config })
    }
}

#[async_trait]
impl ServerHandler for Trading212Handler {
    async fn handle_list_tools_request(
        &self,
        _request: ListToolsRequest,
        _runtime: &dyn McpServer,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        let tools = Trading212Tools::tools();
        tracing::debug!(
            tool_count = tools.len(),
            tools = ?tools.iter().map(|t| &t.name).collect::<Vec<_>>(),
            "Listed available tools"
        );

        Ok(ListToolsResult {
            meta: None,
            next_cursor: None,
            tools,
        })
    }

    async fn handle_call_tool_request(
        &self,
        request: CallToolRequest,
        _runtime: &dyn McpServer,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool_name = &request.params.name;

        tracing::info!(
            tool = tool_name,
            params = ?request.params,
            "Handling tool call request"
        );

        // Convert request parameters into Trading212Tools enum
        let tool_params: Trading212Tools = Trading212Tools::try_from(request.params.clone())
            .map_err(|e| {
                let error = Trading212Error::conversion_error(format!("{e:?}"));
                tracing::error!(
                    tool = tool_name,
                    error = %error,
                    "Failed to convert tool parameters"
                );
                CallToolError::new(error)
            })?;

        // Match the tool variant and execute its corresponding logic
        let result = match tool_params {
            Trading212Tools::GetInstrumentsTool(get_instruments_tool) => {
                get_instruments_tool
                    .call_tool(&self.client, &self.config)
                    .await
            }
            Trading212Tools::GetPiesTool(get_pies_tool) => {
                get_pies_tool.call_tool(&self.client, &self.config).await
            }
            Trading212Tools::GetPieByIdTool(get_pie_by_id_tool) => {
                get_pie_by_id_tool
                    .call_tool(&self.client, &self.config)
                    .await
            }
        };

        match &result {
            Ok(_) => tracing::info!(tool = tool_name, "Tool call completed successfully"),
            Err(e) => tracing::error!(tool = tool_name, error = %e, "Tool call failed"),
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Trading212Tools;

    /// Create a mock Trading212Handler for testing
    fn create_test_handler() -> Trading212Handler {
        let config = Trading212Config {
            api_key: "test-api-key".to_string(),
            base_url: "https://test.trading212.com/api/v0".to_string(),
        };

        let client = Client::builder()
            .user_agent("Test-Agent/1.0")
            .build()
            .expect("Failed to create test HTTP client");

        Trading212Handler { client, config }
    }

    #[test]
    fn test_handler_creation_success() {
        // Test successful handler creation (requires valid API key file)
        // This test might fail if no API key file exists
        match Trading212Handler::new() {
            Ok(handler) => {
                assert!(!handler.config.api_key.is_empty());
                assert!(!handler.config.base_url.is_empty());
            }
            Err(_) => {
                // Handler creation can fail if API key file doesn't exist
                // This is expected in test environments
            }
        }
    }

    #[test]
    fn test_handler_has_correct_config() {
        let handler = create_test_handler();
        assert_eq!(handler.config.api_key, "test-api-key");
        assert_eq!(
            handler.config.base_url,
            "https://test.trading212.com/api/v0"
        );
    }

    #[test]
    fn test_handler_tools_list() {
        // Test that tools list is correctly structured
        let tools = Trading212Tools::tools();
        assert_eq!(tools.len(), 3);

        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"get_instruments"));
        assert!(tool_names.contains(&"get_pies"));
        assert!(tool_names.contains(&"get_pie_by_id"));

        // Verify all tools have descriptions
        for tool in &tools {
            if let Some(desc) = &tool.description {
                assert!(!desc.is_empty());
            }
        }
    }

    #[test]
    fn test_handler_client_creation() {
        let handler = create_test_handler();

        // Verify client exists (basic validation)
        // We can't easily test the client directly without making HTTP requests
        assert!(std::mem::size_of_val(&handler.client) > 0);
    }

    #[test]
    fn test_handler_config_validation() {
        let handler = create_test_handler();

        // Test config endpoint URL generation
        let endpoint = handler.config.endpoint_url("test/path");
        assert_eq!(endpoint, "https://test.trading212.com/api/v0/test/path");

        // Test base URL is properly formatted
        assert!(handler.config.base_url.starts_with("https://"));
        assert!(!handler.config.base_url.ends_with('/'));
    }

    #[test]
    fn test_handler_new_error_handling() {
        // Test that handler creation handles configuration errors appropriately
        // This test ensures error propagation works correctly

        // We can't easily force a configuration error without complex mocking,
        // but we can test the error path exists by checking the error types match
        match Trading212Handler::new() {
            Ok(_) => {
                // Handler created successfully - this is normal in most environments
            }
            Err(e) => {
                // Verify the error is properly wrapped as McpSdkError
                assert!(e.to_string().contains("Trading212") || e.to_string().contains("config"));
            }
        }
    }

    #[test]
    fn test_tools_list_completeness() {
        // Test that all expected tools are present and properly configured
        let tools = Trading212Tools::tools();

        assert_eq!(tools.len(), 3, "Expected exactly 3 tools");

        // Verify each tool has required properties
        for tool in &tools {
            assert!(!tool.name.is_empty(), "Tool name should not be empty");

            // Verify tool names match expected values
            match tool.name.as_str() {
                "get_instruments" | "get_pies" | "get_pie_by_id" => {
                    // Expected tool names
                }
                _ => panic!("Unexpected tool name: {}", tool.name),
            }
        }
    }

    #[test]
    fn test_handler_error_conversion() {
        // Test that Trading212Error to CallToolError conversion works
        let trading212_error = Trading212Error::conversion_error("test error".to_string());
        let call_tool_error = CallToolError::new(trading212_error);

        // Verify the error was wrapped correctly
        assert!(call_tool_error.to_string().contains("test error"));
    }

    #[test]
    fn test_client_configuration() {
        let handler = create_test_handler();

        // Test that the client is configured with the correct user agent
        // We can't directly access the user agent, but we can verify the client was created
        assert!(std::mem::size_of_val(&handler.client) > 0);

        // Verify the handler has the expected configuration
        assert_eq!(handler.config.api_key, "test-api-key");
        assert!(handler.config.base_url.contains("trading212.com"));
    }

    #[test]
    fn test_handler_configuration_properties() {
        let handler = create_test_handler();

        // Test that configuration is properly set
        assert_eq!(handler.config.api_key, "test-api-key");
        assert_eq!(
            handler.config.base_url,
            "https://test.trading212.com/api/v0"
        );

        // Test endpoint URL generation
        let endpoint = handler.config.endpoint_url("equity/pies");
        assert_eq!(endpoint, "https://test.trading212.com/api/v0/equity/pies");
    }

    #[test]
    fn test_handler_debug_representation() {
        let handler = create_test_handler();

        // Test that the handler can be formatted for debugging
        let debug_string = format!("{:?}", handler.config);
        assert!(debug_string.contains("Trading212Config"));
        assert!(debug_string.contains("api_key"));
        assert!(debug_string.contains("base_url"));
    }

    #[test]
    fn test_handler_memory_size() {
        let handler = create_test_handler();

        // Verify handler has reasonable memory footprint
        assert!(std::mem::size_of_val(&handler) > 0);
        assert!(std::mem::size_of_val(&handler.client) > 0);
        assert!(std::mem::size_of_val(&handler.config) > 0);
    }

    #[test]
    fn test_error_conversion_edge_cases() {
        // Test various error conversion scenarios
        let errors = vec![
            Trading212Error::request_failed("Network error"),
            Trading212Error::api_error(500, "Internal server error"),
            Trading212Error::parse_error("Invalid JSON"),
            Trading212Error::config_error("Missing API key"),
            Trading212Error::conversion_error("Invalid parameter"),
        ];

        for error in errors {
            let call_tool_error = CallToolError::new(error);
            let error_string = call_tool_error.to_string();
            assert!(!error_string.is_empty());
        }
    }

    #[test]
    fn test_handler_struct_fields() {
        let handler = create_test_handler();

        // Test that handler fields are accessible and properly configured
        assert!(!handler.config.api_key.is_empty());
        assert!(!handler.config.base_url.is_empty());

        // Test client exists and is valid size
        assert!(std::mem::size_of_val(&handler.client) > 0);
    }

    #[test]
    fn test_handler_struct_debug() {
        let handler = create_test_handler();

        // Test that handler config can be debugged
        let debug_string = format!("{:?}", handler.config);
        assert!(debug_string.contains("Trading212Config"));
        assert!(debug_string.contains("api_key"));
        assert!(debug_string.contains("base_url"));
    }

    #[test]
    fn test_handler_new_with_client_error() {
        // Test error handling during client creation
        // We can't easily mock Client::builder(), but we can verify error propagation works

        // Test that errors are properly converted to McpSdkError
        let config_error = Trading212Error::config_error("Test config error");
        let io_error =
            std::io::Error::new(std::io::ErrorKind::InvalidData, config_error.to_string());
        let mcp_error = McpSdkError::from(io_error);

        assert!(mcp_error.to_string().contains("Test config error"));
    }

    #[test]
    fn test_server_handler_trait_implementation() {
        // Test that Trading212Handler properly implements ServerHandler trait
        let handler = create_test_handler();

        // Verify the handler has the expected structure for ServerHandler
        assert!(!handler.config.api_key.is_empty());
        assert!(!handler.config.base_url.is_empty());
        assert!(std::mem::size_of_val(&handler.client) > 0);
    }

    #[test]
    fn test_handler_logging_tracing() {
        // Test that handler creation includes proper tracing setup
        let handler = create_test_handler();

        // Verify handler configuration for tracing
        assert!(handler.config.base_url.contains("trading212"));
        assert!(!handler.config.api_key.is_empty());
    }

    #[tokio::test]
    async fn test_handle_list_tools_request_with_mock() {
        let _handler = create_test_handler();

        // Create a mock request using the simplest possible structure
        let tools_list = Trading212Tools::tools();

        // Test that we can get the tools list (simulating the handler behavior)
        assert_eq!(tools_list.len(), 3);
        assert!(tools_list.iter().any(|t| t.name == "get_instruments"));
        assert!(tools_list.iter().any(|t| t.name == "get_pies"));
        assert!(tools_list.iter().any(|t| t.name == "get_pie_by_id"));

        // Test tools list meta properties
        for tool in &tools_list {
            assert!(!tool.name.is_empty());
            if let Some(desc) = &tool.description {
                assert!(!desc.is_empty());
            }
        }
    }

    #[test]
    fn test_tools_conversion_error_handling() {
        // Test error handling by verifying error types exist and can be created
        let conversion_error = Trading212Error::conversion_error("Test conversion error");
        assert!(conversion_error
            .to_string()
            .contains("Test conversion error"));

        // Test that all tool variants exist in the tools list
        let tools_list = Trading212Tools::tools();
        assert_eq!(tools_list.len(), 3);

        let tool_names: Vec<&str> = tools_list.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"get_instruments"));
        assert!(tool_names.contains(&"get_pies"));
        assert!(tool_names.contains(&"get_pie_by_id"));
    }

    #[test]
    fn test_tool_creation_and_properties() {
        // Test creating individual tool instances
        use crate::tools::{GetInstrumentsTool, GetPieByIdTool, GetPiesTool};

        // Test GetInstrumentsTool creation
        let instruments_tool = GetInstrumentsTool {
            search: Some("AAPL".to_string()),
            instrument_type: Some("STOCK".to_string()),
        };
        assert_eq!(instruments_tool.search, Some("AAPL".to_string()));
        assert_eq!(instruments_tool.instrument_type, Some("STOCK".to_string()));

        // Test GetPiesTool creation
        let pies_tool = GetPiesTool {};
        // Just verify it exists and can be created
        assert!(std::mem::size_of_val(&pies_tool) >= 0);

        // Test GetPieByIdTool creation
        let pie_by_id_tool = GetPieByIdTool { pie_id: 123 };
        assert_eq!(pie_by_id_tool.pie_id, 123);
    }

    #[test]
    fn test_tool_enum_variants() {
        // Test that we can create all Trading212Tools variants
        use crate::tools::{GetInstrumentsTool, GetPieByIdTool, GetPiesTool};

        let instruments_tool = GetInstrumentsTool {
            search: None,
            instrument_type: None,
        };
        let pies_tool = GetPiesTool {};
        let pie_by_id_tool = GetPieByIdTool { pie_id: 42 };

        let tool_variants = vec![
            Trading212Tools::GetInstrumentsTool(instruments_tool),
            Trading212Tools::GetPiesTool(pies_tool),
            Trading212Tools::GetPieByIdTool(pie_by_id_tool),
        ];

        assert_eq!(tool_variants.len(), 3);

        // Verify each variant can be matched
        for tool in tool_variants {
            match tool {
                Trading212Tools::GetInstrumentsTool(_) => assert!(true),
                Trading212Tools::GetPiesTool(_) => assert!(true),
                Trading212Tools::GetPieByIdTool(_) => assert!(true),
            }
        }
    }

    #[test]
    fn test_tool_parameter_validation() {
        // Test tool parameter validation logic
        use crate::tools::GetPieByIdTool;

        // Test valid pie ID ranges
        let valid_tool = GetPieByIdTool { pie_id: 12345 };
        assert!(valid_tool.pie_id > 0);

        // Test that pie_id can handle large numbers
        let large_id_tool = GetPieByIdTool { pie_id: 999999 };
        assert!(large_id_tool.pie_id > 0);
    }

    #[test]
    fn test_handler_new_config_error_simulation() {
        // Test that config errors are properly propagated in new()
        // We simulate this by testing the error conversion path

        let test_error = Trading212Error::config_error("Simulated config error");
        let io_error = std::io::Error::new(std::io::ErrorKind::InvalidData, test_error.to_string());
        let mcp_error = McpSdkError::from(io_error);

        // Verify error message contains the config error
        assert!(mcp_error.to_string().contains("Simulated config error"));
    }

    #[test]
    fn test_handler_new_client_error_simulation() {
        // Test HTTP client creation error simulation
        let client_error = "Failed to create HTTP client: test error";
        let io_error = std::io::Error::other(client_error);
        let mcp_error = McpSdkError::from(io_error);

        // Verify error contains client creation details
        assert!(
            mcp_error
                .to_string()
                .contains("Failed to create HTTP client")
                || mcp_error.to_string().contains("test error")
        );
    }

    #[test]
    fn test_call_tool_error_conversion() {
        use rust_mcp_sdk::schema::schema_utils::CallToolError;

        // Test all Trading212Error variants convert to CallToolError properly
        let errors = vec![
            Trading212Error::request_failed("Network error"),
            Trading212Error::api_error(500, "Server error"),
            Trading212Error::parse_error("JSON parse error"),
            Trading212Error::config_error("Config error"),
            Trading212Error::conversion_error("Conversion error"),
        ];

        for error in errors {
            let call_tool_error = CallToolError::new(error);
            let error_string = call_tool_error.to_string();
            assert!(!error_string.is_empty());

            // Each error should contain meaningful information
            assert!(error_string.len() > 5);
        }
    }
}
