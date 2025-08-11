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

/// Build URL with query parameters for instrument search.
fn build_instruments_url(
    config: &Trading212Config,
    search: Option<&String>,
    instrument_type: Option<&String>,
) -> String {
    let mut url = config.endpoint_url("equity/metadata/instruments");
    let mut params = Vec::new();

    if let Some(search) = search {
        params.push(format!("search={}", urlencoding::encode(search)));
    }
    if let Some(t) = instrument_type {
        params.push(format!("type={}", urlencoding::encode(t)));
    }

    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }

    url
}

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

/// Process a successful HTTP response and parse JSON.
async fn process_response<T>(response: reqwest::Response) -> Result<T, Trading212Error>
where
    T: serde::de::DeserializeOwned,
{
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
}

/// Handle an error HTTP response.
async fn handle_error_response(response: reqwest::Response) -> Trading212Error {
    let status = response.status();
    let error_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Unknown error".to_string());

    tracing::error!(
        status_code = status.as_u16(),
        response_body = %error_text,
        "API returned non-success status"
    );

    Trading212Error::api_error(status.as_u16(), error_text)
}

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
        process_response(response).await
    } else {
        Err(handle_error_response(response).await)
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

        let url =
            build_instruments_url(config, self.search.as_ref(), self.instrument_type.as_ref());

        match make_api_request::<Vec<Instrument>>(client, &config.api_key, &url).await {
            Ok(instruments) => {
                tracing::info!(
                    count = instruments.len(),
                    "Successfully retrieved instruments"
                );
                create_json_response(&instruments, "instruments", instruments.len())
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

        match make_api_request::<Vec<Pie>>(client, &config.api_key, &url).await {
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

        match make_api_request::<serde_json::Value>(client, &config.api_key, &url).await {
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

tool_box! {
    Trading212Tools,
    [GetInstrumentsTool, GetPiesTool, GetPieByIdTool]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_instruments_url_no_params() {
        let config = Trading212Config {
            api_key: "test_key".to_string(),
            base_url: "https://demo.trading212.com/api/v0".to_string(),
        };

        let url = build_instruments_url(&config, None, None);

        assert_eq!(
            url,
            "https://demo.trading212.com/api/v0/equity/metadata/instruments"
        );
    }

    #[test]
    fn test_build_instruments_url_with_search() {
        let config = Trading212Config {
            api_key: "test_key".to_string(),
            base_url: "https://demo.trading212.com/api/v0".to_string(),
        };
        let search = Some("AAPL".to_string());

        let url = build_instruments_url(&config, search.as_ref(), None);

        assert_eq!(
            url,
            "https://demo.trading212.com/api/v0/equity/metadata/instruments?search=AAPL"
        );
    }

    #[test]
    fn test_build_instruments_url_with_type() {
        let config = Trading212Config {
            api_key: "test_key".to_string(),
            base_url: "https://demo.trading212.com/api/v0".to_string(),
        };
        let instrument_type = Some("STOCK".to_string());

        let url = build_instruments_url(&config, None, instrument_type.as_ref());

        assert_eq!(
            url,
            "https://demo.trading212.com/api/v0/equity/metadata/instruments?type=STOCK"
        );
    }

    #[test]
    fn test_build_instruments_url_with_both_params() {
        let config = Trading212Config {
            api_key: "test_key".to_string(),
            base_url: "https://demo.trading212.com/api/v0".to_string(),
        };
        let search = Some("Tesla".to_string());
        let instrument_type = Some("STOCK".to_string());

        let url = build_instruments_url(&config, search.as_ref(), instrument_type.as_ref());

        assert_eq!(url, "https://demo.trading212.com/api/v0/equity/metadata/instruments?search=Tesla&type=STOCK");
    }

    #[test]
    fn test_build_instruments_url_with_special_characters() {
        let config = Trading212Config {
            api_key: "test_key".to_string(),
            base_url: "https://demo.trading212.com/api/v0".to_string(),
        };
        let search = Some("S&P 500".to_string());

        let url = build_instruments_url(&config, search.as_ref(), None);

        assert_eq!(
            url,
            "https://demo.trading212.com/api/v0/equity/metadata/instruments?search=S%26P%20500"
        );
    }

    #[test]
    fn test_build_instruments_url_empty_strings() {
        let config = Trading212Config {
            api_key: "test_key".to_string(),
            base_url: "https://demo.trading212.com/api/v0".to_string(),
        };
        let search = Some("".to_string());
        let instrument_type = Some("".to_string());

        let url = build_instruments_url(&config, search.as_ref(), instrument_type.as_ref());

        assert_eq!(
            url,
            "https://demo.trading212.com/api/v0/equity/metadata/instruments?search=&type="
        );
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
            let tool = GetInstrumentsTool {
                search: None,
                instrument_type: None,
            };

            let result = tool.call_tool(&client, &config).await;

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
            };

            let result = tool.call_tool(&client, &config).await;

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
            let tool = GetInstrumentsTool {
                search: None,
                instrument_type: None,
            };

            let result = tool.call_tool(&client, &config).await;

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
            let tool = GetInstrumentsTool {
                search: None,
                instrument_type: None,
            };

            let result = tool.call_tool(&client, &config).await;

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
            let tool = GetInstrumentsTool {
                search: None,
                instrument_type: None,
            };

            let result = tool.call_tool(&client, &config).await;

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
            let tool = GetInstrumentsTool {
                search: None,
                instrument_type: None,
            };

            let result = tool.call_tool(&client, &config).await;

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

            let result = tool.call_tool(&client, &config).await;

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

            let result = tool.call_tool(&client, &config).await;

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

            let result = tool.call_tool(&client, &config).await;

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

            let tool = GetInstrumentsTool {
                search: None,
                instrument_type: None,
            };

            let result = tool.call_tool(&client, &config).await;

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

        #[tokio::test]
        async fn test_process_response_success() {
            use wiremock::matchers::method;
            use wiremock::{Mock, MockServer, ResponseTemplate};

            let mock_server = MockServer::start().await;
            let test_data = TestData {
                name: "ProcessTest".to_string(),
                value: 456,
            };

            Mock::given(method("GET"))
                .respond_with(ResponseTemplate::new(200).set_body_json(&test_data))
                .mount(&mock_server)
                .await;

            let client = reqwest::Client::new();
            let response = client.get(&mock_server.uri()).send().await.unwrap();

            let result: Result<TestData, Trading212Error> = process_response(response).await;

            assert!(result.is_ok());
            let parsed_data = result.unwrap();
            assert_eq!(parsed_data, test_data);
        }

        #[tokio::test]
        async fn test_process_response_invalid_json() {
            use wiremock::matchers::method;
            use wiremock::{Mock, MockServer, ResponseTemplate};

            let mock_server = MockServer::start().await;

            Mock::given(method("GET"))
                .respond_with(ResponseTemplate::new(200).set_body_string("invalid json {"))
                .mount(&mock_server)
                .await;

            let client = reqwest::Client::new();
            let response = client.get(&mock_server.uri()).send().await.unwrap();

            let result: Result<TestData, Trading212Error> = process_response(response).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert!(error.to_string().contains("Failed to parse JSON response"));
        }

        #[tokio::test]
        async fn test_process_response_empty_body() {
            use wiremock::matchers::method;
            use wiremock::{Mock, MockServer, ResponseTemplate};

            let mock_server = MockServer::start().await;

            Mock::given(method("GET"))
                .respond_with(ResponseTemplate::new(200).set_body_string(""))
                .mount(&mock_server)
                .await;

            let client = reqwest::Client::new();
            let response = client.get(&mock_server.uri()).send().await.unwrap();

            let result: Result<TestData, Trading212Error> = process_response(response).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert!(error.to_string().contains("Failed to parse JSON response"));
        }
    }
}
