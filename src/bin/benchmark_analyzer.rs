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

use clap::Parser;
use std::io;
use std::path::PathBuf;

/// CLI argument parser for the benchmark analyzer tool
#[derive(Parser, Debug)]
#[command(
    name = "benchmark-analyzer",
    about = "Analyze benchmark outputs and generate comparison reports"
)]
struct CliArgs {
    /// Path to the benchmark results JSON file produced by Criterion
    #[arg(value_name = "RESULTS_JSON")]
    results: PathBuf,

    /// Optional baseline results JSON file for regression detection
    #[arg(value_name = "BASELINE_JSON")]
    baseline: Option<PathBuf>,
}

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
    let args = CliArgs::parse();

    analyzer::BenchmarkCli::run(args.results.as_path(), args.baseline.as_deref())
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    println!();
    println!("✨ Analysis complete!");
    Ok(())
}
