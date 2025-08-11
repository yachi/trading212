//! Configuration management for the Trading212 MCP server.
//!
//! This module provides the [`Trading212Config`] struct for managing server configuration,
//! including API key loading, environment variable parsing, and default values.

use crate::errors::Trading212Error;
use std::{env, fs, path::PathBuf};

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
}

impl Trading212Config {
    /// Create a new configuration with default values
    pub fn new() -> Result<Self, Trading212Error> {
        let api_key = Self::load_api_key()?;

        Ok(Self {
            api_key,
            base_url: env::var("TRADING212_BASE_URL")
                .unwrap_or_else(|_| "https://live.trading212.com/api/v0".to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_endpoint_url_basic() {
        let config = Trading212Config {
            api_key: "test_key".to_string(),
            base_url: "https://demo.trading212.com/api/v0".to_string(),
        };

        let url = config.endpoint_url("equity/pies");
        assert_eq!(url, "https://demo.trading212.com/api/v0/equity/pies");
    }

    #[test]
    fn test_endpoint_url_with_trailing_slash() {
        let config = Trading212Config {
            api_key: "test_key".to_string(),
            base_url: "https://demo.trading212.com/api/v0/".to_string(),
        };

        let url = config.endpoint_url("equity/pies");
        assert_eq!(url, "https://demo.trading212.com/api/v0/equity/pies");
    }

    #[test]
    fn test_endpoint_url_with_leading_slash() {
        let config = Trading212Config {
            api_key: "test_key".to_string(),
            base_url: "https://demo.trading212.com/api/v0".to_string(),
        };

        let url = config.endpoint_url("/equity/pies");
        assert_eq!(url, "https://demo.trading212.com/api/v0/equity/pies");
    }

    #[test]
    fn test_endpoint_url_with_both_slashes() {
        let config = Trading212Config {
            api_key: "test_key".to_string(),
            base_url: "https://demo.trading212.com/api/v0/".to_string(),
        };

        let url = config.endpoint_url("/equity/pies");
        assert_eq!(url, "https://demo.trading212.com/api/v0/equity/pies");
    }

    #[test]
    fn test_endpoint_url_empty_endpoint() {
        let config = Trading212Config {
            api_key: "test_key".to_string(),
            base_url: "https://demo.trading212.com/api/v0".to_string(),
        };

        let url = config.endpoint_url("");
        assert_eq!(url, "https://demo.trading212.com/api/v0/");
    }

    #[test]
    fn test_load_api_key_success() {
        let temp_dir = TempDir::new().unwrap();
        let api_key_content = "test_api_key_12345";

        // Create temporary API key file
        let api_key_path = temp_dir.path().join(".trading212-api-key");
        fs::write(&api_key_path, api_key_content).unwrap();

        // Set HOME to temp directory
        let original_home = env::var("HOME").ok();
        env::set_var("HOME", temp_dir.path());

        let result = Trading212Config::load_api_key();

        // Restore original HOME
        match original_home {
            Some(home) => env::set_var("HOME", home),
            None => env::remove_var("HOME"),
        }

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), api_key_content);
    }

    #[test]
    fn test_load_api_key_with_whitespace() {
        let temp_dir = TempDir::new().unwrap();
        let api_key_content = "  test_api_key_12345  \n";

        // Create temporary API key file with whitespace
        let api_key_path = temp_dir.path().join(".trading212-api-key");
        fs::write(&api_key_path, api_key_content).unwrap();

        // Set HOME to temp directory
        let original_home = env::var("HOME").ok();
        env::set_var("HOME", temp_dir.path());

        let result = Trading212Config::load_api_key();

        // Restore original HOME
        match original_home {
            Some(home) => env::set_var("HOME", home),
            None => env::remove_var("HOME"),
        }

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_api_key_12345");
    }

    #[test]
    fn test_load_api_key_empty_file() {
        let temp_dir = TempDir::new().unwrap();

        // Create empty API key file
        let api_key_path = temp_dir.path().join(".trading212-api-key");
        fs::write(&api_key_path, "").unwrap();

        // Set HOME to temp directory
        let original_home = env::var("HOME").ok();
        env::set_var("HOME", temp_dir.path());

        let result = Trading212Config::load_api_key();

        // Restore original HOME
        match original_home {
            Some(home) => env::set_var("HOME", home),
            None => env::remove_var("HOME"),
        }

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("is empty"));
    }

    #[test]
    fn test_load_api_key_whitespace_only() {
        let temp_dir = TempDir::new().unwrap();

        // Create API key file with only whitespace
        let api_key_path = temp_dir.path().join(".trading212-api-key");
        fs::write(&api_key_path, "   \n\t  ").unwrap();

        // Set HOME to temp directory
        let original_home = env::var("HOME").ok();
        env::set_var("HOME", temp_dir.path());

        let result = Trading212Config::load_api_key();

        // Restore original HOME
        match original_home {
            Some(home) => env::set_var("HOME", home),
            None => env::remove_var("HOME"),
        }

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("is empty"));
    }

    #[test]
    fn test_load_api_key_file_not_found() {
        let temp_dir = TempDir::new().unwrap();

        // Set HOME to temp directory (no API key file exists)
        let original_home = env::var("HOME").ok();
        env::set_var("HOME", temp_dir.path());

        let result = Trading212Config::load_api_key();

        // Restore original HOME
        match original_home {
            Some(home) => env::set_var("HOME", home),
            None => env::remove_var("HOME"),
        }

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Failed to read API key"));
    }

    #[test]
    fn test_load_api_key_no_home_env() {
        // Remove HOME environment variable
        let original_home = env::var("HOME").ok();
        env::remove_var("HOME");

        let result = Trading212Config::load_api_key();

        // Restore original HOME
        match original_home {
            Some(home) => env::set_var("HOME", home),
            None => env::remove_var("HOME"),
        }

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error
            .to_string()
            .contains("HOME environment variable not set"));
    }

    #[test]
    fn test_new_with_default_base_url() {
        let temp_dir = TempDir::new().unwrap();
        let api_key_content = "test_api_key";

        // Create API key file
        let api_key_path = temp_dir.path().join(".trading212-api-key");
        fs::write(&api_key_path, api_key_content).unwrap();

        // Set HOME and remove TRADING212_BASE_URL if it exists
        let original_home = env::var("HOME").ok();
        let original_base_url = env::var("TRADING212_BASE_URL").ok();
        env::set_var("HOME", temp_dir.path());
        env::remove_var("TRADING212_BASE_URL");

        let result = Trading212Config::new();

        // Restore environment variables
        match original_home {
            Some(home) => env::set_var("HOME", home),
            None => env::remove_var("HOME"),
        }
        match original_base_url {
            Some(url) => env::set_var("TRADING212_BASE_URL", url),
            None => env::remove_var("TRADING212_BASE_URL"),
        }

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.api_key, api_key_content);
        assert_eq!(config.base_url, "https://live.trading212.com/api/v0");
    }

    #[test]
    fn test_new_with_custom_base_url() {
        let temp_dir = TempDir::new().unwrap();
        let api_key_content = "test_api_key";
        let custom_base_url = "https://demo.trading212.com/api/v0";

        // Create API key file
        let api_key_path = temp_dir.path().join(".trading212-api-key");
        fs::write(&api_key_path, api_key_content).unwrap();

        // Set environment variables
        let original_home = env::var("HOME").ok();
        let original_base_url = env::var("TRADING212_BASE_URL").ok();
        env::set_var("HOME", temp_dir.path());
        env::set_var("TRADING212_BASE_URL", custom_base_url);

        let result = Trading212Config::new();

        // Validate result before restoring environment
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.api_key, api_key_content);
        assert_eq!(config.base_url, custom_base_url);

        // Restore environment variables
        match original_home {
            Some(home) => env::set_var("HOME", home),
            None => env::remove_var("HOME"),
        }
        match original_base_url {
            Some(url) => env::set_var("TRADING212_BASE_URL", url),
            None => env::remove_var("TRADING212_BASE_URL"),
        }
    }
}
