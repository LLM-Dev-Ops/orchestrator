// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! LLM Orchestrator CLI.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use llm_orchestrator_benchmarks::{
    benchmarks::io::{write_raw_results, write_summary},
    run_all_benchmarks,
};
use llm_orchestrator_core::workflow::Workflow;
use llm_orchestrator_core::{LLMProvider, WorkflowDAG, WorkflowExecutor};
use llm_orchestrator_providers::{AnthropicProvider, OpenAIProvider};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "llm-orchestrator")]
#[command(version, about = "LLM Workflow Orchestrator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a workflow definition
    Validate {
        /// Path to workflow file
        #[arg(value_name = "FILE")]
        file: String,
    },

    /// Run a workflow
    Run {
        /// Path to workflow file
        #[arg(value_name = "FILE")]
        file: String,

        /// Input JSON string or file
        #[arg(short, long)]
        input: Option<String>,

        /// Maximum concurrent steps
        #[arg(long, default_value = "4")]
        max_concurrency: usize,
    },

    /// Run the canonical benchmark suite
    Benchmark {
        /// Output directory for benchmark results
        #[arg(short, long, default_value = "benchmarks/output")]
        output: String,

        /// Output format: json, markdown, or both
        #[arg(short, long, default_value = "both")]
        format: String,

        /// Run benchmarks quietly (no progress output)
        #[arg(short, long)]
        quiet: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize tracing
    let log_level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("llm_orchestrator={}", log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let result = match cli.command {
        Commands::Validate { file } => validate_workflow(&file),
        Commands::Run {
            file,
            input,
            max_concurrency,
        } => run_workflow(&file, input.as_deref(), max_concurrency).await,
        Commands::Benchmark {
            output,
            format,
            quiet,
        } => run_benchmarks(&output, &format, quiet).await,
    };

    if let Err(e) = result {
        error!("{}", e);
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn validate_workflow(file_path: &str) -> Result<()> {
    info!("Validating workflow: {}", file_path);
    println!("{} {}", "Validating workflow:".cyan().bold(), file_path);

    // Read workflow file
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read workflow file: {}", file_path))?;

    // Parse workflow
    let workflow: Workflow = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse workflow YAML: {}", file_path))?;

    info!("Parsed workflow: {} v{}", workflow.name, workflow.version);

    // Validate workflow
    workflow
        .validate()
        .with_context(|| "Workflow validation failed")?;

    // Build DAG to check for cycles
    let _dag = WorkflowDAG::from_workflow(&workflow)
        .with_context(|| "Failed to build workflow DAG (possible cycle detected)")?;

    println!("{}", "✓ Workflow is valid".green().bold());
    println!("  Name: {}", workflow.name);
    println!("  Version: {}", workflow.version);
    println!("  Steps: {}", workflow.steps.len());

    Ok(())
}

async fn run_workflow(
    file_path: &str,
    input: Option<&str>,
    max_concurrency: usize,
) -> Result<()> {
    info!("Running workflow: {}", file_path);
    println!("{} {}", "Running workflow:".cyan().bold(), file_path);

    // Read workflow file
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read workflow file: {}", file_path))?;

    // Parse workflow
    let workflow: Workflow = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse workflow YAML: {}", file_path))?;

    info!("Parsed workflow: {} v{}", workflow.name, workflow.version);

    // Validate workflow
    workflow
        .validate()
        .with_context(|| "Workflow validation failed")?;

    // Parse input
    let inputs = if let Some(input_str) = input {
        parse_input(input_str)?
    } else {
        HashMap::new()
    };

    info!("Workflow inputs: {:?}", inputs);

    // Create providers
    let mut providers: HashMap<String, Arc<dyn LLMProvider>> = HashMap::new();

    // Try to create OpenAI provider from environment
    if let Ok(openai) = OpenAIProvider::from_env() {
        info!("Registered OpenAI provider");
        providers.insert("openai".to_string(), Arc::new(openai));
    } else {
        info!("OpenAI provider not available (OPENAI_API_KEY not set)");
    }

    // Try to create Anthropic provider from environment
    if let Ok(anthropic) = AnthropicProvider::from_env() {
        info!("Registered Anthropic provider");
        providers.insert("anthropic".to_string(), Arc::new(anthropic));
    } else {
        info!("Anthropic provider not available (ANTHROPIC_API_KEY not set)");
    }

    if providers.is_empty() {
        anyhow::bail!(
            "No LLM providers available. Please set OPENAI_API_KEY or ANTHROPIC_API_KEY environment variable."
        );
    }

    // Create executor
    let mut executor = WorkflowExecutor::new(workflow, inputs)
        .with_context(|| "Failed to create workflow executor")?
        .with_max_concurrency(max_concurrency);

    // Register providers
    for (name, provider) in providers {
        executor = executor.with_provider(name, provider);
    }

    println!("{}", "Executing workflow...".cyan());

    // Execute workflow
    let result = executor
        .execute()
        .await
        .with_context(|| "Workflow execution failed")?;

    println!("{}", "✓ Workflow completed successfully".green().bold());
    println!("\n{}", "Results:".cyan().bold());
    println!(
        "{}",
        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|_| format!("{:?}", result))
    );

    Ok(())
}

fn parse_input(input_str: &str) -> Result<HashMap<String, Value>> {
    // Check if input is a file path
    if Path::new(input_str).exists() {
        let content = fs::read_to_string(input_str)
            .with_context(|| format!("Failed to read input file: {}", input_str))?;
        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse input JSON from file: {}", input_str))
    } else {
        // Try to parse as JSON string
        serde_json::from_str(input_str)
            .with_context(|| "Failed to parse input JSON string")
    }
}

/// Runs the canonical benchmark suite.
async fn run_benchmarks(output_dir: &str, format: &str, quiet: bool) -> Result<()> {
    if !quiet {
        println!("{}", "Running LLM Orchestrator Benchmarks...".cyan().bold());
        println!();
    }

    info!("Starting benchmark suite");

    // Run all benchmarks
    let results = run_all_benchmarks().await;

    if !quiet {
        println!(
            "{} {} benchmarks",
            "✓ Completed".green().bold(),
            results.len()
        );
        println!();

        // Print summary
        for result in &results {
            let duration = result
                .metrics
                .get("duration_ms")
                .and_then(|v| v.as_f64())
                .map(|d| format!("{:.2}ms", d))
                .unwrap_or_else(|| "N/A".to_string());

            let ops = result
                .metrics
                .get("ops_per_sec")
                .and_then(|v| v.as_f64())
                .map(|o| format!("{:.0} ops/sec", o))
                .unwrap_or_else(|| "".to_string());

            println!(
                "  {} {} - {} {}",
                "●".green(),
                result.target_id.cyan(),
                duration,
                ops.dimmed()
            );
        }
        println!();
    }

    // Create output directory
    fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir))?;

    // Write output based on format
    let write_json = format == "json" || format == "both";
    let write_md = format == "markdown" || format == "both";

    if write_json {
        let paths = write_raw_results(&results, output_dir)
            .with_context(|| "Failed to write raw benchmark results")?;

        if !quiet {
            println!("{} JSON results written:", "✓".green().bold());
            for path in &paths {
                println!("  {}", path.dimmed());
            }
        }
    }

    if write_md {
        let summary_path = write_summary(&results, output_dir)
            .with_context(|| "Failed to write benchmark summary")?;

        if !quiet {
            println!(
                "{} Summary written: {}",
                "✓".green().bold(),
                summary_path.dimmed()
            );
        }
    }

    if !quiet {
        println!();
        println!("{}", "Benchmark suite completed successfully!".green().bold());
    }

    info!("Benchmark suite completed with {} results", results.len());

    Ok(())
}
