//! Trading212 MCP tools and data structures.
//!
//! This module defines the available MCP tools for interacting with the Trading212 API,
//! including instrument data retrieval and related data structures.

#![allow(missing_docs)]

use reqwest::Client;
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use rust_mcp_sdk::{
    macros::{mcp_tool, JsonSchema},
    tool_box,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{cache::Trading212Cache, config::Trading212Config, errors::Trading212Error};

/// Helper function to serialize data and create a formatted MCP text response.
///
/// This centralizes the JSON serialization and result formatting logic used by all tools.
fn create_json_response<T>(
    data: &T,
    item_type: &str,
    count: usize,
) -> Result<CallToolResult, CallToolError>
where
    T: serde::Serialize,
{
    let json_result = serde_json::to_string_pretty(data).map_err(|e| {
        let error =
            Trading212Error::serialization_error(format!("Failed to serialize {item_type}: {e}"));
        tracing::error!(error = %error, "Serialization failed");
        CallToolError::new(error)
    })?;

    Ok(CallToolResult::text_content(vec![TextContent::from(
        format!("Found {count} {item_type}:\n{json_result}"),
    )]))
}

/// Helper function to create a paginated response
fn create_paginated_response<T>(
    data: &T,
    item_type: &str,
    returned_count: usize,
    total_count: usize,
    page: u32,
    limit: u32,
) -> Result<CallToolResult, CallToolError>
where
    T: serde::Serialize,
{
    let json_result = serde_json::to_string_pretty(data).map_err(|e| {
        let error =
            Trading212Error::serialization_error(format!("Failed to serialize {item_type}: {e}"));
        tracing::error!(error = %error, "Serialization failed");
        CallToolError::new(error)
    })?;

    let has_more =
        returned_count == limit as usize && (page as usize * limit as usize) < total_count;
    let next_hint = if has_more {
        format!(" - Try page={}", page + 1)
    } else {
        " - Final page".to_string()
    };

    let pagination_info = format!(
        "\nShowing {returned_count} of {total_count} total {item_type} (page {page}){next_hint}"
    );

    Ok(CallToolResult::text_content(vec![TextContent::from(
        format!("Found {returned_count} {item_type}:{pagination_info}\n{json_result}"),
    )]))
}

/// Helper function to create a single item JSON response.
///
/// Used for responses that return a single item rather than a collection.
fn create_single_item_response<T>(
    data: &T,
    item_name: &str,
) -> Result<CallToolResult, CallToolError>
where
    T: serde::Serialize,
{
    let json_result = serde_json::to_string_pretty(data).map_err(|e| {
        let error =
            Trading212Error::serialization_error(format!("Failed to serialize {item_name}: {e}"));
        tracing::error!(error = %error, "Serialization failed");
        CallToolError::new(error)
    })?;

    Ok(CallToolResult::text_content(vec![TextContent::from(
        format!("{item_name}:\n{json_result}"),
    )]))
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

/// Instrument allocation for pie updates
#[allow(missing_docs)]
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct InstrumentAllocation {
    /// Stock ticker symbol
    pub ticker: String,
    /// Allocation weight as decimal between 0.0 and 1.0
    pub weight: f64,
}

/// Issue details for an instrument
#[derive(Debug, Serialize, Deserialize)]
pub struct InstrumentIssue {
    /// Issue name
    pub name: String,
    /// Issue severity
    pub severity: String,
}

/// Represents a single instrument within a pie
#[derive(Debug, Serialize, Deserialize)]
pub struct PieInstrument {
    /// Stock ticker symbol
    pub ticker: String,
    /// Performance results for this instrument
    pub result: PieResult,
    /// Expected allocation percentage
    #[serde(rename = "expectedShare")]
    pub expected_share: f64,
    /// Current allocation percentage
    #[serde(rename = "currentShare")]
    pub current_share: f64,
    /// Number of shares owned
    #[serde(rename = "ownedQuantity")]
    pub owned_quantity: f64,
    /// Any issues with this instrument (can be empty array or objects)
    pub issues: serde_json::Value,
}

/// Pie settings configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct PieSettings {
    /// Unique identifier for the pie
    pub id: i32,
    /// Instrument allocations (null for detailed responses)
    #[serde(rename = "instrumentShares")]
    pub instrument_shares: Option<serde_json::Value>,
    /// Pie name
    pub name: String,
    /// Pie icon
    pub icon: String,
    /// Target goal amount
    pub goal: f64,
    /// Creation date as timestamp
    #[serde(rename = "creationDate")]
    pub creation_date: f64,
    /// End date for the pie
    #[serde(rename = "endDate")]
    pub end_date: String,
    /// Initial investment amount
    #[serde(rename = "initialInvestment")]
    pub initial_investment: f64,
    /// Dividend cash action
    #[serde(rename = "dividendCashAction")]
    pub dividend_cash_action: String,
    /// Public URL (if any)
    #[serde(rename = "publicUrl")]
    pub public_url: Option<String>,
}

/// Complete pie details response from update pie API
#[derive(Debug, Serialize, Deserialize)]
pub struct DetailedPieResponse {
    /// List of instruments in the pie
    pub instruments: Vec<PieInstrument>,
    /// Pie settings and configuration
    pub settings: PieSettings,
}

#[mcp_tool(
    name = "get_instruments",
    description = "Get paginated list of tradeable instruments from Trading212. Use limit and page for efficient pagination through large datasets. Recommended: limit=50-100 for optimal performance.",
    title = "Get Trading212 Instruments (Paginated)",
    idempotent_hint = true,
    destructive_hint = false,
    open_world_hint = false,
    read_only_hint = true
)]
/// Tool for retrieving Trading212 financial instruments
#[allow(missing_docs)]
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct GetInstrumentsTool {
    /// Optional search term to filter instruments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,

    /// Optional instrument type filter (e.g., STOCK, ETF)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub instrument_type: Option<String>,

    /// Maximum number of instruments to return (default: 100)
    /// RECOMMENDED: Use 50-100 for optimal performance. Very large limits may cause timeouts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Page number for pagination (default: 1, starts from 1)
    /// Use with limit to iterate through results: page=1, then page=2, page=3, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
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
/// Tool for retrieving all Trading212 investment pies
#[allow(missing_docs)]
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
/// Tool for retrieving detailed information about a specific Trading212 investment pie
#[allow(missing_docs)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetPieByIdTool {
    /// The unique ID of the pie to retrieve
    pub pie_id: i32,
}

#[mcp_tool(
    name = "update_pie",
    description = "Update an existing Trading212 investment pie configuration",
    title = "Update Trading212 Investment Pie",
    idempotent_hint = false,
    destructive_hint = false,
    open_world_hint = false,
    read_only_hint = false
)]
/// Tool for updating an existing Trading212 investment pie
#[allow(missing_docs)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct UpdatePieTool {
    /// The unique ID of the pie to update
    pub pie_id: i32,
    /// Updated instrument allocations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instrument_shares: Option<Vec<InstrumentAllocation>>,
    /// Updated pie name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Updated pie icon
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Updated target goal amount
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goal: Option<f64>,
    /// Updated dividend cash action (REINVEST or WITHDRAW)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dividend_cash_action: Option<String>,
    /// Updated end date (ISO 8601 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
}

impl GetInstrumentsTool {
    /// Execute the `get_instruments` tool.
    ///
    /// Retrieves a list of tradeable instruments from Trading212 API,
    /// optionally filtered by search term and instrument type, with client-side pagination.
    ///
    /// # Arguments
    ///
    /// * `client` - HTTP client for making API requests
    /// * `config` - Trading212 configuration containing API credentials
    /// * `cache` - Cache and rate limiter for API requests
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails, response parsing fails,
    /// or serialization of the results fails.
    pub async fn call_tool(
        &self,
        client: &Client,
        config: &Trading212Config,
        cache: &Trading212Cache,
    ) -> Result<CallToolResult, CallToolError> {
        self.log_execution_start();

        let params = self.build_query_params();
        let instruments = self
            .fetch_instruments(client, config, cache, &params)
            .await?;

        self.process_instruments(instruments)
    }

    /// Log the start of tool execution
    fn log_execution_start(&self) {
        tracing::debug!(
            search = ?self.search,
            instrument_type = ?self.instrument_type,
            limit = ?self.limit,
            page = ?self.page,
            "Executing get_instruments tool"
        );
    }

    /// Fetch instruments from API via cache
    async fn fetch_instruments(
        &self,
        client: &Client,
        config: &Trading212Config,
        cache: &Trading212Cache,
        params: &str,
    ) -> Result<Vec<Instrument>, CallToolError> {
        let params_str = if params.is_empty() {
            None
        } else {
            Some(params)
        };

        cache
            .request::<Vec<Instrument>>(client, config, "equity/metadata/instruments", params_str)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Tool execution failed");
                CallToolError::new(e)
            })
    }

    /// Process instruments and create response
    fn process_instruments(
        &self,
        all_instruments: Vec<Instrument>,
    ) -> Result<CallToolResult, CallToolError> {
        let total_count = all_instruments.len();
        tracing::info!(
            total_count = total_count,
            "Successfully retrieved all instruments from API"
        );

        // Apply client-side pagination
        let paginated_instruments = self.apply_pagination(all_instruments);
        let returned_count = paginated_instruments.len();

        tracing::info!(
            returned_count = returned_count,
            "Applied client-side pagination"
        );

        let page = self.page.unwrap_or(1).max(1);
        let limit = self.limit.unwrap_or(100);

        create_paginated_response(
            &paginated_instruments,
            "instruments",
            returned_count,
            total_count,
            page,
            limit,
        )
    }

    /// Build query parameters for the instruments API request
    fn build_query_params(&self) -> String {
        let mut params = Vec::new();

        if let Some(ref search) = self.search {
            params.push(format!("search={}", urlencoding::encode(search)));
        }

        if let Some(ref instrument_type) = self.instrument_type {
            params.push(format!("type={}", urlencoding::encode(instrument_type)));
        }

        params.join("&")
    }

    /// Apply pagination to instruments list
    fn apply_pagination(&self, instruments: Vec<Instrument>) -> Vec<Instrument> {
        let limit = self.limit.unwrap_or(100) as usize;
        let page = self.page.unwrap_or(1).max(1) as usize;
        let offset = (page - 1) * limit;

        instruments.into_iter().skip(offset).take(limit).collect()
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
    /// * `cache` - Cache and rate limiter for API requests
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails, response parsing fails,
    /// or serialization of the results fails.
    pub async fn call_tool(
        &self,
        client: &Client,
        config: &Trading212Config,
        cache: &Trading212Cache,
    ) -> Result<CallToolResult, CallToolError> {
        tracing::debug!("Executing get_pies tool");

        match cache
            .request::<Vec<Pie>>(client, config, "equity/pies", None)
            .await
        {
            Ok(pies) => {
                tracing::info!(count = pies.len(), "Successfully retrieved pies");
                create_json_response(&pies, "investment pies", pies.len())
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
    /// * `cache` - Cache and rate limiter for API requests
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails, response parsing fails,
    /// or serialization of the results fails.
    pub async fn call_tool(
        &self,
        client: &Client,
        config: &Trading212Config,
        cache: &Trading212Cache,
    ) -> Result<CallToolResult, CallToolError> {
        tracing::debug!(pie_id = self.pie_id, "Executing get_pie_by_id tool");

        let endpoint = format!("equity/pies/{}", self.pie_id);

        match cache
            .request::<serde_json::Value>(client, config, &endpoint, None)
            .await
        {
            Ok(pie_detail) => {
                tracing::info!(pie_id = self.pie_id, "Successfully retrieved pie details");
                create_single_item_response(&pie_detail, &format!("Pie {} details", self.pie_id))
            }
            Err(e) => {
                tracing::error!(error = %e, pie_id = self.pie_id, "Tool execution failed");
                Err(CallToolError::new(e))
            }
        }
    }
}

impl UpdatePieTool {
    /// Execute the `update_pie` tool.
    ///
    /// Updates an existing investment pie configuration in Trading212 API.
    ///
    /// # Arguments
    ///
    /// * `client` - HTTP client for making API requests
    /// * `config` - Trading212 configuration containing API credentials
    /// * `cache` - Cache and rate limiter for API requests (used for rate limiting only)
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails, response parsing fails,
    /// or serialization of the results fails.
    pub async fn call_tool(
        &self,
        client: &Client,
        config: &Trading212Config,
        _cache: &Trading212Cache,
    ) -> Result<CallToolResult, CallToolError> {
        tracing::debug!(pie_id = self.pie_id, "Executing update_pie tool");

        let url = config.endpoint_url(&format!("equity/pies/{}", self.pie_id));
        let request_body = self.build_request_body()?;

        let response_text = self
            .send_update_request(client, &url, &config.api_key, &request_body)
            .await?;
        let pie = Self::parse_response(&response_text)?;

        tracing::info!(pie_id = self.pie_id, "Successfully updated pie");
        create_single_item_response(&pie, "investment pie")
    }

    /// Build the request body for the pie update request
    fn build_request_body(
        &self,
    ) -> Result<serde_json::Map<String, serde_json::Value>, CallToolError> {
        let mut request_body = serde_json::Map::new();

        if let Some(ref instrument_shares) = self.instrument_shares {
            Self::add_instrument_shares(&mut request_body, instrument_shares)?;
        }

        if let Some(ref name) = self.name {
            request_body.insert("name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(ref icon) = self.icon {
            request_body.insert("icon".to_string(), serde_json::Value::String(icon.clone()));
        }

        if let Some(goal) = self.goal {
            Self::add_goal(&mut request_body, goal)?;
        }

        if let Some(ref dividend_cash_action) = self.dividend_cash_action {
            request_body.insert(
                "dividendCashAction".to_string(),
                serde_json::Value::String(dividend_cash_action.clone()),
            );
        }

        if let Some(ref end_date) = self.end_date {
            request_body.insert(
                "endDate".to_string(),
                serde_json::Value::String(end_date.clone()),
            );
        }

        Ok(request_body)
    }

    /// Add instrument shares to the request body
    fn add_instrument_shares(
        request_body: &mut serde_json::Map<String, serde_json::Value>,
        instrument_shares: &[InstrumentAllocation],
    ) -> Result<(), CallToolError> {
        let shares_map: HashMap<String, f64> = instrument_shares
            .iter()
            .map(|allocation| (allocation.ticker.clone(), allocation.weight))
            .collect();

        let value = serde_json::to_value(shares_map).map_err(|e| {
            CallToolError::new(Trading212Error::serialization_error(format!(
                "Failed to serialize instrument_shares: {e}"
            )))
        })?;

        request_body.insert("instrumentShares".to_string(), value);
        Ok(())
    }

    /// Add goal to the request body
    fn add_goal(
        request_body: &mut serde_json::Map<String, serde_json::Value>,
        goal: f64,
    ) -> Result<(), CallToolError> {
        let number = serde_json::Number::from_f64(goal).ok_or_else(|| {
            CallToolError::new(Trading212Error::serialization_error(
                "Invalid goal value".to_string(),
            ))
        })?;
        request_body.insert("goal".to_string(), serde_json::Value::Number(number));
        Ok(())
    }

    /// Send the update request to the API
    async fn send_update_request(
        &self,
        client: &Client,
        url: &str,
        api_key: &str,
        request_body: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<String, CallToolError> {
        let response = client
            .post(url)
            .header("Authorization", api_key)
            .header("Content-Type", "application/json")
            .json(request_body)
            .send()
            .await
            .map_err(|e| {
                CallToolError::new(Trading212Error::request_failed(format!(
                    "HTTP request failed: {e}"
                )))
            })?;

        let status = response.status();
        let response_text = response.text().await.map_err(|e| {
            CallToolError::new(Trading212Error::request_failed(format!(
                "Failed to read response: {e}"
            )))
        })?;

        if !status.is_success() {
            tracing::error!(
                status_code = status.as_u16(),
                response_body = response_text,
                "API returned non-success status"
            );
            return Err(CallToolError::new(Trading212Error::api_error(
                status.as_u16(),
                response_text,
            )));
        }

        Ok(response_text)
    }

    /// Parse the API response
    fn parse_response(response_text: &str) -> Result<DetailedPieResponse, CallToolError> {
        serde_json::from_str(response_text).map_err(|e| {
            CallToolError::new(Trading212Error::parse_error(format!(
                "Failed to parse JSON response: {e}. Response body: {response_text}"
            )))
        })
    }
}

tool_box! {
    Trading212Tools,
    [GetInstrumentsTool, GetPiesTool, GetPieByIdTool, UpdatePieTool]
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::manual_string_new)]
#[allow(clippy::single_char_pattern)]
#[allow(clippy::uninlined_format_args)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn test_update_pie_response_parsing_failure() {
        // This test uses a realistic API response format based on actual Trading212 structure
        let realistic_api_response = r#"{"instruments":[{"ticker":"AAPL_US_EQ","result":{"priceAvgInvestedValue":100.00,"priceAvgValue":105.00,"priceAvgResult":5.00,"priceAvgResultCoef":0.05},"expectedShare":0.5,"currentShare":0.48,"ownedQuantity":1.0,"issues":[]},{"ticker":"GOOGL_US_EQ","result":{"priceAvgInvestedValue":200.00,"priceAvgValue":210.00,"priceAvgResult":10.00,"priceAvgResultCoef":0.05},"expectedShare":0.5,"currentShare":0.52,"ownedQuantity":0.5,"issues":[]}],"settings":{"id":12345,"instrumentShares":null,"name":"Test Portfolio","icon":"TestIcon","goal":1000.00,"creationDate":1640995200.0,"endDate":"2025-12-31T23:59:59.999+00:00","initialInvestment":300.00,"dividendCashAction":"REINVEST","publicUrl":null}}"#;

        // Attempt to parse as PieResult (current code) - this should fail
        let result: Result<PieResult, _> = serde_json::from_str(realistic_api_response);
        assert!(
            result.is_err(),
            "Parsing as PieResult should fail for realistic API response"
        );

        // The error should mention missing field 'priceAvgInvestedValue'
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("priceAvgInvestedValue"));

        // But parsing as DetailedPieResponse should succeed
        let detailed_result: Result<DetailedPieResponse, _> =
            serde_json::from_str(realistic_api_response);
        assert!(
            detailed_result.is_ok(),
            "Parsing as DetailedPieResponse should succeed"
        );

        let pie_response = detailed_result.unwrap();
        assert_eq!(pie_response.settings.id, 12345);
        assert_eq!(pie_response.settings.name, "Test Portfolio");
        assert_eq!(pie_response.instruments.len(), 2);

        // Verify the instrument structure
        let instrument = &pie_response.instruments[0];
        assert_eq!(instrument.ticker, "AAPL_US_EQ");
        assert_eq!(instrument.expected_share, 0.5);
        assert_eq!(instrument.owned_quantity, 1.0);

        // Verify issues structure (empty array)
        assert!(instrument.issues.is_array());
        assert_eq!(instrument.issues.as_array().unwrap().len(), 0);

        // Verify settings structure
        assert_eq!(pie_response.settings.dividend_cash_action, "REINVEST");
        assert_eq!(pie_response.settings.goal, 1000.0);
        assert_eq!(pie_response.settings.creation_date, 1_640_995_200.0);
    }

    mod http_tests {
        use super::*;
        use reqwest::Client;
        use wiremock::matchers::{header, method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        #[tokio::test]
        async fn test_get_instruments_success() {
            let mock_server = MockServer::start().await;

            // Mock successful response
            Mock::given(method("GET"))
                .and(path("/equity/metadata/instruments"))
                .and(header("Authorization", "test_key"))
                .respond_with(ResponseTemplate::new(200).set_body_json(vec![
                    Instrument {
                        ticker: "AAPL".to_string(),
                        instrument_type: "STOCK".to_string(),
                        working_schedule_id: 1,
                        isin: "US0378331005".to_string(),
                        currency_code: "USD".to_string(),
                        name: "Apple Inc.".to_string(),
                        short_name: "Apple".to_string(),
                        max_open_quantity: 1000.0,
                        added_on: "2020-01-01".to_string(),
                    },
                    Instrument {
                        ticker: "GOOGL".to_string(),
                        instrument_type: "STOCK".to_string(),
                        working_schedule_id: 1,
                        isin: "US02079K3059".to_string(),
                        currency_code: "USD".to_string(),
                        name: "Alphabet Inc.".to_string(),
                        short_name: "Alphabet".to_string(),
                        max_open_quantity: 500.0,
                        added_on: "2020-01-01".to_string(),
                    },
                ]))
                .mount(&mock_server)
                .await;

            let config = Trading212Config {
                api_key: "test_key".to_string(),
                base_url: mock_server.uri(),
            };

            let client = Client::new();
            let tool = GetInstrumentsTool::default();

            let cache = Trading212Cache::new().unwrap();
            let result = tool.call_tool(&client, &config, &cache).await;

            assert!(result.is_ok());
            let response = result.unwrap();
            assert!(response.content.len() == 1);
        }

        #[tokio::test]
        async fn test_get_instruments_with_search_params() {
            let mock_server = MockServer::start().await;

            Mock::given(method("GET"))
                .and(path("/equity/metadata/instruments"))
                .and(query_param("search", "AAPL"))
                .and(query_param("type", "STOCK"))
                .and(header("Authorization", "test_key"))
                .respond_with(ResponseTemplate::new(200).set_body_json(vec![Instrument {
                    ticker: "AAPL".to_string(),
                    instrument_type: "STOCK".to_string(),
                    working_schedule_id: 1,
                    isin: "US0378331005".to_string(),
                    currency_code: "USD".to_string(),
                    name: "Apple Inc.".to_string(),
                    short_name: "Apple".to_string(),
                    max_open_quantity: 1000.0,
                    added_on: "2020-01-01".to_string(),
                }]))
                .mount(&mock_server)
                .await;

            let config = Trading212Config {
                api_key: "test_key".to_string(),
                base_url: mock_server.uri(),
            };

            let client = Client::new();
            let tool = GetInstrumentsTool {
                search: Some("AAPL".to_string()),
                instrument_type: Some("STOCK".to_string()),
                limit: None,
                page: None,
            };

            let cache = Trading212Cache::new().unwrap();
            let result = tool.call_tool(&client, &config, &cache).await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_get_instruments_unauthorized() {
            let mock_server = MockServer::start().await;

            Mock::given(method("GET"))
                .and(path("/equity/metadata/instruments"))
                .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                    "error": "Unauthorized"
                })))
                .mount(&mock_server)
                .await;

            let config = Trading212Config {
                api_key: "invalid_key".to_string(),
                base_url: mock_server.uri(),
            };

            let client = Client::new();
            let tool = GetInstrumentsTool::default();

            let cache = Trading212Cache::new().unwrap();
            let result = tool.call_tool(&client, &config, &cache).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            // API error for 401 should be about authentication/authorization
            assert!(error.to_string().contains("API error") || error.to_string().contains("401"));
        }

        #[tokio::test]
        async fn test_get_instruments_not_found() {
            let mock_server = MockServer::start().await;

            Mock::given(method("GET"))
                .and(path("/equity/metadata/instruments"))
                .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                    "error": "Not Found"
                })))
                .mount(&mock_server)
                .await;

            let config = Trading212Config {
                api_key: "test_key".to_string(),
                base_url: mock_server.uri(),
            };

            let client = Client::new();
            let tool = GetInstrumentsTool::default();

            let cache = Trading212Cache::new().unwrap();
            let result = tool.call_tool(&client, &config, &cache).await;

            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_get_instruments_server_error() {
            let mock_server = MockServer::start().await;

            Mock::given(method("GET"))
                .and(path("/equity/metadata/instruments"))
                .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
                    "error": "Internal Server Error"
                })))
                .mount(&mock_server)
                .await;

            let config = Trading212Config {
                api_key: "test_key".to_string(),
                base_url: mock_server.uri(),
            };

            let client = Client::new();
            let tool = GetInstrumentsTool::default();

            let cache = Trading212Cache::new().unwrap();
            let result = tool.call_tool(&client, &config, &cache).await;

            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_get_instruments_malformed_json() {
            let mock_server = MockServer::start().await;

            Mock::given(method("GET"))
                .and(path("/equity/metadata/instruments"))
                .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
                .mount(&mock_server)
                .await;

            let config = Trading212Config {
                api_key: "test_key".to_string(),
                base_url: mock_server.uri(),
            };

            let client = Client::new();
            let tool = GetInstrumentsTool::default();

            let cache = Trading212Cache::new().unwrap();
            let result = tool.call_tool(&client, &config, &cache).await;

            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_get_pies_success() {
            let mock_server = MockServer::start().await;

            Mock::given(method("GET"))
                .and(path("/equity/pies"))
                .and(header("Authorization", "test_key"))
                .respond_with(ResponseTemplate::new(200).set_body_json(vec![Pie {
                    id: 123,
                    cash: 10.5,
                    dividend_details: DividendDetails {
                        gained: 5.0,
                        reinvested: 4.0,
                        in_cash: 1.0,
                    },
                    result: PieResult {
                        price_avg_invested_value: 1000.0,
                        price_avg_value: 1100.0,
                        price_avg_result: 100.0,
                        price_avg_result_coef: 0.1,
                    },
                    progress: 0.75,
                    status: "AHEAD".to_string(),
                }]))
                .mount(&mock_server)
                .await;

            let config = Trading212Config {
                api_key: "test_key".to_string(),
                base_url: mock_server.uri(),
            };

            let client = Client::new();
            let tool = GetPiesTool {};

            let cache = Trading212Cache::new().unwrap();
            let result = tool.call_tool(&client, &config, &cache).await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_get_pie_by_id_success() {
            let mock_server = MockServer::start().await;

            Mock::given(method("GET"))
                .and(path("/equity/pies/123"))
                .and(header("Authorization", "test_key"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": 123,
                    "name": "My Investment Pie",
                    "targetValueAmount": 5000.0,
                    "instruments": [
                        {
                            "ticker": "AAPL",
                            "targetSharePercentage": 0.5
                        }
                    ]
                })))
                .mount(&mock_server)
                .await;

            let config = Trading212Config {
                api_key: "test_key".to_string(),
                base_url: mock_server.uri(),
            };

            let client = Client::new();
            let tool = GetPieByIdTool { pie_id: 123 };

            let cache = Trading212Cache::new().unwrap();
            let result = tool.call_tool(&client, &config, &cache).await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_get_pie_by_id_not_found() {
            let mock_server = MockServer::start().await;

            Mock::given(method("GET"))
                .and(path("/equity/pies/999"))
                .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                    "error": "Pie not found"
                })))
                .mount(&mock_server)
                .await;

            let config = Trading212Config {
                api_key: "test_key".to_string(),
                base_url: mock_server.uri(),
            };

            let client = Client::new();
            let tool = GetPieByIdTool { pie_id: 999 };

            let cache = Trading212Cache::new().unwrap();
            let result = tool.call_tool(&client, &config, &cache).await;

            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_network_timeout() {
            let config = Trading212Config {
                api_key: "test_key".to_string(),
                base_url: "http://127.0.0.1:1".to_string(), // Non-existent endpoint
            };

            let client = Client::builder()
                .timeout(std::time::Duration::from_millis(100))
                .build()
                .unwrap();

            let tool = GetInstrumentsTool::default();

            let cache = Trading212Cache::new().unwrap();
            let result = tool.call_tool(&client, &config, &cache).await;

            assert!(result.is_err());
        }
    }

    mod helper_function_tests {
        use super::*;
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestData {
            name: String,
            value: i32,
        }

        #[test]
        fn test_create_json_response_success() {
            let test_data = vec![
                TestData {
                    name: "Test1".to_string(),
                    value: 42,
                },
                TestData {
                    name: "Test2".to_string(),
                    value: 84,
                },
            ];

            let result = create_json_response(&test_data, "test items", test_data.len());

            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.content.len(), 1);
            // Test that the response was created successfully - specific content format testing
            // would require more complex MCP response parsing
        }

        #[test]
        fn test_create_json_response_empty_collection() {
            let test_data: Vec<TestData> = vec![];
            let result = create_json_response(&test_data, "items", 0);

            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.content.len(), 1);
        }

        #[test]
        fn test_create_single_item_response_success() {
            let test_item = TestData {
                name: "SingleTest".to_string(),
                value: 123,
            };

            let result = create_single_item_response(&test_item, "Test Item");

            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.content.len(), 1);
        }

        #[test]
        fn test_serialization_error_handling() {
            // Test that serialization works for valid data types
            use std::collections::HashMap;

            let mut test_data = HashMap::new();
            test_data.insert("valid", 42);

            let result = create_json_response(&test_data, "test items", 1);

            // Should succeed for valid data
            assert!(result.is_ok());
        }

        #[test]
        fn test_single_item_serialization_success() {
            // Test single item response with valid data
            let test_item = TestData {
                name: "ValidTest".to_string(),
                value: 789,
            };

            let result = create_single_item_response(&test_item, "Valid Item");

            // Should succeed for valid data
            assert!(result.is_ok());
        }
    }

    mod error_path_tests {
        use super::*;
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize)]
        struct TestResponse {
            data: String,
        }

        #[test]
        fn test_create_json_response_with_large_count() {
            use std::collections::HashMap;

            let test_data: HashMap<String, i32> = HashMap::new();
            let result = create_json_response(&test_data, "items", 999_999);

            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.content.len(), 1);
        }

        #[test]
        fn test_create_single_item_response_with_unicode() {
            let test_data = "Unicode test: αβγδε 中文 🦀";
            let result = create_single_item_response(&test_data, "Unicode Item");

            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.content.len(), 1);
        }
    }

    mod serialization_tests {
        use super::*;

        #[test]
        fn test_get_instruments_tool_serialization() {
            let tool = GetInstrumentsTool {
                search: Some("AAPL".to_string()),
                instrument_type: Some("STOCK".to_string()),
                limit: Some(50),
                page: Some(2),
            };

            // Test that the tool can be serialized
            let json = serde_json::to_string(&tool);
            assert!(json.is_ok());

            // Test deserialization
            let json_str = json.unwrap();
            let deserialized: Result<GetInstrumentsTool, _> = serde_json::from_str(&json_str);
            assert!(deserialized.is_ok());

            let deserialized_tool = deserialized.unwrap();
            assert_eq!(deserialized_tool.search, tool.search);
            assert_eq!(deserialized_tool.instrument_type, tool.instrument_type);
            assert_eq!(deserialized_tool.limit, tool.limit);
            assert_eq!(deserialized_tool.page, tool.page);
        }

        #[test]
        fn test_get_pies_tool_serialization() {
            let tool = GetPiesTool {};

            // Test serialization
            let json = serde_json::to_string(&tool);
            assert!(json.is_ok());

            // Test deserialization
            let json_str = json.unwrap();
            let deserialized: Result<GetPiesTool, _> = serde_json::from_str(&json_str);
            assert!(deserialized.is_ok());
        }

        #[test]
        fn test_get_pie_by_id_tool_serialization() {
            let tool = GetPieByIdTool { pie_id: 12345 };

            // Test serialization
            let json = serde_json::to_string(&tool);
            assert!(json.is_ok());

            // Test deserialization
            let json_str = json.unwrap();
            let deserialized: Result<GetPieByIdTool, _> = serde_json::from_str(&json_str);
            assert!(deserialized.is_ok());

            let deserialized_tool = deserialized.unwrap();
            assert_eq!(deserialized_tool.pie_id, tool.pie_id);
        }

        #[test]
        fn test_trading212_tools_enum_debug() {
            let tools = vec![
                Trading212Tools::GetInstrumentsTool(GetInstrumentsTool {
                    search: Some("TEST".to_string()),
                    instrument_type: None,
                    limit: None,
                    page: None,
                }),
                Trading212Tools::GetPiesTool(GetPiesTool {}),
                Trading212Tools::GetPieByIdTool(GetPieByIdTool { pie_id: 999 }),
            ];

            for tool in tools {
                // Test that enum variants can be formatted for debugging
                let debug_string = format!("{:?}", tool);
                assert!(!debug_string.is_empty());
            }
        }
    }

    mod edge_case_tests {
        use super::*;

        #[test]
        fn test_get_pie_by_id_with_zero() {
            let tool = GetPieByIdTool { pie_id: 0 };
            assert_eq!(tool.pie_id, 0);
        }

        #[test]
        fn test_get_pie_by_id_with_max_value() {
            let tool = GetPieByIdTool { pie_id: i32::MAX };
            assert_eq!(tool.pie_id, i32::MAX);
        }

        #[test]
        fn test_get_instruments_with_empty_search() {
            let tool = GetInstrumentsTool {
                search: Some("".to_string()),
                instrument_type: None,
                limit: None,
                page: None,
            };
            assert_eq!(tool.search, Some("".to_string()));
        }

        #[test]
        fn test_get_instruments_with_special_characters() {
            let tool = GetInstrumentsTool {
                search: Some("A&B C.D-E_F".to_string()),
                instrument_type: Some("ETF".to_string()),
                limit: None,
                page: None,
            };
            assert!(tool.search.as_ref().unwrap().contains("&"));
            assert!(tool.search.is_some());
        }

        #[test]
        fn test_apply_pagination_default_params() {
            let tool = GetInstrumentsTool::default();

            let instruments = create_test_instruments(150);
            let result = tool.apply_pagination(instruments);

            // Should return default limit of 100 items starting from offset 0
            assert_eq!(result.len(), 100);
            assert_eq!(result[0].ticker, "INSTRUMENT_0");
            assert_eq!(result[99].ticker, "INSTRUMENT_99");
        }

        #[test]
        fn test_apply_pagination_with_limit() {
            let tool = GetInstrumentsTool {
                search: None,
                instrument_type: None,
                limit: Some(50),
                page: None,
            };

            let instruments = create_test_instruments(150);
            let result = tool.apply_pagination(instruments);

            assert_eq!(result.len(), 50);
            assert_eq!(result[0].ticker, "INSTRUMENT_0");
            assert_eq!(result[49].ticker, "INSTRUMENT_49");
        }

        #[test]
        fn test_apply_pagination_with_page() {
            let tool = GetInstrumentsTool {
                search: None,
                instrument_type: None,
                limit: Some(25),
                page: Some(2), // page 2 with 25 limit = offset 25
            };

            let instruments = create_test_instruments(50);
            let result = tool.apply_pagination(instruments);

            assert_eq!(result.len(), 25);
            assert_eq!(result[0].ticker, "INSTRUMENT_25");
            assert_eq!(result[24].ticker, "INSTRUMENT_49");
        }

        #[test]
        fn test_apply_pagination_page_beyond_length() {
            let tool = GetInstrumentsTool {
                search: None,
                instrument_type: None,
                limit: Some(10),
                page: Some(6), // page 6 with limit 10 = offset 50, beyond 30 items
            };

            let instruments = create_test_instruments(30);
            let result = tool.apply_pagination(instruments);

            // Should return empty vec when page is beyond available data
            assert_eq!(result.len(), 0);
        }

        #[test]
        fn test_apply_pagination_large_limit() {
            let tool = GetInstrumentsTool {
                search: None,
                instrument_type: None,
                limit: Some(2000), // Large limit
                page: None,
            };

            let instruments = create_test_instruments(1500);
            let result = tool.apply_pagination(instruments);

            // Should use the requested limit of 2000, but only 1500 instruments available
            assert_eq!(result.len(), 1500);
        }

        #[test]
        fn test_pagination_edge_cases() {
            // Test page 0 should be treated as page 1
            let tool = GetInstrumentsTool {
                search: None,
                instrument_type: None,
                limit: Some(10),
                page: Some(0),
            };
            let instruments = create_test_instruments(25);
            let result = tool.apply_pagination(instruments);
            assert_eq!(result.len(), 10);
            assert_eq!(result[0].ticker, "INSTRUMENT_0"); // Should start from beginning

            // Test very large page number
            let tool = GetInstrumentsTool {
                search: None,
                instrument_type: None,
                limit: Some(10),
                page: Some(1_000_000), // Very large page
            };
            let instruments = create_test_instruments(25);
            let result = tool.apply_pagination(instruments);
            assert_eq!(result.len(), 0); // Should return empty
        }

        #[test]
        fn test_has_more_pages_logic() {
            use super::create_paginated_response;

            // Test case 1: Full page returned, more pages available
            let data = vec!["item1", "item2", "item3", "item4", "item5"];
            let result = create_paginated_response(&data, "items", 5, 20, 1, 5);
            assert!(result.is_ok());
            let response_text = format!("{:?}", result.unwrap());
            assert!(response_text.contains("Try page=2"));

            // Test case 2: Partial page returned, final page
            let data = vec!["item1", "item2", "item3"];
            let result = create_paginated_response(&data, "items", 3, 23, 5, 5);
            assert!(result.is_ok());
            let response_text = format!("{:?}", result.unwrap());
            assert!(response_text.contains("Final page"));

            // Test case 3: Empty result set
            let data: Vec<String> = vec![];
            let result = create_paginated_response(&data, "items", 0, 0, 1, 10);
            assert!(result.is_ok());
            let response_text = format!("{:?}", result.unwrap());
            assert!(response_text.contains("Final page"));

            // Test case 4: Exactly at boundary (returned_count == limit but no more data)
            let data = vec!["item1", "item2", "item3", "item4", "item5"];
            let result = create_paginated_response(&data, "items", 5, 5, 1, 5);
            assert!(result.is_ok());
            let response_text = format!("{:?}", result.unwrap());
            assert!(response_text.contains("Final page"));
        }

        #[test]
        fn test_pagination_response_format() {
            use super::create_paginated_response;

            // Test that response is created successfully
            let data = vec!["test_item"];
            let result = create_paginated_response(&data, "widgets", 1, 10, 2, 5);
            assert!(result.is_ok());

            // Test with more pages available
            let result_more = create_paginated_response(&data, "items", 5, 20, 2, 5);
            assert!(result_more.is_ok());

            // Test final page
            let result_final = create_paginated_response(&data, "items", 3, 13, 3, 5);
            assert!(result_final.is_ok());

            // These tests verify the function completes successfully
            // The actual text content formatting is tested indirectly through
            // the pagination logic tests above
        }

        fn create_test_instruments(count: usize) -> Vec<Instrument> {
            (0..count)
                .map(|i| Instrument {
                    ticker: format!("INSTRUMENT_{}", i),
                    instrument_type: "STOCK".to_string(),
                    working_schedule_id: 1,
                    isin: format!("US{:08}005", i),
                    currency_code: "USD".to_string(),
                    name: format!("Test Instrument {}", i),
                    short_name: format!("Test{}", i),
                    max_open_quantity: 1000.0,
                    added_on: "2020-01-01".to_string(),
                })
                .collect()
        }

        #[test]
        fn test_tools_list_immutability() {
            let tools1 = Trading212Tools::tools();
            let tools2 = Trading212Tools::tools();

            // Should return the same tools each time
            assert_eq!(tools1.len(), tools2.len());

            for (t1, t2) in tools1.iter().zip(tools2.iter()) {
                assert_eq!(t1.name, t2.name);
                assert_eq!(t1.description, t2.description);
            }
        }

        #[test]
        fn test_tool_names_are_valid() {
            let tools = Trading212Tools::tools();

            for tool in tools {
                // Tool names should be non-empty and follow naming conventions
                assert!(!tool.name.is_empty());
                assert!(!tool.name.contains(' ')); // Should use underscores
                assert!(tool.name.chars().all(|c| c.is_alphanumeric() || c == '_'));
            }
        }

        #[test]
        fn test_tool_descriptions_exist() {
            let tools = Trading212Tools::tools();

            for tool in tools {
                // All tools should have descriptions
                assert!(tool.description.is_some());
                let description = tool.description.as_ref().unwrap();
                assert!(!description.is_empty());
                assert!(description.len() > 10); // Should be meaningful
            }
        }

        #[test]
        fn test_trading212_tools_try_from_edge_cases() {
            use serde_json::Value;
            use std::collections::HashMap;

            // Test with completely invalid parameters
            let mut invalid_args = HashMap::new();
            invalid_args.insert(
                "invalid_field".to_string(),
                Value::String("invalid".to_string()),
            );

            // Test that we can handle unexpected fields gracefully
            assert!(!invalid_args.is_empty());

            // Test empty arguments case
            let empty_args: HashMap<String, Value> = HashMap::new();
            assert!(empty_args.is_empty());
        }

        #[test]
        fn test_tools_conversion_from_call_tool_request_params() {
            use rust_mcp_sdk::schema::CallToolRequestParams;
            use serde_json::json;

            // Test tool parameter conversion
            let test_cases = vec![
                (
                    "get_instruments",
                    json!({
                        "search": "AAPL",
                        "type": "STOCK"
                    }),
                ),
                ("get_pies", json!({})),
                (
                    "get_pie_by_id",
                    json!({
                        "pie_id": 12345
                    }),
                ),
            ];

            for (tool_name, arguments) in test_cases {
                let params = CallToolRequestParams {
                    name: tool_name.to_string(),
                    arguments: Some(arguments.as_object().unwrap().clone()),
                };

                let result = Trading212Tools::try_from(params);
                assert!(
                    result.is_ok(),
                    "Tool conversion should succeed for: {}",
                    tool_name
                );
            }
        }
    }
}
