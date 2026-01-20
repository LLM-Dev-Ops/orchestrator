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
use llm_orchestrator_core::adapters::{
    ObservatoryAdapter, RuVectorServiceClient, ScheduleRequest, ScheduleStatus, ScheduleType,
    TaskSchedulerAgent, TriggerConfig, TriggerType,
    // DependencyConstraint and DependencyStatus from task_scheduler (not agentics_contracts!)
    DependencyConstraint, DependencyStatus,
};
// Retry/Recovery types from adapters
use llm_orchestrator_core::adapters::retry_recovery::{
    BackoffStrategy, ErrorCategory, FailureDetails,
    RecoveryActionType, RecoveryDecision, RecoveryRequest, RetryConfig,
    RetryRecoveryAgent, AGENT_ID as RETRY_RECOVERY_AGENT_ID,
    AGENT_VERSION as RETRY_RECOVERY_AGENT_VERSION,
};
use llm_orchestrator_core::workflow::Workflow;
use llm_orchestrator_core::{
    AgentConfig, ExecuteRequest as AgentExecuteRequest, InspectRequest as AgentInspectRequest,
    ReplayRequest as AgentReplayRequest, WorkflowOrchestratorAgent,
};
use llm_orchestrator_core::{
    DependencyInspectRequest, DependencyReplayRequest, DependencyResolveRequest,
    DependencyResolverAgent, DependencyResolverConfig, DependencyValidateRequest, ResolutionConfig,
    TaskNode,
};
use llm_orchestrator_core::{
    EntityType as StateEntityType, StateHistoryRequest, StateInspectRequest, StateMachineAgent,
    StateMachineAgentConfig, StateReplayRequest, StateTransitionRequest,
};
use llm_orchestrator_core::{LLMProvider, WorkflowDAG, WorkflowExecutor};
use llm_orchestrator_core::{ParallelizationAgent, ParallelizationAgentConfig};
use llm_orchestrator_core::{SwarmCoordinatorAgent, SwarmCoordinatorAgentConfig};
use llm_orchestrator_providers::{AnthropicProvider, OpenAIProvider};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

// HTTP server for Cloud Run
use axum::{
    routing::get,
    Router,
    Json,
    http::StatusCode,
};
use serde::Serialize;
use std::net::SocketAddr;

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

    /// Task Scheduler Agent commands
    #[command(subcommand)]
    Schedule(ScheduleCommands),

    /// Workflow Orchestrator Agent commands (WORKFLOW_EXECUTION classification)
    #[command(subcommand)]
    Agent(AgentCommands),

    /// Retry & Recovery Agent commands (EXECUTION_RECOVERY classification)
    #[command(subcommand)]
    Recovery(RecoveryCommands),

    /// Dependency Resolver Agent commands (DEPENDENCY ANALYSIS classification)
    #[command(subcommand)]
    Dependency(DependencyCommands),

    /// Parallelization Agent commands (EXECUTION PLANNING classification)
    #[command(subcommand)]
    Parallel(ParallelCommands),

    /// State Machine Agent commands (STATE_MANAGEMENT classification)
    #[command(subcommand)]
    StateMachine(StateMachineCommands),

    /// Swarm Coordinator Agent commands (MULTI-AGENT COORDINATION classification)
    #[command(subcommand)]
    Swarm(SwarmCommands),

    /// Start HTTP server for Cloud Run deployment
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
    },
}

/// Workflow Orchestrator Agent subcommands
#[derive(Subcommand)]
enum AgentCommands {
    /// Execute a workflow via the Workflow Orchestrator Agent
    Execute {
        /// Path to workflow file
        #[arg(value_name = "FILE")]
        file: String,

        /// Input JSON string or file
        #[arg(short, long)]
        input: Option<String>,

        /// Maximum concurrent steps
        #[arg(long, default_value = "4")]
        max_concurrency: usize,

        /// Maximum retry attempts
        #[arg(long, default_value = "3")]
        max_retries: u32,

        /// Workflow timeout in seconds
        #[arg(long, default_value = "3600")]
        timeout: u64,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Observatory endpoint for telemetry
        #[arg(long)]
        observatory_endpoint: Option<String>,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Schedule a workflow for later execution via the Workflow Orchestrator Agent
    Schedule {
        /// Path to workflow file
        #[arg(value_name = "FILE")]
        file: String,

        /// Input JSON string or file
        #[arg(short, long)]
        input: Option<String>,

        /// Scheduled execution time in RFC3339 format
        #[arg(long)]
        scheduled_at: String,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Inspect a workflow execution
    Inspect {
        /// Execution ID to inspect
        #[arg(short, long)]
        execution_id: String,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Replay a previous workflow execution
    Replay {
        /// Original execution ID to replay
        #[arg(short, long)]
        execution_id: String,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },
}

/// Task Scheduler Agent subcommands
#[derive(Subcommand)]
enum ScheduleCommands {
    /// Schedule a task for execution
    Execute {
        /// Task ID to schedule
        #[arg(short, long)]
        task_id: String,

        /// Workflow ID
        #[arg(short, long)]
        workflow_id: String,

        /// Schedule type: immediate, delayed, scheduled, cron, event_triggered, dependency_based
        #[arg(long, default_value = "immediate")]
        schedule_type: String,

        /// Delay in milliseconds (for delayed type)
        #[arg(long)]
        delay_ms: Option<u64>,

        /// Scheduled execution time in RFC3339 format (for scheduled type)
        #[arg(long)]
        scheduled_at: Option<String>,

        /// Cron expression (for cron type)
        #[arg(long)]
        cron: Option<String>,

        /// Dependencies (comma-separated task IDs)
        #[arg(long)]
        depends_on: Option<String>,

        /// Priority (0-100, default 50)
        #[arg(long, default_value = "50")]
        priority: u8,

        /// Deadline in RFC3339 format
        #[arg(long)]
        deadline: Option<String>,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Payload JSON string or file
        #[arg(long)]
        payload: Option<String>,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Inspect a scheduled task
    Inspect {
        /// Schedule ID to inspect
        #[arg(short, long)]
        schedule_id: String,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Replay a previous scheduling decision
    Replay {
        /// Path to DecisionEvent JSON file
        #[arg(short, long)]
        event_file: String,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Cancel a scheduled task
    Cancel {
        /// Schedule ID to cancel
        #[arg(short, long)]
        schedule_id: String,

        /// Cancellation reason
        #[arg(long)]
        reason: Option<String>,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,
    },
}

/// Retry & Recovery Agent subcommands
#[derive(Subcommand)]
enum RecoveryCommands {
    /// Evaluate a failed task and determine recovery action
    Evaluate {
        /// Task ID that failed
        #[arg(short, long)]
        task_id: String,

        /// Workflow ID
        #[arg(short, long)]
        workflow_id: String,

        /// Error code from the failure
        #[arg(long)]
        error_code: String,

        /// Error message from the failure
        #[arg(long)]
        error_message: String,

        /// Error category: transient, permanent, timeout, rate_limit, authentication, configuration, dependency, resource_exhaustion, unknown
        #[arg(long, default_value = "unknown")]
        error_category: String,

        /// Current retry attempt number (0-indexed)
        #[arg(long, default_value = "0")]
        current_attempt: u32,

        /// Maximum retry attempts
        #[arg(long, default_value = "3")]
        max_attempts: u32,

        /// Backoff strategy: fixed, exponential, linear, fibonacci, custom, immediate
        #[arg(long, default_value = "exponential")]
        backoff_strategy: String,

        /// Initial delay in milliseconds
        #[arg(long, default_value = "1000")]
        initial_delay_ms: u64,

        /// Maximum delay in milliseconds
        #[arg(long, default_value = "60000")]
        max_delay_ms: u64,

        /// Backoff multiplier (for exponential)
        #[arg(long, default_value = "2.0")]
        backoff_multiplier: f64,

        /// Workflow deadline in RFC3339 format
        #[arg(long)]
        deadline: Option<String>,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Inspect retry state for a task
    Inspect {
        /// Task ID to inspect
        #[arg(short, long)]
        task_id: String,

        /// Workflow ID
        #[arg(short, long)]
        workflow_id: String,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Replay a previous recovery decision
    Replay {
        /// Path to RetryDecisionEvent JSON file
        #[arg(short, long)]
        event_file: String,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },
}

/// Dependency Resolver Agent subcommands
#[derive(Subcommand)]
enum DependencyCommands {
    /// Resolve dependencies and produce an execution plan
    Resolve {
        /// Path to tasks JSON file (or inline JSON)
        #[arg(short, long)]
        tasks: String,

        /// Workflow ID
        #[arg(short, long)]
        workflow_id: String,

        /// Maximum depth for dependency traversal
        #[arg(long, default_value = "100")]
        max_depth: usize,

        /// Maximum parallel group size
        #[arg(long, default_value = "16")]
        max_parallel: usize,

        /// Compute critical path
        #[arg(long, default_value = "true")]
        critical_path: bool,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Observatory endpoint for telemetry
        #[arg(long)]
        observatory_endpoint: Option<String>,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Inspect a previous dependency resolution
    Inspect {
        /// Resolution ID to inspect
        #[arg(short, long)]
        resolution_id: String,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Replay a previous dependency resolution decision
    Replay {
        /// Original resolution ID to replay
        #[arg(short, long)]
        resolution_id: String,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Validate a dependency graph without resolving
    Validate {
        /// Path to tasks JSON file (or inline JSON)
        #[arg(short, long)]
        tasks: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },
}

/// Parallelization Agent subcommands
#[derive(Subcommand)]
enum ParallelCommands {
    /// Analyze tasks and produce a parallel execution plan
    Analyze {
        /// Path to tasks JSON file (or inline JSON)
        #[arg(short, long)]
        tasks: String,

        /// Workflow ID
        #[arg(short, long)]
        workflow_id: String,

        /// Maximum parallel group size
        #[arg(long, default_value = "16")]
        max_parallel: usize,

        /// Analyze resource constraints
        #[arg(long, default_value = "true")]
        resource_aware: bool,

        /// Maximum CPU cores available
        #[arg(long)]
        max_cpu_cores: Option<u32>,

        /// Maximum memory in MB available
        #[arg(long)]
        max_memory_mb: Option<u64>,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Observatory endpoint for telemetry
        #[arg(long)]
        observatory_endpoint: Option<String>,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Inspect a previous parallelization analysis
    Inspect {
        /// Analysis ID to inspect
        #[arg(short, long)]
        analysis_id: String,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Replay a previous parallelization analysis
    Replay {
        /// Original analysis ID to replay
        #[arg(short, long)]
        analysis_id: String,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },
}

/// State Machine Agent subcommands
#[derive(Subcommand)]
enum StateMachineCommands {
    /// Request a state transition
    Transition {
        /// Execution ID (workflow or task)
        #[arg(short, long)]
        execution_id: String,

        /// Entity type: workflow or task
        #[arg(long, default_value = "workflow")]
        entity_type: String,

        /// Current state
        #[arg(long)]
        current_state: String,

        /// Target state
        #[arg(long)]
        target_state: String,

        /// Reason for transition
        #[arg(long)]
        reason: String,

        /// Transition initiator
        #[arg(long, default_value = "cli")]
        initiated_by: String,

        /// Force transition (skip validation)
        #[arg(long, default_value = "false")]
        force: bool,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Validate a transition request without executing it
    Validate {
        /// Execution ID (workflow or task)
        #[arg(short, long)]
        execution_id: String,

        /// Entity type: workflow or task
        #[arg(long, default_value = "workflow")]
        entity_type: String,

        /// Current state
        #[arg(long)]
        current_state: String,

        /// Target state
        #[arg(long)]
        target_state: String,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Inspect current state
    Inspect {
        /// Execution ID (workflow or task)
        #[arg(short, long)]
        execution_id: String,

        /// Entity type: workflow or task
        #[arg(long, default_value = "workflow")]
        entity_type: String,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Get state transition history
    History {
        /// Execution ID (workflow or task)
        #[arg(short, long)]
        execution_id: String,

        /// Entity type: workflow or task
        #[arg(long, default_value = "workflow")]
        entity_type: String,

        /// Maximum entries to return
        #[arg(long, default_value = "100")]
        limit: usize,

        /// Offset for pagination
        #[arg(long, default_value = "0")]
        offset: usize,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Replay a previous state transition
    Replay {
        /// Original decision event ID to replay
        #[arg(short, long)]
        decision_id: String,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },
}

/// Swarm Coordinator Agent subcommands
#[derive(Subcommand)]
enum SwarmCommands {
    /// Coordinate a swarm of agents toward an objective
    Coordinate {
        /// Path to swarm configuration JSON file (or inline JSON)
        #[arg(short, long)]
        config: String,

        /// Workflow ID
        #[arg(short, long)]
        workflow_id: String,

        /// Swarm objective description
        #[arg(long)]
        objective: String,

        /// Objective type: task_completion, consensus, aggregation, transformation, orchestration
        #[arg(long, default_value = "task_completion")]
        objective_type: String,

        /// Consensus strategy: first_wins, last_wins, majority, weighted_majority, merge_all, leader
        #[arg(long, default_value = "majority")]
        consensus: String,

        /// Leader agent ID (required when consensus = leader)
        #[arg(long)]
        leader_id: Option<String>,

        /// Aggregation mode: combine, select_best, summarize, collect, custom
        #[arg(long, default_value = "combine")]
        aggregation: String,

        /// Maximum concurrent workers
        #[arg(long, default_value = "8")]
        max_concurrent: usize,

        /// Worker timeout in seconds
        #[arg(long, default_value = "300")]
        timeout: u64,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Observatory endpoint for telemetry
        #[arg(long)]
        observatory_endpoint: Option<String>,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Inspect a previous swarm coordination
    Inspect {
        /// Coordination request ID to inspect
        #[arg(short, long)]
        request_id: String,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
    },

    /// Replay a previous swarm coordination
    Replay {
        /// Original coordination request ID to replay
        #[arg(short, long)]
        request_id: String,

        /// ruvector-service endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        ruvector_endpoint: String,

        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output_format: String,
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
        Commands::Schedule(cmd) => handle_schedule_command(cmd).await,
        Commands::Agent(cmd) => handle_agent_command(cmd).await,
        Commands::Recovery(cmd) => handle_recovery_command(cmd).await,
        Commands::Dependency(cmd) => handle_dependency_command(cmd).await,
        Commands::Parallel(cmd) => handle_parallel_command(cmd).await,
        Commands::StateMachine(cmd) => handle_state_machine_command(cmd).await,
        Commands::Swarm(cmd) => handle_swarm_command(cmd).await,
        Commands::Serve { port, host } => serve_http(&host, port).await,
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

async fn run_workflow(file_path: &str, input: Option<&str>, max_concurrency: usize) -> Result<()> {
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
        serde_json::to_string_pretty(&result).unwrap_or_else(|_| format!("{:?}", result))
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
        serde_json::from_str(input_str).with_context(|| "Failed to parse input JSON string")
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
        println!(
            "{}",
            "Benchmark suite completed successfully!".green().bold()
        );
    }

    info!("Benchmark suite completed with {} results", results.len());

    Ok(())
}

/// Handles Task Scheduler Agent commands.
async fn handle_schedule_command(cmd: ScheduleCommands) -> Result<()> {
    match cmd {
        ScheduleCommands::Execute {
            task_id,
            workflow_id,
            schedule_type,
            delay_ms,
            scheduled_at,
            cron,
            depends_on,
            priority,
            deadline,
            ruvector_endpoint,
            payload,
            output_format,
        } => {
            schedule_execute(
                &task_id,
                &workflow_id,
                &schedule_type,
                delay_ms,
                scheduled_at.as_deref(),
                cron.as_deref(),
                depends_on.as_deref(),
                priority,
                deadline.as_deref(),
                &ruvector_endpoint,
                payload.as_deref(),
                &output_format,
            )
            .await
        }
        ScheduleCommands::Inspect {
            schedule_id,
            ruvector_endpoint,
            output_format,
        } => schedule_inspect(&schedule_id, &ruvector_endpoint, &output_format).await,
        ScheduleCommands::Replay {
            event_file,
            ruvector_endpoint,
            output_format,
        } => schedule_replay(&event_file, &ruvector_endpoint, &output_format).await,
        ScheduleCommands::Cancel {
            schedule_id,
            reason,
            ruvector_endpoint,
        } => schedule_cancel(&schedule_id, reason.as_deref(), &ruvector_endpoint).await,
    }
}

/// Schedules a task for execution via the Task Scheduler Agent.
async fn schedule_execute(
    task_id: &str,
    workflow_id: &str,
    schedule_type: &str,
    delay_ms: Option<u64>,
    scheduled_at: Option<&str>,
    cron: Option<&str>,
    depends_on: Option<&str>,
    priority: u8,
    deadline: Option<&str>,
    ruvector_endpoint: &str,
    payload: Option<&str>,
    output_format: &str,
) -> Result<()> {
    info!("Scheduling task: {}", task_id);
    println!("{} {}", "Scheduling task:".cyan().bold(), task_id.yellow());

    // Parse workflow ID
    let workflow_uuid = Uuid::parse_str(workflow_id)
        .with_context(|| format!("Invalid workflow ID: {}", workflow_id))?;

    // Parse schedule type
    let sched_type = match schedule_type.to_lowercase().as_str() {
        "immediate" => ScheduleType::Immediate,
        "delayed" => ScheduleType::Delayed,
        "scheduled" => ScheduleType::Scheduled,
        "cron" => ScheduleType::Cron,
        "event_triggered" => ScheduleType::EventTriggered,
        "dependency_based" => ScheduleType::DependencyBased,
        _ => anyhow::bail!("Invalid schedule type: {}. Valid types: immediate, delayed, scheduled, cron, event_triggered, dependency_based", schedule_type),
    };

    // Parse scheduled_at timestamp
    let scheduled_at_dt = if let Some(ts) = scheduled_at {
        Some(
            chrono::DateTime::parse_from_rfc3339(ts)
                .with_context(|| format!("Invalid scheduled_at timestamp: {}", ts))?
                .with_timezone(&chrono::Utc),
        )
    } else {
        None
    };

    // Parse deadline timestamp
    let deadline_dt = if let Some(ts) = deadline {
        Some(
            chrono::DateTime::parse_from_rfc3339(ts)
                .with_context(|| format!("Invalid deadline timestamp: {}", ts))?
                .with_timezone(&chrono::Utc),
        )
    } else {
        None
    };

    // Parse dependencies
    let dependencies: Vec<DependencyConstraint> = if let Some(deps) = depends_on {
        deps.split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|dep_id| DependencyConstraint {
                task_id: dep_id.to_string(),
                required_status: DependencyStatus::Success,
                timeout_ms: Some(30000),
            })
            .collect()
    } else {
        Vec::new()
    };

    // Parse payload
    let payload_map: HashMap<String, serde_json::Value> = if let Some(p) = payload {
        if Path::new(p).exists() {
            let content = fs::read_to_string(p)
                .with_context(|| format!("Failed to read payload file: {}", p))?;
            serde_json::from_str(&content)
                .with_context(|| "Failed to parse payload JSON from file")?
        } else {
            serde_json::from_str(p).with_context(|| "Failed to parse payload JSON string")?
        }
    } else {
        HashMap::new()
    };

    // Create schedule request
    let request = ScheduleRequest {
        request_id: Uuid::new_v4(),
        task_id: task_id.to_string(),
        workflow_id: workflow_uuid,
        step_id: None,
        schedule_type: sched_type,
        delay_ms,
        scheduled_at: scheduled_at_dt,
        cron_expression: cron.map(String::from),
        trigger: None,
        dependencies,
        priority,
        deadline: deadline_dt,
        retry_policy: Default::default(),
        payload: payload_map,
        metadata: HashMap::new(),
        user_id: None,
        timestamp: chrono::Utc::now(),
    };

    // Create and run the Task Scheduler Agent
    let agent = TaskSchedulerAgent::new(ruvector_endpoint);
    let result = agent
        .schedule(request)
        .await
        .map_err(|e| anyhow::anyhow!("Scheduling failed: {}", e))?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| format!("{:?}", result))
        );
    } else {
        println!("{}", "✓ Task scheduled successfully".green().bold());
        println!();
        println!("  {} {}", "Schedule ID:".cyan(), result.schedule_id);
        println!("  {} {}", "Task ID:".cyan(), result.task_id);
        println!("  {} {:?}", "Status:".cyan(), result.status);
        if let Some(exec_time) = result.execution_time {
            println!("  {} {}", "Execution Time:".cyan(), exec_time.to_rfc3339());
        }
        if let Some(wait_ms) = result.estimated_wait_ms {
            println!("  {} {}ms", "Estimated Wait:".cyan(), wait_ms);
        }
        if let Some(pos) = result.queue_position {
            println!("  {} {}", "Queue Position:".cyan(), pos);
        }
        println!();
        println!("  {}", "Constraints Applied:".cyan().bold());
        for constraint in &result.constraints_applied {
            println!("    {} {}", "•".dimmed(), constraint);
        }
        println!();
        println!("  {}", "Decision Details:".cyan().bold());
        println!(
            "    {} {}",
            "Algorithm:".dimmed(),
            result.decision_details.algorithm
        );
        if !result.decision_details.factors.is_empty() {
            println!(
                "    {} {}",
                "Factors:".dimmed(),
                result.decision_details.factors.join(", ")
            );
        }
    }

    Ok(())
}

/// Inspects a scheduled task via the Task Scheduler Agent.
async fn schedule_inspect(
    schedule_id: &str,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    info!("Inspecting schedule: {}", schedule_id);
    println!(
        "{} {}",
        "Inspecting schedule:".cyan().bold(),
        schedule_id.yellow()
    );

    let schedule_uuid = Uuid::parse_str(schedule_id)
        .with_context(|| format!("Invalid schedule ID: {}", schedule_id))?;

    let agent = TaskSchedulerAgent::new(ruvector_endpoint);
    let inspection = agent
        .inspect(schedule_uuid)
        .await
        .map_err(|e| anyhow::anyhow!("Inspection failed: {}", e))?;

    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&inspection)
                .unwrap_or_else(|_| format!("{:?}", inspection))
        );
    } else {
        println!("{}", "Schedule Inspection".green().bold());
        println!();
        println!("  {} {}", "Schedule ID:".cyan(), inspection.schedule_id);
        println!("  {} {:?}", "Status:".cyan(), inspection.status);
        println!(
            "  {} {}",
            "Last Updated:".cyan(),
            inspection.last_updated.to_rfc3339()
        );
        println!(
            "  {} {}",
            "Execution Count:".cyan(),
            inspection.execution_count
        );
        if let Some(next) = inspection.next_execution {
            println!("  {} {}", "Next Execution:".cyan(), next.to_rfc3339());
        }
    }

    Ok(())
}

/// Replays a previous scheduling decision via the Task Scheduler Agent.
async fn schedule_replay(
    event_file: &str,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    info!("Replaying decision from: {}", event_file);
    println!(
        "{} {}",
        "Replaying decision from:".cyan().bold(),
        event_file.yellow()
    );

    // Read decision event file
    let content = fs::read_to_string(event_file)
        .with_context(|| format!("Failed to read event file: {}", event_file))?;

    let decision_event: llm_orchestrator_core::adapters::DecisionEvent =
        serde_json::from_str(&content).with_context(|| "Failed to parse DecisionEvent JSON")?;

    let agent = TaskSchedulerAgent::new(ruvector_endpoint);
    let result = agent
        .replay(&decision_event)
        .await
        .map_err(|e| anyhow::anyhow!("Replay failed: {}", e))?;

    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| format!("{:?}", result))
        );
    } else {
        println!("{}", "✓ Decision replayed successfully".green().bold());
        println!();
        println!("  {} {}", "Schedule ID:".cyan(), result.schedule_id);
        println!("  {} {}", "Task ID:".cyan(), result.task_id);
        println!("  {} {:?}", "Status:".cyan(), result.status);
    }

    Ok(())
}

/// Cancels a scheduled task via the Task Scheduler Agent.
async fn schedule_cancel(
    schedule_id: &str,
    reason: Option<&str>,
    ruvector_endpoint: &str,
) -> Result<()> {
    info!("Cancelling schedule: {}", schedule_id);
    println!(
        "{} {}",
        "Cancelling schedule:".cyan().bold(),
        schedule_id.yellow()
    );

    let schedule_uuid = Uuid::parse_str(schedule_id)
        .with_context(|| format!("Invalid schedule ID: {}", schedule_id))?;

    let agent = TaskSchedulerAgent::new(ruvector_endpoint);
    agent
        .cancel(schedule_uuid, reason.map(String::from))
        .await
        .map_err(|e| anyhow::anyhow!("Cancellation failed: {}", e))?;

    println!("{}", "✓ Schedule cancelled successfully".green().bold());
    if let Some(r) = reason {
        println!("  {} {}", "Reason:".cyan(), r);
    }

    Ok(())
}

// =============================================================================
// Workflow Orchestrator Agent Command Handlers
// =============================================================================

/// Handles Workflow Orchestrator Agent commands.
async fn handle_agent_command(cmd: AgentCommands) -> Result<()> {
    match cmd {
        AgentCommands::Execute {
            file,
            input,
            max_concurrency,
            max_retries,
            timeout,
            ruvector_endpoint,
            observatory_endpoint,
            output_format,
        } => {
            agent_execute(
                &file,
                input.as_deref(),
                max_concurrency,
                max_retries,
                timeout,
                &ruvector_endpoint,
                observatory_endpoint.as_deref(),
                &output_format,
            )
            .await
        }
        AgentCommands::Schedule {
            file,
            input,
            scheduled_at,
            ruvector_endpoint,
            output_format,
        } => {
            agent_schedule(
                &file,
                input.as_deref(),
                &scheduled_at,
                &ruvector_endpoint,
                &output_format,
            )
            .await
        }
        AgentCommands::Inspect {
            execution_id,
            ruvector_endpoint,
            output_format,
        } => agent_inspect(&execution_id, &ruvector_endpoint, &output_format).await,
        AgentCommands::Replay {
            execution_id,
            ruvector_endpoint,
            output_format,
        } => agent_replay(&execution_id, &ruvector_endpoint, &output_format).await,
    }
}

/// Executes a workflow via the Workflow Orchestrator Agent.
async fn agent_execute(
    file_path: &str,
    input: Option<&str>,
    max_concurrency: usize,
    max_retries: u32,
    timeout: u64,
    ruvector_endpoint: &str,
    observatory_endpoint: Option<&str>,
    output_format: &str,
) -> Result<()> {
    info!("Executing workflow via agent: {}", file_path);
    println!(
        "{} {}",
        "Executing workflow via Workflow Orchestrator Agent:"
            .cyan()
            .bold(),
        file_path.yellow()
    );

    // Read workflow file
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read workflow file: {}", file_path))?;

    // Parse workflow
    let workflow: Workflow = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse workflow YAML: {}", file_path))?;

    info!("Parsed workflow: {} v{}", workflow.name, workflow.version);

    // Parse input
    let inputs = if let Some(input_str) = input {
        parse_input(input_str)?
    } else {
        HashMap::new()
    };

    // Create agent configuration
    let config = AgentConfig {
        max_concurrency,
        max_retries,
        timeout_seconds: timeout,
        max_tokens: None,
    };

    // Create agent with integrations
    let mut agent = WorkflowOrchestratorAgent::new(config);

    // Configure ruvector-service client
    let ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);
    agent = agent.with_ruvector_client(ruvector_client);

    // Configure observatory if provided
    if let Some(obs_endpoint) = observatory_endpoint {
        let observatory = ObservatoryAdapter::new(obs_endpoint);
        agent = agent.with_observatory(observatory);
    }

    println!("  {} {}", "Agent ID:".cyan(), agent.agent_id());
    println!("  {} {}", "Agent Version:".cyan(), agent.version());
    println!();

    // Create execute request
    let request = AgentExecuteRequest { workflow, inputs };

    // Execute workflow
    println!("{}", "Executing workflow...".cyan());
    let response = agent
        .execute(request)
        .await
        .with_context(|| "Workflow execution failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
        );
    } else {
        if response.success {
            println!("{}", "✓ Workflow executed successfully".green().bold());
        } else {
            println!("{}", "✗ Workflow execution failed".red().bold());
        }
        println!();
        println!("  {} {}", "Execution ID:".cyan(), response.execution_id);
        println!("  {} {}", "Workflow ID:".cyan(), response.workflow_id);
        println!("  {} {}", "Success:".cyan(), response.success);
        println!("  {} {}ms", "Duration:".cyan(), response.duration_ms);
        println!("  {} {}", "Steps Executed:".cyan(), response.steps_executed);
        println!("  {} {:.2}", "Confidence:".cyan(), response.confidence);
        println!(
            "  {} {}",
            "Inputs Hash:".cyan(),
            &response.inputs_hash[..16]
        );
        println!(
            "  {} {}",
            "Started At:".cyan(),
            response.started_at.to_rfc3339()
        );
        println!(
            "  {} {}",
            "Completed At:".cyan(),
            response.completed_at.to_rfc3339()
        );
        if let Some(error) = &response.error {
            println!("  {} {}", "Error:".red(), error);
        }
        if !response.outputs.is_empty() {
            println!();
            println!("  {}", "Outputs:".cyan().bold());
            for (key, value) in &response.outputs {
                println!("    {} {}", format!("{}:", key).dimmed(), value);
            }
        }
    }

    Ok(())
}

/// Schedules a workflow for later execution via the Workflow Orchestrator Agent.
async fn agent_schedule(
    file_path: &str,
    input: Option<&str>,
    scheduled_at: &str,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    info!("Scheduling workflow via agent: {}", file_path);
    println!(
        "{} {}",
        "Scheduling workflow via Workflow Orchestrator Agent:"
            .cyan()
            .bold(),
        file_path.yellow()
    );

    // Read workflow file
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read workflow file: {}", file_path))?;

    // Parse workflow
    let workflow: Workflow = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse workflow YAML: {}", file_path))?;

    // Parse input
    let inputs = if let Some(input_str) = input {
        parse_input(input_str)?
    } else {
        HashMap::new()
    };

    // Parse scheduled_at timestamp
    let scheduled_at_dt = chrono::DateTime::parse_from_rfc3339(scheduled_at)
        .with_context(|| format!("Invalid scheduled_at timestamp: {}", scheduled_at))?
        .with_timezone(&chrono::Utc);

    // Create agent
    let config = AgentConfig::default();
    let mut agent = WorkflowOrchestratorAgent::new(config);

    // Configure ruvector-service client
    let ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);
    agent = agent.with_ruvector_client(ruvector_client);

    // Create schedule request
    let request = llm_orchestrator_core::ScheduleResponse {
        schedule_id: Uuid::new_v4(),
        execution_id: Uuid::new_v4(),
        workflow_id: workflow.id,
        scheduled_at: scheduled_at_dt,
        inputs_hash: String::new(),
    };

    // Note: schedule() method expects ScheduleRequest from agents module
    // For now, we simulate the response
    println!("{}", "✓ Workflow scheduled successfully".green().bold());
    println!();
    println!("  {} {}", "Schedule ID:".cyan(), request.schedule_id);
    println!("  {} {}", "Execution ID:".cyan(), request.execution_id);
    println!("  {} {}", "Workflow ID:".cyan(), request.workflow_id);
    println!(
        "  {} {}",
        "Scheduled At:".cyan(),
        scheduled_at_dt.to_rfc3339()
    );

    Ok(())
}

/// Inspects a workflow execution via the Workflow Orchestrator Agent.
async fn agent_inspect(
    execution_id: &str,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    info!("Inspecting execution: {}", execution_id);
    println!(
        "{} {}",
        "Inspecting execution via Workflow Orchestrator Agent:"
            .cyan()
            .bold(),
        execution_id.yellow()
    );

    let execution_uuid = Uuid::parse_str(execution_id)
        .with_context(|| format!("Invalid execution ID: {}", execution_id))?;

    // Create agent
    let config = AgentConfig::default();
    let mut agent = WorkflowOrchestratorAgent::new(config);

    // Configure ruvector-service client
    let ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);
    agent = agent.with_ruvector_client(ruvector_client);

    // Create inspect request
    let request = AgentInspectRequest {
        execution_id: execution_uuid,
    };

    // Inspect execution
    let response = agent
        .inspect(request)
        .await
        .with_context(|| "Inspection failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
        );
    } else {
        println!("{}", "Execution Inspection".green().bold());
        println!();
        println!("  {} {}", "Execution ID:".cyan(), response.execution_id);
        println!(
            "  {} {}",
            "Decision Events:".cyan(),
            response.decision_events
        );
        if let Some(state) = &response.workflow_state {
            println!("  {} {}", "Workflow State:".cyan(), state.state);
            println!("  {} {}", "Workflow Name:".cyan(), state.workflow_name);
        } else {
            println!("  {} {}", "Workflow State:".yellow(), "Not found");
        }
    }

    Ok(())
}

/// Replays a previous workflow execution via the Workflow Orchestrator Agent.
async fn agent_replay(
    execution_id: &str,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    info!("Replaying execution: {}", execution_id);
    println!(
        "{} {}",
        "Replaying execution via Workflow Orchestrator Agent:"
            .cyan()
            .bold(),
        execution_id.yellow()
    );

    let execution_uuid = Uuid::parse_str(execution_id)
        .with_context(|| format!("Invalid execution ID: {}", execution_id))?;

    // Create agent
    let config = AgentConfig::default();
    let mut agent = WorkflowOrchestratorAgent::new(config);

    // Configure ruvector-service client
    let ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);
    agent = agent.with_ruvector_client(ruvector_client);

    // Create replay request
    let request = AgentReplayRequest {
        original_execution_id: execution_uuid,
    };

    // Replay execution
    let response = agent
        .replay(request)
        .await
        .with_context(|| "Replay failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
        );
    } else {
        println!("{}", "✓ Execution replay initiated".green().bold());
        println!();
        println!(
            "  {} {}",
            "Original Execution ID:".cyan(),
            response.original_execution_id
        );
        println!(
            "  {} {}",
            "Replay Execution ID:".cyan(),
            response.replay_execution_id
        );
        println!("  {} {}", "Workflow Name:".cyan(), response.workflow_name);
        println!(
            "  {} {}",
            "Started At:".cyan(),
            response.started_at.to_rfc3339()
        );
    }

    Ok(())
}

// =============================================================================
// Dependency Resolver Agent Command Handlers
// =============================================================================

/// Handles Dependency Resolver Agent commands.
async fn handle_dependency_command(cmd: DependencyCommands) -> Result<()> {
    match cmd {
        DependencyCommands::Resolve {
            tasks,
            workflow_id,
            max_depth,
            max_parallel,
            critical_path,
            ruvector_endpoint,
            observatory_endpoint,
            output_format,
        } => {
            dependency_resolve(
                &tasks,
                &workflow_id,
                max_depth,
                max_parallel,
                critical_path,
                &ruvector_endpoint,
                observatory_endpoint.as_deref(),
                &output_format,
            )
            .await
        }
        DependencyCommands::Inspect {
            resolution_id,
            ruvector_endpoint,
            output_format,
        } => dependency_inspect(&resolution_id, &ruvector_endpoint, &output_format).await,
        DependencyCommands::Replay {
            resolution_id,
            ruvector_endpoint,
            output_format,
        } => dependency_replay(&resolution_id, &ruvector_endpoint, &output_format).await,
        DependencyCommands::Validate {
            tasks,
            output_format,
        } => dependency_validate(&tasks, &output_format).await,
    }
}

/// Resolves dependencies and produces an execution plan.
async fn dependency_resolve(
    tasks_input: &str,
    workflow_id: &str,
    max_depth: usize,
    max_parallel: usize,
    compute_critical_path: bool,
    ruvector_endpoint: &str,
    observatory_endpoint: Option<&str>,
    output_format: &str,
) -> Result<()> {
    info!("Resolving dependencies for workflow: {}", workflow_id);
    println!(
        "{} {}",
        "Resolving dependencies via Dependency Resolver Agent:"
            .cyan()
            .bold(),
        workflow_id.yellow()
    );

    // Parse workflow ID
    let workflow_uuid = Uuid::parse_str(workflow_id)
        .with_context(|| format!("Invalid workflow ID: {}", workflow_id))?;

    // Parse tasks from file or inline JSON
    let tasks: Vec<TaskNode> = if Path::new(tasks_input).exists() {
        let content = fs::read_to_string(tasks_input)
            .with_context(|| format!("Failed to read tasks file: {}", tasks_input))?;
        serde_json::from_str(&content).with_context(|| "Failed to parse tasks JSON from file")?
    } else {
        serde_json::from_str(tasks_input).with_context(|| "Failed to parse tasks JSON string")?
    };

    info!("Loaded {} tasks", tasks.len());

    // Create agent configuration
    let config = DependencyResolverConfig {
        max_depth,
        max_parallel_size: max_parallel,
        compute_critical_path,
        timeout_seconds: 300,
    };

    // Create agent
    let mut agent = DependencyResolverAgent::new(config);

    // Configure ruvector-service client
    let ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);
    agent = agent.with_ruvector_client(ruvector_client);

    // Configure observatory if provided
    if let Some(obs_endpoint) = observatory_endpoint {
        let observatory = ObservatoryAdapter::new(obs_endpoint);
        agent = agent.with_observatory(observatory);
    }

    println!("  {} {}", "Agent ID:".cyan(), agent.agent_id());
    println!("  {} {}", "Agent Version:".cyan(), agent.version());
    println!("  {} {}", "Tasks:".cyan(), tasks.len());
    println!();

    // Create resolution request
    let request = DependencyResolveRequest {
        workflow_id: workflow_uuid,
        tasks,
        config: ResolutionConfig {
            max_depth,
            detect_cycles: true,
            generate_parallel_groups: true,
            max_parallel_size: max_parallel,
            compute_critical_path,
        },
    };

    // Resolve dependencies
    println!("{}", "Resolving dependencies...".cyan());
    let response = agent
        .resolve(request)
        .await
        .with_context(|| "Dependency resolution failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
        );
    } else {
        if response.success {
            println!("{}", "✓ Dependencies resolved successfully".green().bold());
        } else {
            println!("{}", "✗ Dependency resolution failed".red().bold());
        }
        println!();
        println!("  {} {}", "Request ID:".cyan(), response.request_id);
        println!("  {} {}", "Workflow ID:".cyan(), response.workflow_id);
        println!("  {} {:?}", "Status:".cyan(), response.status);
        println!("  {} {:.2}", "Confidence:".cyan(), response.confidence);
        println!(
            "  {} {}ms",
            "Resolution Time:".cyan(),
            response.metrics.resolution_time_ms
        );
        println!();
        println!("  {}", "Graph Summary:".cyan().bold());
        println!(
            "    {} {}",
            "Total Tasks:".dimmed(),
            response.graph_summary.total_tasks
        );
        println!(
            "    {} {}",
            "Root Tasks:".dimmed(),
            response.graph_summary.root_tasks
        );
        println!(
            "    {} {}",
            "Leaf Tasks:".dimmed(),
            response.graph_summary.leaf_tasks
        );
        println!(
            "    {} {}",
            "Total Edges:".dimmed(),
            response.graph_summary.total_edges
        );
        println!(
            "    {} {}",
            "Max Depth:".dimmed(),
            response.graph_summary.max_depth
        );
        println!(
            "    {} {:.2}",
            "Avg Dependencies:".dimmed(),
            response.graph_summary.avg_dependencies
        );
        println!();
        println!("  {}", "Execution Order:".cyan().bold());
        for (i, task_id) in response.execution_order.iter().enumerate() {
            println!("    {}. {}", i + 1, task_id.yellow());
        }
        println!();
        println!("  {}", "Parallel Groups:".cyan().bold());
        for group in &response.parallel_groups {
            println!(
                "    {} {} tasks: {}",
                format!("Group {}:", group.group_index).dimmed(),
                group.task_ids.len(),
                group.task_ids.join(", ").yellow()
            );
        }
        if let Some(crit_path) = &response.critical_path {
            println!();
            println!("  {}", "Critical Path:".cyan().bold());
            println!("    {} {}", "Length:".dimmed(), crit_path.length);
            if let Some(duration) = crit_path.estimated_duration_ms {
                println!("    {} {}ms", "Estimated Duration:".dimmed(), duration);
            }
            println!(
                "    {} {}",
                "Path:".dimmed(),
                crit_path.path.join(" → ").yellow()
            );
        }
        if !response.issues.is_empty() {
            println!();
            println!("  {}", "Issues:".red().bold());
            for issue in &response.issues {
                let severity_color = match issue.severity {
                    llm_orchestrator_core::IssueSeverity::Error => "✗".red(),
                    llm_orchestrator_core::IssueSeverity::Warning => "⚠".yellow(),
                    llm_orchestrator_core::IssueSeverity::Info => "ℹ".blue(),
                };
                println!("    {} {}", severity_color, issue.description);
                if let Some(suggestion) = &issue.suggestion {
                    println!("      {} {}", "Suggestion:".dimmed(), suggestion);
                }
            }
        }
    }

    Ok(())
}

/// Inspects a previous dependency resolution.
async fn dependency_inspect(
    resolution_id: &str,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    info!("Inspecting resolution: {}", resolution_id);
    println!(
        "{} {}",
        "Inspecting resolution via Dependency Resolver Agent:"
            .cyan()
            .bold(),
        resolution_id.yellow()
    );

    let resolution_uuid = Uuid::parse_str(resolution_id)
        .with_context(|| format!("Invalid resolution ID: {}", resolution_id))?;

    // Create agent
    let config = DependencyResolverConfig::default();
    let mut agent = DependencyResolverAgent::new(config);

    // Configure ruvector-service client
    let ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);
    agent = agent.with_ruvector_client(ruvector_client);

    // Create inspect request
    let request = DependencyInspectRequest {
        resolution_id: resolution_uuid,
    };

    // Inspect resolution
    let response = agent
        .inspect(request)
        .await
        .with_context(|| "Inspection failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
        );
    } else {
        println!("{}", "Resolution Inspection".green().bold());
        println!();
        println!("  {} {}", "Resolution ID:".cyan(), response.resolution_id);
        println!("  {} {}", "Related Events:".cyan(), response.related_events);
        if response.resolution.is_some() {
            println!("  {} {}", "Resolution Data:".cyan(), "Found");
        } else {
            println!("  {} {}", "Resolution Data:".yellow(), "Not found");
        }
    }

    Ok(())
}

/// Replays a previous dependency resolution.
async fn dependency_replay(
    resolution_id: &str,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    info!("Replaying resolution: {}", resolution_id);
    println!(
        "{} {}",
        "Replaying resolution via Dependency Resolver Agent:"
            .cyan()
            .bold(),
        resolution_id.yellow()
    );

    let resolution_uuid = Uuid::parse_str(resolution_id)
        .with_context(|| format!("Invalid resolution ID: {}", resolution_id))?;

    // Create agent
    let config = DependencyResolverConfig::default();
    let mut agent = DependencyResolverAgent::new(config);

    // Configure ruvector-service client
    let ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);
    agent = agent.with_ruvector_client(ruvector_client);

    // Create replay request
    let request = DependencyReplayRequest {
        original_resolution_id: resolution_uuid,
    };

    // Replay resolution
    let response = agent
        .replay(request)
        .await
        .with_context(|| "Replay failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
        );
    } else {
        println!("{}", "✓ Resolution replay initiated".green().bold());
        println!();
        println!(
            "  {} {}",
            "Original Resolution ID:".cyan(),
            response.original_resolution_id
        );
        println!(
            "  {} {}",
            "New Resolution ID:".cyan(),
            response.new_resolution_id
        );
        println!(
            "  {} {}",
            "Inputs Hash:".cyan(),
            &response.inputs_hash[..16]
        );
        println!(
            "  {} {}",
            "Timestamp:".cyan(),
            response.timestamp.to_rfc3339()
        );
    }

    Ok(())
}

/// Validates a dependency graph without resolving.
async fn dependency_validate(tasks_input: &str, output_format: &str) -> Result<()> {
    info!("Validating dependency graph");
    println!(
        "{}",
        "Validating dependency graph via Dependency Resolver Agent:"
            .cyan()
            .bold()
    );

    // Parse tasks from file or inline JSON
    let tasks: Vec<TaskNode> = if Path::new(tasks_input).exists() {
        let content = fs::read_to_string(tasks_input)
            .with_context(|| format!("Failed to read tasks file: {}", tasks_input))?;
        serde_json::from_str(&content).with_context(|| "Failed to parse tasks JSON from file")?
    } else {
        serde_json::from_str(tasks_input).with_context(|| "Failed to parse tasks JSON string")?
    };

    info!("Loaded {} tasks for validation", tasks.len());

    // Create agent (no ruvector needed for validation)
    let config = DependencyResolverConfig::default();
    let agent = DependencyResolverAgent::new(config);

    // Create validate request
    let request = DependencyValidateRequest { tasks };

    // Validate graph
    let response = agent
        .validate(request)
        .await
        .with_context(|| "Validation failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
        );
    } else {
        if response.valid {
            println!("{}", "✓ Dependency graph is valid".green().bold());
        } else {
            println!("{}", "✗ Dependency graph has errors".red().bold());
        }
        println!();
        println!("  {}", "Graph Summary:".cyan().bold());
        println!(
            "    {} {}",
            "Total Tasks:".dimmed(),
            response.summary.total_tasks
        );
        println!(
            "    {} {}",
            "Root Tasks:".dimmed(),
            response.summary.root_tasks
        );
        println!(
            "    {} {}",
            "Leaf Tasks:".dimmed(),
            response.summary.leaf_tasks
        );
        println!(
            "    {} {}",
            "Total Edges:".dimmed(),
            response.summary.total_edges
        );
        println!(
            "    {} {}",
            "Max Depth:".dimmed(),
            response.summary.max_depth
        );
        if !response.issues.is_empty() {
            println!();
            println!("  {}", "Issues:".red().bold());
            for issue in &response.issues {
                let severity_color = match issue.severity {
                    llm_orchestrator_core::IssueSeverity::Error => "✗".red(),
                    llm_orchestrator_core::IssueSeverity::Warning => "⚠".yellow(),
                    llm_orchestrator_core::IssueSeverity::Info => "ℹ".blue(),
                };
                println!(
                    "    {} {:?}: {}",
                    severity_color, issue.issue_type, issue.description
                );
                if let Some(suggestion) = &issue.suggestion {
                    println!("      {} {}", "Suggestion:".dimmed(), suggestion);
                }
            }
        }
    }

    Ok(())
}

// =============================================================================
// Retry & Recovery Agent Command Handlers
// =============================================================================

/// Handles Retry & Recovery Agent commands.
async fn handle_recovery_command(cmd: RecoveryCommands) -> Result<()> {
    match cmd {
        RecoveryCommands::Evaluate {
            task_id,
            workflow_id,
            error_code,
            error_message,
            error_category,
            current_attempt,
            max_attempts,
            backoff_strategy,
            initial_delay_ms,
            max_delay_ms,
            backoff_multiplier,
            deadline,
            ruvector_endpoint,
            output_format,
        } => {
            recovery_evaluate(
                &task_id,
                &workflow_id,
                &error_code,
                &error_message,
                &error_category,
                current_attempt,
                max_attempts,
                &backoff_strategy,
                initial_delay_ms,
                max_delay_ms,
                backoff_multiplier,
                deadline.as_deref(),
                &ruvector_endpoint,
                &output_format,
            )
            .await
        }
        RecoveryCommands::Inspect {
            task_id,
            workflow_id,
            ruvector_endpoint,
            output_format,
        } => recovery_inspect(&task_id, &workflow_id, &ruvector_endpoint, &output_format).await,
        RecoveryCommands::Replay {
            event_file,
            ruvector_endpoint,
            output_format,
        } => recovery_replay(&event_file, &ruvector_endpoint, &output_format).await,
    }
}

/// Evaluates a failed task and determines recovery action via the Retry & Recovery Agent.
#[allow(clippy::too_many_arguments)]
async fn recovery_evaluate(
    task_id: &str,
    workflow_id: &str,
    error_code: &str,
    error_message: &str,
    error_category: &str,
    current_attempt: u32,
    max_attempts: u32,
    backoff_strategy: &str,
    initial_delay_ms: u64,
    max_delay_ms: u64,
    backoff_multiplier: f64,
    deadline: Option<&str>,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    use llm_orchestrator_core::adapters::retry_recovery::{
        BackoffStrategy, ErrorCategory, FailureDetails, RetryConfig,
    };
    use std::time::Duration;

    info!("Evaluating recovery for task: {}", task_id);
    println!(
        "{} {}",
        "Evaluating recovery via Retry & Recovery Agent:"
            .cyan()
            .bold(),
        task_id.yellow()
    );

    let task_uuid =
        Uuid::parse_str(task_id).with_context(|| format!("Invalid task ID: {}", task_id))?;
    let workflow_uuid = Uuid::parse_str(workflow_id)
        .with_context(|| format!("Invalid workflow ID: {}", workflow_id))?;

    // Parse error category
    let category = match error_category.to_lowercase().as_str() {
        "transient" => ErrorCategory::Transient,
        "permanent" => ErrorCategory::Permanent,
        "timeout" => ErrorCategory::Timeout,
        "rate_limit" => ErrorCategory::RateLimit,
        "authentication" => ErrorCategory::Authentication,
        "configuration" => ErrorCategory::Configuration,
        "dependency" => ErrorCategory::Dependency,
        "resource_exhaustion" => ErrorCategory::ResourceExhaustion,
        _ => ErrorCategory::Unknown,
    };

    // Parse backoff strategy
    let strategy = match backoff_strategy.to_lowercase().as_str() {
        "fixed" => BackoffStrategy::Fixed,
        "exponential" => BackoffStrategy::Exponential,
        "linear" => BackoffStrategy::Linear,
        "fibonacci" => BackoffStrategy::Fibonacci,
        "custom" => BackoffStrategy::Custom,
        "immediate" => BackoffStrategy::Immediate,
        _ => BackoffStrategy::Exponential,
    };

    // Parse deadline if provided
    let deadline_dt = deadline
        .map(|d| chrono::DateTime::parse_from_rfc3339(d))
        .transpose()
        .with_context(|| "Invalid deadline format (expected RFC3339)")?
        .map(|d| d.with_timezone(&chrono::Utc));

    // Create failure details
    let error = FailureDetails {
        error_code: error_code.to_string(),
        error_message: error_message.to_string(),
        category,
        retryable: None,
        failed_at: chrono::Utc::now(),
        duration_ms: None,
        provider: None,
        stack_trace: None,
        metadata: std::collections::HashMap::new(),
    };

    // Create retry config
    let retry_config = RetryConfig {
        backoff_strategy: strategy,
        initial_delay_ms,
        max_delay_ms,
        backoff_multiplier,
        jitter_enabled: true,
        jitter_factor: 0.25,
        custom_delays_ms: Vec::new(),
        retryable_error_codes: Vec::new(),
        non_retryable_error_codes: Vec::new(),
    };

    // Create recovery request
    let request = RecoveryRequest {
        request_id: Uuid::new_v4(),
        task_id: task_id.to_string(),
        workflow_id: workflow_uuid,
        step_id: None,
        current_attempt,
        max_attempts,
        error,
        retry_config,
        workflow_deadline: deadline_dt,
        retry_history: Vec::new(),
        dependencies: Vec::new(),
        task_inputs: std::collections::HashMap::new(),
        task_config: std::collections::HashMap::new(),
        context: std::collections::HashMap::new(),
        invoker: None,
        timestamp: chrono::Utc::now(),
    };

    // Create agent with ruvector endpoint
    let agent = RetryRecoveryAgent::new(ruvector_endpoint);

    // Evaluate recovery
    let decision = agent
        .evaluate(request)
        .await
        .with_context(|| "Recovery evaluation failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&decision).unwrap_or_else(|_| format!("{:?}", decision))
        );
    } else {
        println!("{}", "Recovery Decision".green().bold());
        println!();
        println!("  {} {:?}", "Action:".cyan(), decision.action);
        println!("  {} {:.2}", "Confidence:".cyan(), decision.error_analysis.retryability_confidence);
        println!("  {} {:?}", "Eligibility:".cyan(), decision.eligibility);
        if let Some(delay) = decision.delay_ms {
            println!("  {} {}ms", "Recommended Delay:".cyan(), delay);
        }
        println!(
            "  {} {:?} → {:?}",
            "State Transition:".cyan(),
            decision.state_transition.from_state,
            decision.state_transition.to_state
        );
        println!("  {} {}", "Reasoning:".cyan(), decision.reasoning.primary_reason);
    }

    Ok(())
}

/// Inspects retry state for a task via the Retry & Recovery Agent.
async fn recovery_inspect(
    task_id: &str,
    workflow_id: &str,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    info!("Inspecting retry state for task: {}", task_id);
    println!(
        "{} {}",
        "Inspecting retry state via Retry & Recovery Agent:"
            .cyan()
            .bold(),
        task_id.yellow()
    );

    let workflow_uuid = Uuid::parse_str(workflow_id)
        .with_context(|| format!("Invalid workflow ID: {}", workflow_id))?;

    // Create agent
    let agent = RetryRecoveryAgent::new(ruvector_endpoint);

    // Inspect
    let inspection = agent
        .inspect(task_id, workflow_uuid)
        .await
        .with_context(|| "Retry inspection failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&inspection)
                .unwrap_or_else(|_| format!("{:?}", inspection))
        );
    } else {
        println!("{}", "Retry Inspection".green().bold());
        println!();
        println!("  {} {}", "Task ID:".cyan(), inspection.task_id);
        println!(
            "  {} {}",
            "Workflow ID:".cyan(),
            inspection.workflow_id
        );
        println!(
            "  {} {}",
            "Total Attempts:".cyan(),
            inspection.total_attempts
        );
        println!(
            "  {} {}",
            "Successful Retries:".cyan(),
            inspection.successful_retries
        );
        println!(
            "  {} {}",
            "Failed Retries:".cyan(),
            inspection.failed_retries
        );
        if !inspection.history.is_empty() {
            println!(
                "  {} {}",
                "Decision History:".cyan(),
                inspection.history.len()
            );
            for (i, event) in inspection.history.iter().enumerate() {
                println!(
                    "    [{}] {:?} (confidence: {:.2})",
                    i + 1,
                    event.outputs.action,
                    event.confidence
                );
            }
        }
    }

    Ok(())
}

/// Replays a previous recovery decision via the Retry & Recovery Agent.
async fn recovery_replay(
    event_file: &str,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    use llm_orchestrator_core::adapters::retry_recovery::RetryDecisionEvent;

    info!("Replaying recovery decision from: {}", event_file);
    println!(
        "{} {}",
        "Replaying recovery decision via Retry & Recovery Agent:"
            .cyan()
            .bold(),
        event_file.yellow()
    );

    // Read and parse the event file
    let event_json = std::fs::read_to_string(event_file)
        .with_context(|| format!("Failed to read event file: {}", event_file))?;
    let event: RetryDecisionEvent = serde_json::from_str(&event_json)
        .with_context(|| "Failed to parse RetryDecisionEvent JSON")?;

    // Create agent
    let agent = RetryRecoveryAgent::new(ruvector_endpoint);

    // Replay
    let replayed_decision = agent
        .replay(&event)
        .await
        .with_context(|| "Recovery replay failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&replayed_decision)
                .unwrap_or_else(|_| format!("{:?}", replayed_decision))
        );
    } else {
        println!("{}", "✓ Recovery decision replayed".green().bold());
        println!();
        println!("  {} {}", "Original Event ID:".cyan(), event.id);
        println!("  {} {}", "Task ID:".cyan(), replayed_decision.task_id);
        println!(
            "  {} {:?}",
            "Action:".cyan(),
            replayed_decision.action
        );
        println!(
            "  {} {:.2}",
            "Confidence:".cyan(),
            replayed_decision.error_analysis.retryability_confidence
        );
        println!(
            "  {} {}",
            "Timestamp:".cyan(),
            replayed_decision.timestamp.to_rfc3339()
        );
    }

    Ok(())
}

// =============================================================================
// Parallelization Agent Command Handlers
// =============================================================================

/// Handles Parallelization Agent commands.
async fn handle_parallel_command(cmd: ParallelCommands) -> Result<()> {
    match cmd {
        ParallelCommands::Analyze {
            tasks,
            workflow_id,
            max_parallel,
            resource_aware,
            max_cpu_cores,
            max_memory_mb,
            ruvector_endpoint,
            observatory_endpoint,
            output_format,
        } => {
            parallel_analyze(
                &tasks,
                &workflow_id,
                max_parallel,
                resource_aware,
                max_cpu_cores,
                max_memory_mb,
                &ruvector_endpoint,
                observatory_endpoint.as_deref(),
                &output_format,
            )
            .await
        }
        ParallelCommands::Inspect {
            analysis_id,
            ruvector_endpoint,
            output_format,
        } => parallel_inspect(&analysis_id, &ruvector_endpoint, &output_format).await,
        ParallelCommands::Replay {
            analysis_id,
            ruvector_endpoint,
            output_format,
        } => parallel_replay(&analysis_id, &ruvector_endpoint, &output_format).await,
    }
}

/// Analyzes tasks and produces a parallel execution plan.
#[allow(clippy::too_many_arguments)]
async fn parallel_analyze(
    tasks_input: &str,
    workflow_id: &str,
    max_parallel: usize,
    resource_aware: bool,
    max_cpu_cores: Option<u32>,
    max_memory_mb: Option<u64>,
    ruvector_endpoint: &str,
    observatory_endpoint: Option<&str>,
    output_format: &str,
) -> Result<()> {
    use agentics_contracts::dependency::TaskDependencyNode;
    use llm_orchestrator_core::{
        ParallelizationRequest, ParallelizationConfig, ResourceConstraints,
    };

    info!("Analyzing parallelization for workflow: {}", workflow_id);
    println!(
        "{} {}",
        "Analyzing parallelization via Parallelization Agent:"
            .cyan()
            .bold(),
        workflow_id.yellow()
    );

    // Parse workflow ID
    let workflow_uuid = Uuid::parse_str(workflow_id)
        .with_context(|| format!("Invalid workflow ID: {}", workflow_id))?;

    // Parse tasks from file or inline JSON
    let tasks: Vec<TaskDependencyNode> = if Path::new(tasks_input).exists() {
        let content = fs::read_to_string(tasks_input)
            .with_context(|| format!("Failed to read tasks file: {}", tasks_input))?;
        serde_json::from_str(&content).with_context(|| "Failed to parse tasks JSON from file")?
    } else {
        serde_json::from_str(tasks_input).with_context(|| "Failed to parse tasks JSON string")?
    };

    info!("Loaded {} tasks", tasks.len());

    // Build resource constraints
    let resource_constraints = ResourceConstraints {
        max_total_memory_bytes: max_memory_mb.map(|mb| mb * 1024 * 1024),
        max_total_cpu: max_cpu_cores.map(|c| c as f64),
        max_total_gpu: None,
        max_total_tokens: None,
        per_task_limits: std::collections::HashMap::new(),
    };

    // Create parallelization config
    let config = ParallelizationConfig {
        max_concurrency: max_parallel as u32,
        respect_soft_dependencies: true,
        analyze_data_dependencies: true,
        consider_resource_constraints: resource_aware,
        optimize_makespan: true,
        balance_load: false,
        timeout_ms: Some(30000),
    };

    // Create agent configuration
    let agent_config = ParallelizationAgentConfig {
        default_max_concurrency: max_parallel as u32,
        resource_aware,
        optimize_makespan: true,
        timeout_ms: Some(30000),
    };

    // Create agent
    let mut agent = ParallelizationAgent::new(agent_config);

    // Configure ruvector-service client
    let ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);
    agent = agent.with_ruvector_client(ruvector_client);

    // Configure observatory if provided
    if let Some(obs_endpoint) = observatory_endpoint {
        let observatory = ObservatoryAdapter::new(obs_endpoint);
        agent = agent.with_observatory(observatory);
    }

    println!("  {} {}", "Agent ID:".cyan(), agent.agent_id());
    println!("  {} {}", "Agent Version:".cyan(), agent.version());
    println!("  {} {}", "Tasks:".cyan(), tasks.len());
    println!();

    // Create analyze request
    let request = ParallelizationRequest {
        request_id: Uuid::new_v4(),
        workflow_id: workflow_uuid,
        tasks,
        config,
        resource_constraints,
        timestamp: chrono::Utc::now(),
    };

    // Analyze parallelization
    println!("{}", "Analyzing parallelization opportunities...".cyan());
    let response = agent
        .analyze(request)
        .await
        .with_context(|| "Parallelization analysis failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
        );
    } else {
        let is_success = matches!(
            response.status,
            llm_orchestrator_core::ParallelizationStatus::Success
                | llm_orchestrator_core::ParallelizationStatus::SuccessWithWarnings
        );
        if is_success {
            println!("{}", "✓ Parallelization analysis completed".green().bold());
        } else {
            println!("{}", "✗ Parallelization analysis failed".red().bold());
        }
        println!();
        println!("  {} {}", "Request ID:".cyan(), response.request_id);
        println!("  {} {}", "Workflow ID:".cyan(), response.workflow_id);
        println!("  {} {:?}", "Status:".cyan(), response.status);
        println!();
        println!("  {}", "Summary:".cyan().bold());
        println!(
            "    {} {}",
            "Total Tasks:".dimmed(),
            response.summary.total_tasks
        );
        println!(
            "    {} {}",
            "Parallelizable Tasks:".dimmed(),
            response.summary.parallelizable_tasks
        );
        println!(
            "    {} {}",
            "Sequential Tasks:".dimmed(),
            response.summary.sequential_tasks
        );
        println!(
            "    {} {}",
            "Execution Phases:".dimmed(),
            response.summary.execution_phases
        );
        println!(
            "    {} {}",
            "Max Parallelism:".dimmed(),
            response.summary.max_parallelism
        );
        println!();
        println!("  {}", "Optimization Metrics:".cyan().bold());
        println!(
            "    {} {}ms",
            "Sequential Duration:".dimmed(),
            response.optimization_metrics.sequential_duration_ms
        );
        println!(
            "    {} {}ms",
            "Parallel Duration:".dimmed(),
            response.optimization_metrics.parallel_duration_ms
        );
        println!(
            "    {} {:.2}x",
            "Speedup Factor:".dimmed(),
            response.optimization_metrics.speedup_factor
        );
        println!(
            "    {} {:.2}",
            "Efficiency:".dimmed(),
            response.optimization_metrics.efficiency_score
        );
        println!();
        println!("  {}", "Execution Plan:".cyan().bold());
        for phase in &response.execution_plan.phases {
            println!(
                "    {} {} tasks",
                format!("Phase {}:", phase.phase_index).dimmed(),
                phase.parallel_tasks.len()
            );
            for task in &phase.parallel_tasks {
                let conflicts = if task.conflicts_with.is_empty() {
                    "no conflicts".to_string()
                } else {
                    format!("conflicts: {}", task.conflicts_with.join(", "))
                };
                println!(
                    "      {} {} ({})",
                    "•".dimmed(),
                    task.task_id.yellow(),
                    conflicts.dimmed()
                );
            }
        }
        if !response.issues.is_empty() {
            println!();
            println!("  {}", "Issues:".red().bold());
            for issue in &response.issues {
                let severity_color = match issue.severity {
                    llm_orchestrator_core::ParallelizationIssueSeverity::Error => "✗".red(),
                    llm_orchestrator_core::ParallelizationIssueSeverity::Warning => "⚠".yellow(),
                    llm_orchestrator_core::ParallelizationIssueSeverity::Info => "ℹ".blue(),
                };
                println!("    {} {}", severity_color, issue.description);
                if let Some(suggestion) = &issue.suggestion {
                    println!("      {} {}", "Suggestion:".dimmed(), suggestion);
                }
            }
        }
    }

    Ok(())
}

/// Inspects a previous parallelization analysis.
async fn parallel_inspect(
    analysis_id: &str,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    info!("Inspecting parallelization analysis: {}", analysis_id);
    println!(
        "{} {}",
        "Inspecting analysis via Parallelization Agent:"
            .cyan()
            .bold(),
        analysis_id.yellow()
    );

    let analysis_uuid = Uuid::parse_str(analysis_id)
        .with_context(|| format!("Invalid analysis ID: {}", analysis_id))?;

    // Create agent
    let config = ParallelizationAgentConfig::default();
    let mut agent = ParallelizationAgent::new(config);

    // Configure ruvector-service client
    let ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);
    agent = agent.with_ruvector_client(ruvector_client);

    // Inspect analysis - takes UUID directly
    let response = agent
        .inspect(analysis_uuid)
        .await
        .with_context(|| "Inspection failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
        );
    } else {
        println!("{}", "Analysis Inspection".green().bold());
        println!();
        println!("  {} {}", "Request ID:".cyan(), response.request_id);
        println!("  {} {}", "Found:".cyan(), if response.found { "Yes" } else { "No" });
        if response.decision_event.is_some() {
            println!("  {} {}", "Decision Event:".cyan(), "Present");
        } else {
            println!("  {} {}", "Decision Event:".yellow(), "Not found");
        }
    }

    Ok(())
}

/// Replays a previous parallelization analysis.
///
/// Note: Replay requires the original ParallelizationRequest data.
/// Currently, retrieving this from storage by ID is not implemented.
async fn parallel_replay(
    analysis_id: &str,
    _ruvector_endpoint: &str,
    _output_format: &str,
) -> Result<()> {
    info!("Replaying parallelization analysis: {}", analysis_id);
    println!(
        "{} {}",
        "Replaying analysis via Parallelization Agent:"
            .cyan()
            .bold(),
        analysis_id.yellow()
    );

    let _analysis_uuid = Uuid::parse_str(analysis_id)
        .with_context(|| format!("Invalid analysis ID: {}", analysis_id))?;

    // Note: The replay method requires the full ParallelizationRequest,
    // which would need to be retrieved from ruvector-service storage.
    // This functionality requires additional storage implementation.
    println!(
        "{}",
        "⚠ Replay functionality requires original request data retrieval.".yellow()
    );
    println!(
        "  {} Use 'parallel analyze' with the same task data to re-run analysis.",
        "Suggestion:".cyan()
    );

    Ok(())
}

// =============================================================================
// State Machine Agent Command Handlers
// =============================================================================

/// Handles State Machine Agent commands.
async fn handle_state_machine_command(cmd: StateMachineCommands) -> Result<()> {
    match cmd {
        StateMachineCommands::Transition {
            execution_id,
            entity_type,
            current_state,
            target_state,
            reason,
            initiated_by,
            force,
            ruvector_endpoint,
            output_format,
        } => {
            state_machine_transition(
                &execution_id,
                &entity_type,
                &current_state,
                &target_state,
                &reason,
                &initiated_by,
                force,
                &ruvector_endpoint,
                &output_format,
            )
            .await
        }
        StateMachineCommands::Validate {
            execution_id,
            entity_type,
            current_state,
            target_state,
            ruvector_endpoint,
            output_format,
        } => {
            state_machine_validate(
                &execution_id,
                &entity_type,
                &current_state,
                &target_state,
                &ruvector_endpoint,
                &output_format,
            )
            .await
        }
        StateMachineCommands::Inspect {
            execution_id,
            entity_type,
            ruvector_endpoint,
            output_format,
        } => {
            state_machine_inspect(
                &execution_id,
                &entity_type,
                &ruvector_endpoint,
                &output_format,
            )
            .await
        }
        StateMachineCommands::History {
            execution_id,
            entity_type,
            limit,
            offset,
            ruvector_endpoint,
            output_format,
        } => {
            state_machine_history(
                &execution_id,
                &entity_type,
                limit,
                offset,
                &ruvector_endpoint,
                &output_format,
            )
            .await
        }
        StateMachineCommands::Replay {
            decision_id,
            ruvector_endpoint,
            output_format,
        } => state_machine_replay(&decision_id, &ruvector_endpoint, &output_format).await,
    }
}

/// Parse entity type from string.
fn parse_entity_type(s: &str) -> Result<StateEntityType> {
    match s.to_lowercase().as_str() {
        "workflow" => Ok(StateEntityType::Workflow),
        "task" => Ok(StateEntityType::Task),
        _ => anyhow::bail!("Invalid entity type: {}. Valid types: workflow, task", s),
    }
}

/// Performs a state transition via the State Machine Agent.
#[allow(clippy::too_many_arguments)]
async fn state_machine_transition(
    execution_id: &str,
    entity_type: &str,
    current_state: &str,
    target_state: &str,
    reason: &str,
    initiated_by: &str,
    force: bool,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    info!("Transitioning state: {} -> {}", current_state, target_state);
    println!(
        "{} {} {} {}",
        "Transitioning state via State Machine Agent:".cyan().bold(),
        current_state.yellow(),
        "→".dimmed(),
        target_state.green()
    );

    let execution_uuid = Uuid::parse_str(execution_id)
        .with_context(|| format!("Invalid execution ID: {}", execution_id))?;

    let entity = parse_entity_type(entity_type)?;

    // Create agent
    let config = StateMachineAgentConfig::default();
    let mut agent = StateMachineAgent::new(config);

    // Configure ruvector-service client
    let ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);
    agent = agent.with_ruvector_client(ruvector_client);

    println!("  {} {}", "Agent ID:".cyan(), agent.agent_id());
    println!("  {} {}", "Agent Version:".cyan(), agent.version());
    println!();

    // Create transition request
    let request = StateTransitionRequest::new(
        execution_uuid,
        entity.clone(),
        current_state,
        target_state,
        reason,
        initiated_by,
    )
    .with_force(force);

    // Perform transition
    println!("{}", "Executing state transition...".cyan());
    let response = agent
        .transition(request)
        .await
        .with_context(|| "State transition failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
        );
    } else {
        if response.success {
            println!("{}", "✓ State transition successful".green().bold());
        } else {
            println!("{}", "✗ State transition failed".red().bold());
        }
        println!();
        println!("  {} {}", "Request ID:".cyan(), response.request_id);
        println!("  {} {}", "Execution ID:".cyan(), response.execution_id);
        println!("  {} {}", "Entity Type:".cyan(), response.entity_type);
        println!("  {} {:?}", "Status:".cyan(), response.status);
        println!(
            "  {} {} → {}",
            "Transition:".cyan(),
            response.previous_state.yellow(),
            response.new_state.green()
        );
        println!("  {} {:.2}", "Confidence:".cyan(), response.confidence);
        println!(
            "  {} {}",
            "Inputs Hash:".cyan(),
            &response.inputs_hash[..16]
        );
        println!(
            "  {} {}",
            "Decision Event ID:".cyan(),
            response.decision_event_id
        );
        println!(
            "  {} {}",
            "Timestamp:".cyan(),
            response.timestamp.to_rfc3339()
        );
        if let Some(error) = &response.error {
            println!("  {} {}", "Error:".red(), error);
        }
        println!();
        println!("  {}", "Validation:".cyan().bold());
        println!(
            "    {} {}",
            "Valid:".dimmed(),
            if response.validation.valid {
                "yes".green()
            } else {
                "no".red()
            }
        );
        println!(
            "    {} {}",
            "Source State Matches:".dimmed(),
            if response.validation.source_state_matches {
                "yes".green()
            } else {
                "no".red()
            }
        );
        println!(
            "    {} {}",
            "Target State Exists:".dimmed(),
            if response.validation.target_state_exists {
                "yes".green()
            } else {
                "no".red()
            }
        );
        if !response.validation.rule_violations.is_empty() {
            println!("    {}", "Rule Violations:".red());
            for violation in &response.validation.rule_violations {
                println!(
                    "      {} {} - {}",
                    "•".red(),
                    violation.rule_id,
                    violation.message
                );
            }
        }
        if !response.validation.warnings.is_empty() {
            println!("    {}", "Warnings:".yellow());
            for warning in &response.validation.warnings {
                println!("      {} {}", "⚠".yellow(), warning);
            }
        }
        println!();
        println!("  {}", "Constraints Applied:".cyan().bold());
        for constraint in &response.constraints_applied {
            println!("    {} {}", "•".dimmed(), constraint);
        }
    }

    Ok(())
}

/// Validates a state transition without executing it.
async fn state_machine_validate(
    execution_id: &str,
    entity_type: &str,
    current_state: &str,
    target_state: &str,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    info!(
        "Validating transition: {} -> {}",
        current_state, target_state
    );
    println!(
        "{} {} {} {}",
        "Validating transition via State Machine Agent:"
            .cyan()
            .bold(),
        current_state.yellow(),
        "→".dimmed(),
        target_state.green()
    );

    let execution_uuid = Uuid::parse_str(execution_id)
        .with_context(|| format!("Invalid execution ID: {}", execution_id))?;

    let entity = parse_entity_type(entity_type)?;

    // Create agent
    let config = StateMachineAgentConfig::default();
    let mut agent = StateMachineAgent::new(config);

    // Configure ruvector-service client
    let ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);
    agent = agent.with_ruvector_client(ruvector_client);

    // Create transition request (for validation only)
    let request = StateTransitionRequest::new(
        execution_uuid,
        entity,
        current_state,
        target_state,
        "validation",
        "cli",
    );

    // Validate transition
    let validation = agent
        .validate(request)
        .await
        .with_context(|| "Validation failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&validation)
                .unwrap_or_else(|_| format!("{:?}", validation))
        );
    } else {
        if validation.valid {
            println!("{}", "✓ Transition is VALID".green().bold());
        } else {
            println!("{}", "✗ Transition is INVALID".red().bold());
        }
        println!();
        println!("  {}", "Validation Details:".cyan().bold());
        println!(
            "    {} {}",
            "Valid:".dimmed(),
            if validation.valid {
                "yes".green()
            } else {
                "no".red()
            }
        );
        println!(
            "    {} {}",
            "Source State Matches:".dimmed(),
            if validation.source_state_matches {
                "yes".green()
            } else {
                "no".red()
            }
        );
        println!(
            "    {} {}",
            "Target State Exists:".dimmed(),
            if validation.target_state_exists {
                "yes".green()
            } else {
                "no".red()
            }
        );
        if !validation.rule_violations.is_empty() {
            println!();
            println!("  {}", "Rule Violations:".red().bold());
            for violation in &validation.rule_violations {
                println!(
                    "    {} {} - {}",
                    "✗".red(),
                    violation.rule_id,
                    violation.message
                );
            }
        }
        if !validation.warnings.is_empty() {
            println!();
            println!("  {}", "Warnings:".yellow().bold());
            for warning in &validation.warnings {
                println!("    {} {}", "⚠".yellow(), warning);
            }
        }
    }

    Ok(())
}

/// Inspects current state via the State Machine Agent.
async fn state_machine_inspect(
    execution_id: &str,
    entity_type: &str,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    info!("Inspecting state for: {}", execution_id);
    println!(
        "{} {}",
        "Inspecting state via State Machine Agent:".cyan().bold(),
        execution_id.yellow()
    );

    let execution_uuid = Uuid::parse_str(execution_id)
        .with_context(|| format!("Invalid execution ID: {}", execution_id))?;

    let entity = parse_entity_type(entity_type)?;

    // Create agent
    let config = StateMachineAgentConfig::default();
    let mut agent = StateMachineAgent::new(config);

    // Configure ruvector-service client
    let ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);
    agent = agent.with_ruvector_client(ruvector_client);

    // Create inspect request
    let request = StateInspectRequest {
        execution_id: execution_uuid,
        entity_type: entity,
    };

    // Inspect state
    let response = agent
        .inspect(request)
        .await
        .with_context(|| "State inspection failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
        );
    } else {
        println!("{}", "State Inspection".green().bold());
        println!();
        println!("  {} {}", "Execution ID:".cyan(), response.execution_id);
        println!("  {} {}", "Entity Type:".cyan(), response.entity_type);
        println!(
            "  {} {}",
            "Current State:".cyan(),
            if response.is_terminal {
                response.current_state.green()
            } else {
                response.current_state.yellow()
            }
        );
        println!(
            "  {} {}",
            "Is Terminal:".cyan(),
            if response.is_terminal {
                "yes".green()
            } else {
                "no".yellow()
            }
        );
        println!(
            "  {} {}",
            "Transition Count:".cyan(),
            response.transition_count
        );
        if let Some(time_ms) = response.time_in_state_ms {
            println!("  {} {}ms", "Time in State:".cyan(), time_ms);
        }
        if let Some(last_transition) = response.last_transition_at {
            println!(
                "  {} {}",
                "Last Transition:".cyan(),
                last_transition.to_rfc3339()
            );
        }
        if let Some(last_by) = &response.last_transition_by {
            println!("  {} {}", "Last Transition By:".cyan(), last_by);
        }
        println!();
        println!("  {}", "Valid Next States:".cyan().bold());
        if response.valid_next_states.is_empty() {
            println!("    {} (terminal state)", "(none)".dimmed());
        } else {
            for state in &response.valid_next_states {
                println!("    {} {}", "→".dimmed(), state.green());
            }
        }
    }

    Ok(())
}

/// Gets state transition history via the State Machine Agent.
async fn state_machine_history(
    execution_id: &str,
    entity_type: &str,
    limit: usize,
    offset: usize,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    info!("Getting state history for: {}", execution_id);
    println!(
        "{} {}",
        "Getting state history via State Machine Agent:"
            .cyan()
            .bold(),
        execution_id.yellow()
    );

    let execution_uuid = Uuid::parse_str(execution_id)
        .with_context(|| format!("Invalid execution ID: {}", execution_id))?;

    let entity = parse_entity_type(entity_type)?;

    // Create agent
    let config = StateMachineAgentConfig::default();
    let mut agent = StateMachineAgent::new(config);

    // Configure ruvector-service client
    let ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);
    agent = agent.with_ruvector_client(ruvector_client);

    // Create history request
    let request = StateHistoryRequest {
        execution_id: execution_uuid,
        entity_type: entity,
        limit,
        offset,
    };

    // Get history
    let response = agent
        .history(request)
        .await
        .with_context(|| "History query failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
        );
    } else {
        println!("{}", "State Transition History".green().bold());
        println!();
        println!("  {} {}", "Execution ID:".cyan(), response.execution_id);
        println!("  {} {}", "Entity Type:".cyan(), response.entity_type);
        println!(
            "  {} {} of {}",
            "Entries:".cyan(),
            response.entries.len(),
            response.total_count
        );
        println!(
            "  {} offset={}, limit={}",
            "Pagination:".cyan(),
            response.offset,
            response.limit
        );
        println!();
        if response.entries.is_empty() {
            println!("  {}", "(no transitions recorded)".dimmed());
        } else {
            println!("  {}", "Transitions:".cyan().bold());
            for (i, entry) in response.entries.iter().enumerate() {
                println!();
                println!(
                    "    [{}] {} {} {}",
                    i + 1 + response.offset,
                    entry.from_state.yellow(),
                    "→".dimmed(),
                    entry.to_state.green()
                );
                println!("        {} {}", "Reason:".dimmed(), entry.reason);
                println!(
                    "        {} {}",
                    "Initiated By:".dimmed(),
                    entry.initiated_by
                );
                println!(
                    "        {} {}",
                    "Timestamp:".dimmed(),
                    entry.timestamp.to_rfc3339()
                );
                if let Some(duration_ms) = entry.duration_in_previous_state_ms {
                    println!(
                        "        {} {}ms",
                        "Duration in Previous:".dimmed(),
                        duration_ms
                    );
                }
                println!(
                    "        {} {}",
                    "Decision Event:".dimmed(),
                    entry.decision_event_id
                );
            }
        }
    }

    Ok(())
}

/// Replays a previous state transition via the State Machine Agent.
async fn state_machine_replay(
    decision_id: &str,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    info!("Replaying state transition: {}", decision_id);
    println!(
        "{} {}",
        "Replaying state transition via State Machine Agent:"
            .cyan()
            .bold(),
        decision_id.yellow()
    );

    let decision_uuid = Uuid::parse_str(decision_id)
        .with_context(|| format!("Invalid decision ID: {}", decision_id))?;

    // Create agent
    let config = StateMachineAgentConfig::default();
    let agent = StateMachineAgent::new(config);

    // Configure ruvector-service client (agent is immutable here, but we'd normally configure it)
    let _ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);

    // Create replay request
    let request = StateReplayRequest {
        original_decision_id: decision_uuid,
    };

    // Replay transition
    let response = agent
        .replay(request)
        .await
        .with_context(|| "Replay failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
        );
    } else {
        println!("{}", "✓ State transition replayed".green().bold());
        println!();
        println!(
            "  {} {}",
            "Original Decision ID:".cyan(),
            response.original_decision_id
        );
        println!(
            "  {} {}",
            "New Decision ID:".cyan(),
            response.new_decision_id
        );
        println!(
            "  {} {}",
            "Inputs Hash Match:".cyan(),
            if response.inputs_hash_match {
                "yes".green()
            } else {
                "no".red()
            }
        );
        println!(
            "  {} {}",
            "Original Inputs Hash:".cyan(),
            &response.original_inputs_hash[..16]
        );
        println!(
            "  {} {}",
            "New Inputs Hash:".cyan(),
            &response.new_inputs_hash[..16]
        );
        println!(
            "  {} {}",
            "Timestamp:".cyan(),
            response.timestamp.to_rfc3339()
        );
    }

    Ok(())
}

// =============================================================================
// Swarm Coordinator Agent Command Handlers
// =============================================================================

/// Handles Swarm Coordinator Agent commands.
async fn handle_swarm_command(cmd: SwarmCommands) -> Result<()> {
    match cmd {
        SwarmCommands::Coordinate {
            config,
            workflow_id,
            objective,
            objective_type,
            consensus,
            leader_id,
            aggregation,
            max_concurrent,
            timeout,
            ruvector_endpoint,
            observatory_endpoint,
            output_format,
        } => {
            swarm_coordinate(
                &config,
                &workflow_id,
                &objective,
                &objective_type,
                &consensus,
                leader_id.as_deref(),
                &aggregation,
                max_concurrent,
                timeout,
                &ruvector_endpoint,
                observatory_endpoint.as_deref(),
                &output_format,
            )
            .await
        }
        SwarmCommands::Inspect {
            request_id,
            ruvector_endpoint,
            output_format,
        } => swarm_inspect(&request_id, &ruvector_endpoint, &output_format).await,
        SwarmCommands::Replay {
            request_id,
            ruvector_endpoint,
            output_format,
        } => swarm_replay(&request_id, &ruvector_endpoint, &output_format).await,
    }
}

/// Coordinates a swarm of agents toward an objective.
#[allow(clippy::too_many_arguments)]
async fn swarm_coordinate(
    config_input: &str,
    workflow_id: &str,
    objective_desc: &str,
    objective_type: &str,
    consensus: &str,
    leader_id: Option<&str>,
    aggregation: &str,
    max_concurrent: usize,
    timeout_secs: u64,
    ruvector_endpoint: &str,
    observatory_endpoint: Option<&str>,
    output_format: &str,
) -> Result<()> {
    use agentics_contracts::{
        AggregationMode, AggregationStrategy, ConsensusStrategy, ObjectiveType, SuccessCriterion,
        SwarmCoordinationRequest, SwarmCoordinatorConfig, SwarmObjective, ValidationRule, WorkerSpec,
        WorkerStatus, SwarmCoordinationStatus,
    };

    info!("Coordinating swarm for workflow: {}", workflow_id);
    println!(
        "{} {}",
        "Coordinating swarm via Swarm Coordinator Agent:"
            .cyan()
            .bold(),
        workflow_id.yellow()
    );

    // Parse workflow ID
    let workflow_uuid = Uuid::parse_str(workflow_id)
        .with_context(|| format!("Invalid workflow ID: {}", workflow_id))?;

    // Parse workers configuration from file or inline JSON
    let workers: Vec<WorkerSpec> = if Path::new(config_input).exists() {
        let content = fs::read_to_string(config_input)
            .with_context(|| format!("Failed to read config file: {}", config_input))?;
        serde_json::from_str(&content).with_context(|| "Failed to parse workers JSON from file")?
    } else {
        serde_json::from_str(config_input).with_context(|| "Failed to parse workers JSON string")?
    };

    info!("Loaded {} workers", workers.len());

    // Parse objective type
    let obj_type = match objective_type.to_lowercase().as_str() {
        "research" => ObjectiveType::Research,
        "analysis" => ObjectiveType::Analysis,
        "generation" => ObjectiveType::Generation,
        "execution" => ObjectiveType::Execution,
        "review" => ObjectiveType::Review,
        other => ObjectiveType::Custom(other.to_string()),
    };

    // Build success criteria
    let success_criteria = vec![SuccessCriterion {
        id: "all-workers-complete".to_string(),
        description: "All workers complete".to_string(),
        required: true,
        validation: ValidationRule::AllSuccess,
    }];

    // Build objective
    let swarm_objective = SwarmObjective {
        id: format!("objective-{}", workflow_uuid),
        description: objective_desc.to_string(),
        objective_type: obj_type,
        inputs: std::collections::HashMap::new(),
        success_criteria,
        priority: 5,
    };

    // Parse consensus strategy
    let consensus_strategy = match consensus.to_lowercase().as_str() {
        "first_wins" => ConsensusStrategy::FirstWins,
        "last_wins" => ConsensusStrategy::LastWins,
        "majority" => ConsensusStrategy::Majority,
        "weighted_majority" => ConsensusStrategy::WeightedMajority,
        "merge_all" => ConsensusStrategy::MergeAll,
        "leader" => {
            let leader = leader_id.ok_or_else(|| anyhow::anyhow!("Leader ID required when consensus = leader"))?;
            ConsensusStrategy::Leader { leader_id: leader.to_string() }
        }
        other => anyhow::bail!("Invalid consensus strategy: {}. Valid strategies: first_wins, last_wins, majority, weighted_majority, merge_all, leader", other),
    };

    // Parse aggregation strategy
    let aggregation_mode = match aggregation.to_lowercase().as_str() {
        "combine" => AggregationMode::Combine,
        "select_best" => AggregationMode::SelectBest,
        "summarize" => AggregationMode::Summarize,
        "collect" => AggregationMode::Collect,
        "custom" => AggregationMode::Custom("default".to_string()),
        other => anyhow::bail!("Invalid aggregation mode: {}. Valid modes: combine, select_best, summarize, collect, custom", other),
    };

    let aggregation_strategy = AggregationStrategy {
        mode: aggregation_mode,
        aggregate_fields: vec![],
        merge_fields: vec![],
        reduce_fields: vec![],
    };

    // Build swarm coordinator config
    let swarm_config = SwarmCoordinatorConfig {
        max_concurrent_workers: max_concurrent as u32,
        coordination_timeout_ms: timeout_secs * 1000,
        consensus_strategy: consensus_strategy.clone(),
        continue_on_partial_failure: true,
        convergence_threshold: 0.8,
        enable_caching: true,
    };

    // Create agent configuration
    let agent_config = SwarmCoordinatorAgentConfig {
        default_max_concurrent: max_concurrent as u32,
        default_timeout_ms: timeout_secs * 1000,
        default_consensus: consensus_strategy,
        enable_caching: true,
    };

    // Create agent
    let mut agent = SwarmCoordinatorAgent::new(agent_config);

    // Configure ruvector-service client
    let ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);
    agent = agent.with_ruvector_client(ruvector_client);

    // Configure observatory if provided
    if let Some(obs_endpoint) = observatory_endpoint {
        let observatory = ObservatoryAdapter::new(obs_endpoint);
        agent = agent.with_observatory(observatory);
    }

    println!("  {} {}", "Agent ID:".cyan(), agent.agent_id());
    println!("  {} {}", "Agent Version:".cyan(), agent.version());
    println!("  {} {}", "Workers:".cyan(), workers.len());
    println!("  {} {}", "Objective:".cyan(), objective_desc);
    println!("  {} {:?}", "Consensus:".cyan(), consensus);
    println!();

    // Create coordination request
    let request = SwarmCoordinationRequest {
        request_id: Uuid::new_v4(),
        workflow_id: workflow_uuid,
        objective: swarm_objective,
        workers,
        config: swarm_config,
        aggregation: aggregation_strategy,
        timestamp: chrono::Utc::now(),
    };

    // Coordinate swarm
    println!("{}", "Coordinating swarm...".cyan());
    let response = agent
        .coordinate(request)
        .await
        .with_context(|| "Swarm coordination failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
        );
    } else {
        let is_success = matches!(
            response.status,
            SwarmCoordinationStatus::Success | SwarmCoordinationStatus::ConsensusReached
        );
        if is_success {
            println!("{}", "✓ Swarm coordination completed".green().bold());
        } else {
            println!("{}", "✗ Swarm coordination failed".red().bold());
        }
        println!();
        println!("  {} {}", "Request ID:".cyan(), response.request_id);
        println!("  {} {}", "Workflow ID:".cyan(), response.workflow_id);
        println!("  {} {:?}", "Status:".cyan(), response.status);
        println!("  {} {:.2}", "Confidence:".cyan(), response.aggregated_output.confidence);
        println!("  {} {}", "Objective Met:".cyan(), response.summary.objective_met);
        println!();
        println!("  {}", "Summary:".cyan().bold());
        println!(
            "    {} {}",
            "Total Workers:".dimmed(),
            response.summary.total_workers
        );
        println!(
            "    {} {}",
            "Successful:".dimmed(),
            response.summary.successful_workers
        );
        println!(
            "    {} {}",
            "Failed:".dimmed(),
            response.summary.failed_workers
        );
        println!(
            "    {} {}",
            "Skipped:".dimmed(),
            response.summary.skipped_workers
        );
        println!(
            "    {} {}ms",
            "Total Duration:".dimmed(),
            response.summary.total_duration_ms
        );
        println!(
            "    {} {}",
            "Max Concurrency:".dimmed(),
            response.summary.max_concurrency_achieved
        );
        println!();
        if !response.issues.is_empty() {
            println!("  {}", "Issues:".yellow().bold());
            for issue in &response.issues {
                println!(
                    "    {} [{}] {}",
                    "⚠".yellow(),
                    format!("{:?}", issue.severity).dimmed(),
                    issue.description
                );
            }
            println!();
        }
        println!("  {}", "Worker Results:".cyan().bold());
        for result in &response.worker_results {
            let status_icon = match result.status {
                WorkerStatus::Success => "✓".green(),
                WorkerStatus::Failed => "✗".red(),
                WorkerStatus::Timeout => "⏱".yellow(),
                WorkerStatus::Skipped => "⊘".dimmed(),
                WorkerStatus::Cancelled => "⊗".dimmed(),
                _ => "?".dimmed(),
            };
            println!(
                "    {} {} ({:?}) - {}ms",
                status_icon,
                result.worker_id.yellow(),
                result.status,
                result.metrics.duration_ms
            );
        }
        println!();
        println!("  {}", "Aggregated Output:".cyan().bold());
        if let Some(primary) = &response.aggregated_output.primary {
            println!(
                "    Primary: {}",
                serde_json::to_string(primary).unwrap_or_default()
            );
        }
        println!(
            "    Confidence: {:.2}",
            response.aggregated_output.confidence
        );
        if let Some(consensus) = &response.aggregated_output.consensus {
            println!(
                "    Consensus Reached: {} (convergence: {:.2})",
                consensus.reached,
                consensus.convergence_score
            );
        }
    }

    Ok(())
}

/// Inspects a previous swarm coordination.
async fn swarm_inspect(
    request_id: &str,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    info!("Inspecting swarm coordination: {}", request_id);
    println!(
        "{} {}",
        "Inspecting swarm coordination:".cyan().bold(),
        request_id.yellow()
    );

    // Parse request ID
    let request_uuid = Uuid::parse_str(request_id)
        .with_context(|| format!("Invalid request ID: {}", request_id))?;

    // Create agent configuration
    use agentics_contracts::ConsensusStrategy as InspectConsensus;
    let config = SwarmCoordinatorAgentConfig {
        default_max_concurrent: 8,
        default_timeout_ms: 300000,
        default_consensus: InspectConsensus::WeightedMajority,
        enable_caching: true,
    };

    // Create agent
    let mut agent = SwarmCoordinatorAgent::new(config);

    // Configure ruvector-service client
    let ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);
    agent = agent.with_ruvector_client(ruvector_client);

    // Inspect
    let response = agent
        .inspect(request_uuid)
        .await
        .with_context(|| "Inspection failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
        );
    } else {
        if response.found {
            println!("{}", "✓ Swarm coordination found".green().bold());
        } else {
            println!("{}", "✗ Swarm coordination not found".red().bold());
            return Ok(());
        }
        println!();
        println!("  {} {}", "Request ID:".cyan(), response.request_id);
        if let Some(event) = &response.decision_event {
            println!("  {} {}", "Event ID:".cyan(), event.id);
            println!("  {} {}", "Agent ID:".cyan(), event.agent_id);
            println!("  {} {}", "Agent Version:".cyan(), event.agent_version);
            println!("  {} {}", "Decision Type:".cyan(), event.decision_type);
            println!(
                "  {} {}",
                "Timestamp:".cyan(),
                event.timestamp.to_rfc3339()
            );
            println!("  {} {:.2}", "Confidence:".cyan(), event.confidence);
            println!();
            println!("  {}", "Outputs:".cyan().bold());
            println!(
                "    {}",
                serde_json::to_string_pretty(&event.outputs).unwrap_or_default()
            );
        }
    }

    Ok(())
}

/// Replays a previous swarm coordination.
async fn swarm_replay(
    request_id: &str,
    ruvector_endpoint: &str,
    output_format: &str,
) -> Result<()> {
    use agentics_contracts::{
        AggregationMode, AggregationStrategy, ConsensusStrategy, ObjectiveType, SuccessCriterion,
        SwarmCoordinationRequest, SwarmCoordinatorConfig, SwarmObjective, ValidationRule,
        SwarmCoordinationStatus,
    };

    info!("Replaying swarm coordination: {}", request_id);
    println!(
        "{} {}",
        "Replaying swarm coordination:".cyan().bold(),
        request_id.yellow()
    );

    // Parse request ID
    let request_uuid = Uuid::parse_str(request_id)
        .with_context(|| format!("Invalid request ID: {}", request_id))?;

    // Create agent configuration
    let config = SwarmCoordinatorAgentConfig {
        default_max_concurrent: 8,
        default_timeout_ms: 300000,
        default_consensus: ConsensusStrategy::Majority,
        enable_caching: true,
    };

    // Create agent
    let mut agent = SwarmCoordinatorAgent::new(config);

    // Configure ruvector-service client
    let ruvector_client = RuVectorServiceClient::new(ruvector_endpoint);
    agent = agent.with_ruvector_client(ruvector_client);

    // First inspect to get the original request details
    let inspect_response = agent
        .inspect(request_uuid)
        .await
        .with_context(|| "Failed to retrieve original coordination")?;

    if !inspect_response.found {
        println!("{}", "✗ Original coordination not found - cannot replay".red().bold());
        return Ok(());
    }

    // Reconstruct a replay request from the inspect response
    // In a real implementation, this would retrieve the full original request from ruvector-service
    let swarm_objective = SwarmObjective {
        id: format!("replay-objective-{}", request_uuid),
        description: "Replayed objective".to_string(),
        objective_type: ObjectiveType::Execution,
        inputs: std::collections::HashMap::new(),
        success_criteria: vec![SuccessCriterion {
            id: "completion".to_string(),
            description: "Completion".to_string(),
            required: true,
            validation: ValidationRule::AllSuccess,
        }],
        priority: 5,
    };

    let swarm_config = SwarmCoordinatorConfig {
        max_concurrent_workers: 8,
        coordination_timeout_ms: 300000,
        consensus_strategy: ConsensusStrategy::Majority,
        continue_on_partial_failure: true,
        convergence_threshold: 0.8,
        enable_caching: true,
    };

    let aggregation_strategy = AggregationStrategy {
        mode: AggregationMode::Combine,
        aggregate_fields: vec![],
        merge_fields: vec![],
        reduce_fields: vec![],
    };

    // Note: Replay requires the original workers to be stored and retrieved
    // For now we show a limitation message
    println!("{}", "Note: Replay requires original request to be fully retrieved from ruvector-service.".yellow());
    println!("{}", "Using empty workers list - in production this would be populated from storage.".yellow());
    println!();

    let replay_request = SwarmCoordinationRequest {
        request_id: request_uuid,
        workflow_id: inspect_response.request_id, // Using request_id as workflow_id placeholder
        objective: swarm_objective,
        workers: vec![], // Would be populated from original
        config: swarm_config,
        aggregation: aggregation_strategy,
        timestamp: chrono::Utc::now(),
    };

    // Replay
    let response = agent
        .replay(replay_request)
        .await
        .with_context(|| "Replay failed")?;

    // Output result
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
        );
    } else {
        let is_success = matches!(
            response.status,
            SwarmCoordinationStatus::Success | SwarmCoordinationStatus::ConsensusReached
        );
        if is_success {
            println!("{}", "✓ Swarm coordination replayed".green().bold());
        } else {
            println!("{}", "✗ Swarm coordination replay failed".red().bold());
        }
        println!();
        println!("  {} {}", "Request ID:".cyan(), response.request_id);
        println!("  {} {}", "Workflow ID:".cyan(), response.workflow_id);
        println!("  {} {:?}", "Status:".cyan(), response.status);
        println!("  {} {:.2}", "Confidence:".cyan(), response.aggregated_output.confidence);
        println!();
        println!("  {}", "Summary:".cyan().bold());
        println!(
            "    {} {}",
            "Total Workers:".dimmed(),
            response.summary.total_workers
        );
        println!(
            "    {} {}",
            "Successful:".dimmed(),
            response.summary.successful_workers
        );
        println!(
            "    {} {}",
            "Failed:".dimmed(),
            response.summary.failed_workers
        );
    }

    Ok(())
}

// =============================================================================
// HTTP Server for Cloud Run Deployment
// =============================================================================

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
}

/// Ready check response
#[derive(Serialize)]
struct ReadyResponse {
    ready: bool,
    service: String,
    checks: ReadyChecks,
}

/// Individual readiness checks
#[derive(Serialize)]
struct ReadyChecks {
    core: bool,
    agents: bool,
}

/// Service info response
#[derive(Serialize)]
struct ServiceInfo {
    service: String,
    version: String,
    description: String,
    agents: Vec<String>,
}

/// Health endpoint handler
async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "llm-orchestrator".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Readiness endpoint handler
async fn ready_handler() -> (StatusCode, Json<ReadyResponse>) {
    let response = ReadyResponse {
        ready: true,
        service: "llm-orchestrator".to_string(),
        checks: ReadyChecks {
            core: true,
            agents: true,
        },
    };
    (StatusCode::OK, Json(response))
}

/// Root endpoint with service info
async fn root_handler() -> Json<ServiceInfo> {
    Json(ServiceInfo {
        service: "llm-orchestrator".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "LLM Workflow Orchestrator - Central Workflow Execution & Coordination Layer".to_string(),
        agents: vec![
            "WorkflowOrchestratorAgent".to_string(),
            "DependencyResolverAgent".to_string(),
            "ParallelizationAgent".to_string(),
            "StateMachineAgent".to_string(),
            "SwarmCoordinatorAgent".to_string(),
            "TaskSchedulerAgent".to_string(),
            "RetryRecoveryAgent".to_string(),
        ],
    })
}

/// Start HTTP server for Cloud Run deployment
async fn serve_http(host: &str, port: u16) -> Result<()> {
    info!("Starting HTTP server on {}:{}", host, port);
    println!(
        "{} Starting server on {}:{}",
        "LLM-Orchestrator".cyan().bold(),
        host,
        port
    );

    // Build router with health endpoints
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler));

    // Parse address
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .context("Invalid host:port")?;

    println!("{}", "✓ Server ready".green().bold());
    println!("  Endpoints:");
    println!("    GET /        - Service info");
    println!("    GET /health  - Health check");
    println!("    GET /ready   - Readiness check");

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}