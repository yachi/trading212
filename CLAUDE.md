# Claude Code Project Instructions

## Required Reading

Before working on this project, **always read `AGENTS.md`** for project-specific instructions and code quality requirements.

## Project Overview

This is a Trading212 MCP (Model Context Protocol) server implementation that provides AI assistants access to Trading212 API functionality.

## Development Workflow

1. **Read Documentation**: Review `AGENTS.md` and `trading212-api.md` before starting
2. **Code Quality**: Follow the quality requirements specified in `AGENTS.md`
3. **Testing**: Ensure all changes work with the Trading212 API
4. **Verification**: Pre-commit hooks handle formatting; CI runs comprehensive checks (clippy, tests, build) on PRs

## Key Files

- `AGENTS.md` - Development instructions and code quality requirements
- `trading212-api.md` - Trading212 API reference and usage guidelines
- `trading212-mcp-server/README.md` - User documentation and setup instructions

## API Considerations

- Trading212 has strict rate limits - avoid retry logic
- Always use debug logging for API troubleshooting
- Respect the `pies:read` scope requirement for pie-related functionality

## Running the MCP Server Locally

The `trading212-mcp` entry registered for this project is the **HTTP** transport
(`http://localhost:3000/mcp/v1`), served by the `remote-server` binary — not the stdio
`main.rs` path. Nothing listens on that port by default, so a fresh session reports
`ConnectionRefused` and registers **zero** `mcp__trading212-mcp__*` tools.

Bring it up from `trading212-mcp-server/`:

```bash
cargo build --bin remote-server
nohup ./target/debug/remote-server >> /tmp/t212-server.log 2>&1 < /dev/null & disown
lsof -nP -iTCP:3000 -sTCP:LISTEN   # confirm, in a separate command from the launch
```

- Use `nohup … & disown`, **not** a harness background task — a background task is SIGTERM'd
  when the task is stopped, killing the server out from under an active session.
- Override the bind with `MCP_HTTP_HOST` / `MCP_HTTP_PORT` (defaults `0.0.0.0:3000`).
- **No API key is needed to serve.** `remote-server` takes the key per-request from the
  `Authorization: Bearer` (or `X-Trading212-API-Key`) header the client sends. The
  `~/.trading212-api-key` file and `TRADING212_BASE_URL` in `config.rs` apply to the stdio path.
- After starting the server, reconnect with `/mcp` — **the client does not retry a server it
  marked failed at startup**, so the tools stay absent no matter how healthy the port is.
- Never print the client config (`~/.claude.json`) to inspect this wiring; the entry carries a
  live API key in its `headers`. Project the `type`/`url` fields instead.

## Tool Semantics

Four tools, all defined in `src/tools.rs`: `get_all_pies_with_holdings`, `get_instruments`,
`create_pie`, `update_pie`.

- **There is no "buckets" concept.** Users may say "bucket"; the API, the codebase, and the
  tool names all say *pie*.
- `get_all_pies_with_holdings` returns **no currency field** on any figure —
  `priceAvgValue`, `priceAvgInvestedValue`, `cash`, and dividend amounts are bare floats in the
  account's currency. Do not stamp a currency symbol on them when reporting.
- When summarising pies, compute totals with a tool rather than summing the column by hand, and
  cross-check that the sum of per-pie `priceAvgResult` equals total value − total invested.
