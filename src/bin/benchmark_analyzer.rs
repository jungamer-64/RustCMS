//! Benchmark Analyzer CLI
//!
//! Command-line tool for analyzing benchmark results and generating reports.
//!
//! Usage:
//!   benchmark-analyzer <results.json> [baseline.json]
//!
//! Examples:
//!   benchmark-analyzer current-results.json
//!   benchmark-analyzer current-results.json baseline-results.json

#[path = "../../benches/analyzer/mod.rs"]
mod analyzer;

use std::env;

/// Main function that returns Result for better error handling
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Security: Use env::args_os() to handle invalid UTF-8 arguments gracefully
    // Convert OsString to String, propagating errors for non-UTF-8 arguments
    // Note: We don't rely on args[0] for security purposes, only for usage display
    let args: Vec<String> = env::args_os()
        .map(std::ffi::OsString::into_string)
        .collect::<Result<Vec<String>, _>>()
        .map_err(|invalid_os_str| {
            format!(
                "Invalid UTF-8 in command line argument: {}",
                invalid_os_str.to_string_lossy()
            )
        })?;

    if args.len() < 2 {
        eprintln!("❌ Error: Missing required arguments");
        eprintln!();
        eprintln!("Usage: benchmark-analyzer <results.json> [baseline.json]");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  benchmark-analyzer current-results.json");
        eprintln!("  benchmark-analyzer current-results.json baseline-results.json");
        return Err("Missing required arguments".into());
    }

    analyzer::BenchmarkCli::run(&args)?;

    println!();
    println!("✨ Analysis complete!");
    Ok(())
}
