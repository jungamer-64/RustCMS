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
///
/// # Security Note
///
/// This tool uses `env::args_os()` to handle command-line arguments, including
/// those that may contain invalid UTF-8. While the documentation warns against
/// relying on `args[0]` for security purposes, this tool:
/// 
/// 1. **Does NOT use args[0] for security decisions** - only for usage display
/// 2. **Validates all arguments** - converts OsString to String with error handling
/// 3. **Operates on local files only** - no network or privileged operations
/// 4. **Is a development tool** - not exposed in production environments
///
/// The args[0] value (program name) is only used for displaying usage information
/// and does not influence any security-critical decisions.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // SAFETY: Using env::args_os() to handle invalid UTF-8 arguments gracefully.
    // All arguments are validated and converted to UTF-8 String before use.
    // The first argument (program name) is only used for display purposes in
    // error messages and does not affect program logic or security decisions.
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
