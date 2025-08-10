# Trading212 MCP Server

A Model Context Protocol (MCP) server that provides access to Trading212 API functionality. This server enables AI assistants to retrieve information about tradeable instruments from Trading212's platform through the MCP protocol specification.

## Features

- **Instrument Retrieval**: Get comprehensive information about tradeable instruments
- **Investment Pies Analytics**: Access financial performance data and detailed holdings for all your Trading212 investment pies
- **Search & Filtering**: Filter instruments by search terms and instrument types
- **Performance Tracking**: View profit/loss, dividend details, and goal progress for pies
- **Real-time Data**: Direct API integration for up-to-date financial information
- **Structured Logging**: Comprehensive logging with tracing support
- **Error Handling**: Robust error handling with detailed error types
- **MCP 2025-06-18 Compliance**: Full compliance with latest MCP protocol specification

## Prerequisites

- Rust 1.75 or later
- Trading212 API key

## Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd trading212-mcp-server
```

2. Build the project:
```bash
cargo build --release
```

## Configuration

### API Key Setup

Create a file containing your Trading212 API key:

```bash
echo "your-trading212-api-key" > ~/.trading212-api-key
```

### Environment Variables

The server supports the following optional environment variables:

- `TRADING212_BASE_URL`: API base URL (default: `https://live.trading212.com/api/v0`)
- `TRADING212_TIMEOUT`: Request timeout in seconds (default: `30`)
- `TRADING212_MAX_RETRIES`: Maximum retry attempts (default: `3`)

## Usage

### Running the Server

```bash
./target/release/trading212-mcp-server
```

The server communicates via stdio using the MCP protocol.

### Available Tools

#### `get_instruments`

Retrieves a list of all tradeable instruments from Trading212.

**Parameters:**
- `search` (optional): Search term to filter instruments
- `type` (optional): Instrument type filter (e.g., "STOCK", "ETF")

**Example MCP Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "get_instruments",
    "arguments": {
      "search": "AAPL",
      "type": "STOCK"
    }
  }
}
```

#### `get_pies`

Retrieves financial summary data for all investment pies from your Trading212 account.

**Parameters:** None

**Scope Required:** `pies:read`

**Example MCP Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "get_pies",
    "arguments": {}
  }
}
```

**Response includes:**
- Pie ID and current status
- Cash balance in each pie
- Dividend details (gained, reinvested, cash)
- Performance metrics (invested value, current value, profit/loss)
- Goal progress percentage
- Overall performance status (e.g., "AHEAD")

**Sample Response:**
```json
{
  "id": 3311223,
  "cash": 1.29,
  "dividendDetails": {
    "gained": 0.71,
    "reinvested": 0.71,
    "inCash": 0.0
  },
  "result": {
    "priceAvgInvestedValue": 3386.1,
    "priceAvgValue": 3589.01,
    "priceAvgResult": 202.91,
    "priceAvgResultCoef": 0.0599
  },
  "progress": 0.1639,
  "status": "AHEAD"
}
```

#### `get_pie_by_id`

Retrieves detailed information about a specific investment pie by its ID, including individual instrument holdings and allocations.

**Parameters:**
- `pie_id` (required): The unique ID of the pie to retrieve

**Scope Required:** `pies:read`

**Example MCP Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "get_pie_by_id",
    "arguments": {
      "pie_id": 3311223
    }
  }
}
```

**Response includes:**
- **Instrument Holdings**: Detailed breakdown of each stock/ETF in the pie
- **Individual Performance**: Profit/loss for each instrument
- **Share Allocations**: Current vs expected allocation percentages
- **Pie Settings**: Name, icon, goal, creation date, dividend policy
- **Quantities Owned**: Exact shares held of each instrument

**Sample Response Structure:**
```json
{
  "instruments": [
    {
      "ticker": "NVDA_US_EQ",
      "currentShare": 0.0803,
      "expectedShare": 0.14,
      "ownedQuantity": 2.1225186,
      "result": {
        "priceAvgInvestedValue": 287.06,
        "priceAvgResult": 1.13,
        "priceAvgResultCoef": 0.0039
      }
    }
  ],
  "settings": {
    "name": "Growth Focused Strategy",
    "goal": 21908.0,
    "icon": "Coins",
    "dividendCashAction": "REINVEST"
  }
}
```

## Integration with Claude Desktop

Add the following to your Claude Desktop MCP configuration:

```json
{
  "mcpServers": {
    "trading212": {
      "command": "/path/to/trading212-mcp-server/target/release/trading212-mcp-server"
    }
  }
}
```

## Development

### Code Quality

This project uses comprehensive linting with Clippy:

```bash
# Format code
cargo fmt

# Run lints
cargo clippy

# Run tests
cargo test
```

### Linting Configuration

The project enforces strict code quality standards:
- No unsafe code allowed
- Comprehensive error handling (no `.unwrap()` calls)
- Full documentation requirements
- Pedantic clippy lints enabled

## Architecture

### Components

- **`main.rs`**: Server initialization and MCP protocol setup
- **`config.rs`**: Configuration management and API key loading
- **`errors.rs`**: Custom error types and error handling
- **`handler.rs`**: MCP protocol message handler
- **`tools.rs`**: Tool implementations and Trading212 API integration

### Data Flow

1. MCP client sends tool request via stdio
2. Handler validates and processes the request
3. Tool implementation makes API call to Trading212
4. Results are formatted and returned via MCP protocol

## Error Handling

The server implements comprehensive error handling for:
- API authentication failures
- Network connectivity issues
- Rate limiting (with immediate failure to respect Trading212's strict limits)
- JSON parsing errors with detailed response logging
- Configuration errors

**Note**: The server does not retry failed requests to respect Trading212's rate limits. Failed requests return immediately with detailed error information.

## Logging

Structured logging is available with environment variable control:

```bash
# Set log level
export RUST_LOG=debug

# Run with debug logging
./target/release/trading212-mcp-server
```

## Security

- API keys are stored in separate files outside the repository
- No hardcoded credentials in source code
- Secure HTTP client configuration with timeout handling

## License

MIT License

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Ensure all lints pass: `cargo clippy`
5. Format code: `cargo fmt`
6. Submit a pull request

## Troubleshooting

### Common Issues

**"Failed to read API key"**: Ensure `~/.trading212-api-key` exists and contains your valid API key.

**Connection timeout**: Check your internet connection and Trading212 API status.

**Invalid API response**: Verify your API key is valid and has appropriate permissions.

### Debug Mode

Run with debug logging for detailed information:

```bash
RUST_LOG=debug ./target/release/trading212-mcp-server
```

Debug logging includes:
- Raw API request/response bodies
- HTTP status codes and headers
- JSON parsing details
- Request timing information

## API Documentation

For detailed Trading212 API documentation, visit: [Trading212 API Documentation](https://t212public-api-docs.redoc.ly/)