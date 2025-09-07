//! Configuration management for the Trading212 MCP server.
//!
//! This module provides the [`Trading212Config`] struct for managing server configuration,
//! including API key loading, environment variable parsing, and default values.

use crate::errors::Trading212Error;
use std::{env, fmt, fs, path::PathBuf};

/// Trait for accessing environment variables - allows for testing with mocked values
pub trait EnvProvider {
    /// Get an environment variable value
    fn var(&self, key: &str) -> Result<String, std::env::VarError>;
}

/// Default implementation using `std::env`
pub struct SystemEnvProvider;

impl EnvProvider for SystemEnvProvider {
    fn var(&self, key: &str) -> Result<String, std::env::VarError> {
        env::var(key)
    }
}

/// Configuration for the Trading212 MCP server.
///
/// Contains all necessary configuration parameters including API credentials,
/// server endpoints, and request handling settings.
#[derive(Clone)]
pub struct Trading212Config {
    /// Trading212 API key for authentication
    pub api_key: String,
    /// Base URL for the Trading212 API
    pub base_url: String,
}

impl fmt::Debug for Trading212Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Trading212Config")
            .field("api_key", &"[REDACTED]")
            .field("base_url", &self.base_url)
            .finish()
    }
}

impl Trading212Config {
    /// Create a new configuration with default values using system environment
    pub fn new() -> Result<Self, Trading212Error> {
        Self::with_env_provider(&SystemEnvProvider)
    }

    /// Create a new configuration with a provided API key (useful for testing)
    #[allow(dead_code)]
    pub fn new_with_api_key(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://live.trading212.com/api/v0".to_string(),
        }
    }

    /// Create a new configuration with a custom environment provider (useful for testing)
    pub fn with_env_provider(env_provider: &dyn EnvProvider) -> Result<Self, Trading212Error> {
        let api_key = Self::load_api_key_with_env(env_provider)?;
        let base_url = env_provider
            .var("TRADING212_BASE_URL")
            .unwrap_or_else(|_| "https://live.trading212.com/api/v0".to_string());

        // Validate base URL format
        if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
            return Err(Trading212Error::config_error(
                "Base URL must start with http:// or https://",
            ));
        }

        Ok(Self { api_key, base_url })
    }

    /// Load API key from ~/.trading212-api-key file with custom env provider
    fn load_api_key_with_env(env_provider: &dyn EnvProvider) -> Result<String, Trading212Error> {
        let home_dir = env_provider
            .var("HOME")
            .map_err(|_| Trading212Error::config_error("HOME environment variable not set"))?;

        // Validate home directory path for security
        if home_dir.is_empty() || home_dir.contains("..") || !home_dir.starts_with('/') {
            return Err(Trading212Error::config_error(
                "Invalid HOME directory path - must be absolute and not contain '..'",
            ));
        }

        let mut api_key_path = PathBuf::from(home_dir);
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

        tracing::info!("Successfully loaded Trading212 API key");

        Ok(api_key)
    }

    /// Get the full URL for an endpoint
    pub fn endpoint_url(&self, endpoint: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        let endpoint = endpoint.trim_start_matches('/');
        format!("{base}/{endpoint}")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::uninlined_format_args)]
mod tests {
    use super::*;
    use std::{collections::HashMap, fs};
    use tempfile::TempDir;

    /// Mock environment provider for testing
    struct MockEnvProvider {
        vars: HashMap<String, String>,
    }

    impl MockEnvProvider {
        fn new() -> Self {
            Self {
                vars: HashMap::new(),
            }
        }

        fn set(&mut self, key: &str, value: &str) {
            self.vars.insert(key.to_string(), value.to_string());
        }
    }

    impl EnvProvider for MockEnvProvider {
        fn var(&self, key: &str) -> Result<String, std::env::VarError> {
            self.vars
                .get(key)
                .cloned()
                .ok_or(std::env::VarError::NotPresent)
        }
    }

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
    fn test_load_api_key_success() {
        let temp_dir = TempDir::new().unwrap();
        let api_key_content = "test_api_key_12345";

        // Create temporary API key file
        let api_key_path = temp_dir.path().join(".trading212-api-key");
        fs::write(&api_key_path, api_key_content).unwrap();

        // Mock environment with HOME set
        let mut mock_env = MockEnvProvider::new();
        mock_env.set("HOME", temp_dir.path().to_str().unwrap());

        let result = Trading212Config::load_api_key_with_env(&mock_env);

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

        // Mock environment with HOME set
        let mut mock_env = MockEnvProvider::new();
        mock_env.set("HOME", temp_dir.path().to_str().unwrap());

        let result = Trading212Config::load_api_key_with_env(&mock_env);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_api_key_12345");
    }

    #[test]
    fn test_load_api_key_empty_file() {
        let temp_dir = TempDir::new().unwrap();

        // Create empty API key file
        let api_key_path = temp_dir.path().join(".trading212-api-key");
        fs::write(&api_key_path, "").unwrap();

        // Mock environment with HOME set
        let mut mock_env = MockEnvProvider::new();
        mock_env.set("HOME", temp_dir.path().to_str().unwrap());

        let result = Trading212Config::load_api_key_with_env(&mock_env);

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

        // Mock environment with HOME set
        let mut mock_env = MockEnvProvider::new();
        mock_env.set("HOME", temp_dir.path().to_str().unwrap());

        let result = Trading212Config::load_api_key_with_env(&mock_env);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("is empty"));
    }

    #[test]
    fn test_load_api_key_file_not_found() {
        let temp_dir = TempDir::new().unwrap();

        // Mock environment with HOME set (no API key file exists)
        let mut mock_env = MockEnvProvider::new();
        mock_env.set("HOME", temp_dir.path().to_str().unwrap());

        let result = Trading212Config::load_api_key_with_env(&mock_env);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Failed to read API key"));
    }

    #[test]
    fn test_load_api_key_no_home_env() {
        // Mock environment without HOME variable
        let mock_env = MockEnvProvider::new();
        // Don't set HOME to test missing environment variable

        let result = Trading212Config::load_api_key_with_env(&mock_env);

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

        // Mock environment with HOME set, but no TRADING212_BASE_URL
        let mut mock_env = MockEnvProvider::new();
        mock_env.set("HOME", temp_dir.path().to_str().unwrap());
        // Don't set TRADING212_BASE_URL to test default behavior

        let result = Trading212Config::with_env_provider(&mock_env);

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

        // Mock environment with both HOME and TRADING212_BASE_URL set
        let mut mock_env = MockEnvProvider::new();
        mock_env.set("HOME", temp_dir.path().to_str().unwrap());
        mock_env.set("TRADING212_BASE_URL", custom_base_url);

        let result = Trading212Config::with_env_provider(&mock_env);

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.api_key, api_key_content);
        assert_eq!(config.base_url, custom_base_url);
    }

    #[test]
    fn test_endpoint_url_complex_path() {
        let config = Trading212Config {
            api_key: "test_key".to_string(),
            base_url: "https://demo.trading212.com/api/v0".to_string(),
        };

        // Test with complex path including query parameters
        let url = config.endpoint_url("equity/metadata/instruments?search=AAPL&type=STOCK");
        assert_eq!(
            url,
            "https://demo.trading212.com/api/v0/equity/metadata/instruments?search=AAPL&type=STOCK"
        );
    }

    #[test]
    fn test_config_debug_representation() {
        let config = Trading212Config {
            api_key: "secret_key".to_string(),
            base_url: "https://test.trading212.com/api/v0".to_string(),
        };

        let debug_string = format!("{:?}", config);
        assert!(debug_string.contains("Trading212Config"));
        assert!(debug_string.contains("[REDACTED]"));
        assert!(debug_string.contains("base_url"));
        assert!(!debug_string.contains("secret_key"));
    }

    #[test]
    fn test_invalid_home_directory_path() {
        let mut mock_env = MockEnvProvider::new();
        mock_env.set("HOME", "../etc");

        let result = Trading212Config::load_api_key_with_env(&mock_env);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid HOME directory path"));
    }

    #[test]
    fn test_empty_home_directory_path() {
        let mut mock_env = MockEnvProvider::new();
        mock_env.set("HOME", ""); // Empty home directory

        let result = Trading212Config::load_api_key_with_env(&mock_env);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid HOME directory path"));
    }

    #[test]
    fn test_relative_home_directory_path() {
        let mut mock_env = MockEnvProvider::new();
        mock_env.set("HOME", "home/user"); // Relative path without leading /

        let result = Trading212Config::load_api_key_with_env(&mock_env);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid HOME directory path"));
    }

    #[test]
    fn test_home_directory_with_parent_traversal() {
        let mut mock_env = MockEnvProvider::new();
        mock_env.set("HOME", "/home/user/../etc"); // Absolute path but contains ".."

        let result = Trading212Config::load_api_key_with_env(&mock_env);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid HOME directory path"));
    }

    #[test]
    fn test_home_directory_relative_with_parent_traversal() {
        let mut mock_env = MockEnvProvider::new();
        mock_env.set("HOME", "home/../etc"); // Relative path with ".." - should fail validation

        let result = Trading212Config::load_api_key_with_env(&mock_env);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid HOME directory path"));
    }

    #[test]
    fn test_invalid_base_url() {
        let temp_dir = TempDir::new().unwrap();
        let api_key_path = temp_dir.path().join(".trading212-api-key");
        fs::write(&api_key_path, "test_key").unwrap();

        let mut mock_env = MockEnvProvider::new();
        mock_env.set("HOME", temp_dir.path().to_str().unwrap());
        mock_env.set("TRADING212_BASE_URL", "ftp://invalid.com");

        let result = Trading212Config::with_env_provider(&mock_env);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Base URL must start with http:// or https://"));
    }

    #[test]
    fn test_config_integration_with_custom_base_url() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let api_key_path = temp_dir.path().join(".trading212-api-key");
        fs::write(&api_key_path, "test_api_key_12345").expect("Failed to write API key");

        let mut mock_env = MockEnvProvider::new();
        mock_env.set("HOME", temp_dir.path().to_str().unwrap());
        mock_env.set("TRADING212_BASE_URL", "https://demo.trading212.com/api/v0");

        let config = Trading212Config::with_env_provider(&mock_env);
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.api_key, "test_api_key_12345");
        assert_eq!(config.base_url, "https://demo.trading212.com/api/v0");
    }
}
