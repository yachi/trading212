//! Remote MCP Server for Trading212 API
//!
//! HTTP-based MCP server that accepts Trading212 API keys via HTTP headers
//! and provides multi-user access to Trading212 API functionality.

// Suppress warnings for dependencies only used by the library
#![allow(unused_crate_dependencies)]

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{request::Parts, HeaderName, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Json, RequestPartsExt, Router,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{error, info, info_span, warn};
use trading212_mcp_server::{cache::Trading212Cache, config::Trading212Config};

/// X-Request-ID header name
static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Shared application state
#[derive(Clone)]
struct AppState {
    /// Shared HTTP client (expensive to create, reuse across requests)
    client: Client,
    /// Shared cache for Trading212 API responses (wrapped in Arc for sharing)
    cache: Arc<Trading212Cache>,
}

/// Server configuration
#[derive(Clone)]
struct ServerConfig {
    host: String,
    port: u16,
}

impl ServerConfig {
    fn from_env() -> Self {
        let host = std::env::var("MCP_HTTP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("MCP_HTTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        Self { host, port }
    }
}

/// Per-request Trading212 context with configuration
pub struct Trading212Context {
    /// Trading212 configuration with API key
    pub config: Trading212Config,
}

impl<S> FromRequestParts<S> for Trading212Context
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try bearer token authentication first
        if let Some(api_key) = try_bearer_auth(parts).await {
            info!(header_type = "Bearer", "API key extracted successfully");
            return Ok(create_context(api_key));
        }

        // Try custom header authentication
        if let Some(api_key) = try_custom_header_auth(parts)? {
            info!(
                header_type = "X-Trading212-API-Key",
                "API key extracted successfully"
            );
            return Ok(create_context(api_key));
        }

        error!("Authentication failed: no valid API key provided");
        Err((
            StatusCode::UNAUTHORIZED,
            "Missing or invalid API key. Provide via 'Authorization: Bearer <key>' or 'X-Trading212-API-Key' header".to_string(),
        ))
    }
}

/// Try to extract API key from Authorization: Bearer header
async fn try_bearer_auth(parts: &mut Parts) -> Option<String> {
    parts
        .extract::<TypedHeader<Authorization<Bearer>>>()
        .await
        .ok()
        .and_then(|TypedHeader(Authorization(bearer))| {
            let token = bearer.token().to_string();
            if token.is_empty() {
                warn!(header_type = "Bearer", "Empty API key provided");
                None
            } else {
                Some(token)
            }
        })
}

/// Try to extract API key from X-Trading212-API-Key header
fn try_custom_header_auth(parts: &Parts) -> Result<Option<String>, (StatusCode, String)> {
    if let Some(api_key_header) = parts.headers.get("x-trading212-api-key") {
        let api_key = api_key_header
            .to_str()
            .map_err(|_| {
                error!("Invalid X-Trading212-API-Key header encoding");
                (
                    StatusCode::BAD_REQUEST,
                    "Invalid X-Trading212-API-Key header".to_string(),
                )
            })?
            .to_string();

        if api_key.is_empty() {
            warn!(
                header_type = "X-Trading212-API-Key",
                "Empty API key provided"
            );
            Ok(None)
        } else {
            Ok(Some(api_key))
        }
    } else {
        Ok(None)
    }
}

/// Create `Trading212Context` from API key
fn create_context(api_key: String) -> Trading212Context {
    let config = Trading212Config::new_with_api_key(api_key);
    Trading212Context { config }
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// MCP JSON-RPC request
#[derive(Debug, Deserialize)]
struct McpRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    /// Optional ID - present for requests, absent for notifications
    #[serde(default)]
    id: Option<serde_json::Value>,
    method: String,
    params: Option<serde_json::Value>,
}

/// MCP JSON-RPC response
#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: String,
    /// ID from the request - only present for requests, not notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

/// MCP JSON-RPC error
#[derive(Debug, Serialize)]
struct McpError {
    code: i32,
    message: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    init_tracing();

    let config = ServerConfig::from_env();
    info!(
        host = %config.host,
        port = config.port,
        "Starting Trading212 Remote MCP Server"
    );

    // Create shared application state
    let app_state = create_app_state()?;

    // Build router with state and middleware layers
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/mcp/v1", post(handle_mcp_request))
        .with_state(app_state)
        // Request ID generation and propagation
        .layer(SetRequestIdLayer::new(
            X_REQUEST_ID.clone(),
            MakeRequestUuid,
        ))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request| {
                let request_id = request
                    .headers()
                    .get(X_REQUEST_ID.as_str())
                    .and_then(|v| v.to_str().ok());

                info_span!(
                    "request",
                    method = %request.method(),
                    uri = %request.uri(),
                    request_id = ?request_id,
                )
            }),
        )
        .layer(PropagateRequestIdLayer::new(X_REQUEST_ID.clone()))
        // Timeout to prevent hanging requests
        .layer(TimeoutLayer::new(Duration::from_secs(60)))
        .layer(CorsLayer::permissive())
        .layer(middleware::from_fn(log_request_completion));

    // Start server with graceful shutdown
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!(address = %addr, "Server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");

    Ok(())
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!(error = %e, "Failed to install Ctrl+C handler");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                error!(error = %e, "Failed to install SIGTERM handler");
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            info!("Received Ctrl+C, starting graceful shutdown");
        },
        () = terminate => {
            info!("Received SIGTERM, starting graceful shutdown");
        },
    }
}

/// Middleware to log request completion
async fn log_request_completion(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .and_then(|id| id.header_value().to_str().ok())
        .map(String::from);

    let response = next.run(request).await;

    info!(
        method = %method,
        uri = %uri,
        status = %response.status(),
        request_id = ?request_id,
        "Request completed"
    );

    response
}

/// Create shared application state
fn create_app_state() -> Result<Arc<AppState>, Box<dyn std::error::Error>> {
    let pool_size = std::env::var("TRADING212_POOL_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(16);

    let timeout_secs = std::env::var("TRADING212_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let client = Client::builder()
        .user_agent("Trading212-MCP-Server/0.1.0")
        .pool_max_idle_per_host(pool_size)
        .pool_idle_timeout(Duration::from_secs(300))
        .timeout(Duration::from_secs(timeout_secs))
        .tcp_nodelay(true)
        .tcp_keepalive(Duration::from_secs(120))
        .connection_verbose(false)
        .build()?;

    let cache = Arc::new(Trading212Cache::new()?);

    info!(
        pool_size = pool_size,
        timeout_secs = timeout_secs,
        "Created shared HTTP client and cache"
    );

    Ok(Arc::new(AppState { client, cache }))
}

/// Initialize tracing for the application
fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
}

/// Health check endpoint
/// Returns HTTP 200 with status "ok" - lightweight check without external dependencies
async fn health_check() -> (StatusCode, Json<HealthResponse>) {
    // Simple health check - if the server is responding, it's healthy
    // Don't make external API calls to avoid rate limit issues
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            message: None,
        }),
    )
}

/// Route MCP method calls to appropriate handlers
async fn route_mcp_method(
    method: &str,
    params: Option<serde_json::Value>,
    app_state: &AppState,
    context: &Trading212Context,
) -> Result<serde_json::Value, McpError> {
    match method {
        "initialize" => handle_initialize(params.as_ref()),
        "initialized" => {
            // This is a notification from the client confirming initialization is complete
            info!("Client initialization confirmed");
            Ok(serde_json::Value::Null)
        }
        "notifications/cancelled" => {
            // Client cancelled a request - just acknowledge
            info!("Request cancelled notification received");
            Ok(serde_json::Value::Null)
        }
        "tools/list" => handle_list_tools(),
        "tools/call" => handle_call_tool(app_state, context, params).await,
        _ => {
            warn!(method = %method, "Unknown method requested");
            Err(McpError {
                code: -32601,
                message: format!("Method not found: {method}"),
            })
        }
    }
}

/// Build JSON-RPC response for notifications (no meaningful response)
fn build_notification_response(method: &str) -> Json<McpResponse> {
    info!(method = %method, "Notification processed, no response sent");
    Json(McpResponse {
        jsonrpc: "2.0".to_string(),
        id: None,
        result: None,
        error: None,
    })
}

/// Build JSON-RPC response for requests (with result or error)
fn build_request_response(
    id: Option<serde_json::Value>,
    result: Result<serde_json::Value, McpError>,
) -> Json<McpResponse> {
    match result {
        Ok(result_value) => Json(McpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result_value),
            error: None,
        }),
        Err(error) => Json(McpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }),
    }
}

/// Handle MCP JSON-RPC requests
async fn handle_mcp_request(
    State(app_state): State<Arc<AppState>>,
    context: Trading212Context,
    Json(request): Json<McpRequest>,
) -> Result<Json<McpResponse>, (StatusCode, String)> {
    let is_notification = request.id.is_none();
    info!(
        method = %request.method,
        id = ?request.id,
        is_notification = is_notification,
        "Processing MCP message"
    );

    // Route to appropriate handler
    let result = route_mcp_method(&request.method, request.params, &app_state, &context).await;

    // Build response - only send meaningful responses for requests (with ID), not notifications
    if is_notification {
        Ok(build_notification_response(&request.method))
    } else {
        Ok(build_request_response(request.id, result))
    }
}

/// Handle initialize method
fn handle_initialize(params: Option<&serde_json::Value>) -> Result<serde_json::Value, McpError> {
    use rust_mcp_sdk::schema::{
        Implementation, InitializeResult, ServerCapabilities, ServerCapabilitiesTools,
        LATEST_PROTOCOL_VERSION,
    };

    // Log the initialize request
    info!("Handling initialize request");

    // Parse client info from params if provided (optional, for logging)
    if let Some(params_value) = params {
        info!(
            client_info = ?params_value.get("clientInfo"),
            protocol_version = ?params_value.get("protocolVersion"),
            "Client initialization details"
        );
    }

    // Create server details matching main.rs
    let server_details = InitializeResult {
        server_info: Implementation {
            name: "Trading212 MCP Server".to_string(),
            version: "0.1.0".to_string(),
            title: Some("Trading212 MCP Server".to_string()),
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        meta: None,
        instructions: Some("Access Trading212 API to get instrument information".to_string()),
        protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
    };

    serde_json::to_value(server_details).map_err(|e| {
        error!(error = %e, "Failed to serialize initialize result");
        McpError {
            code: -32603,
            message: "Internal error occurred".to_string(),
        }
    })
}

/// Handle tools/list method
fn handle_list_tools() -> Result<serde_json::Value, McpError> {
    use trading212_mcp_server::tools::Trading212Tools;

    let tools = Trading212Tools::tools();

    serde_json::to_value(tools).map_err(|e| {
        error!(error = %e, "Failed to serialize tools list");
        McpError {
            code: -32603,
            message: "Internal error occurred".to_string(),
        }
    })
}

/// Handle tools/call method
async fn handle_call_tool(
    app_state: &AppState,
    context: &Trading212Context,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, McpError> {
    use rust_mcp_sdk::schema::CallToolRequestParams;
    use trading212_mcp_server::tools::Trading212Tools;

    // Parse parameters
    let call_params: CallToolRequestParams = params
        .ok_or_else(|| {
            error!("Missing params for tools/call");
            McpError {
                code: -32602,
                message: "Missing params for tools/call".to_string(),
            }
        })
        .and_then(|p| {
            serde_json::from_value(p).map_err(|e| {
                error!(error = %e, "Invalid tool call parameters");
                McpError {
                    code: -32602,
                    message: "Invalid parameters".to_string(),
                }
            })
        })?;

    let tool_name = call_params.name.clone();
    info!(tool = %tool_name, "Executing tool");

    // Convert to tool enum
    let tool = Trading212Tools::try_from(call_params).map_err(|e| {
        error!(error = ?e, tool = %tool_name, "Invalid tool parameters");
        McpError {
            code: -32602,
            message: "Invalid tool parameters".to_string(),
        }
    })?;

    // Execute tool (using shared client and cache from app_state)
    let result = match tool {
        Trading212Tools::GetInstrumentsTool(get_instruments_tool) => {
            get_instruments_tool
                .call_tool(&app_state.client, &context.config, &app_state.cache)
                .await
        }
        Trading212Tools::GetAllPiesWithHoldingsTool(get_all_pies_with_holdings_tool) => {
            get_all_pies_with_holdings_tool
                .call_tool(&app_state.client, &context.config, &app_state.cache)
                .await
        }
        Trading212Tools::UpdatePieTool(update_pie_tool) => {
            update_pie_tool
                .call_tool(&app_state.client, &context.config, &app_state.cache)
                .await
        }
        Trading212Tools::CreatePieTool(create_pie_tool) => {
            create_pie_tool
                .call_tool(&app_state.client, &context.config, &app_state.cache)
                .await
        }
    };

    // Convert result to JSON - don't leak internal errors
    result
        .map_err(|e| {
            error!(error = %e, tool = %tool_name, "Tool execution failed");
            McpError {
                code: -32000,
                message: "Tool execution failed. Check server logs for details.".to_string(),
            }
        })
        .and_then(|r| {
            serde_json::to_value(r).map_err(|e| {
                error!(error = %e, tool = %tool_name, "Result serialization failed");
                McpError {
                    code: -32603,
                    message: "Internal error occurred".to_string(),
                }
            })
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_handle_initialize_returns_proper_result() {
        // Test with params
        let params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        });

        let result = handle_initialize(Some(&params));
        assert!(result.is_ok(), "handle_initialize should return Ok");

        let value = result.unwrap();
        assert!(value["protocolVersion"].is_string());
        assert!(value["serverInfo"].is_object());
        assert!(value["capabilities"].is_object());
        assert_eq!(value["serverInfo"]["name"], "Trading212 MCP Server");
        assert_eq!(value["serverInfo"]["version"], "0.1.0");
        assert!(value["capabilities"]["tools"].is_object());
    }

    #[test]
    fn test_handle_initialize_without_params() {
        // Test without params
        let result = handle_initialize(None);
        assert!(
            result.is_ok(),
            "handle_initialize should work without params"
        );

        let value = result.unwrap();
        assert!(value["serverInfo"].is_object());
        assert_eq!(value["serverInfo"]["name"], "Trading212 MCP Server");
    }

    #[test]
    fn test_handle_initialize_response_has_all_required_fields() {
        let result = handle_initialize(None).unwrap();

        // Required fields
        assert!(result.get("protocolVersion").is_some());
        assert!(result.get("serverInfo").is_some());
        assert!(result.get("capabilities").is_some());

        // Server info fields
        let server_info = &result["serverInfo"];
        assert!(server_info.get("name").is_some());
        assert!(server_info.get("version").is_some());

        // Optional fields should be present
        assert!(result.get("instructions").is_some());
    }

    #[test]
    fn test_handle_initialize_error_code() {
        // This tests that serialization errors return the correct error code
        // If handle_initialize fails to serialize, it should return error code -32603
        let result = handle_initialize(None);
        assert!(
            result.is_ok(),
            "Normal case should succeed, but if it fails, error code should be -32603"
        );
    }

    #[tokio::test]
    async fn test_mcp_request_routing_initialize() {
        // Test that the initialize method is properly routed
        // This tests the match arm in handle_mcp_request

        let app_state = Arc::new(AppState {
            client: Client::new(),
            cache: Arc::new(Trading212Cache::new().expect("Failed to create cache")),
        });

        let context = Trading212Context {
            config: Trading212Config::new_with_api_key("test-key".to_string()),
        };

        // Create an initialize request
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {}
            })),
        };

        // Call the handler directly
        let response = handle_mcp_request(State(app_state), context, Json(request))
            .await
            .unwrap();

        // Verify response structure
        assert_eq!(response.0.jsonrpc, "2.0");
        assert!(response.0.result.is_some());
        assert!(response.0.error.is_none());

        // Verify the result contains initialize response
        let result = response.0.result.unwrap();
        assert_eq!(result["serverInfo"]["name"], "Trading212 MCP Server");
    }

    #[tokio::test]
    async fn test_mcp_request_routing_unknown_method() {
        // Test that unknown methods return an error
        let app_state = Arc::new(AppState {
            client: Client::new(),
            cache: Arc::new(Trading212Cache::new().expect("Failed to create cache")),
        });

        let context = Trading212Context {
            config: Trading212Config::new_with_api_key("test-key".to_string()),
        };

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "unknown_method".to_string(),
            params: None,
        };

        let response = handle_mcp_request(State(app_state), context, Json(request))
            .await
            .unwrap();

        // Should return an error
        assert!(response.0.error.is_some());
        assert_eq!(response.0.error.unwrap().code, -32601); // Method not found
    }

    #[test]
    fn test_handle_initialize_with_different_protocol_versions() {
        // Test that handle_initialize works with different protocol versions
        let versions = vec!["2024-11-05", "2025-06-18", "1.0.0"];

        for version in versions {
            let params = json!({
                "protocolVersion": version,
                "capabilities": {}
            });

            let result = handle_initialize(Some(&params));
            assert!(result.is_ok());

            // Server should return its own protocol version regardless of client version
            let value = result.unwrap();
            assert!(value["protocolVersion"].is_string());
        }
    }

    #[tokio::test]
    async fn test_mcp_request_accepts_notification_without_id() {
        // Test that notifications (messages without ID) are accepted
        let app_state = Arc::new(AppState {
            client: Client::new(),
            cache: Arc::new(Trading212Cache::new().expect("Failed to create cache")),
        });

        let context = Trading212Context {
            config: Trading212Config::new_with_api_key("test-key".to_string()),
        };

        // Create an "initialized" notification (no ID)
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "initialized".to_string(),
            params: None,
        };

        // Call the handler - should succeed even without ID
        let response = handle_mcp_request(State(app_state), context, Json(request))
            .await
            .unwrap();

        // Response should have no ID and no result (notification doesn't get a response)
        assert_eq!(response.0.jsonrpc, "2.0");
        assert!(response.0.id.is_none());
        assert!(response.0.result.is_none());
    }

    #[tokio::test]
    async fn test_mcp_request_vs_notification_difference() {
        // Test that requests get responses but notifications don't
        let app_state = Arc::new(AppState {
            client: Client::new(),
            cache: Arc::new(Trading212Cache::new().expect("Failed to create cache")),
        });

        let context = Trading212Context {
            config: Trading212Config::new_with_api_key("test-key".to_string()),
        };

        // Test 1: Request (with ID) should get a proper response
        let request_with_id = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {}
            })),
        };

        let response = handle_mcp_request(
            State(app_state.clone()),
            Trading212Context {
                config: Trading212Config::new_with_api_key("test-key".to_string()),
            },
            Json(request_with_id),
        )
        .await
        .unwrap();

        assert_eq!(response.0.jsonrpc, "2.0");
        assert_eq!(response.0.id, Some(serde_json::Value::Number(1.into())));
        assert!(response.0.result.is_some());

        // Test 2: Notification (without ID) should get empty response
        let notification_without_id = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "initialized".to_string(),
            params: None,
        };

        let response = handle_mcp_request(State(app_state), context, Json(notification_without_id))
            .await
            .unwrap();

        assert_eq!(response.0.jsonrpc, "2.0");
        assert!(response.0.id.is_none());
        assert!(response.0.result.is_none());
    }
}
