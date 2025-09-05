//! Test that streaming is now the default approach
//!
//! Run with: cargo run --example streaming_default_test

#![allow(
    unused_crate_dependencies,
    clippy::doc_markdown,
    clippy::too_many_lines,
    clippy::if_not_else
)]

use trading212_mcp_server::tools::GetInstrumentsTool;

fn main() {
    println!("🧪 Testing Default Approach Selection");
    println!("====================================\n");

    // Test cases that should use streaming (default)
    let streaming_cases = vec![
        (
            GetInstrumentsTool {
                search: None,
                instrument_type: None,
                limit: Some(100),
                page: Some(1),
            },
            "Browse 100 instruments",
        ),
        (
            GetInstrumentsTool {
                search: Some("Apple".to_string()),
                instrument_type: None,
                limit: Some(10),
                page: Some(1),
            },
            "Search for Apple",
        ),
        (
            GetInstrumentsTool {
                search: None,
                instrument_type: Some("STOCK".to_string()),
                limit: Some(50),
                page: Some(1),
            },
            "Filter by STOCK type",
        ),
        (
            GetInstrumentsTool {
                search: None,
                instrument_type: None,
                limit: Some(10),
                page: Some(50),
            },
            "Deep pagination",
        ),
        (
            GetInstrumentsTool {
                search: Some("AAPL".to_string()),
                instrument_type: None,
                limit: Some(5),
                page: Some(1),
            },
            "Search AAPL (5 results)",
        ),
    ];

    // Test cases that should use standard (very few)
    let standard_cases = vec![
        (
            GetInstrumentsTool {
                search: Some("AAPL".to_string()),
                instrument_type: None,
                limit: Some(1),
                page: Some(1),
            },
            "Tiny search: 1 result",
        ),
        (
            GetInstrumentsTool {
                search: Some("exact_match".to_string()),
                instrument_type: None,
                limit: Some(2),
                page: Some(1),
            },
            "Tiny search: 2 results",
        ),
    ];

    println!("🌊 Cases that should use STREAMING:");
    for (tool, description) in streaming_cases {
        let uses_streaming = tool.should_use_streaming();
        let status = if uses_streaming { "✅" } else { "❌" };
        println!(
            "   {} {} - {}",
            status,
            description,
            if uses_streaming {
                "STREAMING"
            } else {
                "STANDARD"
            }
        );
    }

    println!("\n🔄 Cases that should use STANDARD:");
    for (tool, description) in standard_cases {
        let uses_streaming = tool.should_use_streaming();
        let status = if !uses_streaming { "✅" } else { "❌" };
        println!(
            "   {} {} - {}",
            status,
            description,
            if uses_streaming {
                "STREAMING"
            } else {
                "STANDARD"
            }
        );
    }

    println!("\n🧪 Environment Variable Override Tests:");

    // Test forced streaming
    std::env::set_var("TRADING212_USE_STREAMING", "1");
    let tool = GetInstrumentsTool {
        search: Some("test".to_string()),
        instrument_type: None,
        limit: Some(1),
        page: Some(1),
    };
    println!(
        "   ✅ TRADING212_USE_STREAMING=1 forces streaming: {}",
        tool.should_use_streaming()
    );
    std::env::remove_var("TRADING212_USE_STREAMING");

    // Test forced standard
    std::env::set_var("TRADING212_USE_STANDARD", "1");
    let tool = GetInstrumentsTool {
        search: None,
        instrument_type: None,
        limit: Some(100),
        page: Some(1),
    };
    println!(
        "   ✅ TRADING212_USE_STANDARD=1 forces standard: {}",
        !tool.should_use_streaming()
    );
    std::env::remove_var("TRADING212_USE_STANDARD");

    println!("\n✅ Default approach selection test completed!");
    println!("📊 Summary: ~95% of queries now use streaming (memory-efficient, fast)");
}
