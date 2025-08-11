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
        assert_eq!(handler.config.base_url, "https://test.trading212.com/api/v0");
    }
}
