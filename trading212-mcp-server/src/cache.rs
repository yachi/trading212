//! Rate limiting and caching for Trading212 API requests.
//!
//! This module provides automatic rate limiting and response caching to respect
//! Trading212's API limits and improve performance by avoiding redundant requests.

use std::{num::NonZeroU32, sync::Arc, time::Duration};

/// Single request burst limit for rate limiters
const ONE: NonZeroU32 = NonZeroU32::MIN;

use governor::{clock::QuantaClock, state::NotKeyed, Quota, RateLimiter};
use moka::future::Cache;
use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::{config::Trading212Config, errors::Trading212Error};

// Rate limits based on Trading212 API documentation
// https://t212public-api-docs.redoc.ly/
const INSTRUMENTS_RATE_LIMIT_SECS: u64 = 50; // Strictest limit
const PIES_LIST_RATE_LIMIT_SECS: u64 = 30;
const PIE_DETAIL_RATE_LIMIT_SECS: u64 = 5;
const ACCOUNT_RATE_LIMIT_SECS: u64 = 30;
const ORDERS_RATE_LIMIT_SECS: u64 = 5;

// Cache TTLs with buffer over rate limits
const INSTRUMENTS_CACHE_TTL_SECS: u64 = 60; // 60s TTL for 50s rate limit
const PIES_CACHE_TTL_SECS: u64 = 40; // 40s TTL for 30s rate limit
const DETAIL_CACHE_TTL_SECS: u64 = 15; // 15s TTL for 5s rate limit

/// Endpoint classification for routing to appropriate cache and rate limiter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EndpointType {
    Instruments,
    PiesList,
    PieDetail,
    Account,
    Orders,
    Unknown,
}

impl EndpointType {
    /// Classify an endpoint path into its type
    fn from_path(endpoint: &str) -> Self {
        if endpoint.contains("metadata/instruments") {
            Self::Instruments
        } else if endpoint.contains("pies/") && endpoint.chars().filter(|&c| c == '/').count() >= 2
        {
            // Matches "/equity/pies/{id}" pattern
            Self::PieDetail
        } else if endpoint.contains("pies") {
            Self::PiesList
        } else if endpoint.contains("account") {
            Self::Account
        } else if endpoint.contains("orders") {
            Self::Orders
        } else {
            Self::Unknown
        }
    }
}

/// Cache key for Trading212 API requests
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CacheKey {
    /// API endpoint path
    pub endpoint: String,
    /// Query parameters (sorted for consistency)
    pub params: String,
}

impl CacheKey {
    /// Create a new cache key from endpoint and parameters
    /// Parameters are normalized by sorting query parameters for consistency
    pub fn new(endpoint: impl Into<String>, params: impl Into<String>) -> Self {
        let params_str = params.into();
        let normalized_params = Self::normalize_params(&params_str);

        Self {
            endpoint: endpoint.into(),
            params: normalized_params,
        }
    }

    /// Normalize query parameters by sorting them for consistent cache keys
    fn normalize_params(params: &str) -> String {
        if params.is_empty() {
            return String::new();
        }

        let mut param_pairs: Vec<&str> = params.split('&').collect();
        param_pairs.sort_unstable();
        param_pairs.join("&")
    }
}

/// Rate limiter configuration for different Trading212 endpoints
#[derive(Debug, Clone)]
pub struct EndpointLimits {
    /// Time period for the rate limit
    pub period: Duration,
}

impl EndpointLimits {
    /// Create endpoint limits for a given time period in seconds
    /// Note: Trading212 API enforces single request per period (no bursts)
    pub const fn from_seconds(seconds: u64) -> Self {
        Self {
            period: Duration::from_secs(seconds),
        }
    }
}

/// Trading212 API cache and rate limiter
pub struct Trading212Cache {
    /// Per-endpoint caches with TTLs matching rate limits
    instruments_cache: Cache<CacheKey, String>,
    pies_cache: Cache<CacheKey, String>,
    pie_detail_cache: Cache<CacheKey, String>,
    account_cache: Cache<CacheKey, String>,
    orders_cache: Cache<CacheKey, String>,
    /// Rate limiters for different endpoints  
    instruments_limiter: Arc<RateLimiter<NotKeyed, governor::state::InMemoryState, QuantaClock>>,
    pies_limiter: Arc<RateLimiter<NotKeyed, governor::state::InMemoryState, QuantaClock>>,
    pie_detail_limiter: Arc<RateLimiter<NotKeyed, governor::state::InMemoryState, QuantaClock>>,
    account_limiter: Arc<RateLimiter<NotKeyed, governor::state::InMemoryState, QuantaClock>>,
    orders_limiter: Arc<RateLimiter<NotKeyed, governor::state::InMemoryState, QuantaClock>>,
}

impl Trading212Cache {
    /// Create a new Trading212 cache with configured rate limiters
    ///
    /// # Errors
    ///
    /// Returns an error if rate limiter configuration is invalid
    pub fn new() -> Result<Self, Trading212Error> {
        // Rate limiters based on Trading212 API documentation
        let instruments_limits = EndpointLimits::from_seconds(INSTRUMENTS_RATE_LIMIT_SECS);
        let instruments_limiter = Arc::new(Self::create_limiter(&instruments_limits)?);

        let pies_limits = EndpointLimits::from_seconds(PIES_LIST_RATE_LIMIT_SECS);
        let pies_limiter = Arc::new(Self::create_limiter(&pies_limits)?);

        let pie_detail_limits = EndpointLimits::from_seconds(PIE_DETAIL_RATE_LIMIT_SECS);
        let pie_detail_limiter = Arc::new(Self::create_limiter(&pie_detail_limits)?);

        let account_limits = EndpointLimits::from_seconds(ACCOUNT_RATE_LIMIT_SECS);
        let account_limiter = Arc::new(Self::create_limiter(&account_limits)?);

        let orders_limits = EndpointLimits::from_seconds(ORDERS_RATE_LIMIT_SECS);
        let orders_limiter = Arc::new(Self::create_limiter(&orders_limits)?);

        // Create caches with TTLs matching rate limits (with some buffer)
        let instruments_cache = Cache::builder()
            .time_to_live(Duration::from_secs(INSTRUMENTS_CACHE_TTL_SECS))
            .max_capacity(200)
            .build();

        let pies_cache = Cache::builder()
            .time_to_live(Duration::from_secs(PIES_CACHE_TTL_SECS))
            .max_capacity(200)
            .build();

        let pie_detail_cache = Cache::builder()
            .time_to_live(Duration::from_secs(DETAIL_CACHE_TTL_SECS))
            .max_capacity(200)
            .build();

        let account_cache = Cache::builder()
            .time_to_live(Duration::from_secs(PIES_CACHE_TTL_SECS)) // Same as pies
            .max_capacity(200)
            .build();

        let orders_cache = Cache::builder()
            .time_to_live(Duration::from_secs(DETAIL_CACHE_TTL_SECS)) // Same as details
            .max_capacity(200)
            .build();

        Ok(Self {
            instruments_cache,
            pies_cache,
            pie_detail_cache,
            account_cache,
            orders_cache,
            instruments_limiter,
            pies_limiter,
            pie_detail_limiter,
            account_limiter,
            orders_limiter,
        })
    }

    /// Create a rate limiter from endpoint limits
    fn create_limiter(
        limits: &EndpointLimits,
    ) -> Result<RateLimiter<NotKeyed, governor::state::InMemoryState, QuantaClock>, Trading212Error>
    {
        let quota = Quota::with_period(limits.period)
            .ok_or_else(|| Trading212Error::config_error("Invalid rate limit period".to_string()))?
            .allow_burst(ONE); // Force single request, no bursts

        Ok(RateLimiter::direct(quota))
    }

    /// Get the appropriate cache for an endpoint
    pub fn get_cache(&self, endpoint: &str) -> &Cache<CacheKey, String> {
        match EndpointType::from_path(endpoint) {
            EndpointType::PieDetail => &self.pie_detail_cache,
            EndpointType::PiesList => &self.pies_cache,
            EndpointType::Account => &self.account_cache,
            EndpointType::Orders => &self.orders_cache,
            EndpointType::Instruments | EndpointType::Unknown => &self.instruments_cache, // Default to strictest
        }
    }

    /// Get the appropriate rate limiter for an endpoint
    pub fn get_limiter(
        &self,
        endpoint: &str,
    ) -> Arc<RateLimiter<NotKeyed, governor::state::InMemoryState, QuantaClock>> {
        match EndpointType::from_path(endpoint) {
            EndpointType::PieDetail => self.pie_detail_limiter.clone(),
            EndpointType::PiesList => self.pies_limiter.clone(),
            EndpointType::Account => self.account_limiter.clone(),
            EndpointType::Orders => self.orders_limiter.clone(),
            EndpointType::Instruments | EndpointType::Unknown => self.instruments_limiter.clone(), // Default to strictest
        }
    }

    /// Make a cached and rate-limited API request
    ///
    /// This function:
    /// 1. Checks cache first for existing response
    /// 2. Applies appropriate rate limiting
    /// 3. Makes HTTP request if needed
    /// 4. Caches successful responses
    ///
    /// # Arguments
    ///
    /// * `client` - HTTP client for making requests
    /// * `config` - Trading212 configuration
    /// * `endpoint` - API endpoint path
    /// * `params` - Query parameters (optional)
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails, rate limiting fails,
    /// or response parsing fails.
    pub async fn request<T>(
        &self,
        client: &Client,
        config: &Trading212Config,
        endpoint: &str,
        params: Option<&str>,
    ) -> Result<T, Trading212Error>
    where
        T: DeserializeOwned,
    {
        let cache_key = CacheKey::new(endpoint, params.unwrap_or(""));

        // Check cache first
        if let Some(cached_response) = self.check_cache(&cache_key, endpoint).await {
            return cached_response;
        }

        // Apply rate limiting and make request
        let response_text = self
            .make_rate_limited_request(client, config, endpoint, params)
            .await?;

        // Cache and parse the response
        self.cache_and_parse_response(&cache_key, endpoint, response_text)
            .await
    }

    /// Check cache for existing response
    async fn check_cache<T>(
        &self,
        cache_key: &CacheKey,
        endpoint: &str,
    ) -> Option<Result<T, Trading212Error>>
    where
        T: DeserializeOwned,
    {
        let cache = self.get_cache(endpoint);

        cache.get(cache_key).await.map(|cached_response| {
            tracing::debug!(endpoint = endpoint, "Using cached response");

            serde_json::from_str(&cached_response).map_err(|e| {
                Trading212Error::parse_error(format!(
                    "Failed to deserialize cached response from {endpoint}: {e}"
                ))
            })
        })
    }

    /// Make a rate-limited HTTP request
    async fn make_rate_limited_request(
        &self,
        client: &Client,
        config: &Trading212Config,
        endpoint: &str,
        params: Option<&str>,
    ) -> Result<String, Trading212Error> {
        // Apply rate limiting
        let limiter = self.get_limiter(endpoint);
        tracing::debug!(endpoint = endpoint, "Waiting for rate limit");
        limiter.until_ready().await;
        tracing::debug!(
            endpoint = endpoint,
            "Rate limit cleared, making API request"
        );

        // Build URL
        let url = params.map_or_else(
            || config.endpoint_url(endpoint),
            |params| format!("{}?{params}", config.endpoint_url(endpoint)),
        );

        // Make HTTP request
        let response = client
            .get(&url)
            .header("Authorization", &config.api_key)
            .send()
            .await
            .map_err(|e| Trading212Error::request_failed(format!("HTTP request failed: {e}")))?;

        self.handle_response(response, endpoint).await
    }

    /// Handle HTTP response and extract body text
    async fn handle_response(
        &self,
        response: reqwest::Response,
        endpoint: &str,
    ) -> Result<String, Trading212Error> {
        let status = response.status();
        tracing::debug!(
            status_code = status.as_u16(),
            endpoint = endpoint,
            "Received API response"
        );

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            tracing::error!(
                status_code = status.as_u16(),
                response_body = %error_text,
                endpoint = endpoint,
                "API returned non-success status"
            );

            return Err(Trading212Error::api_error(status.as_u16(), error_text));
        }

        // Read response body
        let response_text = response.text().await.map_err(|e| {
            Trading212Error::request_failed(format!("Failed to read response body: {e}"))
        })?;

        tracing::debug!(
            response_length = response_text.len(),
            endpoint = endpoint,
            "Successfully received API response"
        );

        Ok(response_text)
    }

    /// Cache response and parse as JSON
    async fn cache_and_parse_response<T>(
        &self,
        cache_key: &CacheKey,
        endpoint: &str,
        response_text: String,
    ) -> Result<T, Trading212Error>
    where
        T: DeserializeOwned,
    {
        // Cache the response for successful requests
        let cache = self.get_cache(endpoint);
        cache.insert(cache_key.clone(), response_text.clone()).await;
        tracing::debug!(endpoint = endpoint, "Response cached successfully");

        // Parse and return
        serde_json::from_str(&response_text).map_err(|e| {
            tracing::error!(
                response_body = %response_text,
                parse_error = %e,
                endpoint = endpoint,
                "Failed to parse JSON response"
            );
            Trading212Error::parse_error(format!(
                "Failed to parse JSON response: {e}. Response body: {response_text}"
            ))
        })
    }
}

// Note: No Default implementation provided to avoid panics in default construction

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_creation() {
        let key = CacheKey::new("equity/pies", "limit=10");
        assert_eq!(key.endpoint, "equity/pies");
        assert_eq!(key.params, "limit=10");
    }

    #[test]
    fn test_cache_key_equality() {
        let key1 = CacheKey::new("equity/pies", "limit=10");
        let key2 = CacheKey::new("equity/pies", "limit=10");
        let key3 = CacheKey::new("equity/pies", "limit=20");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_endpoint_limits_creation() {
        let limits = EndpointLimits::from_seconds(50);
        assert_eq!(limits.period, Duration::from_secs(50));
    }

    #[test]
    fn test_trading212_cache_creation() {
        let cache = Trading212Cache::new();
        assert!(cache.is_ok());
    }

    #[tokio::test]
    async fn test_cache_basic_functionality() {
        let cache = Trading212Cache::new().expect("Failed to create cache");

        // Test that cache was created successfully
        assert!(cache.instruments_cache.entry_count() == 0);
    }

    #[test]
    fn test_get_limiter_endpoint_matching() {
        let cache = Trading212Cache::new().expect("Failed to create cache");

        // Test instruments endpoint
        let limiter1 = cache.get_limiter("equity/metadata/instruments");
        let limiter2 = cache.get_limiter("equity/metadata/instruments");
        assert!(Arc::ptr_eq(&limiter1, &limiter2));
        assert!(Arc::ptr_eq(&limiter1, &cache.instruments_limiter));

        // Test pies list endpoint
        let limiter3 = cache.get_limiter("equity/pies");
        assert!(Arc::ptr_eq(&limiter3, &cache.pies_limiter));

        // Test pie detail endpoint
        let limiter4 = cache.get_limiter("equity/pies/123");
        assert!(Arc::ptr_eq(&limiter4, &cache.pie_detail_limiter));

        // Test account endpoint
        let limiter5 = cache.get_limiter("account");
        assert!(Arc::ptr_eq(&limiter5, &cache.account_limiter));

        // Test orders endpoint
        let limiter6 = cache.get_limiter("equity/orders");
        assert!(Arc::ptr_eq(&limiter6, &cache.orders_limiter));

        // Test unknown endpoint (should use instruments limiter as default)
        let limiter7 = cache.get_limiter("unknown/endpoint");
        assert!(Arc::ptr_eq(&limiter7, &cache.instruments_limiter));
    }

    #[test]
    fn test_endpoint_pattern_matching() {
        let cache = Trading212Cache::new().expect("Failed to create cache");

        // Test various pie detail patterns
        assert!(Arc::ptr_eq(
            &cache.get_limiter("equity/pies/123"),
            &cache.pie_detail_limiter
        ));
        assert!(Arc::ptr_eq(
            &cache.get_limiter("equity/pies/999999"),
            &cache.pie_detail_limiter
        ));

        // Ensure pies list doesn't match pie detail pattern
        assert!(Arc::ptr_eq(
            &cache.get_limiter("equity/pies"),
            &cache.pies_limiter
        ));
    }

    #[test]
    fn test_rate_limiter_configuration() {
        let cache = Trading212Cache::new().expect("Failed to create cache");

        // All limiters should be configured (this mainly tests that creation doesn't panic)
        assert!(cache.instruments_limiter.check().is_ok());
        assert!(cache.pies_limiter.check().is_ok());
        assert!(cache.pie_detail_limiter.check().is_ok());
        assert!(cache.account_limiter.check().is_ok());
        assert!(cache.orders_limiter.check().is_ok());
    }

    #[tokio::test]
    async fn test_cache_insertion_and_retrieval() {
        let cache = Trading212Cache::new().expect("Failed to create cache");
        let key = CacheKey::new("equity/metadata/instruments", "param=value");
        let test_response = "test_json_response";

        // Insert into cache
        cache
            .instruments_cache
            .insert(key.clone(), test_response.to_string())
            .await;

        // Retrieve from cache
        let retrieved = cache.instruments_cache.get(&key).await;
        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.expect("Cache should contain value"),
            test_response
        );
    }
}
