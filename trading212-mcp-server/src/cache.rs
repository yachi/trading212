//! Rate limiting and caching for Trading212 API requests.
//!
//! This module provides automatic rate limiting and response caching to respect
//! Trading212's API limits and improve performance by avoiding redundant requests.

use std::{num::NonZeroU32, sync::Arc, time::Duration};

const ONE: NonZeroU32 = match NonZeroU32::new(1) {
    Some(n) => n,
    None => panic!("1 is not zero"),
};

use governor::{clock::QuantaClock, state::NotKeyed, Quota, RateLimiter};
use moka::future::Cache;
use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::{config::Trading212Config, errors::Trading212Error};

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
    pub fn new(endpoint: impl Into<String>, params: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            params: params.into(),
        }
    }
}

/// Rate limiter configuration for different Trading212 endpoints
#[derive(Debug, Clone)]
pub struct EndpointLimits {
    /// Requests per period
    pub requests: NonZeroU32,
    /// Time period for the rate limit
    pub period: Duration,
}

impl EndpointLimits {
    /// Create endpoint limits from requests per second
    pub fn per_seconds(requests: u32, seconds: u64) -> Result<Self, Trading212Error> {
        let requests = NonZeroU32::new(requests).ok_or_else(|| {
            Trading212Error::config_error("Rate limit requests must be non-zero".to_string())
        })?;

        Ok(Self {
            requests,
            period: Duration::from_secs(seconds),
        })
    }
}

/// Trading212 API cache and rate limiter
pub struct Trading212Cache {
    /// Response cache with TTL
    cache: Cache<CacheKey, String>,
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
        // Cache with 1 hour TTL and max 1000 entries
        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(3600)) // 1 hour TTL
            .max_capacity(1000)
            .build();

        // Rate limiters based on Trading212 API documentation
        let instruments_limits = EndpointLimits::per_seconds(1, 50)?; // 1 request per 50 seconds (strictest)
        let instruments_limiter = Arc::new(Self::create_limiter(&instruments_limits)?);

        let pies_limits = EndpointLimits::per_seconds(1, 30)?; // 1 request per 30 seconds
        let pies_limiter = Arc::new(Self::create_limiter(&pies_limits)?);

        let pie_detail_limits = EndpointLimits::per_seconds(1, 5)?; // 1 request per 5 seconds
        let pie_detail_limiter = Arc::new(Self::create_limiter(&pie_detail_limits)?);

        let account_limits = EndpointLimits::per_seconds(1, 30)?; // 1 request per 30 seconds
        let account_limiter = Arc::new(Self::create_limiter(&account_limits)?);

        let orders_limits = EndpointLimits::per_seconds(1, 5)?; // 1 request per 5 seconds
        let orders_limiter = Arc::new(Self::create_limiter(&orders_limits)?);

        Ok(Self {
            cache,
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

    /// Get the appropriate rate limiter for an endpoint
    fn get_limiter(
        &self,
        endpoint: &str,
    ) -> Arc<RateLimiter<NotKeyed, governor::state::InMemoryState, QuantaClock>> {
        if endpoint.contains("metadata/instruments") {
            self.instruments_limiter.clone()
        } else if endpoint.contains("pies/") && endpoint.chars().filter(|&c| c == '/').count() > 2 {
            // Matches "/equity/pies/{id}" pattern
            self.pie_detail_limiter.clone()
        } else if endpoint.contains("pies") {
            self.pies_limiter.clone()
        } else if endpoint.contains("account") {
            self.account_limiter.clone()
        } else if endpoint.contains("orders") {
            self.orders_limiter.clone()
        } else {
            // Default to strictest rate limit for unknown endpoints
            self.instruments_limiter.clone()
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
        if let Some(cached_response) = self.cache.get(&cache_key).await {
            tracing::debug!(endpoint = endpoint, "Using cached response");

            return serde_json::from_str(&cached_response).map_err(|e| {
                Trading212Error::parse_error(format!("Failed to deserialize cached response: {e}"))
            });
        }

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
            |params| format!("{}?{}", config.endpoint_url(endpoint), params),
        );

        // Make HTTP request
        let response = client
            .get(&url)
            .header("Authorization", &config.api_key)
            .send()
            .await
            .map_err(|e| Trading212Error::request_failed(format!("HTTP request failed: {e}")))?;

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

        // Cache the response for successful requests
        self.cache.insert(cache_key, response_text.clone()).await;

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

    /// Clear all cached responses
    pub fn clear_cache(&self) {
        self.cache.invalidate_all();
        tracing::info!("Cache cleared");
    }

    /// Get cache statistics for monitoring
    pub fn cache_stats(&self) -> (u64, u64) {
        let entry_count = self.cache.entry_count();
        let weighted_size = self.cache.weighted_size();
        (entry_count, weighted_size)
    }
}

impl Default for Trading212Cache {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| panic!("Failed to create default Trading212Cache: {e}"))
    }
}

#[cfg(test)]
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
        let limits = EndpointLimits::per_seconds(1, 50).unwrap();
        assert_eq!(limits.requests.get(), 1);
        assert_eq!(limits.period, Duration::from_secs(50));
    }

    #[test]
    fn test_endpoint_limits_zero_requests_error() {
        let result = EndpointLimits::per_seconds(0, 50);
        assert!(result.is_err());
    }

    #[test]
    fn test_trading212_cache_creation() {
        let cache = Trading212Cache::new();
        assert!(cache.is_ok());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = Trading212Cache::new().unwrap();
        let (entry_count, weighted_size) = cache.cache_stats();

        // New cache should be empty
        assert_eq!(entry_count, 0);
        assert_eq!(weighted_size, 0);
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let cache = Trading212Cache::new().unwrap();

        // Insert test data
        let key = CacheKey::new("test", "params");
        cache.cache.insert(key, "test_response".to_string()).await;

        // Verify data exists
        let (entry_count, _) = cache.cache_stats();
        assert_eq!(entry_count, 1);

        // Clear cache
        cache.clear_cache();

        // Verify cache is empty
        let (entry_count, _) = cache.cache_stats();
        assert_eq!(entry_count, 0);
    }

    #[test]
    fn test_get_limiter_endpoint_matching() {
        let cache = Trading212Cache::new().unwrap();

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
        let cache = Trading212Cache::new().unwrap();

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
        let cache = Trading212Cache::new().unwrap();

        // All limiters should be configured (this mainly tests that creation doesn't panic)
        assert!(cache.instruments_limiter.check().is_ok());
        assert!(cache.pies_limiter.check().is_ok());
        assert!(cache.pie_detail_limiter.check().is_ok());
        assert!(cache.account_limiter.check().is_ok());
        assert!(cache.orders_limiter.check().is_ok());
    }

    #[tokio::test]
    async fn test_cache_insertion_and_retrieval() {
        let cache = Trading212Cache::new().unwrap();
        let key = CacheKey::new("test_endpoint", "param=value");
        let test_response = "test_json_response";

        // Insert into cache
        cache
            .cache
            .insert(key.clone(), test_response.to_string())
            .await;

        // Retrieve from cache
        let retrieved = cache.cache.get(&key).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), test_response);
    }
}
