//! Benchmark Analysis and Reporting Tool
//!
//! This tool analyzes benchmark results and generates comprehensive reports.
//! Features:
//! - Parse Criterion JSON output
//! - Compare multiple benchmark runs
//! - Detect performance regressions
//! - Generate HTML/CSV/JSON reports
//! - Track performance over time

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// ============================================================================
// Data Structures
// ============================================================================

/// Benchmark result from Criterion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub mean: Duration,
    pub median: Duration,
    pub std_dev: Duration,
    pub min: Duration,
    pub max: Duration,
    pub sample_size: usize,
}

/// Duration in nanoseconds
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Duration {
    pub nanos: u64,
}

impl Duration {
    pub fn from_nanos(nanos: u64) -> Self {
        Self { nanos }
    }

    #[allow(dead_code)]
    pub fn as_secs_f64(&self) -> f64 {
        self.nanos as f64 / 1_000_000_000.0
    }

    pub fn as_millis(&self) -> u64 {
        self.nanos / 1_000_000
    }

    pub fn as_micros(&self) -> u64 {
        self.nanos / 1_000
    }
}

/// Comparison between two benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    pub name: String,
    pub baseline_mean: Duration,
    pub current_mean: Duration,
    pub change_percent: f64,
    pub is_regression: bool,
    pub is_improvement: bool,
    pub significance: Significance,
}

/// Statistical significance level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Significance {
    High,      // > 10% change
    Medium,    // 5-10% change
    Low,       // 1-5% change
    None,      // < 1% change
}

/// Performance category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerformanceCategory {
    Excellent,   // Within targets
    Good,        // 10-20% slower
    Fair,        // 20-50% slower
    Poor,        // > 50% slower
}

// ============================================================================
// Analysis Engine
// ============================================================================

pub struct BenchmarkAnalyzer {
    results: HashMap<String, BenchmarkResult>,
    baseline: Option<HashMap<String, BenchmarkResult>>,
}

impl BenchmarkAnalyzer {
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
            baseline: None,
        }
    }

    /// Load benchmark results from JSON file
    pub fn load_results<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        
        let results: Vec<BenchmarkResult> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        for result in results {
            self.results.insert(result.name.clone(), result);
        }

        Ok(())
    }

    /// Load baseline results for comparison
    pub fn load_baseline<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read baseline: {}", e))?;
        
        let results: Vec<BenchmarkResult> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse baseline: {}", e))?;

        let mut baseline = HashMap::new();
        for result in results {
            baseline.insert(result.name.clone(), result);
        }

        self.baseline = Some(baseline);
        Ok(())
    }

    /// Compare current results with baseline
    pub fn compare_with_baseline(&self) -> Vec<BenchmarkComparison> {
        let mut comparisons = Vec::new();

        if let Some(baseline) = &self.baseline {
            for (name, current) in &self.results {
                if let Some(base) = baseline.get(name) {
                    let comparison = self.compare_results(name, base, current);
                    comparisons.push(comparison);
                }
            }
        }

        comparisons.sort_by(|a, b| {
            b.change_percent.abs().partial_cmp(&a.change_percent.abs()).unwrap()
        });

        comparisons
    }

    /// Compare two benchmark results
    fn compare_results(
        &self,
        name: &str,
        baseline: &BenchmarkResult,
        current: &BenchmarkResult,
    ) -> BenchmarkComparison {
        let change = (current.mean.nanos as f64 - baseline.mean.nanos as f64) 
            / baseline.mean.nanos as f64 * 100.0;

        let significance = if change.abs() > 10.0 {
            Significance::High
        } else if change.abs() > 5.0 {
            Significance::Medium
        } else if change.abs() > 1.0 {
            Significance::Low
        } else {
            Significance::None
        };

        BenchmarkComparison {
            name: name.to_string(),
            baseline_mean: baseline.mean,
            current_mean: current.mean,
            change_percent: change,
            is_regression: change > 5.0,
            is_improvement: change < -5.0,
            significance,
        }
    }

    /// Categorize performance based on targets
    #[allow(dead_code)]
    pub fn categorize_performance(&self, target_nanos: u64) -> HashMap<String, PerformanceCategory> {
        let mut categories = HashMap::new();

        for (name, result) in &self.results {
            let ratio = result.mean.nanos as f64 / target_nanos as f64;
            
            let category = if ratio <= 1.0 {
                PerformanceCategory::Excellent
            } else if ratio <= 1.2 {
                PerformanceCategory::Good
            } else if ratio <= 1.5 {
                PerformanceCategory::Fair
            } else {
                PerformanceCategory::Poor
            };

            categories.insert(name.clone(), category);
        }

        categories
    }

    /// Generate summary statistics
    pub fn generate_summary(&self) -> BenchmarkSummary {
        let total_benchmarks = self.results.len();
        let mut total_time_nanos = 0u64;
        let mut fastest: Option<BenchmarkResult> = None;
        let mut slowest: Option<BenchmarkResult> = None;

        for result in self.results.values() {
            total_time_nanos += result.mean.nanos;

            if fastest.is_none() || result.mean.nanos < fastest.as_ref().unwrap().mean.nanos {
                fastest = Some(result.clone());
            }

            if slowest.is_none() || result.mean.nanos > slowest.as_ref().unwrap().mean.nanos {
                slowest = Some(result.clone());
            }
        }

        let average_time = if total_benchmarks > 0 {
            Duration::from_nanos(total_time_nanos / total_benchmarks as u64)
        } else {
            Duration::from_nanos(0)
        };

        BenchmarkSummary {
            total_benchmarks,
            average_time,
            fastest,
            slowest,
        }
    }
}

impl Default for BenchmarkAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary statistics for all benchmarks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub total_benchmarks: usize,
    pub average_time: Duration,
    pub fastest: Option<BenchmarkResult>,
    pub slowest: Option<BenchmarkResult>,
}

// ============================================================================
// Report Generation
// ============================================================================

pub struct ReportGenerator;

impl ReportGenerator {
    /// Generate Markdown report
    pub fn generate_markdown(
        summary: &BenchmarkSummary,
        comparisons: &[BenchmarkComparison],
    ) -> String {
        let mut report = String::new();

        report.push_str("# Benchmark Analysis Report\n\n");
        report.push_str(&format!("**Date**: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

        // Summary section
        report.push_str("## Summary\n\n");
        report.push_str(&format!("- Total benchmarks: {}\n", summary.total_benchmarks));
        report.push_str(&format!("- Average time: {:.2}ms\n", summary.average_time.as_millis()));
        
        if let Some(fastest) = &summary.fastest {
            report.push_str(&format!("- Fastest: {} ({:.2}µs)\n", 
                fastest.name, fastest.mean.as_micros()));
        }
        
        if let Some(slowest) = &summary.slowest {
            report.push_str(&format!("- Slowest: {} ({:.2}ms)\n\n", 
                slowest.name, slowest.mean.as_millis()));
        }

        // Regressions section
        let regressions: Vec<_> = comparisons.iter()
            .filter(|c| c.is_regression)
            .collect();

        if !regressions.is_empty() {
            report.push_str("## ⚠️ Performance Regressions\n\n");
            report.push_str("| Benchmark | Baseline | Current | Change |\n");
            report.push_str("|-----------|----------|---------|--------|\n");
            
            for comp in regressions {
                report.push_str(&format!(
                    "| {} | {:.2}ms | {:.2}ms | **+{:.1}%** |\n",
                    comp.name,
                    comp.baseline_mean.as_millis(),
                    comp.current_mean.as_millis(),
                    comp.change_percent
                ));
            }
            report.push('\n');
        }

        // Improvements section
        let improvements: Vec<_> = comparisons.iter()
            .filter(|c| c.is_improvement)
            .collect();

        if !improvements.is_empty() {
            report.push_str("## ✅ Performance Improvements\n\n");
            report.push_str("| Benchmark | Baseline | Current | Change |\n");
            report.push_str("|-----------|----------|---------|--------|\n");
            
            for comp in improvements {
                report.push_str(&format!(
                    "| {} | {:.2}ms | {:.2}ms | **{:.1}%** |\n",
                    comp.name,
                    comp.baseline_mean.as_millis(),
                    comp.current_mean.as_millis(),
                    comp.change_percent
                ));
            }
            report.push('\n');
        }

        // All comparisons
        if !comparisons.is_empty() {
            report.push_str("## All Comparisons\n\n");
            report.push_str("| Benchmark | Baseline | Current | Change | Status |\n");
            report.push_str("|-----------|----------|---------|--------|--------|\n");
            
            for comp in comparisons {
                let status = if comp.is_regression {
                    "⚠️ Regression"
                } else if comp.is_improvement {
                    "✅ Improvement"
                } else {
                    "➖ Stable"
                };

                report.push_str(&format!(
                    "| {} | {:.2}ms | {:.2}ms | {:+.1}% | {} |\n",
                    comp.name,
                    comp.baseline_mean.as_millis(),
                    comp.current_mean.as_millis(),
                    comp.change_percent,
                    status
                ));
            }
        }

        report
    }

    /// Generate CSV report
    pub fn generate_csv(comparisons: &[BenchmarkComparison]) -> String {
        let mut csv = String::new();
        csv.push_str("Benchmark,Baseline (ms),Current (ms),Change (%),Status\n");

        for comp in comparisons {
            let status = if comp.is_regression {
                "Regression"
            } else if comp.is_improvement {
                "Improvement"
            } else {
                "Stable"
            };

            csv.push_str(&format!(
                "{},{:.2},{:.2},{:+.2},{}\n",
                comp.name,
                comp.baseline_mean.as_millis(),
                comp.current_mean.as_millis(),
                comp.change_percent,
                status
            ));
        }

        csv
    }

    /// Generate HTML report
    pub fn generate_html(
        summary: &BenchmarkSummary,
        comparisons: &[BenchmarkComparison],
    ) -> String {
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<title>Benchmark Analysis Report</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
        html.push_str("table { border-collapse: collapse; width: 100%; margin: 20px 0; }\n");
        html.push_str("th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n");
        html.push_str("th { background-color: #4CAF50; color: white; }\n");
        html.push_str(".regression { color: red; font-weight: bold; }\n");
        html.push_str(".improvement { color: green; font-weight: bold; }\n");
        html.push_str(".stable { color: gray; }\n");
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");

        html.push_str("<h1>Benchmark Analysis Report</h1>\n");
        html.push_str(&format!("<p>Generated: {}</p>\n", 
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

        // Summary
        html.push_str("<h2>Summary</h2>\n");
        html.push_str("<ul>\n");
        html.push_str(&format!("<li>Total benchmarks: {}</li>\n", summary.total_benchmarks));
        html.push_str(&format!("<li>Average time: {:.2}ms</li>\n", summary.average_time.as_millis()));
        html.push_str("</ul>\n");

        // Comparisons table
        if !comparisons.is_empty() {
            html.push_str("<h2>Performance Comparison</h2>\n");
            html.push_str("<table>\n");
            html.push_str("<tr><th>Benchmark</th><th>Baseline</th><th>Current</th><th>Change</th><th>Status</th></tr>\n");

            for comp in comparisons {
                let (status_class, status_text) = if comp.is_regression {
                    ("regression", "⚠️ Regression")
                } else if comp.is_improvement {
                    ("improvement", "✅ Improvement")
                } else {
                    ("stable", "➖ Stable")
                };

                html.push_str(&format!(
                    "<tr><td>{}</td><td>{:.2}ms</td><td>{:.2}ms</td><td class=\"{}\">{:+.1}%</td><td class=\"{}\">{}</td></tr>\n",
                    comp.name,
                    comp.baseline_mean.as_millis(),
                    comp.current_mean.as_millis(),
                    status_class,
                    comp.change_percent,
                    status_class,
                    status_text
                ));
            }

            html.push_str("</table>\n");
        }

        html.push_str("</body>\n</html>");
        html
    }
}

// ============================================================================
// CLI Tool
// ============================================================================

pub struct BenchmarkCli;

impl BenchmarkCli {
    pub fn run(args: Vec<String>) -> Result<(), String> {
        if args.len() < 2 {
            return Err("Usage: benchmark-analyzer <results.json> [baseline.json]".to_string());
        }

        let mut analyzer = BenchmarkAnalyzer::new();
        analyzer.load_results(&args[1])?;

        if args.len() >= 3 {
            analyzer.load_baseline(&args[2])?;
        }

        let summary = analyzer.generate_summary();
        let comparisons = analyzer.compare_with_baseline();

        // Generate reports
        let markdown = ReportGenerator::generate_markdown(&summary, &comparisons);
        let csv = ReportGenerator::generate_csv(&comparisons);
        let html = ReportGenerator::generate_html(&summary, &comparisons);

        // Save reports
        fs::write("benchmark-report.md", markdown)
            .map_err(|e| format!("Failed to write markdown: {}", e))?;
        fs::write("benchmark-report.csv", csv)
            .map_err(|e| format!("Failed to write CSV: {}", e))?;
        fs::write("benchmark-report.html", html)
            .map_err(|e| format!("Failed to write HTML: {}", e))?;

        println!("✅ Reports generated successfully:");
        println!("   - benchmark-report.md");
        println!("   - benchmark-report.csv");
        println!("   - benchmark-report.html");

        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_conversions() {
        let duration = Duration::from_nanos(1_500_000_000);
        assert_eq!(duration.as_secs_f64(), 1.5);
        assert_eq!(duration.as_millis(), 1500);
        assert_eq!(duration.as_micros(), 1_500_000);
    }

    #[test]
    fn test_significance_levels() {
        let analyzer = BenchmarkAnalyzer::new();
        
        let baseline = BenchmarkResult {
            name: "test".to_string(),
            mean: Duration::from_nanos(1_000_000),
            median: Duration::from_nanos(1_000_000),
            std_dev: Duration::from_nanos(10_000),
            min: Duration::from_nanos(990_000),
            max: Duration::from_nanos(1_010_000),
            sample_size: 100,
        };

        let current = BenchmarkResult {
            name: "test".to_string(),
            mean: Duration::from_nanos(1_150_000),
            median: Duration::from_nanos(1_150_000),
            std_dev: Duration::from_nanos(10_000),
            min: Duration::from_nanos(1_140_000),
            max: Duration::from_nanos(1_160_000),
            sample_size: 100,
        };

        let comparison = analyzer.compare_results("test", &baseline, &current);
        assert_eq!(comparison.significance, Significance::High);
        assert!(comparison.is_regression);
    }

    #[test]
    fn test_performance_categorization() {
        let target = 1_000_000; // 1ms

        let mut results = HashMap::new();
        results.insert(
            "fast".to_string(),
            BenchmarkResult {
                name: "fast".to_string(),
                mean: Duration::from_nanos(500_000),
                median: Duration::from_nanos(500_000),
                std_dev: Duration::from_nanos(10_000),
                min: Duration::from_nanos(490_000),
                max: Duration::from_nanos(510_000),
                sample_size: 100,
            },
        );

        let test_analyzer = BenchmarkAnalyzer {
            results,
            baseline: None,
        };

        let categories = test_analyzer.categorize_performance(target);
        assert_eq!(categories.get("fast"), Some(&PerformanceCategory::Excellent));
    }

    #[test]
    fn test_summary_generation() {
        let mut analyzer = BenchmarkAnalyzer::new();
        
        analyzer.results.insert(
            "test1".to_string(),
            BenchmarkResult {
                name: "test1".to_string(),
                mean: Duration::from_nanos(1_000_000),
                median: Duration::from_nanos(1_000_000),
                std_dev: Duration::from_nanos(10_000),
                min: Duration::from_nanos(990_000),
                max: Duration::from_nanos(1_010_000),
                sample_size: 100,
            },
        );

        analyzer.results.insert(
            "test2".to_string(),
            BenchmarkResult {
                name: "test2".to_string(),
                mean: Duration::from_nanos(2_000_000),
                median: Duration::from_nanos(2_000_000),
                std_dev: Duration::from_nanos(20_000),
                min: Duration::from_nanos(1_980_000),
                max: Duration::from_nanos(2_020_000),
                sample_size: 100,
            },
        );

        let summary = analyzer.generate_summary();
        assert_eq!(summary.total_benchmarks, 2);
        assert_eq!(summary.average_time.as_millis(), 1);
    }

    #[test]
    fn test_report_generation() {
        let summary = BenchmarkSummary {
            total_benchmarks: 2,
            average_time: Duration::from_nanos(1_500_000),
            fastest: None,
            slowest: None,
        };

        let comparisons = vec![];
        let markdown = ReportGenerator::generate_markdown(&summary, &comparisons);
        
        assert!(markdown.contains("Benchmark Analysis Report"));
        assert!(markdown.contains("Total benchmarks: 2"));
    }
}
