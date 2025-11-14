# LLM DevOps Ecosystem Integration Guide

**Version:** 1.0
**Last Updated:** 2025-11-14

---

## Table of Contents

1. [Overview](#overview)
2. [LLM-Forge Integration](#llm-forge-integration)
3. [LLM-Test-Bench Integration](#llm-test-bench-integration)
4. [LLM-Auto-Optimizer Integration](#llm-auto-optimizer-integration)
5. [LLM-Governance-Core Integration](#llm-governance-core-integration)
6. [Cross-System Data Flow](#cross-system-data-flow)
7. [Implementation Examples](#implementation-examples)

---

## Overview

LLM-Orchestrator serves as the central execution engine in the LLM DevOps ecosystem, coordinating workflows across four core systems:

```
┌──────────────────────────────────────────────────────────────────┐
│                     LLM DevOps Ecosystem                         │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────┐         ┌────────────────────────────┐      │
│  │  LLM-Forge     │────────▶│  LLM-Orchestrator          │      │
│  │  (Tools/SDKs)  │         │  (Workflow Engine)         │      │
│  └────────────────┘         └────────────┬───────────────┘      │
│         ▲                                 │                      │
│         │                                 │                      │
│         │                                 ▼                      │
│  ┌──────┴──────────┐         ┌────────────────────────────┐     │
│  │  LLM-Auto-      │◀────────│  LLM-Governance-Core       │     │
│  │  Optimizer      │         │  (Policy/Cost)             │     │
│  │  (Performance)  │         └────────────────────────────┘     │
│  └─────────────────┘                     │                      │
│         ▲                                 │                      │
│         │                                 ▼                      │
│  ┌──────┴──────────┐                                            │
│  │  LLM-Test-Bench │                                            │
│  │  (Evaluation)   │                                            │
│  └─────────────────┘                                            │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### Integration Patterns

1. **LLM-Forge**: Tool invocation and SDK execution
2. **LLM-Test-Bench**: Evaluation callbacks and quality metrics
3. **LLM-Auto-Optimizer**: Performance metric collection and optimization
4. **LLM-Governance-Core**: Policy enforcement and cost tracking

---

## LLM-Forge Integration

### Overview

LLM-Forge provides a unified interface for LLM tools and SDKs. The orchestrator integrates with Forge to:
- Invoke LLM APIs (Claude, GPT-4, Gemini, etc.)
- Execute custom tools and functions
- Manage model configurations
- Handle authentication and rate limiting

### Integration Architecture

```rust
// Integration layer
pub struct LlmForgeIntegration {
    client: llm_forge::Client,
    tool_registry: Arc<ToolRegistry>,
    model_registry: Arc<ModelRegistry>,
    auth_manager: Arc<AuthManager>,
}

// Configuration
pub struct ForgeConfig {
    pub endpoint: String,
    pub api_keys: HashMap<String, String>,
    pub default_model: String,
    pub timeout: Duration,
    pub retry_policy: RetryPolicy,
}
```

### Executor Implementation

```rust
#[async_trait]
impl TaskExecutor for LlmForgeExecutor {
    async fn execute(
        &self,
        inputs: HashMap<String, Value>,
        context: TaskContext,
    ) -> Result<TaskOutput> {
        // 1. Extract parameters
        let prompt = inputs.get("prompt")
            .and_then(|v| v.as_str())
            .ok_or(ExecutorError::MissingInput("prompt"))?;

        let config = inputs.get("config")
            .cloned()
            .unwrap_or_default();

        // 2. Build request
        let request = llm_forge::InferenceRequest {
            prompt: prompt.to_string(),
            model: config.get("model")
                .and_then(|v| v.as_str())
                .unwrap_or(&self.default_model),
            temperature: config.get("temperature")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7),
            max_tokens: config.get("max_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(1000) as i32,
            // ... additional parameters
        };

        // 3. Execute via LLM-Forge
        let response = self.client
            .inference(request)
            .await
            .map_err(|e| ExecutorError::ForgeError(e))?;

        // 4. Return output
        Ok(TaskOutput {
            values: HashMap::from([
                ("response".to_string(), json!(response.text)),
                ("usage".to_string(), json!({
                    "prompt_tokens": response.usage.prompt_tokens,
                    "completion_tokens": response.usage.completion_tokens,
                    "total_tokens": response.usage.total_tokens,
                })),
                ("model".to_string(), json!(response.model)),
            ]),
            metadata: TaskMetadata {
                execution_time: response.latency,
                cost: Some(calculate_cost(&response.usage, &response.model)),
                ..Default::default()
            },
        })
    }
}
```

### Workflow Example

```yaml
apiVersion: orchestrator.llm/v1
kind: Workflow
metadata:
  name: multi-model-comparison
  version: 1.0.0

spec:
  inputs:
    - name: prompt
      type: string
      required: true

  tasks:
    # Task 1: Claude inference via LLM-Forge
    - id: claude_inference
      type: llm
      executor: llm-forge:claude-api
      inputs:
        prompt: ${{ workflow.inputs.prompt }}
        config:
          model: claude-sonnet-4-5-20250929
          temperature: 0.7
          max_tokens: 1000
      outputs:
        response: string
        usage: object

    # Task 2: GPT-4 inference via LLM-Forge
    - id: gpt4_inference
      type: llm
      executor: llm-forge:openai-api
      inputs:
        prompt: ${{ workflow.inputs.prompt }}
        config:
          model: gpt-4-turbo
          temperature: 0.7
          max_tokens: 1000
      outputs:
        response: string
        usage: object

    # Task 3: Custom tool via LLM-Forge
    - id: format_comparison
      type: custom
      executor: llm-forge:tool:response-formatter
      dependsOn:
        - claude_inference
        - gpt4_inference
      inputs:
        responses:
          - model: Claude
            text: ${{ tasks.claude_inference.outputs.response }}
          - model: GPT-4
            text: ${{ tasks.gpt4_inference.outputs.response }}
      outputs:
        formatted_output: string

  outputs:
    comparison_result:
      value: ${{ tasks.format_comparison.outputs.formatted_output }}
```

### API Contract

#### Request Format

```rust
pub struct ForgeInferenceRequest {
    pub prompt: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: i32,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop_sequences: Vec<String>,
    pub metadata: HashMap<String, Value>,
}
```

#### Response Format

```rust
pub struct ForgeInferenceResponse {
    pub text: String,
    pub model: String,
    pub usage: TokenUsage,
    pub latency: Duration,
    pub metadata: HashMap<String, Value>,
}

pub struct TokenUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}
```

### Error Handling

```rust
#[derive(Debug, Error)]
pub enum ForgeError {
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Invalid API key")]
    AuthenticationFailed,

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

// Retry policy for Forge errors
impl RetryableError for ForgeError {
    fn is_retryable(&self) -> bool {
        matches!(
            self,
            ForgeError::RateLimitExceeded(_)
                | ForgeError::Timeout
                | ForgeError::ServiceUnavailable(_)
        )
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            ForgeError::RateLimitExceeded(_) => Some(Duration::from_secs(60)),
            _ => None,
        }
    }
}
```

---

## LLM-Test-Bench Integration

### Overview

LLM-Test-Bench provides evaluation and testing capabilities for LLM outputs. The orchestrator integrates to:
- Execute evaluations as workflow tasks
- Register lifecycle hooks for automatic testing
- Collect quality metrics
- Generate test reports

### Integration Architecture

```rust
pub struct TestBenchIntegration {
    client: llm_test_bench::Client,
    hook_registry: Arc<HookRegistry>,
    metrics_collector: Arc<MetricsCollector>,
}

pub struct TestBenchConfig {
    pub endpoint: String,
    pub auto_evaluate: bool,
    pub metrics: Vec<String>,
    pub ground_truth_source: Option<String>,
}
```

### Lifecycle Hooks

```rust
impl TestBenchIntegration {
    /// Register automatic evaluation hooks
    pub fn register_auto_evaluation(&self, orchestrator: &mut Orchestrator) {
        // Hook: After LLM task completion
        orchestrator.on_task_complete(|task, output, context| {
            if task.task_type == TaskType::Llm {
                let client = self.client.clone();

                async move {
                    // Auto-evaluate LLM response
                    let eval_result = client.evaluate(EvaluationRequest {
                        task_id: task.id.clone(),
                        response: output.values.get("response")?.clone(),
                        ground_truth: context.ground_truth.clone(),
                        metrics: vec![
                            "semantic_similarity",
                            "factual_consistency",
                            "toxicity",
                        ],
                    }).await?;

                    // Store evaluation results
                    context.metadata.insert(
                        "evaluation".to_string(),
                        json!(eval_result),
                    );

                    Ok(())
                }
            }
        });

        // Hook: Workflow completion - generate report
        orchestrator.on_workflow_complete(|execution| {
            let client = self.client.clone();

            async move {
                let report = client.generate_report(GenerateReportRequest {
                    execution_id: execution.id.clone(),
                    include_task_details: true,
                    format: ReportFormat::Json,
                }).await?;

                // Store report
                execution.metadata.test_report = Some(report);

                Ok(())
            }
        });
    }
}
```

### Executor Implementation

```rust
pub struct EvaluationExecutor {
    client: llm_test_bench::Client,
}

#[async_trait]
impl TaskExecutor for EvaluationExecutor {
    async fn execute(
        &self,
        inputs: HashMap<String, Value>,
        context: TaskContext,
    ) -> Result<TaskOutput> {
        let response = inputs.get("response")
            .ok_or(ExecutorError::MissingInput("response"))?;

        let ground_truth = inputs.get("ground_truth");

        let metrics = inputs.get("metrics")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_else(|| vec!["accuracy".to_string()]);

        // Execute evaluation via Test-Bench
        let result = self.client.evaluate(EvaluationRequest {
            task_id: context.task_id.clone(),
            response: response.clone(),
            ground_truth: ground_truth.cloned(),
            metrics,
        }).await?;

        Ok(TaskOutput {
            values: HashMap::from([
                ("score".to_string(), json!(result.overall_score)),
                ("passed".to_string(), json!(result.passed)),
                ("metrics".to_string(), json!(result.metrics)),
                ("details".to_string(), json!(result.details)),
            ]),
            metadata: Default::default(),
        })
    }
}
```

### Workflow Example

```yaml
apiVersion: orchestrator.llm/v1
kind: Workflow
metadata:
  name: llm-with-evaluation
  version: 1.0.0

spec:
  inputs:
    - name: prompt
      type: string
      required: true
    - name: expected_response
      type: string
      required: true
    - name: quality_threshold
      type: float
      default: 0.75

  tasks:
    - id: llm_inference
      type: llm
      executor: llm-forge:claude-api
      inputs:
        prompt: ${{ workflow.inputs.prompt }}
      outputs:
        response: string

    - id: evaluate_response
      type: evaluation
      executor: llm-test-bench:multi-metric
      dependsOn:
        - llm_inference
      inputs:
        response: ${{ tasks.llm_inference.outputs.response }}
        ground_truth: ${{ workflow.inputs.expected_response }}
        metrics:
          - semantic_similarity
          - factual_consistency
          - coherence
          - relevance
      outputs:
        score: float
        passed: boolean
        metrics: object

    - id: quality_gate
      type: custom
      executor: orchestrator:assert
      dependsOn:
        - evaluate_response
      inputs:
        condition: ${{ tasks.evaluate_response.outputs.score >= workflow.inputs.quality_threshold }}
        error_message: "Quality score below threshold"

  outputs:
    response:
      value: ${{ tasks.llm_inference.outputs.response }}
    quality_metrics:
      value: ${{ tasks.evaluate_response.outputs.metrics }}
    passed_quality_gate:
      value: ${{ tasks.evaluate_response.outputs.passed }}
```

### API Contract

#### Evaluation Request

```rust
pub struct EvaluationRequest {
    pub task_id: String,
    pub response: Value,
    pub ground_truth: Option<Value>,
    pub metrics: Vec<String>,
    pub custom_evaluators: Vec<CustomEvaluator>,
}
```

#### Evaluation Response

```rust
pub struct EvaluationResponse {
    pub overall_score: f64,
    pub passed: bool,
    pub metrics: HashMap<String, MetricResult>,
    pub details: EvaluationDetails,
}

pub struct MetricResult {
    pub name: String,
    pub score: f64,
    pub passed: bool,
    pub explanation: Option<String>,
}
```

---

## LLM-Auto-Optimizer Integration

### Overview

LLM-Auto-Optimizer analyzes workflow performance and provides optimization recommendations. The orchestrator integrates to:
- Collect performance metrics during execution
- Submit metrics for analysis
- Receive and apply optimization recommendations
- Track performance improvements

### Integration Architecture

```rust
pub struct AutoOptimizerIntegration {
    client: llm_auto_optimizer::Client,
    metrics_buffer: Arc<Mutex<Vec<PerformanceMetric>>>,
    flush_interval: Duration,
    recommendation_cache: Arc<RwLock<HashMap<WorkflowId, Recommendations>>>,
}

pub struct AutoOptimizerConfig {
    pub endpoint: String,
    pub batch_size: usize,
    pub flush_interval: Duration,
    pub auto_apply_recommendations: bool,
}
```

### Metric Collection

```rust
impl AutoOptimizerIntegration {
    /// Collect metrics from task execution
    pub async fn collect_task_metrics(
        &self,
        task: &TaskDefinition,
        output: &TaskOutput,
        context: &TaskContext,
    ) {
        let metric = PerformanceMetric {
            // Identifiers
            workflow_id: context.workflow_id.clone(),
            task_id: task.id.clone(),
            executor: task.executor.clone(),
            timestamp: Utc::now(),

            // Performance metrics
            execution_time_ms: output.metadata.execution_time.as_millis() as i64,
            cpu_usage_percent: context.resource_usage.cpu_percent,
            memory_usage_mb: context.resource_usage.memory_mb,

            // LLM-specific metrics
            token_count: output.values.get("usage")
                .and_then(|v| v.get("total_tokens"))
                .and_then(|v| v.as_i64()),
            cost_usd: output.metadata.cost,

            // Quality metrics
            quality_score: output.values.get("quality_score")
                .and_then(|v| v.as_f64()),

            // Custom metrics
            custom: output.metadata.custom_metrics.clone(),
        };

        // Buffer metrics for batch submission
        self.metrics_buffer.lock().await.push(metric);

        // Flush if batch size reached
        if self.metrics_buffer.lock().await.len() >= self.config.batch_size {
            self.flush_metrics().await?;
        }
    }

    /// Flush metrics to Auto-Optimizer
    async fn flush_metrics(&self) -> Result<()> {
        let metrics = {
            let mut buffer = self.metrics_buffer.lock().await;
            std::mem::take(&mut *buffer)
        };

        if metrics.is_empty() {
            return Ok(());
        }

        self.client.submit_metrics(SubmitMetricsRequest {
            metrics,
            source: "llm-orchestrator".to_string(),
        }).await?;

        Ok(())
    }

    /// Get optimization recommendations
    pub async fn get_recommendations(
        &self,
        workflow_id: WorkflowId,
    ) -> Result<Recommendations> {
        // Check cache
        if let Some(cached) = self.recommendation_cache.read().await.get(&workflow_id) {
            if cached.age() < Duration::from_secs(300) {
                return Ok(cached.clone());
            }
        }

        // Fetch from Auto-Optimizer
        let recommendations = self.client.get_recommendations(
            GetRecommendationsRequest {
                workflow_id: workflow_id.clone(),
                include_task_level: true,
            }
        ).await?;

        // Update cache
        self.recommendation_cache
            .write()
            .await
            .insert(workflow_id, recommendations.clone());

        Ok(recommendations)
    }

    /// Apply optimization recommendations
    pub async fn apply_recommendations(
        &self,
        workflow: &mut WorkflowDefinition,
        recommendations: &Recommendations,
    ) -> Result<()> {
        for rec in &recommendations.items {
            match &rec.recommendation_type {
                RecommendationType::ModelSwitch { from, to, reason } => {
                    info!("Switching model from {} to {}: {}", from, to, reason);
                    self.apply_model_switch(workflow, from, to)?;
                }

                RecommendationType::ParameterTuning { task_id, parameter, old_value, new_value } => {
                    info!("Tuning {} for task {}: {} -> {}", parameter, task_id, old_value, new_value);
                    self.apply_parameter_tuning(workflow, task_id, parameter, new_value)?;
                }

                RecommendationType::Parallelization { tasks, expected_speedup } => {
                    info!("Parallelizing tasks {:?} (expected speedup: {}x)", tasks, expected_speedup);
                    self.apply_parallelization(workflow, tasks)?;
                }

                RecommendationType::Caching { task_id, ttl } => {
                    info!("Enabling caching for task {} (TTL: {:?})", task_id, ttl);
                    self.apply_caching(workflow, task_id, *ttl)?;
                }
            }
        }

        Ok(())
    }
}
```

### Workflow Example

```yaml
apiVersion: orchestrator.llm/v1
kind: Workflow
metadata:
  name: optimized-inference-pipeline
  version: 1.0.0
  annotations:
    auto-optimizer.enabled: "true"
    auto-optimizer.apply-recommendations: "true"

spec:
  tasks:
    - id: inference
      type: llm
      executor: llm-forge:claude-api
      inputs:
        prompt: ${{ workflow.inputs.prompt }}
        config:
          model: claude-sonnet-4-5-20250929
          temperature: 0.7
      outputs:
        response: string
        usage: object

    # Analytics task for metric collection
    - id: collect_metrics
      type: analytics
      executor: llm-auto-optimizer:metric-collector
      dependsOn:
        - inference
      inputs:
        execution_metrics:
          task_id: inference
          execution_time: ${{ tasks.inference.metadata.execution_time }}
          cost: ${{ tasks.inference.metadata.cost }}
          token_count: ${{ tasks.inference.outputs.usage.total_tokens }}

  events:
    onSuccess:
      - type: custom
        executor: llm-auto-optimizer:request-recommendations
        inputs:
          workflow_id: ${{ workflow.id }}
```

### API Contract

#### Submit Metrics Request

```rust
pub struct SubmitMetricsRequest {
    pub metrics: Vec<PerformanceMetric>,
    pub source: String,
}

pub struct PerformanceMetric {
    pub workflow_id: String,
    pub task_id: String,
    pub executor: String,
    pub timestamp: DateTime<Utc>,
    pub execution_time_ms: i64,
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: i64,
    pub token_count: Option<i64>,
    pub cost_usd: Option<f64>,
    pub quality_score: Option<f64>,
    pub custom: HashMap<String, Value>,
}
```

#### Recommendations Response

```rust
pub struct Recommendations {
    pub workflow_id: String,
    pub generated_at: DateTime<Utc>,
    pub items: Vec<Recommendation>,
    pub estimated_improvement: ImprovementEstimate,
}

pub struct Recommendation {
    pub id: String,
    pub recommendation_type: RecommendationType,
    pub confidence: f64,
    pub expected_impact: ImpactEstimate,
}

pub enum RecommendationType {
    ModelSwitch { from: String, to: String, reason: String },
    ParameterTuning { task_id: String, parameter: String, old_value: Value, new_value: Value },
    Parallelization { tasks: Vec<String>, expected_speedup: f64 },
    Caching { task_id: String, ttl: Duration },
}
```

---

## LLM-Governance-Core Integration

### Overview

LLM-Governance-Core provides policy enforcement and cost management. The orchestrator integrates to:
- Enforce governance policies before task execution
- Track and limit costs
- Audit all LLM operations
- Enforce compliance requirements

### Integration Architecture

```rust
pub struct GovernanceIntegration {
    client: llm_governance::Client,
    policy_cache: Arc<RwLock<HashMap<String, Policy>>>,
    cost_tracker: Arc<CostTracker>,
    audit_logger: Arc<AuditLogger>,
}

pub struct GovernanceConfig {
    pub endpoint: String,
    pub enforce_policies: bool,
    pub cost_limits: CostLimits,
    pub audit_level: AuditLevel,
}
```

### Policy Enforcement

```rust
impl GovernanceIntegration {
    /// Intercept task execution for policy checks
    pub async fn enforce_policies(
        &self,
        task: &TaskDefinition,
        context: &TaskContext,
    ) -> Result<PolicyDecision> {
        // Load applicable policies
        let policies = self.get_applicable_policies(task, context).await?;

        for policy in policies {
            let decision = self.evaluate_policy(&policy, task, context).await?;

            match decision {
                PolicyDecision::Allow => continue,

                PolicyDecision::Deny { reason } => {
                    self.audit_logger.log_policy_violation(AuditEvent {
                        task_id: task.id.clone(),
                        policy_id: policy.id.clone(),
                        decision: "deny".to_string(),
                        reason: reason.clone(),
                        timestamp: Utc::now(),
                    }).await?;

                    return Err(GovernanceError::PolicyViolation {
                        policy: policy.name.clone(),
                        reason,
                    });
                }

                PolicyDecision::RequireApproval { approver } => {
                    // Request manual approval
                    let approved = self.request_approval(
                        &policy,
                        task,
                        &approver,
                    ).await?;

                    if !approved {
                        return Err(GovernanceError::ApprovalDenied {
                            policy: policy.name.clone(),
                        });
                    }
                }
            }
        }

        Ok(PolicyDecision::Allow)
    }

    /// Track costs and enforce budget limits
    pub async fn track_and_enforce_costs(
        &self,
        task: &TaskDefinition,
        output: &TaskOutput,
    ) -> Result<()> {
        let cost = output.metadata.cost.unwrap_or(0.0);

        // Record cost
        self.cost_tracker.record(CostRecord {
            task_id: task.id.clone(),
            workflow_id: output.metadata.workflow_id.clone(),
            executor: task.executor.clone(),
            cost_usd: cost,
            timestamp: Utc::now(),
        }).await?;

        // Check budget limits
        let budget_status = self.client.check_budget(CheckBudgetRequest {
            workflow_id: output.metadata.workflow_id.clone(),
            organization_id: output.metadata.organization_id.clone(),
        }).await?;

        if budget_status.exceeded {
            return Err(GovernanceError::BudgetExceeded {
                limit: budget_status.limit,
                current: budget_status.current,
            });
        }

        // Warn if approaching limit
        if budget_status.utilization > 0.9 {
            warn!(
                "Budget utilization at {}% for workflow {}",
                budget_status.utilization * 100.0,
                output.metadata.workflow_id
            );
        }

        Ok(())
    }
}
```

### Executor Implementation

```rust
pub struct PolicyEnforcementExecutor {
    governance: Arc<GovernanceIntegration>,
}

#[async_trait]
impl TaskExecutor for PolicyEnforcementExecutor {
    async fn execute(
        &self,
        inputs: HashMap<String, Value>,
        context: TaskContext,
    ) -> Result<TaskOutput> {
        // Cost tracking
        let usage = inputs.get("usage")
            .ok_or(ExecutorError::MissingInput("usage"))?;

        let model = inputs.get("model")
            .and_then(|v| v.as_str())
            .ok_or(ExecutorError::MissingInput("model"))?;

        // Calculate cost
        let cost = calculate_cost(usage, model)?;

        // Track in governance system
        self.governance.cost_tracker.record(CostRecord {
            task_id: context.task_id.clone(),
            workflow_id: context.workflow_id.clone(),
            executor: model.to_string(),
            cost_usd: cost,
            timestamp: Utc::now(),
        }).await?;

        // Check budget
        let budget_status = self.governance.client
            .check_budget(CheckBudgetRequest {
                workflow_id: context.workflow_id.clone(),
                organization_id: context.organization_id.clone(),
            })
            .await?;

        Ok(TaskOutput {
            values: HashMap::from([
                ("cost".to_string(), json!(cost)),
                ("within_budget".to_string(), json!(!budget_status.exceeded)),
                ("budget_remaining".to_string(), json!(budget_status.remaining)),
                ("budget_utilization".to_string(), json!(budget_status.utilization)),
            ]),
            metadata: Default::default(),
        })
    }
}
```

### Workflow Example

```yaml
apiVersion: orchestrator.llm/v1
kind: Workflow
metadata:
  name: governed-llm-workflow
  version: 1.0.0
  annotations:
    governance.enforce-policies: "true"
    governance.budget-limit-usd: "10.00"

spec:
  tasks:
    # Pre-execution policy check
    - id: policy_check
      type: policy
      executor: llm-governance:policy-enforcer
      inputs:
        policies:
          - content-safety
          - data-privacy
          - cost-limit
        context:
          prompt: ${{ workflow.inputs.prompt }}

    - id: llm_inference
      type: llm
      executor: llm-forge:claude-api
      dependsOn:
        - policy_check
      inputs:
        prompt: ${{ workflow.inputs.prompt }}
      outputs:
        response: string
        usage: object

    # Post-execution cost tracking
    - id: track_cost
      type: policy
      executor: llm-governance:cost-tracker
      dependsOn:
        - llm_inference
      inputs:
        usage: ${{ tasks.llm_inference.outputs.usage }}
        model: claude-sonnet-4-5-20250929
      outputs:
        cost: float
        within_budget: boolean

  # Conditional: Alert if budget exceeded
  branches:
    - condition: ${{ tasks.track_cost.outputs.within_budget == false }}
      tasks:
        - id: budget_alert
          type: custom
          executor: notification:alert
          inputs:
            severity: high
            message: "Budget exceeded in workflow"

  outputs:
    response:
      value: ${{ tasks.llm_inference.outputs.response }}
    total_cost:
      value: ${{ tasks.track_cost.outputs.cost }}
```

### API Contract

#### Policy Evaluation Request

```rust
pub struct PolicyEvaluationRequest {
    pub policy_id: String,
    pub task: TaskDefinition,
    pub context: TaskContext,
}

pub enum PolicyDecision {
    Allow,
    Deny { reason: String },
    RequireApproval { approver: String },
}
```

#### Cost Tracking

```rust
pub struct CostRecord {
    pub task_id: String,
    pub workflow_id: String,
    pub executor: String,
    pub cost_usd: f64,
    pub timestamp: DateTime<Utc>,
}

pub struct BudgetStatus {
    pub limit: f64,
    pub current: f64,
    pub remaining: f64,
    pub utilization: f64,
    pub exceeded: bool,
}
```

---

## Cross-System Data Flow

### End-to-End Example

```
┌────────────────────────────────────────────────────────────────┐
│ 1. Workflow Submission                                         │
│    User → Orchestrator                                         │
└────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────────┐
│ 2. Policy Enforcement                                          │
│    Orchestrator → Governance-Core                              │
│    • Check content policies                                    │
│    • Verify budget limits                                      │
│    • Audit logging                                             │
└────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────────┐
│ 3. LLM Execution                                               │
│    Orchestrator → LLM-Forge                                    │
│    • Model invocation                                          │
│    • Token usage tracking                                      │
└────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────────┐
│ 4. Quality Evaluation                                          │
│    Orchestrator → Test-Bench                                   │
│    • Semantic similarity                                       │
│    • Factual consistency                                       │
│    • Quality scoring                                           │
└────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────────┐
│ 5. Cost Tracking                                               │
│    Orchestrator → Governance-Core                              │
│    • Record usage costs                                        │
│    • Update budget status                                      │
└────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────────┐
│ 6. Performance Analysis                                        │
│    Orchestrator → Auto-Optimizer                               │
│    • Submit execution metrics                                  │
│    • Receive optimization recommendations                      │
└────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────────┐
│ 7. Result Delivery                                             │
│    Orchestrator → User                                         │
│    • LLM response                                              │
│    • Quality metrics                                           │
│    • Cost information                                          │
│    • Performance data                                          │
└────────────────────────────────────────────────────────────────┘
```

---

## Implementation Examples

### Complete Integration Example

```rust
// Main orchestrator with all integrations
pub struct LlmOrchestrator {
    // Core components
    scheduler: Arc<WorkflowScheduler>,
    executor_pool: Arc<ExecutorPool>,
    state_manager: Arc<StateManager>,

    // Integrations
    forge_integration: Arc<LlmForgeIntegration>,
    test_bench_integration: Arc<TestBenchIntegration>,
    optimizer_integration: Arc<AutoOptimizerIntegration>,
    governance_integration: Arc<GovernanceIntegration>,
}

impl LlmOrchestrator {
    pub async fn new(config: OrchestratorConfig) -> Result<Self> {
        // Initialize integrations
        let forge = LlmForgeIntegration::new(config.forge).await?;
        let test_bench = TestBenchIntegration::new(config.test_bench).await?;
        let optimizer = AutoOptimizerIntegration::new(config.optimizer).await?;
        let governance = GovernanceIntegration::new(config.governance).await?;

        // Register executors from integrations
        let mut executor_registry = ExecutorRegistry::new();
        forge.register_executors(&mut executor_registry)?;
        test_bench.register_executors(&mut executor_registry)?;
        optimizer.register_executors(&mut executor_registry)?;
        governance.register_executors(&mut executor_registry)?;

        Ok(Self {
            scheduler: Arc::new(WorkflowScheduler::new()),
            executor_pool: Arc::new(ExecutorPool::new(executor_registry)),
            state_manager: Arc::new(StateManager::new(config.storage).await?),
            forge_integration: Arc::new(forge),
            test_bench_integration: Arc::new(test_bench),
            optimizer_integration: Arc::new(optimizer),
            governance_integration: Arc::new(governance),
        })
    }

    pub async fn execute_workflow(
        &self,
        workflow: WorkflowDefinition,
        inputs: Value,
    ) -> Result<WorkflowExecutionId> {
        // 1. Policy enforcement (pre-execution)
        self.governance_integration
            .enforce_workflow_policies(&workflow)
            .await?;

        // 2. Apply optimizations (if available)
        let mut optimized_workflow = workflow.clone();
        if let Ok(recommendations) = self.optimizer_integration
            .get_recommendations(workflow.metadata.name.clone())
            .await
        {
            self.optimizer_integration
                .apply_recommendations(&mut optimized_workflow, &recommendations)
                .await?;
        }

        // 3. Register test hooks
        self.test_bench_integration
            .register_auto_evaluation(self)
            .await;

        // 4. Schedule workflow
        let context = ExecutionContext::new(inputs);
        let execution_id = self.scheduler
            .schedule_workflow(optimized_workflow, context)
            .await?;

        // 5. Start metric collection
        let optimizer = self.optimizer_integration.clone();
        let execution_id_clone = execution_id.clone();
        tokio::spawn(async move {
            optimizer.start_metric_collection(execution_id_clone).await;
        });

        Ok(execution_id)
    }
}
```

### Configuration Example

```yaml
# orchestrator-config.yaml
orchestrator:
  storage:
    type: postgresql
    url: postgresql://localhost/orchestrator

  concurrency:
    worker_threads: 8
    max_parallel_tasks: 20

  integrations:
    llm-forge:
      endpoint: http://llm-forge:8080
      timeout: 60s
      retry_policy:
        max_attempts: 3
        backoff_multiplier: 2.0

    llm-test-bench:
      endpoint: http://test-bench:8081
      auto_evaluate: true
      default_metrics:
        - semantic_similarity
        - factual_consistency

    llm-auto-optimizer:
      endpoint: http://auto-optimizer:8082
      batch_size: 100
      flush_interval: 30s
      auto_apply_recommendations: true

    llm-governance-core:
      endpoint: http://governance:8083
      enforce_policies: true
      cost_limits:
        daily: 1000.0
        monthly: 25000.0
      audit_level: full
```

---

This integration guide provides the complete blueprint for connecting LLM-Orchestrator with the entire LLM DevOps ecosystem, enabling seamless workflow orchestration across all components.
