//! Error types for the Trading212 MCP server.
//!
//! This module defines the [`Trading212Error`] enum and its associated methods
//! for handling various error conditions that can occur during server operation.

use std::fmt;

/// Custom error types for the Trading212 MCP server.
///
/// Represents all possible error conditions that can occur when interacting
/// with the Trading212 API or processing MCP requests.
#[derive(Debug)]
pub enum Trading212Error {
    /// HTTP request failed
    RequestFailed(String),
    /// API returned an error response
    ApiError {
        /// HTTP status code returned by the API
        status: u16,
        /// Error message from the API response
        message: String,
    },
    /// Failed to parse JSON response
    ParseError(String),
    /// Failed to serialize data
    SerializationError(String),
    /// Configuration error
    ConfigError(String),
    /// Tool parameter conversion error
    ConversionError(String),
}

impl fmt::Display for Trading212Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::RequestFailed(msg) => write!(f, "Request failed: {msg}"),
            Self::ApiError { status, message } => write!(f, "API error ({status}): {message}"),
            Self::ParseError(msg) => write!(f, "Failed to parse response: {msg}"),
            Self::SerializationError(msg) => write!(f, "Failed to serialize data: {msg}"),
            Self::ConfigError(msg) => write!(f, "Configuration error: {msg}"),
            Self::ConversionError(msg) => write!(f, "Parameter conversion error: {msg}"),
        }
    }
}

impl std::error::Error for Trading212Error {}

impl Trading212Error {
    /// Create a new request failed error.
    pub fn request_failed(msg: impl Into<String>) -> Self {
        Self::RequestFailed(msg.into())
    }

    /// Create a new API error with status code and message.
    pub fn api_error(status: u16, msg: impl Into<String>) -> Self {
        Self::ApiError {
            status,
            message: msg.into(),
        }
    }

    /// Create a new parsing error.
    pub fn parse_error(msg: impl Into<String>) -> Self {
        Self::ParseError(msg.into())
    }

    /// Create a new serialization error.
    pub fn serialization_error(msg: impl Into<String>) -> Self {
        Self::SerializationError(msg.into())
    }

    /// Create a new configuration error.
    pub fn config_error(msg: impl Into<String>) -> Self {
        Self::ConfigError(msg.into())
    }

    /// Create a new parameter conversion error.
    pub fn conversion_error(msg: impl Into<String>) -> Self {
        Self::ConversionError(msg.into())
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args)]
mod tests {
    use super::*;

    #[test]
    fn test_request_failed_constructor() {
        let error = Trading212Error::request_failed("Network timeout");

        match error {
            Trading212Error::RequestFailed(msg) => {
                assert_eq!(msg, "Network timeout");
            }
            _ => panic!("Expected RequestFailed variant"),
        }
    }

    #[test]
    fn test_request_failed_constructor_with_string() {
        let message = String::from("Connection refused");
        let error = Trading212Error::request_failed(message);

        match error {
            Trading212Error::RequestFailed(msg) => {
                assert_eq!(msg, "Connection refused");
            }
            _ => panic!("Expected RequestFailed variant"),
        }
    }

    #[test]
    fn test_api_error_constructor() {
        let error = Trading212Error::api_error(401, "Unauthorized");

        match error {
            Trading212Error::ApiError { status, message } => {
                assert_eq!(status, 401);
                assert_eq!(message, "Unauthorized");
            }
            _ => panic!("Expected ApiError variant"),
        }
    }

    #[test]
    fn test_api_error_constructor_with_string() {
        let message = String::from("Invalid API key");
        let error = Trading212Error::api_error(403, message);

        match error {
            Trading212Error::ApiError { status, message } => {
                assert_eq!(status, 403);
                assert_eq!(message, "Invalid API key");
            }
            _ => panic!("Expected ApiError variant"),
        }
    }

    #[test]
    fn test_parse_error_constructor() {
        let error = Trading212Error::parse_error("Invalid JSON format");

        match error {
            Trading212Error::ParseError(msg) => {
                assert_eq!(msg, "Invalid JSON format");
            }
            _ => panic!("Expected ParseError variant"),
        }
    }

    #[test]
    fn test_serialization_error_constructor() {
        let error = Trading212Error::serialization_error("Failed to serialize response");

        match error {
            Trading212Error::SerializationError(msg) => {
                assert_eq!(msg, "Failed to serialize response");
            }
            _ => panic!("Expected SerializationError variant"),
        }
    }

    #[test]
    fn test_config_error_constructor() {
        let error = Trading212Error::config_error("API key not found");

        match error {
            Trading212Error::ConfigError(msg) => {
                assert_eq!(msg, "API key not found");
            }
            _ => panic!("Expected ConfigError variant"),
        }
    }

    #[test]
    fn test_conversion_error_constructor() {
        let error = Trading212Error::conversion_error("Invalid parameter type");

        match error {
            Trading212Error::ConversionError(msg) => {
                assert_eq!(msg, "Invalid parameter type");
            }
            _ => panic!("Expected ConversionError variant"),
        }
    }

    #[test]
    fn test_request_failed_display() {
        let error = Trading212Error::request_failed("Network timeout");
        let display_string = format!("{}", error);

        assert_eq!(display_string, "Request failed: Network timeout");
    }

    #[test]
    fn test_api_error_display() {
        let error = Trading212Error::api_error(404, "Resource not found");
        let display_string = format!("{}", error);

        assert_eq!(display_string, "API error (404): Resource not found");
    }

    #[test]
    fn test_parse_error_display() {
        let error = Trading212Error::parse_error("Malformed JSON");
        let display_string = format!("{}", error);

        assert_eq!(display_string, "Failed to parse response: Malformed JSON");
    }

    #[test]
    fn test_serialization_error_display() {
        let error = Trading212Error::serialization_error("Cannot serialize null value");
        let display_string = format!("{}", error);

        assert_eq!(
            display_string,
            "Failed to serialize data: Cannot serialize null value"
        );
    }

    #[test]
    fn test_config_error_display() {
        let error = Trading212Error::config_error("Missing environment variable");
        let display_string = format!("{}", error);

        assert_eq!(
            display_string,
            "Configuration error: Missing environment variable"
        );
    }

    #[test]
    fn test_conversion_error_display() {
        let error = Trading212Error::conversion_error("String to integer conversion failed");
        let display_string = format!("{}", error);

        assert_eq!(
            display_string,
            "Parameter conversion error: String to integer conversion failed"
        );
    }

    #[test]
    fn test_error_trait_implementation() {
        let error = Trading212Error::request_failed("Test error");

        // Test that it implements std::error::Error
        let error_trait: &dyn std::error::Error = &error;
        assert!(error_trait.source().is_none());

        // Test Debug formatting
        let debug_string = format!("{:?}", error);
        assert!(debug_string.contains("RequestFailed"));
        assert!(debug_string.contains("Test error"));
    }

    #[test]
    fn test_error_variants_are_debug() {
        // Test all variants can be formatted with Debug
        let errors = vec![
            Trading212Error::request_failed("test"),
            Trading212Error::api_error(500, "test"),
            Trading212Error::parse_error("test"),
            Trading212Error::serialization_error("test"),
            Trading212Error::config_error("test"),
            Trading212Error::conversion_error("test"),
        ];

        for error in errors {
            let debug_string = format!("{:?}", error);
            assert!(!debug_string.is_empty());
        }
    }

    #[test]
    fn test_error_equality_by_variant() {
        // Test that errors of the same variant with same content are logically equivalent
        let error1 = Trading212Error::request_failed("timeout");
        let error2 = Trading212Error::request_failed("timeout");

        // Since Trading212Error doesn't implement PartialEq, we test via string representation
        assert_eq!(format!("{}", error1), format!("{}", error2));
        assert_eq!(format!("{:?}", error1), format!("{:?}", error2));
    }

    #[test]
    fn test_error_different_variants() {
        let request_error = Trading212Error::request_failed("test");
        let config_error = Trading212Error::config_error("test");

        // Different variants should have different string representations
        assert_ne!(format!("{}", request_error), format!("{}", config_error));
        assert_ne!(
            format!("{:?}", request_error),
            format!("{:?}", config_error)
        );
    }

    #[test]
    fn test_error_with_empty_strings() {
        // Test error creation with empty strings
        let errors = vec![
            Trading212Error::request_failed(""),
            Trading212Error::api_error(400, ""),
            Trading212Error::parse_error(""),
            Trading212Error::serialization_error(""),
            Trading212Error::config_error(""),
            Trading212Error::conversion_error(""),
        ];

        for error in errors {
            let display_string = format!("{}", error);
            assert!(!display_string.is_empty());

            // Test debug formatting works
            let debug_string = format!("{:?}", error);
            assert!(!debug_string.is_empty());
        }
    }

    #[test]
    fn test_api_error_with_zero_status() {
        let error = Trading212Error::api_error(0, "Invalid status");

        match &error {
            Trading212Error::ApiError { status, message } => {
                assert_eq!(*status, 0);
                assert_eq!(message, "Invalid status");
            }
            _ => panic!("Expected ApiError variant"),
        }

        let display_string = format!("{}", error);
        assert_eq!(display_string, "API error (0): Invalid status");
    }

    #[test]
    fn test_api_error_with_high_status_code() {
        let error = Trading212Error::api_error(999, "Custom error");

        let display_string = format!("{}", error);
        assert_eq!(display_string, "API error (999): Custom error");
    }

    #[test]
    fn test_error_source_is_none() {
        // Test that all error variants return None for source
        let errors = vec![
            Trading212Error::request_failed("test"),
            Trading212Error::api_error(500, "test"),
            Trading212Error::parse_error("test"),
            Trading212Error::serialization_error("test"),
            Trading212Error::config_error("test"),
            Trading212Error::conversion_error("test"),
        ];

        for error in errors {
            let error_trait: &dyn std::error::Error = &error;
            assert!(error_trait.source().is_none());
        }
    }
}
