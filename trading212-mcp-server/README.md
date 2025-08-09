# Trading212 MCP Server

A Model Context Protocol (MCP) server that provides access to Trading212 API functionality. This server enables AI assistants to retrieve information about tradeable instruments from Trading212's platform through the MCP protocol specification.

## Features

- **Instrument Retrieval**: Get comprehensive information about tradeable instruments
- **Search & Filtering**: Filter instruments by search terms and instrument types
- **Retry Logic**: Built-in HTTP retry mechanism with exponential backoff
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
- Rate limiting and retry logic
- JSON parsing errors
- Configuration errors

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

## API Documentation

For detailed Trading212 API documentation, visit: [Trading212 API Documentation](https://t212public-api-docs.redoc.ly/)