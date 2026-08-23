# Trading212 MCP Server

A Trading212 Model Context Protocol server, written in Rust, that gives AI assistants
access to Trading212 API functionality.

Before working with this codebase, read `AGENTS.md` in the repository root for
development instructions and code quality requirements.

## API constraints

These are easy to get wrong and expensive to get wrong:

- **Trading212 enforces strict rate limits — do not add retry logic.** Backing off and
  retrying compounds the problem; cache instead. Cache TTLs in `src/cache.rs` are
  deliberately generous so batch operations survive rate limiting.
- **Pie functionality requires the `pies:read` scope** (see `trading212-api.md`). The
  server exposes read, create, and update operations only — there is no sell or
  withdraw tool, so do not assume those flows can be automated.
- **Use debug logging when troubleshooting API behaviour** rather than adding
  speculative error handling.

## Toolchain

The Rust version is pinned in `rust-toolchain.toml`. CI runs
`cargo clippy --all-targets --all-features -- -D warnings`, so warnings are build
failures. Bump the pin deliberately and fix any new lints in the same change.
