// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Workflow execution engine with async Tokio runtime.
//!
//! This module provides the core execution engine for running workflows
//! with support for parallel execution, retry logic, and error handling.

use crate::context::ExecutionContext;
use crate::dag::WorkflowDAG;
use crate::error::{OrchestratorError, Result};
use crate::metrics;
use crate::providers::{
    CompletionRequest, EmbeddingInput, EmbeddingProvider, EmbeddingRequest, LLMProvider,
    VectorSearchProvider, VectorSearchRequest,
};
use crate::retry::{RetryExecutor, RetryPolicy};
use crate::workflow::{BackoffStrategy, Step, StepConfig, StepType, Workflow};
use dashmap::DashMap;
use futures::future::select_all;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Notify, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info, warn, instrument};

/// Execution status for a step.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StepStatus {
    /// Step is waiting for dependencies.
    Pending,
    /// Step is currently executing.
    Running,
    /// Step completed successfully.
    Completed,
    /// Step failed with an error.
    Failed,
    /// Step was skipped due to condition.
    Skipped,
}

/// Result of a step execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StepResult {
    /// Step ID.
    pub step_id: String,
    /// Execution status.
    pub status: StepStatus,
    /// Output values from the step.
    pub outputs: HashMap<String, Value>,
    /// Error message if failed.
    pub error: Option<String>,
    /// Execution duration in milliseconds.
    #[serde(serialize_with = "serialize_duration", deserialize_with = "deserialize_duration")]
    pub duration: Duration,
}

fn serialize_duration<S>(duration: &Duration, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u64(duration.as_millis() as u64)
}

fn deserialize_duration<'de, D>(deserializer: D) -> std::result::Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let millis = u64::deserialize(deserializer)?;
    Ok(Duration::from_millis(millis))
}

/// Workflow execution engine.
pub struct WorkflowExecutor {
    /// The workflow to execute.
    workflow: Workflow,
    /// DAG representation of the workflow.
    dag: WorkflowDAG,
    /// Execution context.
    context: Arc<ExecutionContext>,
    /// Step statuses.
    step_statuses: Arc<DashMap<String, StepStatus>>,
    /// Step results.
    step_results: Arc<DashMap<String, StepResult>>,
    /// Maximum concurrent steps (0 = unlimited).
    max_concurrency: usize,
    /// LLM provider registry.
    providers: Arc<DashMap<String, Arc<dyn LLMProvider>>>,
    /// Embedding provider registry.
    embedding_providers: Arc<DashMap<String, Arc<dyn EmbeddingProvider>>>,
    /// Vector database registry.
    vector_dbs: Arc<DashMap<String, Arc<dyn VectorSearchProvider>>>,
    /// Notification for step completion (for event-driven dependency waiting).
    step_completion_notify: Arc<Notify>,
}

impl WorkflowExecutor {
    /// Creates a new workflow executor.
    pub fn new(workflow: Workflow, inputs: HashMap<String, Value>) -> Result<Self> {
        // Validate workflow
        workflow.validate()?;

        // Build DAG
        let dag = WorkflowDAG::from_workflow(&workflow)?;

        // Create execution context
        let context = Arc::new(ExecutionContext::new(inputs));

        // Initialize step statuses
        let step_statuses = Arc::new(DashMap::new());
        for step in &workflow.steps {
            step_statuses.insert(step.id.clone(), StepStatus::Pending);
        }

        Ok(Self {
            workflow,
            dag,
            context,
            step_statuses,
            step_results: Arc::new(DashMap::new()),
            max_concurrency: 0, // Unlimited by default
            providers: Arc::new(DashMap::new()),
            embedding_providers: Arc::new(DashMap::new()),
            vector_dbs: Arc::new(DashMap::new()),
            step_completion_notify: Arc::new(Notify::new()),
        })
    }

    /// Sets the maximum number of concurrent steps.
    pub fn with_max_concurrency(mut self, max: usize) -> Self {
        self.max_concurrency = max;
        self
    }

    /// Registers an LLM provider.
    pub fn with_provider(self, name: impl Into<String>, provider: Arc<dyn LLMProvider>) -> Self {
        self.providers.insert(name.into(), provider);
        self
    }

    /// Registers an embedding provider.
    pub fn with_embedding_provider(self, name: impl Into<String>, provider: Arc<dyn EmbeddingProvider>) -> Self {
        self.embedding_providers.insert(name.into(), provider);
        self
    }

    /// Registers a vector database.
    pub fn with_vector_db(self, name: impl Into<String>, vector_db: Arc<dyn VectorSearchProvider>) -> Self {
        self.vector_dbs.insert(name.into(), vector_db);
        self
    }

    /// Executes the workflow.
    ///
    /// Returns a map of step results indexed by step ID.
    pub async fn execute(&self) -> Result<HashMap<String, StepResult>> {
        // Apply workflow-level timeout if specified
        let timeout_duration = Duration::from_secs(
            self.workflow.timeout_seconds.unwrap_or(3600) // Default: 1 hour
        );

        match timeout(timeout_duration, self.execute_inner()).await {
            Ok(result) => result,
            Err(_) => Err(OrchestratorError::Timeout {
                duration: timeout_duration,
            }),
        }
    }

    /// Internal execution logic (without timeout wrapper).
    #[instrument(skip(self), fields(workflow_id = %self.workflow.id, workflow_name = %self.workflow.name))]
    async fn execute_inner(&self) -> Result<HashMap<String, StepResult>> {
        info!(
            workflow_id = %self.workflow.id,
            workflow_name = %self.workflow.name,
            "Starting workflow execution"
        );

        // Record workflow start metrics
        metrics::record_workflow_start();
        let workflow_start = std::time::Instant::now();

        // Get execution order from DAG
        let execution_order = self.dag.execution_order()?;
        debug!("Execution order: {:?}", execution_order);

        // Track completed steps
        let completed_steps = Arc::new(RwLock::new(HashSet::new()));

        // Execute steps according to DAG dependencies
        let mut tasks = Vec::new();

        for step_id in execution_order {
            let step = self
                .workflow
                .steps
                .iter()
                .find(|s| s.id == step_id)
                .ok_or_else(|| OrchestratorError::StepNotFound(step_id.clone()))?;

            // Wait for dependencies
            self.wait_for_dependencies(step, &completed_steps).await?;

            // Check if we should execute based on condition
            if !self.should_execute(step)? {
                info!(step_id = %step.id, "Skipping step due to condition");
                self.mark_skipped(&step.id);
                continue;
            }

            // Execute step
            let executor = self.clone_executor_context();
            let step_clone = step.clone();
            let completed = completed_steps.clone();
            let notify = self.step_completion_notify.clone();

            let task = tokio::spawn(async move {
                let result = executor.execute_step(&step_clone).await;

                // Mark as completed
                let mut completed_guard = completed.write().await;
                completed_guard.insert(step_clone.id.clone());
                drop(completed_guard);

                // Notify all waiting tasks that a step completed
                notify.notify_waiters();

                result
            });

            tasks.push(task);

            // Enforce concurrency limit
            if self.max_concurrency > 0 && tasks.len() >= self.max_concurrency {
                // Wait for the first task to complete (any task, not just first in vec)
                let (result, _index, remaining_tasks) = select_all(tasks).await;
                tasks = remaining_tasks;

                // Log the completed task result
                if let Err(e) = result {
                    error!("Task failed: {:?}", e);
                }
            }
        }

        // Wait for all remaining tasks
        for task in tasks {
            let _ = task.await;
        }

        // Collect results
        let results: HashMap<String, StepResult> = self
            .step_results
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        // Check for failures
        let failures: Vec<_> = results
            .values()
            .filter(|r| r.status == StepStatus::Failed)
            .collect();

        let _success = failures.is_empty();
        let _workflow_duration = workflow_start.elapsed().as_secs_f64();

        // Record workflow completion metrics
        // TODO: Implement metrics module
        // metrics::record_workflow_complete(&self.workflow.name, workflow_duration, success);

        if !failures.is_empty() {
            warn!(
                "Workflow completed with {} failed steps",
                failures.len()
            );
            // Record error metric for workflow failure
            // TODO: Implement metrics module
            // metrics::record_error("workflow_failure", "executor");
        } else {
            info!("Workflow completed successfully");
        }

        Ok(results)
    }

    /// Waits for all dependencies of a step to complete.
    ///
    /// Uses event-driven notifications instead of polling for efficiency.
    async fn wait_for_dependencies(
        &self,
        step: &Step,
        completed: &Arc<RwLock<HashSet<String>>>,
    ) -> Result<()> {
        // If no dependencies, return immediately
        if step.depends_on.is_empty() {
            return Ok(());
        }

        loop {
            // Check if all dependencies are complete
            {
                let completed_guard = completed.read().await;
                let all_deps_complete = step
                    .depends_on
                    .iter()
                    .all(|dep| completed_guard.contains(dep));

                if all_deps_complete {
                    return Ok(());
                }
            } // Drop read lock

            // Wait for notification that a step completed
            self.step_completion_notify.notified().await;
        }
    }

    /// Checks if a step should execute based on its condition.
    fn should_execute(&self, step: &Step) -> Result<bool> {
        if let Some(condition) = &step.condition {
            Ok(self.context.evaluate_condition(condition)?)
        } else {
            Ok(true)
        }
    }

    /// Marks a step as skipped.
    fn mark_skipped(&self, step_id: &str) {
        self.step_statuses
            .insert(step_id.to_string(), StepStatus::Skipped);
        self.step_results.insert(
            step_id.to_string(),
            StepResult {
                step_id: step_id.to_string(),
                status: StepStatus::Skipped,
                outputs: HashMap::new(),
                error: None,
                duration: Duration::from_secs(0),
            },
        );
    }

    /// Clones the executor context for parallel execution.
    fn clone_executor_context(&self) -> Self {
        Self {
            workflow: self.workflow.clone(),
            dag: self.dag.clone(),
            context: self.context.clone(),
            step_statuses: self.step_statuses.clone(),
            step_results: self.step_results.clone(),
            max_concurrency: self.max_concurrency,
            providers: self.providers.clone(),
            embedding_providers: self.embedding_providers.clone(),
            vector_dbs: self.vector_dbs.clone(),
            step_completion_notify: self.step_completion_notify.clone(),
        }
    }

    /// Executes a single step with retry logic.
    #[instrument(skip(self, step), fields(step_id = %step.id, step_type = ?step.step_type))]
    async fn execute_step(&self, step: &Step) -> Result<StepResult> {
        let start = std::time::Instant::now();

        info!(step_id = %step.id, step_type = ?step.step_type, "Executing step");

        // Update status to running
        self.step_statuses
            .insert(step.id.clone(), StepStatus::Running);

        // Get retry policy from step config or use default
        let retry_policy = self.get_retry_policy(step);
        let retry_executor = RetryExecutor::new(retry_policy);

        // Execute with retry
        let result = retry_executor
            .execute(|| async {
                // Apply timeout if configured
                if let Some(timeout_secs) = step.timeout_seconds {
                    let timeout_duration = Duration::from_secs(timeout_secs);
                    match timeout(timeout_duration, self.execute_step_inner(step)).await {
                        Ok(result) => result,
                        Err(_) => Err(OrchestratorError::Timeout {
                            duration: timeout_duration,
                        }),
                    }
                } else {
                    self.execute_step_inner(step).await
                }
            })
            .await;

        let duration = start.elapsed();

        // Get step type string for metrics
        let _step_type_str = format!("{:?}", step.step_type).to_lowercase();

        let step_result = match result {
            Ok(outputs) => {
                info!(step_id = %step.id, duration_ms = duration.as_millis(), "Step completed successfully");
                self.step_statuses
                    .insert(step.id.clone(), StepStatus::Completed);

                // Record step success metrics
                // TODO: Implement metrics module
                // metrics::record_step_execution(&step_type_str, duration.as_secs_f64(), "success");

                // Store outputs in context as a JSON object
                let outputs_json = serde_json::to_value(&outputs)
                    .unwrap_or_else(|_| Value::Object(serde_json::Map::new()));
                self.context.set_output(&step.id, outputs_json);

                StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Completed,
                    outputs,
                    error: None,
                    duration,
                }
            }
            Err(err) => {
                error!(step_id = %step.id, error = %err, "Step failed");
                self.step_statuses
                    .insert(step.id.clone(), StepStatus::Failed);

                // Record step failure metrics
                // TODO: Implement metrics module
                // metrics::record_step_execution(&step_type_str, duration.as_secs_f64(), "failure");

                // Determine error type for error metrics
                let _error_type = if err.to_string().contains("timeout") {
                    "timeout"
                } else if err.to_string().contains("provider") {
                    "provider_error"
                } else {
                    "execution_error"
                };
                // TODO: Implement metrics module
                // metrics::record_error(error_type, "step_executor");

                StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Failed,
                    outputs: HashMap::new(),
                    error: Some(err.to_string()),
                    duration,
                }
            }
        };

        // Store result
        self.step_results
            .insert(step.id.clone(), step_result.clone());

        Ok(step_result)
    }

    /// Inner step execution logic (actual work).
    async fn execute_step_inner(&self, step: &Step) -> Result<HashMap<String, Value>> {
        match &step.step_type {
            StepType::Llm => self.execute_llm_step(step).await,
            StepType::Embed => self.execute_embed_step(step).await,
            StepType::VectorSearch => self.execute_vector_search_step(step).await,
            StepType::Transform => self.execute_transform_step(step).await,
            StepType::Action => self.execute_action_step(step).await,
            StepType::Parallel => self.execute_parallel_step(step).await,
            StepType::Branch => self.execute_branch_step(step).await,
        }
    }

    /// Gets the retry policy for a step.
    fn get_retry_policy(&self, step: &Step) -> RetryPolicy {
        if let Some(retry_config) = &step.retry {
            // Convert BackoffStrategy to multiplier
            let multiplier = match retry_config.backoff {
                BackoffStrategy::Exponential => 2.0,
                BackoffStrategy::Linear => 1.0,
                BackoffStrategy::Constant => 1.0,
            };

            RetryPolicy::new(
                retry_config.max_attempts,
                Duration::from_millis(retry_config.initial_delay_ms),
                multiplier,
                Duration::from_millis(retry_config.max_delay_ms),
            )
        } else {
            RetryPolicy::default()
        }
    }

    /// Executes an LLM step using the registered provider.
    async fn execute_llm_step(&self, step: &Step) -> Result<HashMap<String, Value>> {
        // Extract LLM config
        let llm_config = match &step.config {
            StepConfig::Llm(config) => config,
            _ => {
                return Err(OrchestratorError::InvalidStepConfig {
                    step_id: step.id.clone(),
                    reason: "Expected LLM step config".to_string(),
                })
            }
        };

        // Get provider
        let provider = self
            .providers
            .get(&llm_config.provider)
            .ok_or_else(|| OrchestratorError::other(format!(
                "Provider '{}' not registered",
                llm_config.provider
            )))?;

        // Render prompt template
        let rendered_prompt = self.context.render_template(&llm_config.prompt)?;

        // Build completion request
        let request = CompletionRequest {
            model: llm_config.model.clone(),
            prompt: rendered_prompt,
            system: llm_config.system.clone(),
            temperature: llm_config.temperature,
            max_tokens: llm_config.max_tokens,
            extra: llm_config.extra.clone(),
        };

        // Call provider with metrics
        debug!(
            step_id = %step.id,
            provider = %llm_config.provider,
            model = %llm_config.model,
            "Calling LLM provider"
        );

        let llm_start = std::time::Instant::now();
        let response_result = provider.complete(request).await;
        let llm_duration = llm_start.elapsed().as_secs_f64();

        let response = match response_result {
            Ok(resp) => {
                // Record successful LLM request
                let input_tokens = resp.metadata.get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .map(|t| t as u32);
                let output_tokens = resp.metadata.get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .map(|t| t as u32);

                metrics::record_llm_request(
                    &llm_config.provider,
                    &llm_config.model,
                    llm_duration,
                    true,
                    input_tokens,
                    output_tokens,
                );

                resp
            }
            Err(e) => {
                // Record failed LLM request
                metrics::record_llm_request(
                    &llm_config.provider,
                    &llm_config.model,
                    llm_duration,
                    false,
                    None,
                    None,
                );

                return Err(OrchestratorError::other(format!("Provider error: {}", e)));
            }
        };

        // Build output
        let mut outputs = HashMap::new();

        // Validate that step has at least one output
        if step.output.is_empty() {
            return Err(OrchestratorError::InvalidStepConfig {
                step_id: step.id.clone(),
                reason: "LLM step must specify at least one output variable".to_string(),
            });
        }

        // Store the main text output in first output variable
        outputs.insert(
            step.output[0].clone(),
            Value::String(response.text.clone())
        );

        // Store metadata in additional output variables if specified
        if step.output.len() > 1 && step.output.len() >= 2 {
            // Second output: model name
            outputs.insert(
                step.output[1].clone(),
                Value::String(response.model.clone())
            );
        }

        if step.output.len() >= 3 {
            // Third output: token usage
            if let Some(tokens) = response.tokens_used {
                outputs.insert(
                    step.output[2].clone(),
                    Value::Number(serde_json::Number::from(tokens))
                );
            }
        }

        if step.output.len() >= 4 {
            // Fourth output: full metadata
            outputs.insert(
                step.output[3].clone(),
                serde_json::to_value(&response.metadata)?
            );
        }

        // Always store full response metadata under special key for debugging
        outputs.insert("_response".to_string(), serde_json::to_value(&response)?);

        debug!(step_id = %step.id, "LLM step completed successfully");

        Ok(outputs)
    }

    /// Executes an embedding step.
    async fn execute_embed_step(&self, step: &Step) -> Result<HashMap<String, Value>> {
        // Extract embedding config
        let embed_config = match &step.config {
            StepConfig::Embed(config) => config,
            _ => {
                return Err(OrchestratorError::InvalidStepConfig {
                    step_id: step.id.clone(),
                    reason: "Expected Embed step config".to_string(),
                })
            }
        };

        // Get embedding provider
        let provider = self
            .embedding_providers
            .get(&embed_config.provider)
            .ok_or_else(|| OrchestratorError::other(format!(
                "Embedding provider '{}' not registered",
                embed_config.provider
            )))?;

        // Render input template
        let rendered_input = self.context.render_template(&embed_config.input)?;

        // Build embedding request
        let request = EmbeddingRequest {
            model: embed_config.model.clone(),
            input: EmbeddingInput::Single {
                input: rendered_input,
            },
            dimensions: embed_config.dimensions,
            extra: HashMap::new(),
        };

        // Call provider
        debug!(
            step_id = %step.id,
            provider = %embed_config.provider,
            model = %embed_config.model,
            "Calling embedding provider"
        );

        let response = provider
            .embed(request)
            .await
            .map_err(|e| OrchestratorError::other(format!("Embedding provider error: {}", e)))?;

        // Build output
        let mut outputs = HashMap::new();

        // Validate that step has at least one output
        if step.output.is_empty() {
            return Err(OrchestratorError::InvalidStepConfig {
                step_id: step.id.clone(),
                reason: "Embed step must specify at least one output variable".to_string(),
            });
        }

        // Store the embedding vector in first output variable
        if !response.embeddings.is_empty() {
            outputs.insert(
                step.output[0].clone(),
                serde_json::to_value(&response.embeddings[0])?
            );
        }

        // Store metadata in second output variable if specified
        if step.output.len() > 1 {
            let metadata = serde_json::json!({
                "model": response.model,
                "dimensions": response.embeddings.first().map(|e| e.len()).unwrap_or(0),
                "tokens_used": response.tokens_used,
            });
            outputs.insert(step.output[1].clone(), metadata);
        }

        // Always store full response under special key for debugging
        outputs.insert("_response".to_string(), serde_json::to_value(&response)?);

        debug!(step_id = %step.id, "Embedding step completed successfully");

        Ok(outputs)
    }

    /// Executes a vector search step.
    async fn execute_vector_search_step(&self, step: &Step) -> Result<HashMap<String, Value>> {
        // Extract vector search config
        let search_config = match &step.config {
            StepConfig::VectorSearch(config) => config,
            _ => {
                return Err(OrchestratorError::InvalidStepConfig {
                    step_id: step.id.clone(),
                    reason: "Expected VectorSearch step config".to_string(),
                })
            }
        };

        // Get vector database
        let vector_db = self
            .vector_dbs
            .get(&search_config.database)
            .ok_or_else(|| OrchestratorError::other(format!(
                "Vector database '{}' not registered",
                search_config.database
            )))?;

        // Render query template to get the vector
        let rendered_query = self.context.render_template(&search_config.query)?;

        // Parse the query - it should be a JSON array of floats (the embedding vector)
        let query_vector: Vec<f32> = serde_json::from_str(&rendered_query)
            .map_err(|e| OrchestratorError::other(format!(
                "Failed to parse query vector: {}. Expected JSON array of floats, got: {}",
                e, rendered_query
            )))?;

        // Build search request
        let request = VectorSearchRequest {
            index: search_config.index.clone(),
            query: query_vector,
            top_k: search_config.top_k,
            namespace: search_config.namespace.clone(),
            filter: search_config.filter.clone(),
            include_metadata: search_config.include_metadata,
            include_vectors: search_config.include_vectors,
        };

        // Call vector database
        debug!(
            step_id = %step.id,
            database = %search_config.database,
            index = %search_config.index,
            top_k = search_config.top_k,
            "Calling vector database"
        );

        let response = vector_db
            .search(request)
            .await
            .map_err(|e| OrchestratorError::other(format!("Vector search error: {}", e)))?;

        // Build output
        let mut outputs = HashMap::new();

        // Validate that step has at least one output
        if step.output.is_empty() {
            return Err(OrchestratorError::InvalidStepConfig {
                step_id: step.id.clone(),
                reason: "VectorSearch step must specify at least one output variable".to_string(),
            });
        }

        // Format results for easier template access
        let formatted_results: Vec<Value> = response
            .results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "score": r.score,
                    "metadata": r.metadata,
                    "vector": r.vector,
                })
            })
            .collect();

        // Store the search results in first output variable
        outputs.insert(
            step.output[0].clone(),
            Value::Array(formatted_results)
        );

        // Store metadata in second output variable if specified
        if step.output.len() > 1 {
            let metadata = serde_json::json!({
                "count": response.results.len(),
                "top_k": search_config.top_k,
                "database": search_config.database,
                "index": search_config.index,
            });
            outputs.insert(step.output[1].clone(), metadata);
        }

        // Always store full response under special key for debugging
        outputs.insert("_response".to_string(), serde_json::to_value(&response)?);

        debug!(
            step_id = %step.id,
            results_count = response.results.len(),
            "Vector search step completed successfully"
        );

        Ok(outputs)
    }

    /// Executes a transform step.
    async fn execute_transform_step(&self, step: &Step) -> Result<HashMap<String, Value>> {
        debug!(step_id = %step.id, "Transform step execution");

        // For now, just return empty outputs
        // This will be expanded with actual transform functions
        Ok(HashMap::new())
    }

    /// Executes an action step.
    async fn execute_action_step(&self, step: &Step) -> Result<HashMap<String, Value>> {
        debug!(step_id = %step.id, "Action step execution");

        // For now, just log and return empty outputs
        // This will be expanded with actual actions
        Ok(HashMap::new())
    }

    /// Executes a parallel step.
    async fn execute_parallel_step(&self, step: &Step) -> Result<HashMap<String, Value>> {
        debug!(step_id = %step.id, "Parallel step execution");

        // This will spawn multiple sub-workflows in parallel
        // For now, return empty outputs
        Ok(HashMap::new())
    }

    /// Executes a branch step.
    async fn execute_branch_step(&self, step: &Step) -> Result<HashMap<String, Value>> {
        debug!(step_id = %step.id, "Branch step execution");

        // This will evaluate conditions and route to different branches
        // For now, return empty outputs
        Ok(HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{LlmStepConfig, RetryConfig, StepConfig};

    fn create_test_workflow() -> Workflow {
        Workflow {
            id: uuid::Uuid::new_v4(),
            name: "test-workflow".to_string(),
            version: "1.0".to_string(),
            description: Some("Test workflow".to_string()),
            timeout_seconds: None,
            steps: vec![
                Step {
                    id: "step1".to_string(),
                    step_type: StepType::Llm,
                    depends_on: vec![],
                    condition: None,
                    config: StepConfig::Llm(LlmStepConfig {
                        provider: "openai".to_string(),
                        model: "gpt-4".to_string(),
                        prompt: "Test prompt".to_string(),
                        temperature: Some(0.7),
                        max_tokens: Some(100),
                        system: None,
                        stream: false,
                        extra: HashMap::new(),
                    }),
                    output: vec!["result".to_string()],
                    timeout_seconds: None,
                    retry: None,
                },
                Step {
                    id: "step2".to_string(),
                    step_type: StepType::Transform,
                    depends_on: vec!["step1".to_string()],
                    condition: None,
                    config: StepConfig::Transform(crate::workflow::TransformConfig {
                        function: "test".to_string(),
                        inputs: vec![],
                        params: HashMap::new(),
                    }),
                    output: vec!["transformed".to_string()],
                    timeout_seconds: None,
                    retry: None,
                },
            ],
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_executor_creation() {
        let workflow = create_test_workflow();
        let inputs = HashMap::new();

        let executor = WorkflowExecutor::new(workflow, inputs);
        assert!(executor.is_ok());
    }

    #[test]
    fn test_executor_with_max_concurrency() {
        let workflow = create_test_workflow();
        let inputs = HashMap::new();

        let executor = WorkflowExecutor::new(workflow, inputs)
            .unwrap()
            .with_max_concurrency(5);

        assert_eq!(executor.max_concurrency, 5);
    }

    #[test]
    fn test_retry_policy_from_config() {
        let workflow = create_test_workflow();
        let inputs = HashMap::new();
        let executor = WorkflowExecutor::new(workflow, inputs).unwrap();

        let step = Step {
            id: "test".to_string(),
            step_type: StepType::Llm,
            depends_on: vec![],
            condition: None,
            config: StepConfig::Llm(LlmStepConfig {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                prompt: "Test".to_string(),
                temperature: None,
                max_tokens: None,
                system: None,
                stream: false,
                extra: HashMap::new(),
            }),
            output: vec![],
            timeout_seconds: None,
            retry: Some(RetryConfig {
                max_attempts: 5,
                backoff: BackoffStrategy::Exponential,
                initial_delay_ms: 200,
                max_delay_ms: 10000,
            }),
        };

        let policy = executor.get_retry_policy(&step);
        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_delay, Duration::from_millis(200));
        assert_eq!(policy.multiplier, 2.0); // Exponential = 2.0 multiplier
        assert_eq!(policy.max_delay, Duration::from_millis(10000));
    }

    #[tokio::test]
    async fn test_transform_step_execution() {
        let workflow = Workflow {
            id: uuid::Uuid::new_v4(),
            name: "transform-test".to_string(),
            version: "1.0".to_string(),
            description: None,
            timeout_seconds: None,
            steps: vec![Step {
                id: "transform1".to_string(),
                step_type: StepType::Transform,
                depends_on: vec![],
                condition: None,
                config: StepConfig::Transform(crate::workflow::TransformConfig {
                    function: "test".to_string(),
                    inputs: vec![],
                    params: HashMap::new(),
                }),
                output: vec!["result".to_string()],
                timeout_seconds: None,
                retry: None,
            }],
            metadata: HashMap::new(),
        };

        let inputs = HashMap::new();
        let executor = WorkflowExecutor::new(workflow, inputs).unwrap();

        // Execute the workflow
        let results = executor.execute().await;

        // Since transform is a placeholder, it should complete with empty outputs
        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results["transform1"].status, StepStatus::Completed);
    }

    #[tokio::test]
    async fn test_step_with_condition_skip() {
        let workflow = Workflow {
            id: uuid::Uuid::new_v4(),
            name: "condition-test".to_string(),
            version: "1.0".to_string(),
            description: None,
            timeout_seconds: None,
            steps: vec![Step {
                id: "conditional".to_string(),
                step_type: StepType::Action,
                depends_on: vec![],
                condition: Some("false".to_string()), // Always false
                config: StepConfig::Action(crate::workflow::ActionConfig {
                    action: "test".to_string(),
                    params: HashMap::new(),
                }),
                output: vec![],
                timeout_seconds: None,
                retry: None,
            }],
            metadata: HashMap::new(),
        };

        let inputs = HashMap::new();
        let executor = WorkflowExecutor::new(workflow, inputs).unwrap();

        let results = executor.execute().await.unwrap();
        assert_eq!(results["conditional"].status, StepStatus::Skipped);
    }

    // RAG Pipeline Integration Tests

    /// Mock embedding provider for testing
    struct MockEmbeddingProvider;

    #[async_trait::async_trait]
    impl crate::providers::EmbeddingProvider for MockEmbeddingProvider {
        async fn embed(&self, request: crate::providers::EmbeddingRequest) -> std::result::Result<crate::providers::EmbeddingResponse, crate::providers::ProviderError> {
            // Return a mock embedding vector (384 dimensions, typical for sentence transformers)
            let embedding = vec![0.1_f32; 384];

            Ok(crate::providers::EmbeddingResponse {
                embeddings: vec![embedding],
                model: request.model.clone(),
                tokens_used: Some(10),
                metadata: HashMap::new(),
            })
        }

        fn name(&self) -> &str {
            "mock_embeddings"
        }
    }

    /// Mock vector search provider for testing
    struct MockVectorSearchProvider;

    #[async_trait::async_trait]
    impl crate::providers::VectorSearchProvider for MockVectorSearchProvider {
        async fn search(&self, _request: crate::providers::VectorSearchRequest) -> std::result::Result<crate::providers::VectorSearchResponse, crate::providers::ProviderError> {
            use crate::providers::SearchResult;

            // Return mock search results
            let results = vec![
                SearchResult {
                    id: "doc1".to_string(),
                    score: 0.95,
                    metadata: Some(serde_json::json!({
                        "text": "This is a test document about Rust programming.",
                        "source": "test_db"
                    })),
                    vector: None,
                },
                SearchResult {
                    id: "doc2".to_string(),
                    score: 0.87,
                    metadata: Some(serde_json::json!({
                        "text": "Another document about Rust ownership and borrowing.",
                        "source": "test_db"
                    })),
                    vector: None,
                },
            ];

            Ok(crate::providers::VectorSearchResponse {
                results,
                metadata: HashMap::new(),
            })
        }

        async fn upsert(&self, _request: crate::providers::UpsertRequest) -> std::result::Result<crate::providers::UpsertResponse, crate::providers::ProviderError> {
            Ok(crate::providers::UpsertResponse {
                upserted_count: 1,
                metadata: HashMap::new(),
            })
        }

        async fn delete(&self, _request: crate::providers::DeleteRequest) -> std::result::Result<crate::providers::DeleteResponse, crate::providers::ProviderError> {
            Ok(crate::providers::DeleteResponse {
                deleted_count: 1,
                metadata: HashMap::new(),
            })
        }

        fn name(&self) -> &str {
            "mock_vectordb"
        }
    }

    #[tokio::test]
    async fn test_embed_step_execution() {
        use crate::workflow::EmbedStepConfig;

        let workflow = Workflow {
            id: uuid::Uuid::new_v4(),
            name: "embed-test".to_string(),
            version: "1.0".to_string(),
            description: None,
            timeout_seconds: None,
            steps: vec![Step {
                id: "embed1".to_string(),
                step_type: StepType::Embed,
                depends_on: vec![],
                condition: None,
                config: StepConfig::Embed(EmbedStepConfig {
                    provider: "mock".to_string(),
                    model: "test-model".to_string(),
                    input: "Test text to embed: {{ query }}".to_string(),
                    dimensions: Some(384),
                    batch_size: None,
                }),
                output: vec!["embedding".to_string(), "metadata".to_string()],
                timeout_seconds: None,
                retry: None,
            }],
            metadata: HashMap::new(),
        };

        let mut inputs = HashMap::new();
        inputs.insert("query".to_string(), serde_json::json!("what is rust?"));

        let executor = WorkflowExecutor::new(workflow, inputs)
            .unwrap()
            .with_embedding_provider("mock", Arc::new(MockEmbeddingProvider));

        let results = executor.execute().await;
        assert!(results.is_ok(), "Embed step should complete successfully");

        let results = results.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results["embed1"].status, StepStatus::Completed);

        // Verify outputs
        let outputs = &results["embed1"].outputs;
        assert!(outputs.contains_key("embedding"), "Should have embedding output");
        assert!(outputs.contains_key("metadata"), "Should have metadata output");
    }

    #[tokio::test]
    async fn test_vector_search_step_execution() {
        use crate::workflow::VectorSearchConfig;

        let workflow = Workflow {
            id: uuid::Uuid::new_v4(),
            name: "search-test".to_string(),
            version: "1.0".to_string(),
            description: None,
            timeout_seconds: None,
            steps: vec![Step {
                id: "search1".to_string(),
                step_type: StepType::VectorSearch,
                depends_on: vec![],
                condition: None,
                config: StepConfig::VectorSearch(VectorSearchConfig {
                    database: "mock".to_string(),
                    index: "test-index".to_string(),
                    query: "[0.1, 0.2, 0.3]".to_string(), // Mock vector
                    top_k: 5,
                    filter: None,
                    namespace: None,
                    include_metadata: true,
                    include_vectors: false,
                }),
                output: vec!["results".to_string(), "metadata".to_string()],
                timeout_seconds: None,
                retry: None,
            }],
            metadata: HashMap::new(),
        };

        let inputs = HashMap::new();
        let executor = WorkflowExecutor::new(workflow, inputs)
            .unwrap()
            .with_vector_db("mock", Arc::new(MockVectorSearchProvider));

        let results = executor.execute().await;
        assert!(results.is_ok(), "Vector search step should complete successfully");

        let results = results.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results["search1"].status, StepStatus::Completed);

        // Verify outputs
        let outputs = &results["search1"].outputs;
        assert!(outputs.contains_key("results"), "Should have results output");

        // Verify results structure
        if let Some(Value::Array(results_array)) = outputs.get("results") {
            assert_eq!(results_array.len(), 2, "Should return 2 mock results");
        } else {
            panic!("Results should be an array");
        }
    }

    #[tokio::test]
    async fn test_rag_pipeline_integration() {
        use crate::workflow::{EmbedStepConfig, VectorSearchConfig};

        // Full RAG pipeline: Embed -> VectorSearch
        let workflow = Workflow {
            id: uuid::Uuid::new_v4(),
            name: "rag-pipeline-test".to_string(),
            version: "1.0".to_string(),
            description: Some("Complete RAG pipeline test".to_string()),
            timeout_seconds: None,
            steps: vec![
                Step {
                    id: "embed_query".to_string(),
                    step_type: StepType::Embed,
                    depends_on: vec![],
                    condition: None,
                    config: StepConfig::Embed(EmbedStepConfig {
                        provider: "mock".to_string(),
                        model: "test-embeddings".to_string(),
                        input: "{{ inputs.query }}".to_string(),
                        dimensions: Some(384),
                        batch_size: None,
                    }),
                    output: vec!["query_vector".to_string()],
                    timeout_seconds: None,
                    retry: None,
                },
                Step {
                    id: "search_docs".to_string(),
                    step_type: StepType::VectorSearch,
                    depends_on: vec!["embed_query".to_string()],
                    condition: None,
                    config: StepConfig::VectorSearch(VectorSearchConfig {
                        database: "mock".to_string(),
                        index: "knowledge-base".to_string(),
                        query: "{{ steps.embed_query.query_vector }}".to_string(),
                        top_k: 3,
                        filter: None,
                        namespace: None,
                        include_metadata: true,
                        include_vectors: false,
                    }),
                    output: vec!["search_results".to_string()],
                    timeout_seconds: None,
                    retry: None,
                },
            ],
            metadata: HashMap::new(),
        };

        let mut inputs = HashMap::new();
        inputs.insert("query".to_string(), serde_json::json!("What is Rust?"));

        let executor = WorkflowExecutor::new(workflow, inputs)
            .unwrap()
            .with_embedding_provider("mock", Arc::new(MockEmbeddingProvider))
            .with_vector_db("mock", Arc::new(MockVectorSearchProvider));

        let results = executor.execute().await;
        assert!(results.is_ok(), "RAG pipeline should complete successfully: {:?}", results);

        let results = results.unwrap();
        assert_eq!(results.len(), 2, "Both steps should complete");

        // Print results for debugging
        for (step_id, result) in &results {
            println!("Step {}: status={:?}, error={:?}", step_id, result.status, result.error);
        }

        // Verify embed step
        if results["embed_query"].status != StepStatus::Completed {
            panic!("Embed step failed: {:?}", results["embed_query"].error);
        }
        assert!(results["embed_query"].outputs.contains_key("query_vector"));

        // Verify search step
        if results["search_docs"].status != StepStatus::Completed {
            panic!("Search step failed: {:?}", results["search_docs"].error);
        }
        assert!(results["search_docs"].outputs.contains_key("search_results"));
    }
}
