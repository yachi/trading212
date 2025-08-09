//! Trading212 MCP tools and data structures.
//!
//! This module defines the available MCP tools for interacting with the Trading212 API,
//! including instrument data retrieval and related data structures.

use reqwest::Client;
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use rust_mcp_sdk::{
    macros::{mcp_tool, JsonSchema},
    tool_box,
};
use serde::{Deserialize, Serialize};

use crate::{config::Trading212Config, errors::Trading212Error};

/// Represents a tradeable instrument from Trading212.
/// 
/// Contains all the metadata associated with a financial instrument
/// available for trading on the Trading212 platform.
#[derive(Debug, Serialize, Deserialize)]
pub struct Instrument {
    /// The instrument's ticker symbol
    pub ticker: String,
    /// The type of instrument (e.g., STOCK, ETF, etc.)
    #[serde(rename = "type")]
    pub instrument_type: String,
    /// Working schedule identifier for trading hours
    #[serde(rename = "workingScheduleId")]
    pub working_schedule_id: i32,
    /// International Securities Identification Number
    pub isin: String,
    /// Currency code for the instrument
    #[serde(rename = "currencyCode")]
    pub currency_code: String,
    /// Full name of the instrument
    pub name: String,
    /// Abbreviated name of the instrument
    #[serde(rename = "shortName")]
    pub short_name: String,
    /// Maximum quantity that can be held in open positions
    #[serde(rename = "maxOpenQuantity")]
    pub max_open_quantity: f64,
    /// Date when the instrument was added to the platform
    #[serde(rename = "addedOn")]
    pub added_on: String,
}

#[mcp_tool(
    name = "get_instruments",
    description = "Get list of all tradeable instruments from Trading212",
    title = "Get Trading212 Instruments",
    idempotent_hint = true,
    destructive_hint = false,
    open_world_hint = false,
    read_only_hint = true
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetInstrumentsTool {
    /// Optional search term to filter instruments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,

    /// Optional instrument type filter (e.g., STOCK, ETF)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub instrument_type: Option<String>,
}

impl GetInstrumentsTool {
    /// Execute the `get_instruments` tool.
    /// 
    /// Retrieves a list of tradeable instruments from Trading212 API,
    /// optionally filtered by search term and instrument type.
    /// 
    /// # Arguments
    /// 
    /// * `client` - HTTP client for making API requests
    /// * `config` - Trading212 configuration containing API credentials
    /// 
    /// # Errors
    /// 
    /// Returns an error if the API request fails, response parsing fails,
    /// or serialization of the results fails.
    pub async fn call_tool(
        &self,
        client: &Client,
        config: &Trading212Config,
    ) -> Result<CallToolResult, CallToolError> {
        tracing::debug!(
            search = ?self.search,
            instrument_type = ?self.instrument_type,
            "Executing get_instruments tool"
        );

        let mut url = config.endpoint_url("equity/metadata/instruments");

        let mut params = Vec::new();
        if let Some(search) = &self.search {
            params.push(format!("search={}", urlencoding::encode(search)));
        }
        if let Some(t) = &self.instrument_type {
            params.push(format!("type={}", urlencoding::encode(t)));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        match self
            .make_request_with_retries::<Vec<Instrument>>(client, config, &url)
            .await
        {
            Ok(instruments) => {
                let json_result = serde_json::to_string_pretty(&instruments).map_err(|e| {
                    let error = Trading212Error::serialization_error(format!(
                        "Failed to serialize instruments: {e}"
                    ));
                    tracing::error!(error = %error, "Serialization failed");
                    CallToolError::new(error)
                })?;

                tracing::info!(
                    count = instruments.len(),
                    "Successfully retrieved instruments"
                );
                Ok(CallToolResult::text_content(vec![TextContent::from(
                    format!("Found {} instruments:\n{}", instruments.len(), json_result),
                )]))
            }
            Err(e) => {
                tracing::error!(error = %e, "Tool execution failed");
                Err(CallToolError::new(e))
            }
        }
    }

    /// Make an HTTP request with automatic retries.
    /// 
    /// Implements exponential backoff retry logic for failed requests.
    async fn make_request_with_retries<T>(
        &self,
        client: &Client,
        config: &Trading212Config,
        url: &str,
    ) -> Result<T, Trading212Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut last_error = None;

        for attempt in 1..=config.max_retries {
            tracing::debug!(
                attempt = attempt,
                max_retries = config.max_retries,
                url = url,
                "Making API request"
            );

            match self.make_request(client, &config.api_key, url).await {
                Ok(result) => {
                    tracing::debug!(attempt = attempt, "Request succeeded");
                    return Ok(result);
                }
                Err(e) => {
                    tracing::warn!(
                        attempt = attempt,
                        error = %e,
                        "Request failed, will retry if attempts remaining"
                    );
                    last_error = Some(e);

                    if attempt < config.max_retries {
                        tokio::time::sleep(std::time::Duration::from_millis(500 * u64::from(attempt)))
                            .await;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| Trading212Error::request_failed("All retry attempts failed")))
    }

    /// Make a single HTTP request to the Trading212 API.
    async fn make_request<T>(
        &self,
        client: &Client,
        api_key: &str,
        url: &str,
    ) -> Result<T, Trading212Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let response = client
            .get(url)
            .header("Authorization", api_key)
            .send()
            .await
            .map_err(|e| Trading212Error::request_failed(format!("HTTP request failed: {e}")))?;

        let status = response.status();
        if status == reqwest::StatusCode::OK {
            response.json::<T>().await.map_err(|e| {
                Trading212Error::parse_error(format!("Failed to parse JSON response: {e}"))
            })
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(Trading212Error::api_error(status.as_u16(), error_text))
        }
    }
}

tool_box! {
    Trading212Tools,
    [GetInstrumentsTool]
}
