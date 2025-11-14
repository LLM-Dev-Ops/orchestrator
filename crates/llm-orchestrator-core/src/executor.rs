// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Workflow execution engine with async Tokio runtime.
//!
//! This module provides the core execution engine for running workflows
//! with support for parallel execution, retry logic, and error handling.

use crate::context::ExecutionContext;
use crate::dag::WorkflowDAG;
use crate::error::{OrchestratorError, Result};
use crate::providers::{CompletionRequest, LLMProvider};
use crate::retry::{RetryExecutor, RetryPolicy};
use crate::workflow::{BackoffStrategy, Step, StepConfig, StepType, Workflow};
use dashmap::DashMap;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

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

    /// Executes the workflow.
    ///
    /// Returns a map of step results indexed by step ID.
    pub async fn execute(&self) -> Result<HashMap<String, StepResult>> {
        info!(
            workflow_id = %self.workflow.id,
            workflow_name = %self.workflow.name,
            "Starting workflow execution"
        );

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

            let task = tokio::spawn(async move {
                let result = executor.execute_step(&step_clone).await;

                // Mark as completed
                let mut completed_guard = completed.write().await;
                completed_guard.insert(step_clone.id.clone());
                drop(completed_guard);

                result
            });

            tasks.push(task);

            // Enforce concurrency limit
            if self.max_concurrency > 0 && tasks.len() >= self.max_concurrency {
                // Wait for at least one task to complete
                if let Some(result) = tasks.first_mut() {
                    let _ = result.await;
                    tasks.remove(0);
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

        if !failures.is_empty() {
            warn!(
                "Workflow completed with {} failed steps",
                failures.len()
            );
        } else {
            info!("Workflow completed successfully");
        }

        Ok(results)
    }

    /// Waits for all dependencies of a step to complete.
    async fn wait_for_dependencies(
        &self,
        step: &Step,
        completed: &Arc<RwLock<HashSet<String>>>,
    ) -> Result<()> {
        loop {
            let completed_guard = completed.read().await;
            let all_deps_complete = step
                .depends_on
                .iter()
                .all(|dep| completed_guard.contains(dep));
            drop(completed_guard);

            if all_deps_complete {
                break;
            }

            // Wait a bit before checking again
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok(())
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
        }
    }

    /// Executes a single step with retry logic.
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

        let step_result = match result {
            Ok(outputs) => {
                info!(step_id = %step.id, duration_ms = duration.as_millis(), "Step completed successfully");
                self.step_statuses
                    .insert(step.id.clone(), StepStatus::Completed);

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

        // Call provider
        debug!(
            step_id = %step.id,
            provider = %llm_config.provider,
            model = %llm_config.model,
            "Calling LLM provider"
        );

        let response = provider
            .complete(request)
            .await
            .map_err(|e| OrchestratorError::other(format!("Provider error: {}", e)))?;

        // Build output
        let mut outputs = HashMap::new();

        // Store the main text output
        if let Some(first_output) = step.output.first() {
            outputs.insert(first_output.clone(), Value::String(response.text.clone()));
        }

        // Store full response metadata
        outputs.insert("_response".to_string(), serde_json::to_value(&response)?);

        debug!(step_id = %step.id, "LLM step completed successfully");

        Ok(outputs)
    }

    /// Executes an embedding step (placeholder).
    async fn execute_embed_step(&self, step: &Step) -> Result<HashMap<String, Value>> {
        debug!(step_id = %step.id, "Embed step execution not yet implemented");
        Err(OrchestratorError::other(
            "Embed provider integration not yet implemented",
        ))
    }

    /// Executes a vector search step (placeholder).
    async fn execute_vector_search_step(&self, step: &Step) -> Result<HashMap<String, Value>> {
        debug!(step_id = %step.id, "Vector search step execution not yet implemented");
        Err(OrchestratorError::other(
            "Vector search integration not yet implemented",
        ))
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
}
