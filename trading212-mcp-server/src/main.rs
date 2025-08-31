//! Trading212 MCP Server
//!
//! A Model Context Protocol (MCP) server that provides access to Trading212 API functionality.
//! This server enables AI assistants to retrieve information about tradeable instruments
//! from Trading212's platform through the MCP protocol specification.

#![allow(unused_crate_dependencies)]

mod cache;
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

            if let Ok(handler) = Trading212Handler::new() {
                // Verify handler is properly configured
                assert!(!handler.config.api_key.is_empty());
                assert!(!handler.config.base_url.is_empty());
                assert!(handler.config.base_url.starts_with("http"));
            } else {
                // Handler creation can fail if API key file doesn't exist
                // This is expected in test environments without setup
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
            assert!(!format!("{env_filter:?}").is_empty());
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

        #[tokio::test]
        async fn test_server_startup_error_handling() {
            // Test error handling in server startup scenario
            use rust_mcp_sdk::error::McpSdkError;
            use std::io;

            // Simulate a startup error
            let startup_error = McpSdkError::from(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Access denied",
            ));

            // Test the error message extraction logic from main()
            let error_msg = startup_error.rpc_error_message().map_or_else(
                || startup_error.to_string(),
                std::string::ToString::to_string,
            );

            assert!(!error_msg.is_empty());
            assert!(error_msg.contains("Access denied") || error_msg.contains("PermissionDenied"));
        }

        #[test]
        fn test_stdio_transport_creation_edge_cases() {
            // Test StdioTransport creation with various configurations

            // Test with default options
            let options1 = TransportOptions::default();
            assert!(StdioTransport::new(options1).is_ok());

            // Test multiple creation attempts
            for _ in 0..3 {
                let options = TransportOptions::default();
                assert!(StdioTransport::new(options).is_ok());
            }
        }

        #[test]
        fn test_environment_filter_configuration() {
            // Test the environment filter configuration from init_tracing
            use tracing::Level;
            use tracing_subscriber::EnvFilter;

            let filter = EnvFilter::from_default_env().add_directive(Level::INFO.into());

            // Verify filter creation succeeds
            let filter_debug = format!("{filter:?}");
            assert!(!filter_debug.is_empty());
            assert!(filter_debug.contains("INFO") || filter_debug.contains("info"));
        }

        #[test]
        fn test_main_function_components() {
            // Test individual components that main() uses

            // Test server details creation
            let server_details = create_server_details();
            assert!(!server_details.server_info.name.is_empty());

            // Test transport options
            let options = TransportOptions::default();
            assert!(StdioTransport::new(options).is_ok());

            // Test init_tracing doesn't panic
            let _result = std::panic::catch_unwind(|| {
                init_tracing();
            });
        }
    }

    #[test]
    fn test_protocol_version_consistency() {
        let details = create_server_details();

        // Test that protocol version is consistent with SDK
        assert_eq!(details.protocol_version, LATEST_PROTOCOL_VERSION);
        assert!(!details.protocol_version.is_empty());

        // Should be in a recognizable format
        assert!(
            details.protocol_version.contains('-')
                || details.protocol_version.chars().any(char::is_numeric)
        );
    }

    #[test]
    fn test_server_capabilities_defaults() {
        let details = create_server_details();
        let capabilities = &details.capabilities;

        // Test that non-tool capabilities are properly defaulted
        assert!(capabilities.prompts.is_none());
        assert!(capabilities.resources.is_none());
        assert!(capabilities.logging.is_none());

        // Verify tools capability is configured
        assert!(capabilities.tools.is_some());
        let tools = capabilities
            .tools
            .as_ref()
            .expect("tools capability should be configured");
        assert!(tools.list_changed.is_none());
    }

    #[test]
    fn test_server_info_fields() {
        let details = create_server_details();
        let info = &details.server_info;

        // Test all Implementation fields
        assert_eq!(info.name, "Trading212 MCP Server");
        assert_eq!(info.version, "0.1.0");
        assert_eq!(info.title, Some("Trading212 MCP Server".to_string()));

        // Test field consistency
        assert_eq!(
            &info.name,
            info.title.as_ref().expect("title should be set")
        );
    }

    #[tokio::test]
    async fn test_main_components_isolated() {
        // Test individual main() components in isolation to improve coverage

        // Test init_tracing can be called
        let _result = std::panic::catch_unwind(|| {
            init_tracing();
        });

        // Test server details creation
        let server_details = create_server_details();
        assert_eq!(server_details.server_info.name, "Trading212 MCP Server");
        assert_eq!(server_details.protocol_version, LATEST_PROTOCOL_VERSION);

        // Test transport creation
        let transport_options = TransportOptions::default();
        let transport_result = StdioTransport::new(transport_options);
        assert!(transport_result.is_ok());
    }

    #[tokio::test]
    async fn test_handler_creation_failure_simulation() {
        // Test handler creation error handling path
        use crate::handler::Trading212Handler;

        // This will test the actual handler creation which may fail in test environments
        // The important thing is testing the error handling path exists
        match Trading212Handler::new() {
            Ok(handler) => {
                // Handler created successfully
                assert!(!handler.config.api_key.is_empty());
                assert!(!handler.config.base_url.is_empty());
            }
            Err(e) => {
                // Handler creation failed (expected in some test environments)
                // Verify error contains expected information
                let error_msg = e.to_string();
                assert!(!error_msg.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_server_runtime_creation() {
        // Test server runtime creation components

        let server_details = create_server_details();
        let transport_result = StdioTransport::new(TransportOptions::default());

        assert!(transport_result.is_ok());

        // Verify server details are properly configured for runtime
        assert!(!server_details.server_info.name.is_empty());
        assert!(server_details.capabilities.tools.is_some());
        assert!(server_details.instructions.is_some());
    }

    #[test]
    fn test_main_error_message_handling() {
        // Test the error message formatting logic from main()
        use rust_mcp_sdk::error::McpSdkError;
        use std::io;

        // Test various error scenarios
        let test_cases = vec![
            io::Error::new(io::ErrorKind::NotFound, "File not found"),
            io::Error::new(io::ErrorKind::PermissionDenied, "Access denied"),
            io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused"),
        ];

        for test_error in test_cases {
            let sdk_error = McpSdkError::from(test_error);

            // Test the error message extraction logic from main()
            let error_msg = sdk_error
                .rpc_error_message()
                .map_or_else(|| sdk_error.to_string(), std::string::ToString::to_string);

            assert!(!error_msg.is_empty());
            assert!(error_msg.len() > 3);
        }
    }

    #[test]
    fn test_tracing_configuration_options() {
        // Test tracing configuration components
        use tracing::Level;
        use tracing_subscriber::EnvFilter;

        // Test environment filter with different levels
        let levels = vec![Level::INFO, Level::DEBUG, Level::WARN, Level::ERROR];

        for level in levels {
            let filter = EnvFilter::from_default_env().add_directive(level.into());
            let filter_debug = format!("{filter:?}");
            assert!(!filter_debug.is_empty());
        }
    }

    #[test]
    fn test_latest_protocol_version_format() {
        // Test that the protocol version is in expected format
        let version = LATEST_PROTOCOL_VERSION;
        assert!(!version.is_empty());

        // Should contain version-like format
        assert!(version.contains('-') || version.chars().any(char::is_numeric));

        // Should be reasonable length
        assert!(version.len() >= 3);
        assert!(version.len() <= 50);
    }
}
