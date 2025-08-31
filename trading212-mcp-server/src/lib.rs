//! Trading212 MCP Server Library
//!
//! A Model Context Protocol (MCP) server that provides access to Trading212 API functionality.

// Suppress unused crate dependency warnings for dev dependencies
use tokio as _;
use tracing_subscriber as _;

#[cfg(test)]
use criterion as _;

// Include cargo-husky for git hooks (dev dependency)
#[cfg(test)]
use cargo_husky as _;

pub mod cache;
pub mod config;
pub mod errors;
pub mod handler;
pub mod tools;

pub use cache::Trading212Cache;
pub use config::Trading212Config;
pub use errors::Trading212Error;
pub use handler::Trading212Handler;
pub use tools::Trading212Tools;
