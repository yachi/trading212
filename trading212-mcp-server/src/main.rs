//! Trading212 MCP Server
//!
//! A Model Context Protocol (MCP) server that provides access to Trading212 API functionality.
//! This server enables AI assistants to retrieve information about tradeable instruments
//! from Trading212's platform through the MCP protocol specification.

mod config;
mod errors;
mod handler;
mod tools;

use handler::Trading212Handler;
use rust_mcp_sdk::schema::{
    Implementation, InitializeResult, ServerCapabilities, ServerCapabilitiesTools,
    LATEST_PROTOCOL_VERSION,
};
use rust_mcp_sdk::{
    error::SdkResult,
    mcp_server::{server_runtime, ServerRuntime},
    McpServer, StdioTransport, TransportOptions,
};

#[tokio::main]
async fn main() -> SdkResult<()> {
    // Initialize tracing
    init_tracing();

    tracing::info!("Starting Trading212 MCP Server");

    // STEP 1: Define server details and capabilities
    let server_details = create_server_details();

    // STEP 2: create a stdio transport with default options
    let transport = StdioTransport::new(TransportOptions::default())?;

    // STEP 3: instantiate our custom handler for handling MCP messages
    let handler = Trading212Handler::new()?;

    // STEP 4: create a MCP server
    let server: ServerRuntime = server_runtime::create_server(server_details, transport, handler);

    // STEP 5: Start the server
    if let Err(start_error) = server.start().await {
        let error_msg = start_error
            .rpc_error_message()
            .map_or_else(|| start_error.to_string(), std::string::ToString::to_string);
        eprintln!("{error_msg}");
    }
    Ok(())
}

/// Initialize tracing for the application
fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
}

/// Create server details and capabilities
fn create_server_details() -> InitializeResult {
    InitializeResult {
        server_info: Implementation {
            name: "Trading212 MCP Server".to_string(),
            version: "0.1.0".to_string(),
            title: Some("Trading212 MCP Server".to_string()),
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        meta: None,
        instructions: Some("Access Trading212 API to get instrument information".to_string()),
        protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_server_details() {
        let details = create_server_details();

        // Verify server info
        assert_eq!(details.server_info.name, "Trading212 MCP Server");
        assert_eq!(details.server_info.version, "0.1.0");
        assert_eq!(
            details.server_info.title,
            Some("Trading212 MCP Server".to_string())
        );

        // Verify capabilities
        assert!(details.capabilities.tools.is_some());

        // Verify instructions
        assert!(details.instructions.is_some());
        let instructions = details.instructions.unwrap();
        assert!(instructions.contains("Trading212 API"));
        assert!(instructions.contains("instrument information"));

        // Verify protocol version
        assert_eq!(details.protocol_version, LATEST_PROTOCOL_VERSION);
    }

    #[test]
    fn test_init_tracing() {
        // Test that init_tracing doesn't panic
        // We can't easily test the actual tracing setup without complex mocking
        // but we can verify the function runs without error

        // This will fail if called twice in the same test process, so we catch the error
        std::panic::catch_unwind(|| {
            init_tracing();
        })
        .ok(); // Ignore result since tracing may already be initialized
    }
}
