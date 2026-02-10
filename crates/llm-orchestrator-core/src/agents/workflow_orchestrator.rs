// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Workflow Orchestrator Agent implementation.
//!
//! # Agent Contract
//!
//! **Classification**: WORKFLOW_EXECUTION
//!
//! **Purpose**: Execute and coordinate multi-step workflows across agents
//! in a deterministic and state-aware manner.
//!
//! ## What This Agent MAY Do
//!
//! - Execute multi-step workflows
//! - Invoke downstream agents via contracts
//! - Manage task dependencies and ordering
//! - Trigger retries and recovery paths
//! - Coordinate parallel execution
//! - Transition workflow states
//!
//! ## What This Agent MUST NOT Do
//!
//! - Intercept runtime traffic directly (that is Edge)
//! - Enforce security policies (that is Shield)
//! - Emit anomaly detections (that is Sentinel)
//! - Perform optimization analysis (that is Auto-Optimizer)
//! - Modify schemas dynamically
//! - Connect directly to Google SQL
//! - Execute SQL statements
//!
//! ## CLI Endpoints
//!
//! - `agent execute` - Execute a workflow
//! - `agent schedule` - Schedule a workflow for later execution
//! - `agent inspect` - Inspect workflow execution state
//! - `agent replay` - Replay a workflow execution

use crate::adapters::observatory::ObservatoryAdapter;
use crate::adapters::ruvector_service::{
    DecisionEventRecord, RuVectorServiceClient, TaskExecutionRecord, WorkflowStateRecord,
};
use crate::context::ExecutionContext;
use crate::dag::WorkflowDAG;
use crate::error::{OrchestratorError, Result};
use crate::workflow::{Step, StepType, Workflow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::feu_collector::FeuSpanCollector;
use agentics_contracts::feu::SpanStatus;

/// Agent version.
pub const AGENT_VERSION: &str = "0.1.0";

/// Agent ID prefix.
pub const AGENT_ID_PREFIX: &str = "workflow-orchestrator";

/// Workflow Orchestrator Agent.
///
/// This agent executes and coordinates multi-step workflows according
/// to the Agent Infrastructure Constitution.
pub struct WorkflowOrchestratorAgent {
    /// Agent configuration.
    config: AgentConfig,
    /// RuVector service client for persistence.
    ruvector_client: RuVectorServiceClient,
    /// Observatory adapter for telemetry.
    observatory: ObservatoryAdapter,
    /// Agent ID.
    agent_id: String,
    /// FEU span collector (optional, for Agentics execution mode).
    feu_collector: Option<FeuSpanCollector>,
}

impl WorkflowOrchestratorAgent {
    /// Creates a new Workflow Orchestrator Agent.
    pub fn new(config: AgentConfig) -> Self {
        let agent_id = format!("{}-{}", AGENT_ID_PREFIX, Uuid::new_v4().to_string()[..8].to_string());

        Self {
            config,
            ruvector_client: RuVectorServiceClient::disabled(),
            observatory: ObservatoryAdapter::disabled(),
            agent_id,
            feu_collector: None,
        }
    }

    /// Creates an agent with ruvector-service integration.
    pub fn with_ruvector_client(mut self, client: RuVectorServiceClient) -> Self {
        self.ruvector_client = client;
        self
    }

    /// Creates an agent with observatory integration.
    pub fn with_observatory(mut self, observatory: ObservatoryAdapter) -> Self {
        self.observatory = observatory;
        self
    }

    /// Sets the FEU span collector for Agentics execution mode.
    pub fn with_feu_collector(mut self, collector: FeuSpanCollector) -> Self {
        self.feu_collector = Some(collector);
        self
    }

    /// Returns the agent ID.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Returns the agent version.
    pub fn version(&self) -> &str {
        AGENT_VERSION
    }

    /// Executes a workflow.
    ///
    /// This is the primary CLI endpoint for workflow execution.
    pub async fn execute(&self, request: ExecuteRequest) -> Result<ExecuteResponse> {
        let execution_id = Uuid::new_v4();
        let start_time = std::time::Instant::now();
        let started_at = Utc::now();

        // Start FEU agent span if in Agentics execution mode
        let feu_span_id = self.feu_collector.as_ref()
            .map(|c| c.start_agent_span("WorkflowOrchestratorAgent"));

        // Emit workflow start telemetry
        self.observatory
            .emit_workflow_start(execution_id, &request.workflow.name)
            .await;

        // Calculate inputs hash for reproducibility
        let inputs_hash = self.hash_inputs(&request.inputs);

        // Validate workflow
        request.workflow.validate()?;

        // Build DAG
        let dag = WorkflowDAG::from_workflow(&request.workflow)?;

        // Persist initial workflow state
        let workflow_state = WorkflowStateRecord {
            workflow_execution_id: execution_id,
            workflow_id: request.workflow.id,
            workflow_name: request.workflow.name.clone(),
            state: "running".to_string(),
            version: 1,
            inputs: serde_json::to_value(&request.inputs).unwrap_or_default(),
            outputs: None,
            error: None,
            created_at: started_at,
            updated_at: started_at,
            completed_at: None,
        };
        self.ruvector_client
            .persist_workflow_state(&workflow_state)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        // Execute workflow steps
        let context = Arc::new(ExecutionContext::new(request.inputs.clone()));
        let step_results = Arc::new(RwLock::new(HashMap::new()));

        // Get execution order from DAG
        let execution_order = dag.execution_order()?;

        // Execute steps
        let mut final_outputs = HashMap::new();
        let mut success = true;
        let mut error_message = None;

        for step_id in &execution_order {
            let step = request
                .workflow
                .steps
                .iter()
                .find(|s| &s.id == step_id)
                .ok_or_else(|| OrchestratorError::validation(format!("Step not found: {}", step_id)))?;

            // Execute step
            match self
                .execute_step(execution_id, step, context.clone(), &step_results)
                .await
            {
                Ok(outputs) => {
                    // Store outputs
                    let mut results = step_results.write().await;
                    results.insert(step_id.clone(), outputs.clone());
                    final_outputs.extend(outputs);
                }
                Err(e) => {
                    success = false;
                    error_message = Some(e.to_string());
                    break;
                }
            }
        }

        // Calculate execution metrics
        let duration = start_time.elapsed();
        let completed_at = Utc::now();

        // Calculate confidence based on execution outcome
        let confidence = self.calculate_confidence(success, &execution_order, &step_results).await;

        // Build constraints applied
        let constraints = self.build_constraints(&request, &dag, &execution_order);

        // Create decision event
        let decision_event = DecisionEventRecord {
            id: Uuid::new_v4(),
            agent_id: self.agent_id.clone(),
            agent_version: AGENT_VERSION.to_string(),
            decision_type: "workflow_execution".to_string(),
            inputs_hash: inputs_hash.clone(),
            outputs: serde_json::to_value(&final_outputs).unwrap_or_default(),
            confidence,
            constraints_applied: serde_json::to_value(&constraints).unwrap_or_default(),
            workflow_execution_id: execution_id,
            step_execution_id: None,
            trace_id: self.feu_collector.as_ref()
                .and_then(|c| c.context().trace_id.clone())
                .or_else(|| Some(Uuid::new_v4().to_string())),
            span_id: feu_span_id.clone()
                .or_else(|| Some(Uuid::new_v4().to_string())),
            timestamp: completed_at,
            metadata: HashMap::new(),
        };

        // Persist decision event (MANDATORY)
        self.ruvector_client
            .persist_decision_event(&decision_event)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        // Update workflow state
        let final_state = if success { "completed" } else { "failed" };
        let updated_state = WorkflowStateRecord {
            workflow_execution_id: execution_id,
            workflow_id: request.workflow.id,
            workflow_name: request.workflow.name.clone(),
            state: final_state.to_string(),
            version: 2,
            inputs: serde_json::to_value(&request.inputs).unwrap_or_default(),
            outputs: Some(serde_json::to_value(&final_outputs).unwrap_or_default()),
            error: error_message.clone(),
            created_at: started_at,
            updated_at: completed_at,
            completed_at: Some(completed_at),
        };
        self.ruvector_client
            .persist_workflow_state(&updated_state)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        // Emit workflow completion telemetry
        self.observatory
            .emit_workflow_complete(execution_id, success, duration)
            .await;

        // End FEU agent span if in Agentics execution mode
        if let (Some(collector), Some(span_id)) = (&self.feu_collector, &feu_span_id) {
            let feu_status = if success { SpanStatus::Ok } else { SpanStatus::Failed };
            collector.end_agent_span(&span_id, feu_status, Vec::new(), vec![
                serde_json::to_value(&decision_event).unwrap_or_default()
            ]);
        }

        Ok(ExecuteResponse {
            execution_id,
            workflow_id: request.workflow.id,
            success,
            outputs: final_outputs,
            duration_ms: duration.as_millis() as u64,
            steps_executed: execution_order.len(),
            confidence,
            inputs_hash,
            started_at,
            completed_at,
            error: error_message,
        })
    }

    /// Schedules a workflow for later execution.
    pub async fn schedule(&self, request: ScheduleRequest) -> Result<ScheduleResponse> {
        let schedule_id = Uuid::new_v4();
        let execution_id = Uuid::new_v4();

        // Validate workflow
        request.workflow.validate()?;

        // Calculate inputs hash
        let inputs_hash = self.hash_inputs(&request.inputs);

        // Create decision event for scheduling
        let decision_event = DecisionEventRecord {
            id: Uuid::new_v4(),
            agent_id: self.agent_id.clone(),
            agent_version: AGENT_VERSION.to_string(),
            decision_type: "schedule".to_string(),
            inputs_hash: inputs_hash.clone(),
            outputs: serde_json::json!({
                "schedule_id": schedule_id.to_string(),
                "execution_id": execution_id.to_string(),
                "scheduled_at": request.scheduled_at.to_rfc3339(),
            }),
            confidence: 1.0, // Scheduling is deterministic
            constraints_applied: serde_json::json!({
                "workflow": {
                    "timeout_seconds": request.workflow.timeout_seconds,
                }
            }),
            workflow_execution_id: execution_id,
            step_execution_id: None,
            trace_id: Some(Uuid::new_v4().to_string()),
            span_id: Some(Uuid::new_v4().to_string()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        // Persist decision event
        self.ruvector_client
            .persist_decision_event(&decision_event)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        // Persist scheduled workflow state
        let workflow_state = WorkflowStateRecord {
            workflow_execution_id: execution_id,
            workflow_id: request.workflow.id,
            workflow_name: request.workflow.name.clone(),
            state: "scheduled".to_string(),
            version: 1,
            inputs: serde_json::to_value(&request.inputs).unwrap_or_default(),
            outputs: None,
            error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
        };
        self.ruvector_client
            .persist_workflow_state(&workflow_state)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        Ok(ScheduleResponse {
            schedule_id,
            execution_id,
            workflow_id: request.workflow.id,
            scheduled_at: request.scheduled_at,
            inputs_hash,
        })
    }

    /// Inspects a workflow execution.
    pub async fn inspect(&self, request: InspectRequest) -> Result<InspectResponse> {
        // Query workflow state from ruvector-service
        let workflow_state = self
            .ruvector_client
            .get_workflow_state(request.execution_id)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        // Query decision events for this execution
        let decision_events = self
            .ruvector_client
            .query_by_workflow(request.execution_id, Some(100))
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        // Create decision event for inspection
        let decision_event = DecisionEventRecord {
            id: Uuid::new_v4(),
            agent_id: self.agent_id.clone(),
            agent_version: AGENT_VERSION.to_string(),
            decision_type: "inspect".to_string(),
            inputs_hash: self.hash_inputs(&HashMap::new()),
            outputs: serde_json::json!({
                "state_found": workflow_state.is_some(),
                "events_count": decision_events.len(),
            }),
            confidence: 1.0,
            constraints_applied: serde_json::json!({}),
            workflow_execution_id: request.execution_id,
            step_execution_id: None,
            trace_id: Some(Uuid::new_v4().to_string()),
            span_id: Some(Uuid::new_v4().to_string()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        self.ruvector_client
            .persist_decision_event(&decision_event)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        Ok(InspectResponse {
            execution_id: request.execution_id,
            workflow_state,
            decision_events: decision_events.len(),
            last_event: decision_events.first().cloned(),
        })
    }

    /// Replays a workflow execution.
    pub async fn replay(&self, request: ReplayRequest) -> Result<ReplayResponse> {
        // Query original workflow state
        let original_state = self
            .ruvector_client
            .get_workflow_state(request.original_execution_id)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?
            .ok_or_else(|| OrchestratorError::validation("Original execution not found"))?;

        // Parse original inputs
        let inputs: HashMap<String, serde_json::Value> =
            serde_json::from_value(original_state.inputs.clone())
                .map_err(|e| OrchestratorError::parse(e.to_string()))?;

        // Create new execution ID for replay
        let replay_execution_id = Uuid::new_v4();

        // Create decision event for replay
        let decision_event = DecisionEventRecord {
            id: Uuid::new_v4(),
            agent_id: self.agent_id.clone(),
            agent_version: AGENT_VERSION.to_string(),
            decision_type: "replay".to_string(),
            inputs_hash: self.hash_inputs(&inputs),
            outputs: serde_json::json!({
                "original_execution_id": request.original_execution_id.to_string(),
                "replay_execution_id": replay_execution_id.to_string(),
            }),
            confidence: 1.0,
            constraints_applied: serde_json::json!({}),
            workflow_execution_id: replay_execution_id,
            step_execution_id: None,
            trace_id: Some(Uuid::new_v4().to_string()),
            span_id: Some(Uuid::new_v4().to_string()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        self.ruvector_client
            .persist_decision_event(&decision_event)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        Ok(ReplayResponse {
            original_execution_id: request.original_execution_id,
            replay_execution_id,
            workflow_name: original_state.workflow_name,
            inputs_hash: self.hash_inputs(&inputs),
            started_at: Utc::now(),
        })
    }

    /// Executes a single step.
    async fn execute_step(
        &self,
        workflow_execution_id: Uuid,
        step: &Step,
        context: Arc<ExecutionContext>,
        step_results: &RwLock<HashMap<String, HashMap<String, serde_json::Value>>>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let step_execution_id = Uuid::new_v4();
        let started_at = Utc::now();
        let start_time = std::time::Instant::now();

        // Persist task execution start
        let task_execution = TaskExecutionRecord {
            id: step_execution_id,
            task_id: Uuid::new_v5(&Uuid::NAMESPACE_OID, step.id.as_bytes()),
            workflow_execution_id,
            task_name: step.id.clone(),
            task_type: format!("{:?}", step.step_type),
            state: "running".to_string(),
            attempt: 1,
            inputs: serde_json::json!({}),
            outputs: None,
            error: None,
            started_at: Some(started_at),
            completed_at: None,
            duration_ms: None,
        };
        self.ruvector_client
            .persist_task_execution(&task_execution)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        // Execute based on step type
        let result = match step.step_type {
            StepType::Llm | StepType::Embed | StepType::VectorSearch => {
                // These require provider integration - return placeholder
                let mut outputs = HashMap::new();
                for output_name in &step.output {
                    outputs.insert(
                        output_name.clone(),
                        serde_json::json!(format!("output_for_{}", output_name)),
                    );
                }
                Ok(outputs)
            }
            StepType::Transform => {
                // Transform step - placeholder implementation
                let mut outputs = HashMap::new();
                for output_name in &step.output {
                    outputs.insert(output_name.clone(), serde_json::json!("transformed"));
                }
                Ok(outputs)
            }
            StepType::Action => {
                // Action step - placeholder implementation
                Ok(HashMap::new())
            }
            StepType::Parallel | StepType::Branch => {
                // These are handled at the DAG level
                Ok(HashMap::new())
            }
        };

        let duration = start_time.elapsed();
        let completed_at = Utc::now();
        let success = result.is_ok();

        // Persist task execution completion
        let completed_execution = TaskExecutionRecord {
            id: step_execution_id,
            task_id: Uuid::new_v5(&Uuid::NAMESPACE_OID, step.id.as_bytes()),
            workflow_execution_id,
            task_name: step.id.clone(),
            task_type: format!("{:?}", step.step_type),
            state: if success { "completed" } else { "failed" }.to_string(),
            attempt: 1,
            inputs: serde_json::json!({}),
            outputs: result.as_ref().ok().map(|o| serde_json::to_value(o).unwrap_or_default()),
            error: result.as_ref().err().map(|e: &OrchestratorError| e.to_string()),
            started_at: Some(started_at),
            completed_at: Some(completed_at),
            duration_ms: Some(duration.as_millis() as u64),
        };
        self.ruvector_client
            .persist_task_execution(&completed_execution)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        // Emit step completion telemetry
        self.observatory
            .emit_step_complete(workflow_execution_id, &step.id, success, duration)
            .await;

        result
    }

    /// Hashes inputs for reproducibility.
    fn hash_inputs(&self, inputs: &HashMap<String, serde_json::Value>) -> String {
        let json = serde_json::to_string(inputs).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Calculates execution confidence.
    async fn calculate_confidence(
        &self,
        success: bool,
        execution_order: &[String],
        step_results: &RwLock<HashMap<String, HashMap<String, serde_json::Value>>>,
    ) -> f64 {
        if !success {
            return 0.0;
        }

        let results = step_results.read().await;
        let completed_steps = results.len();
        let total_steps = execution_order.len();

        if total_steps == 0 {
            return 1.0;
        }

        // Confidence = completed steps / total steps
        // This is a simplified model - production would include:
        // - Provider reliability scores
        // - Historical success rates
        // - Retry counts
        (completed_steps as f64) / (total_steps as f64)
    }

    /// Builds constraints applied during execution.
    fn build_constraints(
        &self,
        request: &ExecuteRequest,
        dag: &WorkflowDAG,
        execution_order: &[String],
    ) -> ExecutionConstraints {
        ExecutionConstraints {
            workflow: WorkflowConstraints {
                timeout_seconds: request.workflow.timeout_seconds,
                max_concurrency: Some(self.config.max_concurrency),
                strict_ordering: true,
                execution_strategy: Some("dag_based".to_string()),
            },
            dependencies: DependencyConstraints {
                dependencies_resolved: execution_order.len(),
                blocked_by: Vec::new(),
                all_satisfied: true,
            },
            retry: RetryConstraints {
                attempt: 1,
                max_attempts: self.config.max_retries,
                backoff_strategy: Some("exponential".to_string()),
                next_delay_ms: None,
            },
            resources: ResourceConstraints {
                memory_limit_bytes: None,
                cpu_limit: None,
                token_limit: self.config.max_tokens,
            },
        }
    }
}

/// Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Maximum concurrent step executions.
    pub max_concurrency: usize,
    /// Maximum retry attempts.
    pub max_retries: u32,
    /// Default timeout in seconds.
    pub timeout_seconds: u64,
    /// Maximum tokens for LLM steps.
    pub max_tokens: Option<u64>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 4,
            max_retries: 3,
            timeout_seconds: 3600,
            max_tokens: None,
        }
    }
}

/// Execute request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRequest {
    /// Workflow to execute.
    pub workflow: Workflow,
    /// Input variables.
    #[serde(default)]
    pub inputs: HashMap<String, serde_json::Value>,
}

/// Execute response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResponse {
    /// Execution ID.
    pub execution_id: Uuid,
    /// Workflow ID.
    pub workflow_id: Uuid,
    /// Whether execution was successful.
    pub success: bool,
    /// Output variables.
    pub outputs: HashMap<String, serde_json::Value>,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// Number of steps executed.
    pub steps_executed: usize,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f64,
    /// Input hash for reproducibility.
    pub inputs_hash: String,
    /// Start timestamp.
    pub started_at: DateTime<Utc>,
    /// Completion timestamp.
    pub completed_at: DateTime<Utc>,
    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Schedule request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRequest {
    /// Workflow to schedule.
    pub workflow: Workflow,
    /// Input variables.
    #[serde(default)]
    pub inputs: HashMap<String, serde_json::Value>,
    /// Scheduled execution time.
    pub scheduled_at: DateTime<Utc>,
}

/// Schedule response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleResponse {
    /// Schedule ID.
    pub schedule_id: Uuid,
    /// Future execution ID.
    pub execution_id: Uuid,
    /// Workflow ID.
    pub workflow_id: Uuid,
    /// Scheduled execution time.
    pub scheduled_at: DateTime<Utc>,
    /// Input hash.
    pub inputs_hash: String,
}

/// Inspect request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectRequest {
    /// Execution ID to inspect.
    pub execution_id: Uuid,
}

/// Inspect response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectResponse {
    /// Execution ID.
    pub execution_id: Uuid,
    /// Workflow state (if found).
    pub workflow_state: Option<WorkflowStateRecord>,
    /// Number of decision events.
    pub decision_events: usize,
    /// Last decision event.
    pub last_event: Option<DecisionEventRecord>,
}

/// Replay request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayRequest {
    /// Original execution ID to replay.
    pub original_execution_id: Uuid,
}

/// Replay response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResponse {
    /// Original execution ID.
    pub original_execution_id: Uuid,
    /// New replay execution ID.
    pub replay_execution_id: Uuid,
    /// Workflow name.
    pub workflow_name: String,
    /// Input hash.
    pub inputs_hash: String,
    /// Start timestamp.
    pub started_at: DateTime<Utc>,
}

/// Execution constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConstraints {
    /// Workflow constraints.
    pub workflow: WorkflowConstraints,
    /// Dependency constraints.
    pub dependencies: DependencyConstraints,
    /// Retry constraints.
    pub retry: RetryConstraints,
    /// Resource constraints.
    pub resources: ResourceConstraints,
}

/// Workflow constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConstraints {
    pub timeout_seconds: Option<u64>,
    pub max_concurrency: Option<usize>,
    pub strict_ordering: bool,
    pub execution_strategy: Option<String>,
}

/// Dependency constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConstraints {
    pub dependencies_resolved: usize,
    pub blocked_by: Vec<String>,
    pub all_satisfied: bool,
}

/// Retry constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConstraints {
    pub attempt: u32,
    pub max_attempts: u32,
    pub backoff_strategy: Option<String>,
    pub next_delay_ms: Option<u64>,
}

/// Resource constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConstraints {
    pub memory_limit_bytes: Option<u64>,
    pub cpu_limit: Option<f64>,
    pub token_limit: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{LlmStepConfig, StepConfig};

    fn create_test_workflow() -> Workflow {
        let mut workflow = Workflow::new("test-workflow");
        workflow.steps.push(Step {
            id: "step1".to_string(),
            step_type: StepType::Transform,
            depends_on: Vec::new(),
            condition: None,
            config: StepConfig::Transform(crate::workflow::TransformConfig {
                function: "identity".to_string(),
                inputs: vec!["input".to_string()],
                params: HashMap::new(),
            }),
            output: vec!["output1".to_string()],
            timeout_seconds: None,
            retry: None,
        });
        workflow
    }

    #[test]
    fn test_agent_creation() {
        let agent = WorkflowOrchestratorAgent::new(AgentConfig::default());
        assert!(agent.agent_id().starts_with(AGENT_ID_PREFIX));
        assert_eq!(agent.version(), AGENT_VERSION);
    }

    #[tokio::test]
    async fn test_execute_workflow() {
        let agent = WorkflowOrchestratorAgent::new(AgentConfig::default());
        let workflow = create_test_workflow();

        let request = ExecuteRequest {
            workflow,
            inputs: HashMap::new(),
        };

        let response = agent.execute(request).await.unwrap();
        assert!(response.success);
        assert_eq!(response.steps_executed, 1);
        assert!(response.confidence > 0.0);
    }

    #[test]
    fn test_hash_inputs() {
        let agent = WorkflowOrchestratorAgent::new(AgentConfig::default());

        let mut inputs1 = HashMap::new();
        inputs1.insert("key".to_string(), serde_json::json!("value"));

        let mut inputs2 = HashMap::new();
        inputs2.insert("key".to_string(), serde_json::json!("value"));

        let hash1 = agent.hash_inputs(&inputs1);
        let hash2 = agent.hash_inputs(&inputs2);

        // Same inputs should produce same hash
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 produces 64 hex chars
    }
}
