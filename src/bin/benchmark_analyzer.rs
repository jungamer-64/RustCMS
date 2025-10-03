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

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("❌ Error: Missing required arguments");
        eprintln!();
        eprintln!("Usage: {} <results.json> [baseline.json]", args[0]);
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  {} current-results.json", args[0]);
        eprintln!("  {} current-results.json baseline-results.json", args[0]);
        std::process::exit(1);
    }

    match analyzer::BenchmarkCli::run(&args) {
        Ok(()) => {
            println!();
            println!("✨ Analysis complete!");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            std::process::exit(1);
        }
    }
}
