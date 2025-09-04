//! Error types for the Trading212 MCP server.
//!
//! This module defines the [`Trading212Error`] enum and its associated methods
//! for handling various error conditions that can occur during server operation.
//!
//! The error system supports:
//! - Error chaining with source error preservation
//! - Structured error codes for programmatic handling
//! - JSON serialization for API responses

use serde::{Deserialize, Serialize};
use std::{error::Error as StdError, fmt};

/// Error codes for programmatic error handling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::enum_variant_names)]
pub enum ErrorCode {
    /// Network or HTTP request failure
    NetworkError,
    /// Trading212 API returned an error
    ApiError,
    /// JSON parsing or response format error
    ParseError,
    /// Data serialization failure
    SerializationError,
    /// Configuration or setup error
    ConfigError,
    /// Parameter type conversion error
    ConversionError,
}

/// Custom error types for the Trading212 MCP server.
///
/// Represents all possible error conditions that can occur when interacting
/// with the Trading212 API or processing MCP requests.
#[derive(Debug, Serialize, Deserialize)]
pub enum Trading212Error {
    /// HTTP request failed
    RequestFailed {
        /// Error message
        message: String,
        /// Optional source error for chaining
        #[serde(skip)]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    /// API returned an error response
    ApiError {
        /// HTTP status code returned by the API
        status: u16,
        /// Error message from the API response
        message: String,
        /// Optional source error for chaining
        #[serde(skip)]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    /// Failed to parse JSON response
    ParseError {
        /// Error message
        message: String,
        /// Optional source error for chaining
        #[serde(skip)]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    /// Failed to serialize data
    SerializationError {
        /// Error message
        message: String,
        /// Optional source error for chaining
        #[serde(skip)]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    /// Configuration error
    ConfigError {
        /// Error message
        message: String,
        /// Optional source error for chaining
        #[serde(skip)]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    /// Tool parameter conversion error
    ConversionError {
        /// Error message
        message: String,
        /// Optional source error for chaining
        #[serde(skip)]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
}

impl fmt::Display for Trading212Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::RequestFailed { message, .. } => write!(f, "Request failed: {message}"),
            Self::ApiError {
                status, message, ..
            } => write!(f, "API error ({status}): {message}"),
            Self::ParseError { message, .. } => write!(f, "Failed to parse response: {message}"),
            Self::SerializationError { message, .. } => {
                write!(f, "Failed to serialize data: {message}")
            }
            Self::ConfigError { message, .. } => write!(f, "Configuration error: {message}"),
            Self::ConversionError { message, .. } => {
                write!(f, "Parameter conversion error: {message}")
            }
        }
    }
}

impl StdError for Trading212Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::RequestFailed { source, .. }
            | Self::ApiError { source, .. }
            | Self::ParseError { source, .. }
            | Self::SerializationError { source, .. }
            | Self::ConfigError { source, .. }
            | Self::ConversionError { source, .. } => source
                .as_ref()
                .map(|e| e.as_ref() as &(dyn StdError + 'static)),
        }
    }
}

#[allow(dead_code)] // Public API methods may not be used internally
impl Trading212Error {
    /// Get the error code for programmatic handling.
    pub const fn error_code(&self) -> ErrorCode {
        match self {
            Self::RequestFailed { .. } => ErrorCode::NetworkError,
            Self::ApiError { .. } => ErrorCode::ApiError,
            Self::ParseError { .. } => ErrorCode::ParseError,
            Self::SerializationError { .. } => ErrorCode::SerializationError,
            Self::ConfigError { .. } => ErrorCode::ConfigError,
            Self::ConversionError { .. } => ErrorCode::ConversionError,
        }
    }

    /// Create a new request failed error.
    pub fn request_failed(msg: impl Into<String>) -> Self {
        Self::RequestFailed {
            message: msg.into(),
            source: None,
        }
    }

    /// Create a new request failed error with source.
    pub fn request_failed_with_source(
        msg: impl Into<String>,
        source: Box<dyn StdError + Send + Sync>,
    ) -> Self {
        Self::RequestFailed {
            message: msg.into(),
            source: Some(source),
        }
    }

    /// Create a new API error with status code and message.
    pub fn api_error(status: u16, msg: impl Into<String>) -> Self {
        Self::ApiError {
            status,
            message: msg.into(),
            source: None,
        }
    }

    /// Create a new API error with source.
    pub fn api_error_with_source(
        status: u16,
        msg: impl Into<String>,
        source: Box<dyn StdError + Send + Sync>,
    ) -> Self {
        Self::ApiError {
            status,
            message: msg.into(),
            source: Some(source),
        }
    }

    /// Create a new parsing error.
    pub fn parse_error(msg: impl Into<String>) -> Self {
        Self::ParseError {
            message: msg.into(),
            source: None,
        }
    }

    /// Create a new parsing error with source.
    pub fn parse_error_with_source(
        msg: impl Into<String>,
        source: Box<dyn StdError + Send + Sync>,
    ) -> Self {
        Self::ParseError {
            message: msg.into(),
            source: Some(source),
        }
    }

    /// Create a new serialization error.
    pub fn serialization_error(msg: impl Into<String>) -> Self {
        Self::SerializationError {
            message: msg.into(),
            source: None,
        }
    }

    /// Create a new serialization error with source.
    pub fn serialization_error_with_source(
        msg: impl Into<String>,
        source: Box<dyn StdError + Send + Sync>,
    ) -> Self {
        Self::SerializationError {
            message: msg.into(),
            source: Some(source),
        }
    }

    /// Create a new configuration error.
    pub fn config_error(msg: impl Into<String>) -> Self {
        Self::ConfigError {
            message: msg.into(),
            source: None,
        }
    }

    /// Create a new configuration error with source.
    pub fn config_error_with_source(
        msg: impl Into<String>,
        source: Box<dyn StdError + Send + Sync>,
    ) -> Self {
        Self::ConfigError {
            message: msg.into(),
            source: Some(source),
        }
    }

    /// Create a new parameter conversion error.
    pub fn conversion_error(msg: impl Into<String>) -> Self {
        Self::ConversionError {
            message: msg.into(),
            source: None,
        }
    }

    /// Create a new parameter conversion error with source.
    pub fn conversion_error_with_source(
        msg: impl Into<String>,
        source: Box<dyn StdError + Send + Sync>,
    ) -> Self {
        Self::ConversionError {
            message: msg.into(),
            source: Some(source),
        }
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_request_failed_constructor() {
        let error = Trading212Error::request_failed("Network timeout");

        match error {
            Trading212Error::RequestFailed { message, source } => {
                assert_eq!(message, "Network timeout");
                assert!(source.is_none());
            }
            _ => panic!("Expected RequestFailed variant"),
        }
    }

    #[test]
    fn test_request_failed_constructor_with_string() {
        let message = String::from("Connection refused");
        let error = Trading212Error::request_failed(message);

        match error {
            Trading212Error::RequestFailed { message, source } => {
                assert_eq!(message, "Connection refused");
                assert!(source.is_none());
            }
            _ => panic!("Expected RequestFailed variant"),
        }
    }

    #[test]
    fn test_api_error_constructor() {
        let error = Trading212Error::api_error(401, "Unauthorized");

        match error {
            Trading212Error::ApiError {
                status,
                message,
                source,
            } => {
                assert_eq!(status, 401);
                assert_eq!(message, "Unauthorized");
                assert!(source.is_none());
            }
            _ => panic!("Expected ApiError variant"),
        }
    }

    #[test]
    fn test_api_error_constructor_with_string() {
        let message = String::from("Invalid API key");
        let error = Trading212Error::api_error(403, message);

        match error {
            Trading212Error::ApiError {
                status,
                message,
                source,
            } => {
                assert_eq!(status, 403);
                assert_eq!(message, "Invalid API key");
                assert!(source.is_none());
            }
            _ => panic!("Expected ApiError variant"),
        }
    }

    #[test]
    fn test_parse_error_constructor() {
        let error = Trading212Error::parse_error("Invalid JSON format");

        match error {
            Trading212Error::ParseError { message, source } => {
                assert_eq!(message, "Invalid JSON format");
                assert!(source.is_none());
            }
            _ => panic!("Expected ParseError variant"),
        }
    }

    #[test]
    fn test_serialization_error_constructor() {
        let error = Trading212Error::serialization_error("Failed to serialize response");

        match error {
            Trading212Error::SerializationError { message, source } => {
                assert_eq!(message, "Failed to serialize response");
                assert!(source.is_none());
            }
            _ => panic!("Expected SerializationError variant"),
        }
    }

    #[test]
    fn test_config_error_constructor() {
        let error = Trading212Error::config_error("API key not found");

        match error {
            Trading212Error::ConfigError { message, source } => {
                assert_eq!(message, "API key not found");
                assert!(source.is_none());
            }
            _ => panic!("Expected ConfigError variant"),
        }
    }

    #[test]
    fn test_conversion_error_constructor() {
        let error = Trading212Error::conversion_error("Invalid parameter type");

        match error {
            Trading212Error::ConversionError { message, source } => {
                assert_eq!(message, "Invalid parameter type");
                assert!(source.is_none());
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
            Trading212Error::ApiError {
                status,
                message,
                source,
            } => {
                assert_eq!(*status, 0);
                assert_eq!(message, "Invalid status");
                assert!(source.is_none());
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
        // Test that all error variants return None for source when created without source
        let errors = vec![
            Trading212Error::request_failed("test"),
            Trading212Error::api_error(500, "test"),
            Trading212Error::parse_error("test"),
            Trading212Error::serialization_error("test"),
            Trading212Error::config_error("test"),
            Trading212Error::conversion_error("test"),
        ];

        for error in errors {
            let error_trait: &dyn StdError = &error;
            assert!(error_trait.source().is_none());
        }
    }

    #[test]
    fn test_error_codes() {
        // Test error code mapping
        assert_eq!(
            Trading212Error::request_failed("test").error_code(),
            ErrorCode::NetworkError
        );
        assert_eq!(
            Trading212Error::api_error(500, "test").error_code(),
            ErrorCode::ApiError
        );
        assert_eq!(
            Trading212Error::parse_error("test").error_code(),
            ErrorCode::ParseError
        );
        assert_eq!(
            Trading212Error::serialization_error("test").error_code(),
            ErrorCode::SerializationError
        );
        assert_eq!(
            Trading212Error::config_error("test").error_code(),
            ErrorCode::ConfigError
        );
        assert_eq!(
            Trading212Error::conversion_error("test").error_code(),
            ErrorCode::ConversionError
        );
    }

    #[test]
    fn test_error_chaining() {
        use std::io;

        // Create a source error
        let source_error = io::Error::new(io::ErrorKind::ConnectionRefused, "connection refused");

        // Test request failed with source
        let error =
            Trading212Error::request_failed_with_source("Network error", Box::new(source_error));

        match &error {
            Trading212Error::RequestFailed { message, source } => {
                assert_eq!(message, "Network error");
                assert!(source.is_some());
            }
            _ => panic!("Expected RequestFailed variant"),
        }

        // Test that source() returns the chained error
        let error_trait: &dyn StdError = &error;
        assert!(error_trait.source().is_some());
        assert_eq!(
            error_trait.source().unwrap().to_string(),
            "connection refused"
        );
    }

    #[test]
    fn test_error_serialization() {
        let error = Trading212Error::api_error(404, "Not found");

        // Test that error can be serialized to JSON
        let json = serde_json::to_string(&error).expect("Should serialize");
        assert!(json.contains("ApiError"));
        assert!(json.contains("404"));
        assert!(json.contains("Not found"));

        // Test that error can be deserialized from JSON
        let deserialized: Trading212Error =
            serde_json::from_str(&json).expect("Should deserialize");
        match deserialized {
            Trading212Error::ApiError {
                status, message, ..
            } => {
                assert_eq!(status, 404);
                assert_eq!(message, "Not found");
            }
            _ => panic!("Expected ApiError variant"),
        }
    }

    #[test]
    fn test_error_code_serialization() {
        let error_code = ErrorCode::ApiError;

        // Test that error code can be serialized
        let json = serde_json::to_string(&error_code).expect("Should serialize");
        assert_eq!(json, "\"ApiError\"");

        // Test that error code can be deserialized
        let deserialized: ErrorCode = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized, ErrorCode::ApiError);
    }

    #[test]
    fn test_all_error_variants_with_source() {
        use std::io;

        // Create separate source errors since they can't be cloned
        #[allow(clippy::io_other_error)] // For compatibility with older Rust versions
        let sources: Vec<Box<dyn StdError + Send + Sync>> = vec![
            Box::new(io::Error::new(io::ErrorKind::Other, "test")),
            Box::new(io::Error::new(io::ErrorKind::Other, "test")),
            Box::new(io::Error::new(io::ErrorKind::Other, "test")),
            Box::new(io::Error::new(io::ErrorKind::Other, "test")),
            Box::new(io::Error::new(io::ErrorKind::Other, "test")),
            Box::new(io::Error::new(io::ErrorKind::Other, "test")),
        ];

        let mut source_iter = sources.into_iter();
        let errors = vec![
            Trading212Error::request_failed_with_source("test", source_iter.next().unwrap()),
            Trading212Error::api_error_with_source(500, "test", source_iter.next().unwrap()),
            Trading212Error::parse_error_with_source("test", source_iter.next().unwrap()),
            Trading212Error::serialization_error_with_source("test", source_iter.next().unwrap()),
            Trading212Error::config_error_with_source("test", source_iter.next().unwrap()),
            Trading212Error::conversion_error_with_source("test", source_iter.next().unwrap()),
        ];

        for error in errors {
            let error_trait: &dyn StdError = &error;
            assert!(error_trait.source().is_some());
            assert_eq!(error_trait.source().unwrap().to_string(), "test");
        }
    }
}
