// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Canonical benchmark module containing result types, I/O operations, and markdown generation.

pub mod io;
pub mod markdown;
pub mod result;

// Re-export for convenience
pub use io::{write_raw_results, write_summary};
pub use markdown::generate_markdown_report;
pub use result::BenchmarkResult;
