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

/// Make a single HTTP request to the Trading212 API.
/// 
/// This is a shared function used by all tools to avoid code duplication.
async fn make_api_request<T>(
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
    tracing::debug!(
        status_code = status.as_u16(),
        url = url,
        "Received API response"
    );

    if status == reqwest::StatusCode::OK {
        let response_text = response.text().await.map_err(|e| {
            Trading212Error::request_failed(format!("Failed to read response body: {e}"))
        })?;

        tracing::debug!(
            response_body = %response_text,
            response_length = response_text.len(),
            "Raw API response received"
        );

        serde_json::from_str::<T>(&response_text).map_err(|e| {
            tracing::error!(
                response_body = %response_text,
                parse_error = %e,
                "Failed to parse JSON response"
            );
            Trading212Error::parse_error(format!(
                "Failed to parse JSON response: {e}. Response body: {response_text}"
            ))
        })
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        tracing::error!(
            status_code = status.as_u16(),
            response_body = %error_text,
            "API returned non-success status"
        );
        Err(Trading212Error::api_error(status.as_u16(), error_text))
    }
}

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

/// Represents an investment pie from Trading212.
///
/// Investment pies allow users to create diversified portfolios with automatic rebalancing.
/// This structure represents the financial summary data returned by the pies API.
#[derive(Debug, Serialize, Deserialize)]
pub struct Pie {
    /// Unique identifier for the pie
    pub id: i32,
    /// Cash balance in the pie
    pub cash: f64,
    /// Dividend details for the pie
    #[serde(rename = "dividendDetails")]
    pub dividend_details: DividendDetails,
    /// Performance results for the pie
    pub result: PieResult,
    /// Progress towards goal (0.0 to 1.0)
    pub progress: f64,
    /// Current status of the pie
    pub status: String,
}

/// Dividend details for a pie
#[derive(Debug, Serialize, Deserialize)]
pub struct DividendDetails {
    /// Total dividends gained
    pub gained: f64,
    /// Amount reinvested
    pub reinvested: f64,
    /// Amount kept as cash
    #[serde(rename = "inCash")]
    pub in_cash: f64,
}

/// Performance results for a pie
#[derive(Debug, Serialize, Deserialize)]
pub struct PieResult {
    /// Average price of invested value
    #[serde(rename = "priceAvgInvestedValue")]
    pub price_avg_invested_value: f64,
    /// Current average price value
    #[serde(rename = "priceAvgValue")]
    pub price_avg_value: f64,
    /// Profit/loss result
    #[serde(rename = "priceAvgResult")]
    pub price_avg_result: f64,
    /// Result coefficient (percentage as decimal)
    #[serde(rename = "priceAvgResultCoef")]
    pub price_avg_result_coef: f64,
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

#[mcp_tool(
    name = "get_pies",
    description = "Get list of all investment pies from Trading212",
    title = "Get Trading212 Investment Pies",
    idempotent_hint = true,
    destructive_hint = false,
    open_world_hint = false,
    read_only_hint = true
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetPiesTool {}

#[mcp_tool(
    name = "get_pie_by_id",
    description = "Get detailed information about a specific investment pie by ID",
    title = "Get Trading212 Pie Details",
    idempotent_hint = true,
    destructive_hint = false,
    open_world_hint = false,
    read_only_hint = true
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetPieByIdTool {
    /// The unique ID of the pie to retrieve
    pub pie_id: i32,
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

        match make_api_request::<Vec<Instrument>>(client, &config.api_key, &url).await
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

}

impl GetPiesTool {
    /// Execute the `get_pies` tool.
    ///
    /// Retrieves a list of all investment pies from Trading212 API.
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
        tracing::debug!("Executing get_pies tool");

        let url = config.endpoint_url("equity/pies");

        match make_api_request::<Vec<Pie>>(client, &config.api_key, &url).await
        {
            Ok(pies) => {
                let json_result = serde_json::to_string_pretty(&pies).map_err(|e| {
                    let error = Trading212Error::serialization_error(format!(
                        "Failed to serialize pies: {e}"
                    ));
                    tracing::error!(error = %error, "Serialization failed");
                    CallToolError::new(error)
                })?;

                tracing::info!(count = pies.len(), "Successfully retrieved pies");
                Ok(CallToolResult::text_content(vec![TextContent::from(
                    format!("Found {} investment pies:\n{}", pies.len(), json_result),
                )]))
            }
            Err(e) => {
                tracing::error!(error = %e, "Tool execution failed");
                Err(CallToolError::new(e))
            }
        }
    }

}

impl GetPieByIdTool {
    /// Execute the `get_pie_by_id` tool.
    ///
    /// Retrieves detailed information about a specific investment pie from Trading212 API.
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
        tracing::debug!(pie_id = self.pie_id, "Executing get_pie_by_id tool");

        let url = config.endpoint_url(&format!("equity/pies/{}", self.pie_id));

        match make_api_request::<serde_json::Value>(client, &config.api_key, &url).await
        {
            Ok(pie_detail) => {
                let json_result = serde_json::to_string_pretty(&pie_detail).map_err(|e| {
                    let error = Trading212Error::serialization_error(format!(
                        "Failed to serialize pie detail: {e}"
                    ));
                    tracing::error!(error = %error, "Serialization failed");
                    CallToolError::new(error)
                })?;

                tracing::info!(pie_id = self.pie_id, "Successfully retrieved pie details");
                Ok(CallToolResult::text_content(vec![TextContent::from(
                    format!("Pie {} details:\n{}", self.pie_id, json_result),
                )]))
            }
            Err(e) => {
                tracing::error!(error = %e, pie_id = self.pie_id, "Tool execution failed");
                Err(CallToolError::new(e))
            }
        }
    }

}

tool_box! {
    Trading212Tools,
    [GetInstrumentsTool, GetPiesTool, GetPieByIdTool]
}
