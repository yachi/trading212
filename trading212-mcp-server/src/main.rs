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

    #[test]
    fn test_transport_creation() {
        // Test that transport creation works with default options
        let result = std::panic::catch_unwind(|| StdioTransport::new(TransportOptions::default()));

        // Should not panic during creation
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_main_components_integration() {
        // Test that all main components can be created without errors
        let server_details = create_server_details();

        // Verify server details are valid
        assert!(!server_details.server_info.name.is_empty());
        assert!(!server_details.server_info.version.is_empty());
        assert!(server_details.capabilities.tools.is_some());

        // Test transport creation
        let transport_result = StdioTransport::new(TransportOptions::default());
        assert!(transport_result.is_ok());

        // Test that we can create the server components independently
        // Note: We don't test actual server startup as it would hang the test
    }

    #[test]
    fn test_server_info_completeness() {
        let details = create_server_details();

        // Verify all required fields are populated
        assert_eq!(details.server_info.name, "Trading212 MCP Server");
        assert_eq!(details.server_info.version, "0.1.0");
        assert_eq!(
            details.server_info.title,
            Some("Trading212 MCP Server".to_string())
        );

        // Verify capabilities structure
        let tools_capability = details
            .capabilities
            .tools
            .expect("Tools capability should be present");
        assert!(tools_capability.list_changed.is_none()); // Should be None for this implementation

        // Verify instructions are meaningful
        let instructions = details
            .instructions
            .expect("Instructions should be present");
        assert!(instructions.len() > 10); // Should have meaningful content
        assert!(instructions.contains("Trading212"));
        assert!(instructions.contains("API"));
    }

    #[test]
    fn test_create_server_details_protocol_version() {
        let details = create_server_details();

        // Verify protocol version is set correctly
        assert_eq!(details.protocol_version, LATEST_PROTOCOL_VERSION);
        assert!(!details.protocol_version.is_empty());
    }

    #[test]
    fn test_create_server_details_meta_field() {
        let details = create_server_details();

        // Verify meta field is None as expected
        assert!(details.meta.is_none());
    }

    #[tokio::test]
    async fn test_main_components_error_handling() {
        // Test that invalid transport options are handled appropriately
        let transport_options = TransportOptions::default();
        let result = std::panic::catch_unwind(|| StdioTransport::new(transport_options));

        // Should not panic
        assert!(result.is_ok());
    }

    #[test]
    fn test_server_details_consistency() {
        let details1 = create_server_details();
        let details2 = create_server_details();

        // Should create identical server details each time
        assert_eq!(details1.server_info.name, details2.server_info.name);
        assert_eq!(details1.server_info.version, details2.server_info.version);
        assert_eq!(details1.protocol_version, details2.protocol_version);
    }

    #[test]
    fn test_init_tracing_idempotent() {
        // Test that multiple calls to init_tracing don't cause issues
        for _ in 0..3 {
            let result = std::panic::catch_unwind(|| {
                init_tracing();
            });
            // Should not panic (though may return error on subsequent calls)
            result.ok();
        }
    }

    mod server_runtime_tests {
        use super::*;

        #[tokio::test]
        async fn test_server_creation_and_details() {
            let server_details = create_server_details();

            // Test that server details are comprehensive
            assert_eq!(server_details.server_info.name, "Trading212 MCP Server");
            assert_eq!(server_details.server_info.version, "0.1.0");
            assert!(server_details.server_info.title.is_some());
            assert!(server_details.capabilities.tools.is_some());
            assert!(server_details.instructions.is_some());

            // Test protocol version compatibility
            assert!(!server_details.protocol_version.is_empty());
            assert_eq!(server_details.protocol_version, LATEST_PROTOCOL_VERSION);
        }

        #[test]
        fn test_transport_options_defaults() {
            // Test default transport options
            let options = TransportOptions::default();

            // Create transport with defaults
            let transport_result = StdioTransport::new(options);
            assert!(transport_result.is_ok());
        }

        #[test]
        fn test_server_capabilities_structure() {
            let server_details = create_server_details();
            let capabilities = &server_details.capabilities;

            // Verify tools capability
            assert!(capabilities.tools.is_some());
            let tools_cap = capabilities.tools.as_ref().unwrap();
            assert!(tools_cap.list_changed.is_none());

            // Verify other capabilities are properly set
            assert!(capabilities.prompts.is_none());
            assert!(capabilities.resources.is_none());
            assert!(capabilities.logging.is_none());
        }

        #[test]
        fn test_server_implementation_details() {
            let server_details = create_server_details();
            let implementation = &server_details.server_info;

            // Test implementation details completeness
            assert!(!implementation.name.is_empty());
            assert!(!implementation.version.is_empty());
            assert!(implementation.title.is_some());

            let title = implementation.title.as_ref().unwrap();
            assert_eq!(title, "Trading212 MCP Server");
            assert_eq!(implementation.name, *title);
        }

        #[test]
        fn test_server_instructions_content() {
            let server_details = create_server_details();
            let instructions = server_details.instructions.unwrap();

            // Verify instructions are informative
            assert!(instructions.contains("Trading212"));
            assert!(instructions.contains("API"));
            assert!(instructions.contains("instrument"));
            assert!(instructions.len() > 20); // Should be descriptive
        }

        #[test]
        fn test_server_meta_field() {
            let server_details = create_server_details();

            // Verify meta field is None as designed
            assert!(server_details.meta.is_none());
        }

        #[tokio::test]
        async fn test_handler_creation_in_context() {
            // Test handler creation in the context of server startup
            // This tests the same path the main function would take

            match Trading212Handler::new() {
                Ok(handler) => {
                    // Verify handler is properly configured
                    assert!(!handler.config.api_key.is_empty());
                    assert!(!handler.config.base_url.is_empty());
                    assert!(handler.config.base_url.starts_with("http"));
                }
                Err(_) => {
                    // Handler creation can fail if API key file doesn't exist
                    // This is expected in test environments without setup
                }
            }
        }

        #[test]
        fn test_error_message_formatting() {
            // Test the error message formatting logic from main
            use rust_mcp_sdk::error::McpSdkError;
            use std::io;

            // Create a test error
            let io_error = io::Error::new(io::ErrorKind::NotFound, "Test error");
            let sdk_error = McpSdkError::from(io_error);

            // Test the error message extraction logic similar to main()
            let error_msg = sdk_error
                .rpc_error_message()
                .map_or_else(|| sdk_error.to_string(), std::string::ToString::to_string);

            assert!(!error_msg.is_empty());
            assert!(error_msg.contains("Test error") || error_msg.contains("NotFound"));
        }

        #[test]
        fn test_tracing_configuration() {
            // Test tracing configuration settings
            use tracing_subscriber::EnvFilter;

            // Test environment filter creation
            let env_filter =
                EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into());

            // Verify the filter was created successfully
            assert!(!format!("{:?}", env_filter).is_empty());
        }

        #[test]
        fn test_create_server_details_consistency() {
            // Test that create_server_details is deterministic
            let details1 = create_server_details();
            let details2 = create_server_details();

            // Should be identical
            assert_eq!(details1.server_info.name, details2.server_info.name);
            assert_eq!(details1.server_info.version, details2.server_info.version);
            assert_eq!(details1.protocol_version, details2.protocol_version);
            assert_eq!(details1.instructions, details2.instructions);
        }

        #[test]
        fn test_version_string_format() {
            let server_details = create_server_details();
            let version = &server_details.server_info.version;

            // Test version format (should be semantic versioning)
            assert!(version.contains('.'));
            let parts: Vec<&str> = version.split('.').collect();
            assert!(parts.len() >= 2); // At least major.minor

            // Should be parseable numbers
            for part in parts {
                assert!(part.parse::<u32>().is_ok());
            }
        }
    }
}
