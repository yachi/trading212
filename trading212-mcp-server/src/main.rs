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
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting Trading212 MCP Server");

    // STEP 1: Define server details and capabilities
    let server_details = InitializeResult {
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
    };

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
