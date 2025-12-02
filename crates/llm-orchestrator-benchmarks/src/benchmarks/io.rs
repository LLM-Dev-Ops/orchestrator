// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! I/O operations for benchmark results.

use super::markdown::generate_markdown_report;
use super::result::BenchmarkResult;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during benchmark I/O operations.
#[derive(Error, Debug)]
pub enum BenchmarkIoError {
    #[error("Failed to create directory: {0}")]
    DirectoryCreation(#[from] std::io::Error),

    #[error("Failed to serialize results: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid output path: {0}")]
    InvalidPath(String),
}

/// Result type for benchmark I/O operations.
pub type Result<T> = std::result::Result<T, BenchmarkIoError>;

/// Writes raw benchmark results to the canonical output directory.
///
/// Each benchmark result is written as a separate JSON file in the
/// `benchmarks/output/raw/` directory, named by target_id and timestamp.
///
/// # Arguments
///
/// * `results` - Slice of BenchmarkResult to write
/// * `output_dir` - Base output directory (typically "benchmarks/output")
///
/// # Returns
///
/// A list of file paths that were written.
///
/// # Example
///
/// ```rust,no_run
/// use llm_orchestrator_benchmarks::benchmarks::io::write_raw_results;
/// use llm_orchestrator_benchmarks::benchmarks::result::BenchmarkResult;
/// use serde_json::json;
///
/// let results = vec![
///     BenchmarkResult::new("test", json!({"duration_ms": 100})),
/// ];
///
/// let paths = write_raw_results(&results, "benchmarks/output").unwrap();
/// ```
pub fn write_raw_results(results: &[BenchmarkResult], output_dir: &str) -> Result<Vec<String>> {
    let raw_dir = Path::new(output_dir).join("raw");
    fs::create_dir_all(&raw_dir)?;

    let mut written_paths = Vec::new();

    for result in results {
        let timestamp_str = result.timestamp.format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("{}_{}.json", result.target_id, timestamp_str);
        let filepath = raw_dir.join(&filename);

        let json = serde_json::to_string_pretty(result)?;
        let mut file = File::create(&filepath)?;
        file.write_all(json.as_bytes())?;

        written_paths.push(filepath.to_string_lossy().to_string());
    }

    // Also write a combined results file
    let combined_path = Path::new(output_dir).join("latest_results.json");
    let combined_json = serde_json::to_string_pretty(results)?;
    let mut combined_file = File::create(&combined_path)?;
    combined_file.write_all(combined_json.as_bytes())?;
    written_paths.push(combined_path.to_string_lossy().to_string());

    Ok(written_paths)
}

/// Writes the benchmark summary markdown file.
///
/// Generates a comprehensive markdown report from the benchmark results
/// and writes it to `benchmarks/output/summary.md`.
///
/// # Arguments
///
/// * `results` - Slice of BenchmarkResult to summarize
/// * `output_dir` - Base output directory (typically "benchmarks/output")
///
/// # Returns
///
/// The path to the written summary file.
///
/// # Example
///
/// ```rust,no_run
/// use llm_orchestrator_benchmarks::benchmarks::io::write_summary;
/// use llm_orchestrator_benchmarks::benchmarks::result::BenchmarkResult;
/// use serde_json::json;
///
/// let results = vec![
///     BenchmarkResult::new("test", json!({"duration_ms": 100})),
/// ];
///
/// let path = write_summary(&results, "benchmarks/output").unwrap();
/// ```
pub fn write_summary(results: &[BenchmarkResult], output_dir: &str) -> Result<String> {
    fs::create_dir_all(output_dir)?;

    let markdown = generate_markdown_report(results);
    let summary_path = Path::new(output_dir).join("summary.md");

    let mut file = File::create(&summary_path)?;
    file.write_all(markdown.as_bytes())?;

    Ok(summary_path.to_string_lossy().to_string())
}

/// Reads benchmark results from the raw output directory.
///
/// Scans the `benchmarks/output/raw/` directory and parses all JSON files
/// as BenchmarkResult objects.
///
/// # Arguments
///
/// * `output_dir` - Base output directory (typically "benchmarks/output")
///
/// # Returns
///
/// A vector of parsed BenchmarkResult objects.
pub fn read_raw_results(output_dir: &str) -> Result<Vec<BenchmarkResult>> {
    let raw_dir = Path::new(output_dir).join("raw");

    if !raw_dir.exists() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();

    for entry in fs::read_dir(&raw_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let content = fs::read_to_string(&path)?;
            if let Ok(result) = serde_json::from_str::<BenchmarkResult>(&content) {
                results.push(result);
            }
        }
    }

    // Sort by timestamp
    results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::env;

    #[test]
    fn test_write_and_read_raw_results() {
        let temp_dir = env::temp_dir().join("benchmark_io_test");
        let output_dir = temp_dir.to_string_lossy().to_string();

        // Clean up any previous test data
        let _ = fs::remove_dir_all(&temp_dir);

        let results = vec![
            BenchmarkResult::new("target_1", json!({"duration_ms": 100.0})),
            BenchmarkResult::new("target_2", json!({"duration_ms": 200.0})),
        ];

        // Write results
        let paths = write_raw_results(&results, &output_dir).expect("Failed to write results");
        assert!(!paths.is_empty());

        // Read results back
        let read_results = read_raw_results(&output_dir).expect("Failed to read results");
        assert_eq!(read_results.len(), 2);

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_write_summary() {
        let temp_dir = env::temp_dir().join("benchmark_summary_test");
        let output_dir = temp_dir.to_string_lossy().to_string();

        // Clean up any previous test data
        let _ = fs::remove_dir_all(&temp_dir);

        let results = vec![BenchmarkResult::new(
            "test_target",
            json!({
                "duration_ms": 150.0,
                "iterations": 100
            }),
        )];

        let path = write_summary(&results, &output_dir).expect("Failed to write summary");
        assert!(Path::new(&path).exists());

        // Verify content
        let content = fs::read_to_string(&path).expect("Failed to read summary");
        assert!(content.contains("test_target"));
        assert!(content.contains("Benchmark Results"));

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
