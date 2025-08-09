//! Configuration management for the Trading212 MCP server.
//!
//! This module provides the [`Trading212Config`] struct for managing server configuration,
//! including API key loading, environment variable parsing, and default values.

use crate::errors::Trading212Error;
use std::{env, fs, path::PathBuf, time::Duration};

/// Configuration for the Trading212 MCP server.
///
/// Contains all necessary configuration parameters including API credentials,
/// server endpoints, and request handling settings.
#[derive(Debug, Clone)]
pub struct Trading212Config {
    /// Trading212 API key for authentication
    pub api_key: String,
    /// Base URL for the Trading212 API
    pub base_url: String,
    /// Request timeout duration
    pub timeout: Duration,
    /// Maximum number of retry attempts for failed requests
    pub max_retries: u32,
}

impl Trading212Config {
    /// Create a new configuration with default values
    pub fn new() -> Result<Self, Trading212Error> {
        let api_key = Self::load_api_key()?;

        Ok(Self {
            api_key,
            base_url: env::var("TRADING212_BASE_URL")
                .unwrap_or_else(|_| "https://live.trading212.com/api/v0".to_string()),
            timeout: Duration::from_secs(
                env::var("TRADING212_TIMEOUT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30),
            ),
            max_retries: env::var("TRADING212_MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
        })
    }

    /// Load API key from ~/.trading212-api-key file
    fn load_api_key() -> Result<String, Trading212Error> {
        let mut api_key_path = PathBuf::new();
        api_key_path.push(
            env::var("HOME")
                .map_err(|_| Trading212Error::config_error("HOME environment variable not set"))?,
        );
        api_key_path.push(".trading212-api-key");

        let api_key = fs::read_to_string(&api_key_path)
            .map_err(|e| {
                Trading212Error::config_error(format!(
                    "Failed to read API key from {}: {e}",
                    api_key_path.display()
                ))
            })?
            .trim()
            .to_string();

        if api_key.is_empty() {
            return Err(Trading212Error::config_error(format!(
                "API key file {} is empty",
                api_key_path.display()
            )));
        }

        tracing::info!(
            api_key_file = ?api_key_path,
            "Successfully loaded Trading212 API key"
        );

        Ok(api_key)
    }

    /// Get the full URL for an endpoint
    pub fn endpoint_url(&self, endpoint: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        )
    }
}
