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
    ApiError { status: u16, message: String },
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
