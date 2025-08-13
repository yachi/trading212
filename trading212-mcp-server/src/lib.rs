//! Trading212 MCP Server Library
//!
//! A Model Context Protocol (MCP) server that provides access to Trading212 API functionality.

pub mod config;
pub mod errors;
pub mod handler;
pub mod tools;

pub use config::Trading212Config;
pub use errors::Trading212Error;
pub use handler::Trading212Handler;
pub use tools::Trading212Tools;
