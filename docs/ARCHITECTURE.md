# LLM-Orchestrator System Architecture

**Version:** 1.0
**Status:** Design Document
**Last Updated:** 2025-11-14

---

## Executive Summary

LLM-Orchestrator is a production-grade workflow orchestration engine designed specifically for LLM-powered applications. It provides DAG-based pipeline execution, event-driven triggers, reactive stream processing, and hybrid orchestration patterns with built-in fault tolerance, state management, and integration with the LLM DevOps ecosystem.

**Core Capabilities:**
- Declarative workflow definitions with type-safe schema validation
- Async task execution with Tokio runtime and parallel branch processing
- Advanced fault tolerance (checkpoints, retries, circuit breakers)
- Multi-mode deployment (CLI, microservice, embedded SDK)
- Native integration with LLM-Forge, LLM-Test-Bench, LLM-Auto-Optimizer, and LLM-Governance-Core

---

## 1. System Architecture Overview

### 1.1 High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          LLM-ORCHESTRATOR SYSTEM                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                    WORKFLOW DEFINITION LAYER                        │  │
│  ├─────────────────────────────────────────────────────────────────────┤  │
│  │  • Workflow Schema Parser (YAML/JSON/DSL)                          │  │
│  │  • Validation Engine (JSON Schema + Custom Rules)                  │  │
│  │  • DAG Builder & Cycle Detection                                   │  │
│  │  • Workflow Registry & Versioning                                  │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                ↓                                           │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                    ORCHESTRATION ENGINE CORE                        │  │
│  ├─────────────────────────────────────────────────────────────────────┤  │
│  │                                                                     │  │
│  │  ┌──────────────────┐  ┌──────────────────┐  ┌─────────────────┐  │  │
│  │  │  Scheduler       │  │  Executor Pool   │  │  State Manager  │  │  │
│  │  │  • Priority Queue│  │  • Tokio Runtime │  │  • FSM Engine   │  │  │
│  │  │  • Backpressure  │  │  • Worker Pool   │  │  • Snapshots    │  │  │
│  │  │  • Rate Limiting │  │  • Task Router   │  │  • Transitions  │  │  │
│  │  └──────────────────┘  └──────────────────┘  └─────────────────┘  │  │
│  │                                                                     │  │
│  │  ┌──────────────────┐  ┌──────────────────┐  ┌─────────────────┐  │  │
│  │  │  Dependency Mgr  │  │  Event Bus       │  │  Output Handler │  │  │
│  │  │  • DAG Traversal │  │  • Pub/Sub       │  │  • Transforms   │  │  │
│  │  │  • Fan-out/in    │  │  • Triggers      │  │  • Streaming    │  │  │
│  │  │  • Conditional   │  │  • Callbacks     │  │  • Aggregation  │  │  │
│  │  └──────────────────┘  └──────────────────┘  └─────────────────┘  │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                ↓                                           │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                    FAULT TOLERANCE LAYER                            │  │
│  ├─────────────────────────────────────────────────────────────────────┤  │
│  │  • Checkpoint Manager (WAL-based persistence)                       │  │
│  │  • Retry Coordinator (Exponential backoff, jitter)                  │  │
│  │  • Circuit Breaker (Half-open state machine)                        │  │
│  │  • Dead Letter Queue (Failed task quarantine)                       │  │
│  │  • Graceful Degradation (Fallback strategies)                       │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                ↓                                           │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                    STATE PERSISTENCE LAYER                          │  │
│  ├─────────────────────────────────────────────────────────────────────┤  │
│  │                                                                     │  │
│  │  ┌──────────────────┐  ┌──────────────────┐  ┌─────────────────┐  │  │
│  │  │  Storage Backend │  │  Cache Layer     │  │  Query Engine   │  │  │
│  │  │  • PostgreSQL    │  │  • Redis         │  │  • GraphQL API  │  │  │
│  │  │  • SQLite        │  │  • In-memory     │  │  • SQL DSL      │  │  │
│  │  │  • S3 (logs)     │  │  • LRU eviction  │  │  • Filtering    │  │  │
│  │  └──────────────────┘  └──────────────────┘  └─────────────────┘  │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                ↓                                           │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                    INTEGRATION LAYER                                │  │
│  ├─────────────────────────────────────────────────────────────────────┤  │
│  │  • LLM-Forge Adapter (Tool/SDK invocation)                          │  │
│  │  • LLM-Test-Bench Hooks (Eval callbacks, metrics)                   │  │
│  │  • LLM-Auto-Optimizer Collectors (Performance data)                 │  │
│  │  • LLM-Governance-Core Interceptors (Policy enforcement, cost)      │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                ↓                                           │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                    DEPLOYMENT INTERFACES                            │  │
│  ├─────────────────────────────────────────────────────────────────────┤  │
│  │  ┌───────────┐  ┌───────────────┐  ┌──────────┐  ┌──────────────┐  │  │
│  │  │  CLI      │  │  gRPC/REST    │  │  SDK     │  │  Hybrid      │  │  │
│  │  │  Runner   │  │  Microservice │  │  Library │  │  Deployment  │  │  │
│  │  └───────────┘  └───────────────┘  └──────────┘  └──────────────┘  │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Technology Stack

**Core Language:** Rust (for performance, memory safety, and concurrency)

**Key Dependencies:**
- `tokio` - Async runtime for concurrent task execution
- `serde` + `serde_json` / `serde_yaml` - Serialization/deserialization
- `petgraph` - Graph algorithms for DAG management
- `sqlx` - Type-safe SQL with async support
- `tonic` - gRPC framework
- `axum` - High-performance REST API
- `tracing` - Structured logging and observability
- `thiserror` - Ergonomic error handling
- `anyhow` - Error propagation for application code

---

## 2. Core System Components

### 2.1 Workflow Definition Language

#### 2.1.1 Schema Design

The workflow definition uses a declarative YAML/JSON format with the following structure:

```yaml
apiVersion: orchestrator.llm/v1
kind: Workflow
metadata:
  name: llm-inference-pipeline
  version: 1.0.0
  description: Multi-step LLM processing with evaluation
  labels:
    team: ai-platform
    environment: production

spec:
  # Global configuration
  config:
    timeout: 3600s
    retryPolicy:
      maxAttempts: 3
      backoffMultiplier: 2.0
      initialInterval: 1s
      maxInterval: 30s
    concurrency:
      maxParallel: 10
      resourceLimits:
        cpu: 4
        memory: 8Gi

  # Input schema validation
  inputs:
    - name: user_prompt
      type: string
      required: true
      validation:
        minLength: 1
        maxLength: 10000
    - name: model_config
      type: object
      schema:
        temperature: float
        max_tokens: int

  # Task definitions (DAG nodes)
  tasks:
    - id: prompt_preprocessing
      type: transform
      executor: llm-forge:text-transform
      inputs:
        prompt: ${{ workflow.inputs.user_prompt }}
      outputs:
        processed_prompt: string
      config:
        timeout: 30s

    - id: model_inference
      type: llm
      executor: llm-forge:claude-api
      dependsOn:
        - prompt_preprocessing
      inputs:
        prompt: ${{ tasks.prompt_preprocessing.outputs.processed_prompt }}
        config: ${{ workflow.inputs.model_config }}
      outputs:
        response: string
        usage: object
      retryPolicy:
        maxAttempts: 5
        retryableErrors:
          - RateLimitError
          - TimeoutError

    - id: response_evaluation
      type: evaluation
      executor: llm-test-bench:semantic-similarity
      dependsOn:
        - model_inference
      inputs:
        response: ${{ tasks.model_inference.outputs.response }}
        ground_truth: ${{ workflow.inputs.expected_response }}
      outputs:
        score: float
        metrics: object

    - id: cost_tracking
      type: policy
      executor: llm-governance:cost-tracker
      dependsOn:
        - model_inference
      inputs:
        usage: ${{ tasks.model_inference.outputs.usage }}
      outputs:
        cost: float
        within_budget: boolean

    - id: optimization_feedback
      type: analytics
      executor: llm-auto-optimizer:metric-collector
      dependsOn:
        - response_evaluation
        - cost_tracking
      inputs:
        metrics: ${{ tasks.response_evaluation.outputs.metrics }}
        cost: ${{ tasks.cost_tracking.outputs.cost }}

  # Conditional branching
  branches:
    - condition: ${{ tasks.response_evaluation.outputs.score < 0.7 }}
      tasks:
        - id: fallback_inference
          type: llm
          executor: llm-forge:gpt4-api
          inputs:
            prompt: ${{ tasks.prompt_preprocessing.outputs.processed_prompt }}

  # Output mapping
  outputs:
    final_response:
      value: ${{ tasks.model_inference.outputs.response }}
      fallback: ${{ tasks.fallback_inference.outputs.response }}
    quality_score: ${{ tasks.response_evaluation.outputs.score }}
    total_cost: ${{ tasks.cost_tracking.outputs.cost }}

  # Event handlers
  events:
    onSuccess:
      - type: webhook
        url: https://api.example.com/workflow-complete
    onFailure:
      - type: alert
        channel: slack
        severity: high
    onTimeout:
      - type: deadletter
        queue: failed-workflows
```

#### 2.1.2 Type System

```rust
// Core workflow types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub api_version: String,
    pub kind: WorkflowKind,
    pub metadata: WorkflowMetadata,
    pub spec: WorkflowSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowKind {
    Workflow,
    Template,
    CronWorkflow,
    EventWorkflow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSpec {
    pub config: WorkflowConfig,
    pub inputs: Vec<InputParameter>,
    pub tasks: Vec<TaskDefinition>,
    pub branches: Vec<ConditionalBranch>,
    pub outputs: HashMap<String, OutputMapping>,
    pub events: EventHandlers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub id: TaskId,
    pub task_type: TaskType,
    pub executor: ExecutorRef,
    pub depends_on: Vec<TaskId>,
    pub inputs: HashMap<String, ValueExpression>,
    pub outputs: HashMap<String, TypeDefinition>,
    pub retry_policy: Option<RetryPolicy>,
    pub timeout: Option<Duration>,
    pub condition: Option<Expression>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Transform,
    Llm,
    Evaluation,
    Policy,
    Analytics,
    Custom(String),
}

// Expression language for dynamic values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueExpression {
    Literal(Value),
    WorkflowInput(String),
    TaskOutput { task_id: TaskId, output_key: String },
    Expression(String), // JSONPath or custom DSL
    Template(String),   // Tera/Handlebars template
}
```

### 2.2 Execution Engine

#### 2.2.1 Scheduler Architecture

```rust
/// Core scheduler managing task execution queue
pub struct WorkflowScheduler {
    /// Priority queue for ready-to-execute tasks
    ready_queue: Arc<RwLock<PriorityQueue<ScheduledTask>>>,

    /// Dependency graph tracker
    dependency_graph: Arc<DependencyGraph>,

    /// Resource manager for capacity planning
    resource_manager: Arc<ResourceManager>,

    /// Backpressure controller
    backpressure: Arc<BackpressureController>,

    /// Task executor pool
    executor_pool: Arc<ExecutorPool>,
}

impl WorkflowScheduler {
    /// Schedule a workflow for execution
    pub async fn schedule_workflow(
        &self,
        workflow: WorkflowDefinition,
        context: ExecutionContext,
    ) -> Result<WorkflowExecutionId> {
        // 1. Validate workflow definition
        self.validate_workflow(&workflow)?;

        // 2. Build dependency graph
        let dag = self.dependency_graph.build(&workflow.spec.tasks)?;

        // 3. Detect cycles
        if dag.has_cycles() {
            return Err(SchedulerError::CyclicDependency);
        }

        // 4. Initialize execution state
        let execution = WorkflowExecution::new(workflow, context);
        let execution_id = execution.id.clone();

        // 5. Persist initial state
        self.state_manager.save_execution(&execution).await?;

        // 6. Enqueue root tasks (no dependencies)
        let root_tasks = dag.get_root_nodes();
        for task in root_tasks {
            self.enqueue_task(execution_id.clone(), task).await?;
        }

        Ok(execution_id)
    }

    /// Main scheduling loop (runs in background)
    async fn run_scheduler_loop(self: Arc<Self>) {
        loop {
            tokio::select! {
                // Process ready tasks
                Some(scheduled_task) = self.ready_queue.write().await.pop() => {
                    // Check backpressure
                    if self.backpressure.should_throttle().await {
                        self.ready_queue.write().await.push(scheduled_task);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }

                    // Check resource availability
                    if !self.resource_manager.can_allocate(&scheduled_task.resources).await {
                        self.ready_queue.write().await.push(scheduled_task);
                        continue;
                    }

                    // Allocate resources and execute
                    let resources = self.resource_manager
                        .allocate(&scheduled_task.resources)
                        .await
                        .unwrap();

                    let scheduler = self.clone();
                    tokio::spawn(async move {
                        scheduler.execute_task(scheduled_task, resources).await;
                    });
                }

                // Handle graceful shutdown
                _ = self.shutdown_signal.notified() => {
                    break;
                }
            }
        }
    }
}

/// Priority queue implementation with resource-aware scheduling
#[derive(Debug)]
pub struct PriorityQueue<T> {
    heap: BinaryHeap<PrioritizedItem<T>>,
    priority_fn: Box<dyn Fn(&T) -> i64 + Send + Sync>,
}

#[derive(Debug)]
struct PrioritizedItem<T> {
    priority: i64,
    timestamp: Instant,
    item: T,
}

impl<T> Ord for PrioritizedItem<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then FIFO for same priority
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.timestamp.cmp(&self.timestamp))
    }
}
```

#### 2.2.2 Executor Pool

```rust
/// Task executor pool with dynamic worker management
pub struct ExecutorPool {
    /// Worker threads (tokio tasks)
    workers: Arc<RwLock<Vec<WorkerHandle>>>,

    /// Task channel (MPSC)
    task_tx: mpsc::UnboundedSender<ExecutionRequest>,
    task_rx: Arc<Mutex<mpsc::UnboundedReceiver<ExecutionRequest>>>,

    /// Executor registry (plugin system)
    executors: Arc<ExecutorRegistry>,

    /// Metrics collector
    metrics: Arc<ExecutorMetrics>,
}

impl ExecutorPool {
    /// Execute a task with the appropriate executor
    pub async fn execute(
        &self,
        task: TaskDefinition,
        context: TaskContext,
    ) -> Result<TaskOutput> {
        // 1. Resolve executor
        let executor = self.executors.get(&task.executor)?;

        // 2. Prepare inputs
        let inputs = self.resolve_inputs(&task.inputs, &context).await?;

        // 3. Execute with timeout and cancellation
        let result = tokio::select! {
            result = executor.execute(inputs, context.clone()) => result,
            _ = context.cancellation_token.cancelled() => {
                Err(ExecutorError::Cancelled)
            }
            _ = tokio::time::sleep(task.timeout.unwrap_or(DEFAULT_TIMEOUT)) => {
                Err(ExecutorError::Timeout)
            }
        };

        // 4. Record metrics
        self.metrics.record_execution(&task, &result);

        result
    }
}

/// Executor trait for custom task implementations
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    async fn execute(
        &self,
        inputs: HashMap<String, Value>,
        context: TaskContext,
    ) -> Result<TaskOutput>;

    fn metadata(&self) -> ExecutorMetadata;

    fn validate_inputs(&self, inputs: &HashMap<String, Value>) -> Result<()>;
}

/// Built-in executors
pub struct TransformExecutor;
pub struct LlmExecutor;
pub struct EvaluationExecutor;
pub struct PolicyExecutor;

// Plugin-based executor registry
pub struct ExecutorRegistry {
    executors: HashMap<ExecutorRef, Box<dyn TaskExecutor>>,
}

impl ExecutorRegistry {
    pub fn register<E: TaskExecutor + 'static>(
        &mut self,
        name: impl Into<String>,
        executor: E,
    ) {
        self.executors.insert(name.into(), Box::new(executor));
    }

    pub fn register_from_plugin(&mut self, plugin_path: &Path) -> Result<()> {
        // Load shared library using libloading
        // This enables runtime executor plugins
        todo!("Plugin loading implementation")
    }
}
```

#### 2.2.3 State Manager

```rust
/// Workflow execution state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionState {
    Pending,
    Running,
    Paused,
    Completed(ExecutionResult),
    Failed(ExecutionError),
    Cancelled,
    TimedOut,
}

/// Finite state machine for workflow execution
pub struct WorkflowStateMachine {
    current_state: ExecutionState,
    state_history: Vec<StateTransition>,
    checkpoints: Vec<Checkpoint>,
}

impl WorkflowStateMachine {
    /// Transition to a new state with validation
    pub fn transition(&mut self, new_state: ExecutionState) -> Result<()> {
        // Validate transition
        if !self.is_valid_transition(&self.current_state, &new_state) {
            return Err(StateError::InvalidTransition {
                from: self.current_state.clone(),
                to: new_state,
            });
        }

        // Record transition
        let transition = StateTransition {
            from: self.current_state.clone(),
            to: new_state.clone(),
            timestamp: Utc::now(),
        };

        self.state_history.push(transition);
        self.current_state = new_state;

        Ok(())
    }

    fn is_valid_transition(&self, from: &ExecutionState, to: &ExecutionState) -> bool {
        use ExecutionState::*;

        matches!(
            (from, to),
            (Pending, Running)
                | (Running, Paused)
                | (Running, Completed(_))
                | (Running, Failed(_))
                | (Running, Cancelled)
                | (Running, TimedOut)
                | (Paused, Running)
                | (Paused, Cancelled)
        )
    }
}

/// State persistence manager with snapshot support
pub struct StateManager {
    storage: Arc<dyn StateStorage>,
    cache: Arc<StateCache>,
    wal: Arc<WriteAheadLog>,
}

impl StateManager {
    /// Save execution state with WAL
    pub async fn save_execution(&self, execution: &WorkflowExecution) -> Result<()> {
        // 1. Write to WAL first (durability)
        self.wal.append(StateUpdate::from(execution)).await?;

        // 2. Update cache (fast reads)
        self.cache.put(execution.id.clone(), execution.clone());

        // 3. Asynchronously persist to storage
        let storage = self.storage.clone();
        let exec = execution.clone();
        tokio::spawn(async move {
            if let Err(e) = storage.save(&exec).await {
                error!("Failed to persist execution state: {}", e);
            }
        });

        Ok(())
    }

    /// Create checkpoint for restart
    pub async fn checkpoint(&self, execution: &WorkflowExecution) -> Result<CheckpointId> {
        let checkpoint = Checkpoint {
            id: CheckpointId::new(),
            execution_id: execution.id.clone(),
            timestamp: Utc::now(),
            state: execution.state.clone(),
            task_states: execution.tasks.clone(),
            outputs: execution.outputs.clone(),
        };

        self.storage.save_checkpoint(&checkpoint).await?;
        Ok(checkpoint.id)
    }

    /// Restore from checkpoint
    pub async fn restore(&self, checkpoint_id: CheckpointId) -> Result<WorkflowExecution> {
        let checkpoint = self.storage.load_checkpoint(checkpoint_id).await?;

        WorkflowExecution::from_checkpoint(checkpoint)
    }
}
```

### 2.3 Dependency Graph Manager

```rust
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::{toposort, is_cyclic_directed};

/// DAG-based dependency graph for task scheduling
pub struct DependencyGraph {
    graph: DiGraph<TaskNode, DependencyEdge>,
    task_index: HashMap<TaskId, NodeIndex>,
}

#[derive(Debug, Clone)]
pub struct TaskNode {
    pub id: TaskId,
    pub task: TaskDefinition,
    pub execution_state: Arc<RwLock<TaskExecutionState>>,
}

#[derive(Debug, Clone)]
pub struct DependencyEdge {
    pub dependency_type: DependencyType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DependencyType {
    /// Task must complete successfully
    Success,
    /// Task must complete (success or failure)
    Completion,
    /// Data dependency (output -> input)
    Data { output_key: String },
}

impl DependencyGraph {
    /// Build graph from task definitions
    pub fn build(&mut self, tasks: &[TaskDefinition]) -> Result<()> {
        // Add all tasks as nodes
        for task in tasks {
            let node = TaskNode {
                id: task.id.clone(),
                task: task.clone(),
                execution_state: Arc::new(RwLock::new(TaskExecutionState::Pending)),
            };
            let index = self.graph.add_node(node);
            self.task_index.insert(task.id.clone(), index);
        }

        // Add dependency edges
        for task in tasks {
            let target_idx = self.task_index[&task.id];

            for dep_id in &task.depends_on {
                let source_idx = self.task_index[dep_id];

                let edge = DependencyEdge {
                    dependency_type: self.infer_dependency_type(task, dep_id),
                };

                self.graph.add_edge(source_idx, target_idx, edge);
            }
        }

        Ok(())
    }

    /// Check for cycles (invalid DAG)
    pub fn has_cycles(&self) -> bool {
        is_cyclic_directed(&self.graph)
    }

    /// Get topological ordering of tasks
    pub fn topological_order(&self) -> Result<Vec<TaskId>> {
        let sorted = toposort(&self.graph, None)
            .map_err(|_| GraphError::CyclicDependency)?;

        Ok(sorted
            .into_iter()
            .map(|idx| self.graph[idx].id.clone())
            .collect())
    }

    /// Get tasks ready for execution (all dependencies met)
    pub fn get_ready_tasks(&self, completed: &HashSet<TaskId>) -> Vec<TaskId> {
        self.graph
            .node_indices()
            .filter(|&idx| {
                let node = &self.graph[idx];

                // Skip if already completed
                if completed.contains(&node.id) {
                    return false;
                }

                // Check if all dependencies are met
                let deps = self.graph.neighbors_directed(idx, Direction::Incoming);
                deps.all(|dep_idx| {
                    let dep_node = &self.graph[dep_idx];
                    completed.contains(&dep_node.id)
                })
            })
            .map(|idx| self.graph[idx].id.clone())
            .collect()
    }

    /// Get downstream tasks (tasks that depend on this one)
    pub fn get_downstream_tasks(&self, task_id: &TaskId) -> Vec<TaskId> {
        let idx = self.task_index[task_id];

        self.graph
            .neighbors_directed(idx, Direction::Outgoing)
            .map(|neighbor_idx| self.graph[neighbor_idx].id.clone())
            .collect()
    }

    /// Fan-out: Split execution to multiple parallel branches
    pub fn fan_out(&self, task_id: &TaskId) -> Vec<Vec<TaskId>> {
        let downstream = self.get_downstream_tasks(task_id);

        // Group independent branches
        // This is a simplified version; production would use
        // graph coloring or component analysis
        downstream
            .into_iter()
            .map(|id| vec![id])
            .collect()
    }

    /// Fan-in: Wait for multiple tasks before proceeding
    pub fn fan_in(&self, task_id: &TaskId) -> Vec<TaskId> {
        let idx = self.task_index[task_id];

        self.graph
            .neighbors_directed(idx, Direction::Incoming)
            .map(|neighbor_idx| self.graph[neighbor_idx].id.clone())
            .collect()
    }
}
```

### 2.4 Task Router and Output Handler

```rust
/// Routes task outputs to downstream tasks and external systems
pub struct OutputRouter {
    /// Output transformation pipeline
    transformers: Vec<Box<dyn OutputTransformer>>,

    /// Streaming output handlers
    stream_handlers: HashMap<StreamId, mpsc::UnboundedSender<OutputChunk>>,

    /// Aggregation strategies
    aggregators: HashMap<AggregationId, Box<dyn OutputAggregator>>,
}

impl OutputRouter {
    /// Route task output to downstream consumers
    pub async fn route_output(
        &self,
        task_id: TaskId,
        output: TaskOutput,
        context: &ExecutionContext,
    ) -> Result<()> {
        // 1. Apply transformations
        let mut transformed = output;
        for transformer in &self.transformers {
            transformed = transformer.transform(transformed, context).await?;
        }

        // 2. Store in execution context
        context.task_outputs.write().await
            .insert(task_id.clone(), transformed.clone());

        // 3. Stream to real-time consumers
        if let Some(stream_id) = context.output_streams.get(&task_id) {
            if let Some(tx) = self.stream_handlers.get(stream_id) {
                tx.send(OutputChunk::from(transformed.clone()))?;
            }
        }

        // 4. Aggregate outputs
        if let Some(agg_id) = context.aggregation_rules.get(&task_id) {
            if let Some(aggregator) = self.aggregators.get(agg_id) {
                aggregator.add_output(task_id, transformed.clone()).await?;
            }
        }

        Ok(())
    }
}

/// Output transformation trait
#[async_trait]
pub trait OutputTransformer: Send + Sync {
    async fn transform(
        &self,
        output: TaskOutput,
        context: &ExecutionContext,
    ) -> Result<TaskOutput>;
}

/// Built-in transformers
pub struct JsonPathTransformer {
    path: String,
}

pub struct TemplateTransformer {
    template: tera::Tera,
}

pub struct SchemaValidator {
    schema: schemars::schema::RootSchema,
}

/// Output aggregation trait
#[async_trait]
pub trait OutputAggregator: Send + Sync {
    async fn add_output(&self, task_id: TaskId, output: TaskOutput) -> Result<()>;
    async fn get_aggregated(&self) -> Result<TaskOutput>;
}

/// Built-in aggregators
pub struct ArrayAggregator {
    outputs: Arc<RwLock<Vec<TaskOutput>>>,
}

pub struct MapAggregator {
    outputs: Arc<RwLock<HashMap<TaskId, TaskOutput>>>,
}

pub struct ReduceAggregator {
    reducer: Box<dyn Fn(TaskOutput, TaskOutput) -> TaskOutput + Send + Sync>,
    accumulator: Arc<RwLock<Option<TaskOutput>>>,
}
```

---

## 3. Orchestration Patterns

### 3.1 DAG-Based Pipeline Execution

```rust
/// DAG pipeline executor with parallel branch processing
pub struct DagPipelineExecutor {
    scheduler: Arc<WorkflowScheduler>,
    dependency_graph: Arc<DependencyGraph>,
    executor_pool: Arc<ExecutorPool>,
}

impl DagPipelineExecutor {
    pub async fn execute_pipeline(
        &self,
        workflow: WorkflowDefinition,
    ) -> Result<PipelineResult> {
        // 1. Build dependency graph
        let mut graph = DependencyGraph::new();
        graph.build(&workflow.spec.tasks)?;

        // 2. Get topological ordering
        let topo_order = graph.topological_order()?;

        // 3. Execute in dependency order with parallelism
        let mut completed = HashSet::new();
        let mut results = HashMap::new();

        while completed.len() < workflow.spec.tasks.len() {
            // Get tasks ready for execution
            let ready_tasks = graph.get_ready_tasks(&completed);

            if ready_tasks.is_empty() && completed.len() < workflow.spec.tasks.len() {
                return Err(PipelineError::Deadlock);
            }

            // Execute ready tasks in parallel
            let futures: Vec<_> = ready_tasks
                .into_iter()
                .map(|task_id| {
                    let task = workflow.spec.tasks
                        .iter()
                        .find(|t| t.id == task_id)
                        .unwrap()
                        .clone();

                    let executor_pool = self.executor_pool.clone();
                    let context = self.build_task_context(&task, &results);

                    async move {
                        let result = executor_pool.execute(task.clone(), context).await;
                        (task.id, result)
                    }
                })
                .collect();

            // Wait for all parallel tasks to complete
            let task_results = futures::future::join_all(futures).await;

            // Process results
            for (task_id, result) in task_results {
                match result {
                    Ok(output) => {
                        results.insert(task_id.clone(), output);
                        completed.insert(task_id);
                    }
                    Err(e) => {
                        return Err(PipelineError::TaskFailed {
                            task_id,
                            error: e,
                        });
                    }
                }
            }
        }

        Ok(PipelineResult { outputs: results })
    }
}
```

### 3.2 Event-Driven Workflow Triggers

```rust
/// Event-driven workflow trigger system
pub struct EventTriggerEngine {
    event_bus: Arc<EventBus>,
    trigger_registry: Arc<RwLock<TriggerRegistry>>,
    scheduler: Arc<WorkflowScheduler>,
}

#[derive(Debug, Clone)]
pub struct WorkflowTrigger {
    pub id: TriggerId,
    pub workflow: WorkflowDefinition,
    pub event_filter: EventFilter,
    pub condition: Option<Expression>,
}

#[derive(Debug, Clone)]
pub enum EventFilter {
    EventType(String),
    EventSource(String),
    CustomFilter(Box<dyn Fn(&Event) -> bool + Send + Sync>),
}

impl EventTriggerEngine {
    pub async fn register_trigger(&self, trigger: WorkflowTrigger) -> Result<()> {
        self.trigger_registry.write().await.register(trigger.clone());

        // Subscribe to relevant events
        let event_bus = self.event_bus.clone();
        let scheduler = self.scheduler.clone();

        event_bus.subscribe(move |event: Event| {
            let trigger = trigger.clone();
            let scheduler = scheduler.clone();

            async move {
                // Check event filter
                if !trigger.event_filter.matches(&event) {
                    return;
                }

                // Evaluate condition
                if let Some(condition) = &trigger.condition {
                    if !condition.evaluate(&event)? {
                        return;
                    }
                }

                // Trigger workflow execution
                let context = ExecutionContext::from_event(event);
                scheduler.schedule_workflow(trigger.workflow, context).await?;

                Ok(())
            }
        }).await?;

        Ok(())
    }
}

/// Event bus with pub/sub pattern
pub struct EventBus {
    subscribers: Arc<RwLock<HashMap<EventType, Vec<EventHandler>>>>,
    event_queue: mpsc::UnboundedSender<Event>,
}

impl EventBus {
    pub async fn publish(&self, event: Event) -> Result<()> {
        self.event_queue.send(event)?;
        Ok(())
    }

    pub async fn subscribe<F, Fut>(&self, handler: F) -> Result<SubscriptionId>
    where
        F: Fn(Event) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let subscription_id = SubscriptionId::new();

        self.subscribers.write().await
            .entry(EventType::All)
            .or_default()
            .push(EventHandler {
                id: subscription_id.clone(),
                handler: Arc::new(handler),
            });

        Ok(subscription_id)
    }
}
```

### 3.3 Reactive Stream Processing

```rust
use tokio_stream::{Stream, StreamExt};

/// Reactive stream processor for continuous data pipelines
pub struct StreamProcessor {
    stream_registry: Arc<RwLock<HashMap<StreamId, StreamDefinition>>>,
    operator_chain: Vec<Box<dyn StreamOperator>>,
}

#[async_trait]
pub trait StreamOperator: Send + Sync {
    async fn process(
        &self,
        input: Pin<Box<dyn Stream<Item = StreamEvent> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = StreamEvent> + Send>>;
}

/// Built-in stream operators
pub struct MapOperator<F> {
    mapper: F,
}

pub struct FilterOperator<F> {
    predicate: F,
}

pub struct WindowOperator {
    window_size: Duration,
    window_type: WindowType,
}

pub enum WindowType {
    Tumbling,
    Sliding { slide: Duration },
    Session { gap: Duration },
}

impl StreamProcessor {
    pub async fn process_stream(
        &self,
        stream_id: StreamId,
        input: impl Stream<Item = StreamEvent> + Send + 'static,
    ) -> Result<Pin<Box<dyn Stream<Item = StreamEvent> + Send>>> {
        let mut stream: Pin<Box<dyn Stream<Item = StreamEvent> + Send>> = Box::pin(input);

        // Apply operator chain
        for operator in &self.operator_chain {
            stream = operator.process(stream).await;
        }

        Ok(stream)
    }
}

/// Example: LLM inference stream processing
pub struct LlmInferenceStream {
    model: String,
    batch_size: usize,
}

#[async_trait]
impl StreamOperator for LlmInferenceStream {
    async fn process(
        &self,
        input: Pin<Box<dyn Stream<Item = StreamEvent> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = StreamEvent> + Send>> {
        let batch_size = self.batch_size;
        let model = self.model.clone();

        Box::pin(input.chunks_timeout(batch_size, Duration::from_secs(1)).map(
            move |batch| {
                let model = model.clone();
                async move {
                    // Batch inference
                    let prompts: Vec<_> = batch.iter().map(|e| e.payload.clone()).collect();
                    let responses = llm_forge::batch_inference(&model, prompts).await?;

                    // Create output events
                    batch
                        .into_iter()
                        .zip(responses)
                        .map(|(event, response)| StreamEvent {
                            id: EventId::new(),
                            timestamp: Utc::now(),
                            payload: response,
                            metadata: event.metadata,
                        })
                        .collect()
                }
            },
        ))
    }
}
```

### 3.4 Hybrid Orchestration Models

```rust
/// Hybrid orchestrator combining DAG, event-driven, and streaming patterns
pub struct HybridOrchestrator {
    dag_executor: Arc<DagPipelineExecutor>,
    event_engine: Arc<EventTriggerEngine>,
    stream_processor: Arc<StreamProcessor>,
}

impl HybridOrchestrator {
    /// Execute a hybrid workflow
    pub async fn execute(
        &self,
        workflow: WorkflowDefinition,
        mode: OrchestrationMode,
    ) -> Result<ExecutionHandle> {
        match mode {
            OrchestrationMode::Dag => {
                self.dag_executor.execute_pipeline(workflow).await
            }

            OrchestrationMode::EventDriven => {
                let trigger = WorkflowTrigger::from(workflow);
                self.event_engine.register_trigger(trigger).await
            }

            OrchestrationMode::Streaming => {
                let stream_def = StreamDefinition::from(workflow);
                self.stream_processor.create_stream(stream_def).await
            }

            OrchestrationMode::Hybrid { dag_tasks, stream_tasks, event_handlers } => {
                // Execute DAG tasks
                let dag_handle = self.dag_executor
                    .execute_pipeline(workflow.clone())
                    .await?;

                // Start stream processing
                let stream_handle = self.stream_processor
                    .process_stream(stream_tasks)
                    .await?;

                // Register event handlers
                for handler in event_handlers {
                    self.event_engine.register_trigger(handler).await?;
                }

                Ok(HybridExecutionHandle {
                    dag: dag_handle,
                    streams: vec![stream_handle],
                })
            }
        }
    }
}
```

---

## 4. Concurrency and Scheduling

### 4.1 Async Task Execution with Tokio

```rust
/// Tokio-based async executor with configurable runtime
pub struct AsyncExecutor {
    runtime: tokio::runtime::Runtime,
    worker_threads: usize,
    max_blocking_threads: usize,
}

impl AsyncExecutor {
    pub fn new(config: ExecutorConfig) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(config.worker_threads)
            .max_blocking_threads(config.max_blocking_threads)
            .thread_name("orchestrator-worker")
            .thread_stack_size(3 * 1024 * 1024)
            .enable_all()
            .build()
            .unwrap();

        Self {
            runtime,
            worker_threads: config.worker_threads,
            max_blocking_threads: config.max_blocking_threads,
        }
    }

    pub async fn spawn_task<F, T>(&self, future: F) -> JoinHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.runtime.spawn(future)
    }

    pub async fn spawn_blocking<F, T>(&self, func: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        self.runtime.spawn_blocking(func)
    }
}
```

### 4.2 Parallel Workflow Branches

```rust
/// Parallel branch executor with join semantics
pub struct ParallelBranchExecutor {
    executor_pool: Arc<ExecutorPool>,
    join_strategy: JoinStrategy,
}

#[derive(Debug, Clone)]
pub enum JoinStrategy {
    /// Wait for all branches to complete
    All,
    /// Wait for any branch to complete
    Any,
    /// Wait for N branches to complete
    AtLeast(usize),
    /// Custom join condition
    Custom(Box<dyn Fn(&[BranchResult]) -> bool + Send + Sync>),
}

impl ParallelBranchExecutor {
    pub async fn execute_parallel(
        &self,
        branches: Vec<TaskDefinition>,
        context: ExecutionContext,
    ) -> Result<Vec<TaskOutput>> {
        // Spawn all branches concurrently
        let futures: Vec<_> = branches
            .into_iter()
            .map(|task| {
                let executor_pool = self.executor_pool.clone();
                let context = context.clone();

                tokio::spawn(async move {
                    executor_pool.execute(task, context).await
                })
            })
            .collect();

        // Apply join strategy
        match &self.join_strategy {
            JoinStrategy::All => {
                // Wait for all branches
                let results = futures::future::try_join_all(futures).await?;
                results.into_iter().collect::<Result<Vec<_>>>()
            }

            JoinStrategy::Any => {
                // Wait for first completion
                let (result, _idx, _remaining) = futures::future::select_all(futures).await;
                Ok(vec![result??])
            }

            JoinStrategy::AtLeast(n) => {
                // Wait for N completions
                let mut completed = Vec::new();
                let mut remaining = futures;

                while completed.len() < *n && !remaining.is_empty() {
                    let (result, _idx, rest) = futures::future::select_all(remaining).await;
                    completed.push(result??);
                    remaining = rest;
                }

                Ok(completed)
            }

            JoinStrategy::Custom(_) => {
                todo!("Custom join strategy")
            }
        }
    }
}
```

### 4.3 Resource-Aware Scheduling

```rust
/// Resource manager for capacity-aware scheduling
pub struct ResourceManager {
    /// Available resources
    resources: Arc<RwLock<ResourcePool>>,

    /// Resource allocation tracking
    allocations: Arc<RwLock<HashMap<TaskId, ResourceAllocation>>>,

    /// Resource quotas
    quotas: HashMap<String, ResourceQuota>,
}

#[derive(Debug, Clone)]
pub struct ResourcePool {
    pub cpu: f64,
    pub memory: u64, // bytes
    pub gpu: u32,
    pub custom: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    pub task_id: TaskId,
    pub allocated: ResourcePool,
    pub allocated_at: DateTime<Utc>,
}

impl ResourceManager {
    /// Check if resources can be allocated
    pub async fn can_allocate(&self, required: &ResourcePool) -> bool {
        let available = self.resources.read().await;

        available.cpu >= required.cpu
            && available.memory >= required.memory
            && available.gpu >= required.gpu
    }

    /// Allocate resources for a task
    pub async fn allocate(&self, required: &ResourcePool) -> Result<ResourceAllocation> {
        let mut resources = self.resources.write().await;

        // Check availability
        if !self.can_allocate(required).await {
            return Err(ResourceError::InsufficientResources);
        }

        // Deduct resources
        resources.cpu -= required.cpu;
        resources.memory -= required.memory;
        resources.gpu -= required.gpu;

        let allocation = ResourceAllocation {
            task_id: TaskId::new(),
            allocated: required.clone(),
            allocated_at: Utc::now(),
        };

        self.allocations.write().await
            .insert(allocation.task_id.clone(), allocation.clone());

        Ok(allocation)
    }

    /// Release resources
    pub async fn release(&self, allocation: ResourceAllocation) {
        let mut resources = self.resources.write().await;

        resources.cpu += allocation.allocated.cpu;
        resources.memory += allocation.allocated.memory;
        resources.gpu += allocation.allocated.gpu;

        self.allocations.write().await.remove(&allocation.task_id);
    }
}
```

### 4.4 Priority Queues and Backpressure

```rust
/// Backpressure controller for queue management
pub struct BackpressureController {
    /// Current queue depth
    queue_depth: Arc<AtomicUsize>,

    /// Thresholds
    high_watermark: usize,
    low_watermark: usize,

    /// Throttling state
    throttled: Arc<AtomicBool>,
}

impl BackpressureController {
    pub async fn should_throttle(&self) -> bool {
        let depth = self.queue_depth.load(Ordering::Relaxed);

        if depth >= self.high_watermark {
            self.throttled.store(true, Ordering::Relaxed);
            true
        } else if depth <= self.low_watermark {
            self.throttled.store(false, Ordering::Relaxed);
            false
        } else {
            self.throttled.load(Ordering::Relaxed)
        }
    }

    pub fn increment(&self) {
        self.queue_depth.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement(&self) {
        self.queue_depth.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Rate limiter for task execution
pub struct RateLimiter {
    /// Token bucket
    tokens: Arc<RwLock<f64>>,

    /// Refill rate (tokens per second)
    refill_rate: f64,

    /// Maximum tokens
    capacity: f64,

    /// Last refill timestamp
    last_refill: Arc<RwLock<Instant>>,
}

impl RateLimiter {
    pub async fn acquire(&self, tokens: f64) -> Result<()> {
        loop {
            // Refill tokens
            self.refill().await;

            let mut current_tokens = self.tokens.write().await;

            if *current_tokens >= tokens {
                *current_tokens -= tokens;
                return Ok(());
            }

            // Wait for refill
            drop(current_tokens);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn refill(&self) {
        let now = Instant::now();
        let mut last_refill = self.last_refill.write().await;

        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        let new_tokens = elapsed * self.refill_rate;

        let mut tokens = self.tokens.write().await;
        *tokens = (*tokens + new_tokens).min(self.capacity);

        *last_refill = now;
    }
}
```

---

## 5. Fault Tolerance

### 5.1 Checkpoint/Restart Mechanisms

```rust
/// Checkpoint manager with WAL-based durability
pub struct CheckpointManager {
    /// Write-ahead log
    wal: Arc<WriteAheadLog>,

    /// Checkpoint storage
    storage: Arc<dyn CheckpointStorage>,

    /// Checkpoint interval
    checkpoint_interval: Duration,
}

impl CheckpointManager {
    /// Create checkpoint for workflow execution
    pub async fn create_checkpoint(
        &self,
        execution: &WorkflowExecution,
    ) -> Result<CheckpointId> {
        let checkpoint = Checkpoint {
            id: CheckpointId::new(),
            execution_id: execution.id.clone(),
            timestamp: Utc::now(),

            // Serialize execution state
            state: bincode::serialize(&execution.state)?,
            task_states: bincode::serialize(&execution.tasks)?,
            outputs: bincode::serialize(&execution.outputs)?,

            // Metadata
            version: CHECKPOINT_VERSION,
            compressed: true,
        };

        // Write to WAL first
        self.wal.append(WalEntry::Checkpoint(checkpoint.clone())).await?;

        // Persist to storage
        self.storage.save(checkpoint.clone()).await?;

        Ok(checkpoint.id)
    }

    /// Restore workflow from checkpoint
    pub async fn restore_from_checkpoint(
        &self,
        checkpoint_id: CheckpointId,
    ) -> Result<WorkflowExecution> {
        // Load checkpoint
        let checkpoint = self.storage.load(checkpoint_id).await?;

        // Deserialize state
        let state = bincode::deserialize(&checkpoint.state)?;
        let task_states = bincode::deserialize(&checkpoint.task_states)?;
        let outputs = bincode::deserialize(&checkpoint.outputs)?;

        // Reconstruct execution
        Ok(WorkflowExecution {
            id: checkpoint.execution_id,
            state,
            tasks: task_states,
            outputs,
            ..Default::default()
        })
    }

    /// Automatic checkpointing background task
    pub async fn start_auto_checkpointing(
        self: Arc<Self>,
        execution_id: WorkflowExecutionId,
    ) {
        let mut interval = tokio::time::interval(self.checkpoint_interval);

        loop {
            interval.tick().await;

            // Load current execution state
            if let Ok(execution) = self.load_execution(&execution_id).await {
                if let Err(e) = self.create_checkpoint(&execution).await {
                    error!("Failed to create checkpoint: {}", e);
                }
            }
        }
    }
}

/// Write-ahead log for durability
pub struct WriteAheadLog {
    log_file: Arc<Mutex<File>>,
    flush_interval: Duration,
}

impl WriteAheadLog {
    pub async fn append(&self, entry: WalEntry) -> Result<()> {
        let mut file = self.log_file.lock().await;

        // Serialize entry
        let serialized = bincode::serialize(&entry)?;

        // Write length prefix
        file.write_all(&(serialized.len() as u64).to_le_bytes()).await?;

        // Write entry
        file.write_all(&serialized).await?;

        // Sync to disk (durability)
        file.sync_all().await?;

        Ok(())
    }

    pub async fn replay(&self) -> Result<Vec<WalEntry>> {
        let mut file = self.log_file.lock().await;
        file.seek(std::io::SeekFrom::Start(0)).await?;

        let mut entries = Vec::new();

        loop {
            // Read length prefix
            let mut len_buf = [0u8; 8];
            match file.read_exact(&mut len_buf).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }

            let len = u64::from_le_bytes(len_buf) as usize;

            // Read entry
            let mut entry_buf = vec![0u8; len];
            file.read_exact(&mut entry_buf).await?;

            let entry: WalEntry = bincode::deserialize(&entry_buf)?;
            entries.push(entry);
        }

        Ok(entries)
    }
}
```

### 5.2 Retry Policies and Circuit Breakers

```rust
/// Retry coordinator with exponential backoff
pub struct RetryCoordinator {
    policy: RetryPolicy,
    circuit_breaker: Arc<CircuitBreaker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_interval: Duration,
    pub max_interval: Duration,
    pub backoff_multiplier: f64,
    pub jitter: bool,
    pub retryable_errors: Vec<ErrorType>,
}

impl RetryCoordinator {
    pub async fn execute_with_retry<F, Fut, T>(
        &self,
        operation: F,
    ) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut interval = self.policy.initial_interval;

        loop {
            // Check circuit breaker
            if !self.circuit_breaker.allow_request().await {
                return Err(RetryError::CircuitBreakerOpen);
            }

            attempt += 1;

            match operation().await {
                Ok(result) => {
                    // Success - record in circuit breaker
                    self.circuit_breaker.record_success().await;
                    return Ok(result);
                }
                Err(e) => {
                    // Record failure
                    self.circuit_breaker.record_failure().await;

                    // Check if error is retryable
                    if !self.is_retryable(&e) {
                        return Err(e);
                    }

                    // Check max attempts
                    if attempt >= self.policy.max_attempts {
                        return Err(RetryError::MaxAttemptsExceeded {
                            attempts: attempt,
                            last_error: Box::new(e),
                        });
                    }

                    // Calculate backoff with jitter
                    let backoff = if self.policy.jitter {
                        let jitter = rand::random::<f64>() * 0.3; // +/- 30%
                        interval.mul_f64(1.0 + jitter - 0.15)
                    } else {
                        interval
                    };

                    info!(
                        "Retrying after {:?} (attempt {}/{})",
                        backoff, attempt, self.policy.max_attempts
                    );

                    tokio::time::sleep(backoff).await;

                    // Exponential backoff
                    interval = (interval.mul_f64(self.policy.backoff_multiplier))
                        .min(self.policy.max_interval);
                }
            }
        }
    }

    fn is_retryable(&self, error: &Error) -> bool {
        self.policy.retryable_errors.iter().any(|e| e.matches(error))
    }
}

/// Circuit breaker pattern implementation
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitBreakerState>>,
    config: CircuitBreakerConfig,
    metrics: Arc<CircuitBreakerMetrics>,
}

#[derive(Debug, Clone)]
pub enum CircuitBreakerState {
    Closed,
    Open { opened_at: Instant },
    HalfOpen,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
    pub half_open_max_requests: u32,
}

impl CircuitBreaker {
    pub async fn allow_request(&self) -> bool {
        let mut state = self.state.write().await;

        match *state {
            CircuitBreakerState::Closed => true,

            CircuitBreakerState::Open { opened_at } => {
                // Check if timeout has elapsed
                if opened_at.elapsed() >= self.config.timeout {
                    *state = CircuitBreakerState::HalfOpen;
                    true
                } else {
                    false
                }
            }

            CircuitBreakerState::HalfOpen => {
                // Allow limited requests in half-open state
                self.metrics.half_open_requests.load(Ordering::Relaxed)
                    < self.config.half_open_max_requests
            }
        }
    }

    pub async fn record_success(&self) {
        let mut state = self.state.write().await;

        match *state {
            CircuitBreakerState::HalfOpen => {
                let successes = self.metrics.consecutive_successes.fetch_add(1, Ordering::Relaxed);

                if successes >= self.config.success_threshold {
                    *state = CircuitBreakerState::Closed;
                    self.metrics.reset();
                    info!("Circuit breaker closed after recovery");
                }
            }
            _ => {
                self.metrics.consecutive_failures.store(0, Ordering::Relaxed);
            }
        }
    }

    pub async fn record_failure(&self) {
        let mut state = self.state.write().await;

        let failures = self.metrics.consecutive_failures.fetch_add(1, Ordering::Relaxed);

        match *state {
            CircuitBreakerState::Closed => {
                if failures >= self.config.failure_threshold {
                    *state = CircuitBreakerState::Open {
                        opened_at: Instant::now(),
                    };
                    warn!("Circuit breaker opened due to failures");
                }
            }

            CircuitBreakerState::HalfOpen => {
                *state = CircuitBreakerState::Open {
                    opened_at: Instant::now(),
                };
                warn!("Circuit breaker re-opened during recovery");
            }

            _ => {}
        }
    }
}
```

### 5.3 Dead Letter Queue

```rust
/// Dead letter queue for failed tasks
pub struct DeadLetterQueue {
    storage: Arc<dyn DlqStorage>,
    max_retries: u32,
    retention_period: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetter {
    pub id: DeadLetterId,
    pub original_task: TaskDefinition,
    pub execution_context: ExecutionContext,
    pub failure_reason: String,
    pub failure_count: u32,
    pub first_failed_at: DateTime<Utc>,
    pub last_failed_at: DateTime<Utc>,
    pub stack_trace: Option<String>,
}

impl DeadLetterQueue {
    /// Add failed task to DLQ
    pub async fn enqueue(&self, dead_letter: DeadLetter) -> Result<()> {
        info!(
            "Adding task {} to dead letter queue (failures: {})",
            dead_letter.original_task.id, dead_letter.failure_count
        );

        self.storage.save(dead_letter).await?;
        Ok(())
    }

    /// Retrieve dead letters for manual processing
    pub async fn list(&self, filter: DlqFilter) -> Result<Vec<DeadLetter>> {
        self.storage.query(filter).await
    }

    /// Retry a dead letter
    pub async fn retry(&self, id: DeadLetterId) -> Result<WorkflowExecutionId> {
        let dead_letter = self.storage.load(id).await?;

        if dead_letter.failure_count >= self.max_retries {
            return Err(DlqError::MaxRetriesExceeded);
        }

        // Re-submit to scheduler
        let execution_id = self.scheduler
            .schedule_task(
                dead_letter.original_task,
                dead_letter.execution_context,
            )
            .await?;

        Ok(execution_id)
    }

    /// Purge old dead letters
    pub async fn purge_expired(&self) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::from_std(self.retention_period)?;

        self.storage.delete_before(cutoff).await
    }
}
```

### 5.4 Graceful Degradation

```rust
/// Graceful degradation with fallback strategies
pub struct DegradationManager {
    fallback_strategies: HashMap<TaskId, FallbackStrategy>,
    health_monitor: Arc<HealthMonitor>,
}

#[derive(Debug, Clone)]
pub enum FallbackStrategy {
    /// Use cached result
    Cache { max_age: Duration },

    /// Use alternative executor
    Alternative { executor: ExecutorRef },

    /// Use default value
    DefaultValue { value: Value },

    /// Skip task and continue
    Skip,

    /// Custom fallback logic
    Custom(Arc<dyn Fn(TaskContext) -> Future<Output = Result<TaskOutput>> + Send + Sync>),
}

impl DegradationManager {
    pub async fn execute_with_fallback(
        &self,
        task: TaskDefinition,
        context: TaskContext,
    ) -> Result<TaskOutput> {
        // Try primary execution
        match self.executor_pool.execute(task.clone(), context.clone()).await {
            Ok(output) => Ok(output),

            Err(e) => {
                warn!("Primary execution failed: {}. Attempting fallback.", e);

                // Apply fallback strategy
                if let Some(strategy) = self.fallback_strategies.get(&task.id) {
                    self.apply_fallback(strategy, &task, &context).await
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn apply_fallback(
        &self,
        strategy: &FallbackStrategy,
        task: &TaskDefinition,
        context: &TaskContext,
    ) -> Result<TaskOutput> {
        match strategy {
            FallbackStrategy::Cache { max_age } => {
                // Try to get cached result
                if let Some(cached) = self.cache.get(&task.id).await {
                    if cached.age() < *max_age {
                        info!("Using cached result for task {}", task.id);
                        return Ok(cached.output);
                    }
                }
                Err(FallbackError::CacheMiss)
            }

            FallbackStrategy::Alternative { executor } => {
                info!("Using alternative executor: {}", executor);
                let alt_task = TaskDefinition {
                    executor: executor.clone(),
                    ..task.clone()
                };
                self.executor_pool.execute(alt_task, context.clone()).await
            }

            FallbackStrategy::DefaultValue { value } => {
                info!("Using default value for task {}", task.id);
                Ok(TaskOutput {
                    values: HashMap::from([("result".to_string(), value.clone())]),
                    metadata: Default::default(),
                })
            }

            FallbackStrategy::Skip => {
                info!("Skipping task {} due to failure", task.id);
                Ok(TaskOutput::empty())
            }

            FallbackStrategy::Custom(handler) => {
                handler(context.clone()).await
            }
        }
    }
}

/// Health monitor for degradation triggers
pub struct HealthMonitor {
    metrics: Arc<MetricsCollector>,
    thresholds: HealthThresholds,
}

#[derive(Debug, Clone)]
pub struct HealthThresholds {
    pub max_error_rate: f64,
    pub max_latency_p99: Duration,
    pub min_success_rate: f64,
}

impl HealthMonitor {
    pub async fn is_healthy(&self, executor: &ExecutorRef) -> bool {
        let metrics = self.metrics.get_executor_metrics(executor).await;

        metrics.error_rate < self.thresholds.max_error_rate
            && metrics.latency_p99 < self.thresholds.max_latency_p99
            && metrics.success_rate > self.thresholds.min_success_rate
    }
}
```

---

## 6. Deployment Modes

### 6.1 CLI-Based Workflow Runner

```rust
/// CLI application for running workflows
#[derive(Parser)]
#[command(name = "llm-orchestrator")]
#[command(about = "Production-grade LLM workflow orchestration")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run a workflow from a definition file
    Run {
        /// Path to workflow definition (YAML/JSON)
        #[arg(short, long)]
        workflow: PathBuf,

        /// Input parameters (JSON)
        #[arg(short, long)]
        inputs: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "json")]
        format: OutputFormat,
    },

    /// Validate a workflow definition
    Validate {
        #[arg(short, long)]
        workflow: PathBuf,
    },

    /// List workflow executions
    List {
        #[arg(short, long)]
        status: Option<ExecutionState>,

        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Get workflow execution status
    Status {
        #[arg(short, long)]
        execution_id: WorkflowExecutionId,
    },

    /// Cancel a running workflow
    Cancel {
        #[arg(short, long)]
        execution_id: WorkflowExecutionId,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize orchestrator
    let config = OrchestratorConfig::from_file("config.yaml")?;
    let orchestrator = Orchestrator::new(config).await?;

    match cli.command {
        Commands::Run { workflow, inputs, format } => {
            // Load workflow definition
            let workflow_def = WorkflowDefinition::from_file(&workflow)?;

            // Parse inputs
            let input_params = if let Some(json) = inputs {
                serde_json::from_str(&json)?
            } else {
                serde_json::Value::Null
            };

            // Execute workflow
            info!("Starting workflow execution: {}", workflow_def.metadata.name);

            let context = ExecutionContext::new(input_params);
            let execution_id = orchestrator.execute(workflow_def, context).await?;

            // Wait for completion
            let result = orchestrator.wait_for_completion(execution_id).await?;

            // Output result
            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                OutputFormat::Yaml => {
                    println!("{}", serde_yaml::to_string(&result)?);
                }
                OutputFormat::Table => {
                    print_result_table(&result);
                }
            }

            Ok(())
        }

        Commands::Validate { workflow } => {
            let workflow_def = WorkflowDefinition::from_file(&workflow)?;

            match orchestrator.validate(&workflow_def) {
                Ok(_) => {
                    println!("✓ Workflow is valid");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("✗ Validation failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::List { status, limit } => {
            let executions = orchestrator.list_executions(status, limit).await?;

            for exec in executions {
                println!(
                    "{} - {} - {}",
                    exec.id, exec.workflow_name, exec.state
                );
            }

            Ok(())
        }

        Commands::Status { execution_id } => {
            let status = orchestrator.get_status(execution_id).await?;

            println!("{}", serde_json::to_string_pretty(&status)?);

            Ok(())
        }

        Commands::Cancel { execution_id } => {
            orchestrator.cancel(execution_id).await?;

            println!("Workflow cancelled: {}", execution_id);

            Ok(())
        }
    }
}
```

### 6.2 Microservice Orchestrator (gRPC/REST API)

```rust
/// gRPC service definition
#[tonic::async_trait]
impl orchestrator_service_server::OrchestratorService for OrchestratorServiceImpl {
    async fn execute_workflow(
        &self,
        request: Request<ExecuteWorkflowRequest>,
    ) -> Result<Response<ExecuteWorkflowResponse>, Status> {
        let req = request.into_inner();

        // Parse workflow definition
        let workflow: WorkflowDefinition = serde_json::from_str(&req.workflow_json)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        // Parse inputs
        let inputs: Value = serde_json::from_str(&req.inputs_json)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        // Execute
        let context = ExecutionContext::new(inputs);
        let execution_id = self.orchestrator
            .execute(workflow, context)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(ExecuteWorkflowResponse {
            execution_id: execution_id.to_string(),
        }))
    }

    async fn get_execution_status(
        &self,
        request: Request<GetExecutionStatusRequest>,
    ) -> Result<Response<ExecutionStatus>, Status> {
        let req = request.into_inner();

        let execution_id = req.execution_id.parse()
            .map_err(|e| Status::invalid_argument(format!("Invalid execution ID: {}", e)))?;

        let status = self.orchestrator
            .get_status(execution_id)
            .await
            .map_err(|e| Status::not_found(e.to_string()))?;

        Ok(Response::new(status.into()))
    }

    async fn cancel_execution(
        &self,
        request: Request<CancelExecutionRequest>,
    ) -> Result<Response<CancelExecutionResponse>, Status> {
        let req = request.into_inner();

        let execution_id = req.execution_id.parse()
            .map_err(|e| Status::invalid_argument(format!("Invalid execution ID: {}", e)))?;

        self.orchestrator
            .cancel(execution_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CancelExecutionResponse {
            success: true,
        }))
    }

    type StreamExecutionEventsStream = ReceiverStream<Result<ExecutionEvent, Status>>;

    async fn stream_execution_events(
        &self,
        request: Request<StreamExecutionEventsRequest>,
    ) -> Result<Response<Self::StreamExecutionEventsStream>, Status> {
        let req = request.into_inner();

        let execution_id = req.execution_id.parse()
            .map_err(|e| Status::invalid_argument(format!("Invalid execution ID: {}", e)))?;

        let (tx, rx) = mpsc::channel(100);

        // Subscribe to execution events
        self.orchestrator
            .subscribe_events(execution_id, move |event| {
                let tx = tx.clone();
                async move {
                    tx.send(Ok(event.into())).await.ok();
                }
            })
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

/// REST API with Axum
pub async fn start_rest_api(orchestrator: Arc<Orchestrator>, port: u16) -> Result<()> {
    let app = Router::new()
        .route("/api/v1/workflows", post(execute_workflow))
        .route("/api/v1/workflows/:id", get(get_workflow_status))
        .route("/api/v1/workflows/:id", delete(cancel_workflow))
        .route("/api/v1/workflows/:id/events", get(stream_workflow_events))
        .route("/api/v1/health", get(health_check))
        .layer(Extension(orchestrator));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    info!("Starting REST API server on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn execute_workflow(
    Extension(orchestrator): Extension<Arc<Orchestrator>>,
    Json(request): Json<ExecuteWorkflowRequest>,
) -> Result<Json<ExecuteWorkflowResponse>, (StatusCode, String)> {
    let workflow: WorkflowDefinition = serde_json::from_str(&request.workflow_json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let inputs: Value = serde_json::from_str(&request.inputs_json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let context = ExecutionContext::new(inputs);
    let execution_id = orchestrator
        .execute(workflow, context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ExecuteWorkflowResponse {
        execution_id: execution_id.to_string(),
    }))
}
```

### 6.3 Embedded SDK Library

```rust
/// Embedded SDK for in-process workflow execution
pub struct OrchestratorSdk {
    orchestrator: Arc<Orchestrator>,
    config: SdkConfig,
}

impl OrchestratorSdk {
    /// Create a new SDK instance
    pub async fn new(config: SdkConfig) -> Result<Self> {
        let orchestrator = Orchestrator::new(config.orchestrator_config).await?;

        Ok(Self {
            orchestrator: Arc::new(orchestrator),
            config,
        })
    }

    /// Execute a workflow synchronously
    pub async fn execute_sync(
        &self,
        workflow: WorkflowDefinition,
        inputs: Value,
    ) -> Result<WorkflowResult> {
        let context = ExecutionContext::new(inputs);
        let execution_id = self.orchestrator.execute(workflow, context).await?;

        self.orchestrator.wait_for_completion(execution_id).await
    }

    /// Execute a workflow asynchronously
    pub async fn execute_async(
        &self,
        workflow: WorkflowDefinition,
        inputs: Value,
    ) -> Result<WorkflowExecutionHandle> {
        let context = ExecutionContext::new(inputs);
        let execution_id = self.orchestrator.execute(workflow, context).await?;

        Ok(WorkflowExecutionHandle {
            execution_id,
            orchestrator: self.orchestrator.clone(),
        })
    }

    /// Create a workflow builder
    pub fn workflow(&self, name: impl Into<String>) -> WorkflowBuilder {
        WorkflowBuilder::new(name.into())
    }
}

/// Fluent workflow builder API
pub struct WorkflowBuilder {
    definition: WorkflowDefinition,
}

impl WorkflowBuilder {
    pub fn new(name: String) -> Self {
        Self {
            definition: WorkflowDefinition {
                metadata: WorkflowMetadata {
                    name,
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    pub fn task(mut self, task: TaskDefinition) -> Self {
        self.definition.spec.tasks.push(task);
        self
    }

    pub fn input(mut self, name: String, type_def: TypeDefinition) -> Self {
        self.definition.spec.inputs.push(InputParameter {
            name,
            type_def,
            required: true,
            default: None,
        });
        self
    }

    pub fn output(mut self, name: String, mapping: OutputMapping) -> Self {
        self.definition.spec.outputs.insert(name, mapping);
        self
    }

    pub fn build(self) -> WorkflowDefinition {
        self.definition
    }
}

/// Execution handle for async workflows
pub struct WorkflowExecutionHandle {
    execution_id: WorkflowExecutionId,
    orchestrator: Arc<Orchestrator>,
}

impl WorkflowExecutionHandle {
    pub async fn wait(self) -> Result<WorkflowResult> {
        self.orchestrator.wait_for_completion(self.execution_id).await
    }

    pub async fn status(&self) -> Result<ExecutionStatus> {
        self.orchestrator.get_status(self.execution_id).await
    }

    pub async fn cancel(self) -> Result<()> {
        self.orchestrator.cancel(self.execution_id).await
    }

    pub async fn subscribe_events<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(ExecutionEvent) -> Future<Output = ()> + Send + Sync + 'static,
    {
        self.orchestrator.subscribe_events(self.execution_id, handler).await
    }
}

// Example usage
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedded_sdk() -> Result<()> {
        let sdk = OrchestratorSdk::new(SdkConfig::default()).await?;

        let workflow = sdk
            .workflow("example-workflow")
            .input("prompt".to_string(), TypeDefinition::String)
            .task(TaskDefinition {
                id: "llm-inference".into(),
                task_type: TaskType::Llm,
                executor: "llm-forge:claude-api".into(),
                inputs: HashMap::from([
                    ("prompt".to_string(), ValueExpression::WorkflowInput("prompt".to_string())),
                ]),
                ..Default::default()
            })
            .output(
                "response".to_string(),
                OutputMapping::TaskOutput {
                    task_id: "llm-inference".into(),
                    output_key: "response".to_string(),
                },
            )
            .build();

        let result = sdk
            .execute_sync(
                workflow,
                json!({ "prompt": "Hello, world!" }),
            )
            .await?;

        println!("Result: {:?}", result);

        Ok(())
    }
}
```

### 6.4 Hybrid Deployment Patterns

```rust
/// Hybrid deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridDeploymentConfig {
    /// Local execution for lightweight workflows
    pub local_executor: LocalExecutorConfig,

    /// Remote execution for heavy workflows
    pub remote_executor: RemoteExecutorConfig,

    /// Routing rules
    pub routing_rules: Vec<RoutingRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub condition: RoutingCondition,
    pub target: ExecutionTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingCondition {
    /// Route based on workflow size
    WorkflowSize { max_tasks: usize },

    /// Route based on estimated resources
    ResourceRequirements {
        max_cpu: f64,
        max_memory: u64,
    },

    /// Route based on workflow metadata
    Label { key: String, value: String },

    /// Custom routing logic
    Custom(String), // Expression or function name
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionTarget {
    Local,
    Remote { endpoint: String },
    Hybrid, // Split execution
}

pub struct HybridOrchestrator {
    local: Arc<Orchestrator>,
    remote_client: Arc<RemoteOrchestratorClient>,
    routing_rules: Vec<RoutingRule>,
}

impl HybridOrchestrator {
    pub async fn execute(
        &self,
        workflow: WorkflowDefinition,
        context: ExecutionContext,
    ) -> Result<WorkflowExecutionId> {
        // Determine execution target
        let target = self.route_workflow(&workflow);

        match target {
            ExecutionTarget::Local => {
                self.local.execute(workflow, context).await
            }

            ExecutionTarget::Remote { endpoint } => {
                self.remote_client.execute(&endpoint, workflow, context).await
            }

            ExecutionTarget::Hybrid => {
                // Split workflow for hybrid execution
                let (local_tasks, remote_tasks) = self.split_workflow(&workflow);

                // Execute both parts
                let local_handle = self.local.execute(local_tasks, context.clone()).await?;
                let remote_handle = self.remote_client
                    .execute(&"default".to_string(), remote_tasks, context)
                    .await?;

                // Create federated execution
                Ok(self.create_federated_execution(local_handle, remote_handle))
            }
        }
    }

    fn route_workflow(&self, workflow: &WorkflowDefinition) -> ExecutionTarget {
        for rule in &self.routing_rules {
            if rule.condition.matches(workflow) {
                return rule.target.clone();
            }
        }

        // Default to local
        ExecutionTarget::Local
    }
}
```

---

## 7. Integration Architecture

### 7.1 LLM-Forge Integration

```rust
/// LLM-Forge adapter for tool/SDK invocation
pub struct LlmForgeAdapter {
    client: llm_forge::Client,
    tool_registry: Arc<ToolRegistry>,
}

#[async_trait]
impl TaskExecutor for LlmForgeAdapter {
    async fn execute(
        &self,
        inputs: HashMap<String, Value>,
        context: TaskContext,
    ) -> Result<TaskOutput> {
        // Extract tool/SDK reference
        let tool_ref = inputs.get("tool")
            .and_then(|v| v.as_str())
            .ok_or(ExecutorError::MissingInput("tool"))?;

        // Resolve tool from registry
        let tool = self.tool_registry.get(tool_ref)?;

        // Prepare tool parameters
        let params = inputs.get("parameters")
            .cloned()
            .unwrap_or(Value::Null);

        // Invoke tool via LLM-Forge
        let result = self.client
            .invoke_tool(tool.id.clone(), params, context.into())
            .await?;

        // Convert result to task output
        Ok(TaskOutput {
            values: HashMap::from([
                ("result".to_string(), result.output),
                ("metadata".to_string(), json!(result.metadata)),
            ]),
            metadata: TaskMetadata {
                execution_time: result.execution_time,
                ..Default::default()
            },
        })
    }

    fn metadata(&self) -> ExecutorMetadata {
        ExecutorMetadata {
            name: "llm-forge".to_string(),
            version: "1.0.0".to_string(),
            description: "LLM-Forge tool/SDK executor".to_string(),
        }
    }

    fn validate_inputs(&self, inputs: &HashMap<String, Value>) -> Result<()> {
        if !inputs.contains_key("tool") {
            return Err(ExecutorError::MissingInput("tool"));
        }
        Ok(())
    }
}

/// Tool registry for LLM-Forge tools
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    pub async fn load_from_forge(&mut self) -> Result<()> {
        let client = llm_forge::Client::new()?;
        let tools = client.list_tools().await?;

        for tool in tools {
            self.tools.insert(tool.id.clone(), tool);
        }

        Ok(())
    }
}
```

### 7.2 LLM-Test-Bench Integration

```rust
/// LLM-Test-Bench integration for evaluation callbacks
pub struct TestBenchHooks {
    client: llm_test_bench::Client,
    metrics_collector: Arc<MetricsCollector>,
}

impl TestBenchHooks {
    /// Register evaluation hooks for workflow tasks
    pub fn register_hooks(&self, orchestrator: &mut Orchestrator) {
        // Hook: Before task execution
        orchestrator.on_task_start(|task, context| {
            let client = self.client.clone();
            async move {
                // Record test case
                client.record_test_case(TestCase {
                    id: task.id.clone(),
                    inputs: context.inputs.clone(),
                    timestamp: Utc::now(),
                }).await?;
                Ok(())
            }
        });

        // Hook: After task execution
        orchestrator.on_task_complete(|task, output, context| {
            let client = self.client.clone();
            let metrics = self.metrics_collector.clone();

            async move {
                // Evaluate output
                let evaluation = client.evaluate(EvaluationRequest {
                    task_id: task.id.clone(),
                    output: output.clone(),
                    ground_truth: context.ground_truth.clone(),
                    metrics: vec!["accuracy", "f1_score", "semantic_similarity"],
                }).await?;

                // Record metrics
                metrics.record_evaluation(task.id.clone(), evaluation).await;

                Ok(())
            }
        });

        // Hook: Workflow completion
        orchestrator.on_workflow_complete(|execution| {
            let client = self.client.clone();
            async move {
                // Generate test report
                let report = client.generate_report(execution.id).await?;

                info!("Test report: {}", report.summary);

                Ok(())
            }
        });
    }
}

/// Evaluation executor for inline evaluations
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
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_else(|| vec!["accuracy"]);

        let evaluation = self.client.evaluate(EvaluationRequest {
            task_id: context.task_id,
            output: response.clone(),
            ground_truth: ground_truth.cloned(),
            metrics,
        }).await?;

        Ok(TaskOutput {
            values: HashMap::from([
                ("score".to_string(), json!(evaluation.overall_score)),
                ("metrics".to_string(), json!(evaluation.metrics)),
                ("passed".to_string(), json!(evaluation.passed)),
            ]),
            metadata: Default::default(),
        })
    }

    fn metadata(&self) -> ExecutorMetadata {
        ExecutorMetadata {
            name: "llm-test-bench".to_string(),
            version: "1.0.0".to_string(),
            description: "LLM evaluation executor".to_string(),
        }
    }

    fn validate_inputs(&self, inputs: &HashMap<String, Value>) -> Result<()> {
        if !inputs.contains_key("response") {
            return Err(ExecutorError::MissingInput("response"));
        }
        Ok(())
    }
}
```

### 7.3 LLM-Auto-Optimizer Integration

```rust
/// LLM-Auto-Optimizer integration for performance metrics
pub struct AutoOptimizerCollector {
    client: llm_auto_optimizer::Client,
    metrics_buffer: Arc<Mutex<Vec<PerformanceMetric>>>,
    flush_interval: Duration,
}

impl AutoOptimizerCollector {
    /// Start background metric collection
    pub async fn start_collection(self: Arc<Self>) {
        let mut interval = tokio::time::interval(self.flush_interval);

        loop {
            interval.tick().await;

            if let Err(e) = self.flush_metrics().await {
                error!("Failed to flush metrics to Auto-Optimizer: {}", e);
            }
        }
    }

    /// Collect metrics from task execution
    pub async fn collect_task_metrics(
        &self,
        task: &TaskDefinition,
        output: &TaskOutput,
        context: &TaskContext,
    ) {
        let metric = PerformanceMetric {
            task_id: task.id.clone(),
            executor: task.executor.clone(),
            timestamp: Utc::now(),

            // Performance metrics
            execution_time: output.metadata.execution_time,
            cpu_usage: context.resource_usage.cpu,
            memory_usage: context.resource_usage.memory,

            // LLM-specific metrics
            token_count: output.metadata.token_count,
            cost: output.metadata.cost,

            // Quality metrics
            quality_score: output.metadata.quality_score,

            // Custom metrics
            custom: output.metadata.custom_metrics.clone(),
        };

        self.metrics_buffer.lock().await.push(metric);
    }

    /// Flush buffered metrics to Auto-Optimizer
    async fn flush_metrics(&self) -> Result<()> {
        let metrics = {
            let mut buffer = self.metrics_buffer.lock().await;
            std::mem::take(&mut *buffer)
        };

        if metrics.is_empty() {
            return Ok(());
        }

        self.client.submit_metrics(metrics).await?;

        Ok(())
    }

    /// Get optimization recommendations
    pub async fn get_recommendations(
        &self,
        workflow_id: WorkflowId,
    ) -> Result<OptimizationRecommendations> {
        self.client.get_recommendations(workflow_id).await
    }
}

/// Analytics executor for inline metric collection
pub struct AnalyticsExecutor {
    collector: Arc<AutoOptimizerCollector>,
}

#[async_trait]
impl TaskExecutor for AnalyticsExecutor {
    async fn execute(
        &self,
        inputs: HashMap<String, Value>,
        context: TaskContext,
    ) -> Result<TaskOutput> {
        // Collect metrics from inputs
        let metrics = inputs.get("metrics")
            .ok_or(ExecutorError::MissingInput("metrics"))?;

        let cost = inputs.get("cost")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        // Submit to Auto-Optimizer
        let metric = PerformanceMetric {
            task_id: context.task_id.clone(),
            timestamp: Utc::now(),
            cost,
            custom: metrics.clone(),
            ..Default::default()
        };

        self.collector.metrics_buffer.lock().await.push(metric);

        Ok(TaskOutput::empty())
    }

    fn metadata(&self) -> ExecutorMetadata {
        ExecutorMetadata {
            name: "analytics".to_string(),
            version: "1.0.0".to_string(),
            description: "Performance analytics executor".to_string(),
        }
    }

    fn validate_inputs(&self, inputs: &HashMap<String, Value>) -> Result<()> {
        if !inputs.contains_key("metrics") {
            return Err(ExecutorError::MissingInput("metrics"));
        }
        Ok(())
    }
}
```

### 7.4 LLM-Governance-Core Integration

```rust
/// LLM-Governance-Core integration for policy enforcement
pub struct GovernanceInterceptor {
    client: llm_governance::Client,
    policy_cache: Arc<RwLock<HashMap<PolicyId, Policy>>>,
}

impl GovernanceInterceptor {
    /// Intercept task execution for policy enforcement
    pub async fn intercept_execution(
        &self,
        task: &TaskDefinition,
        context: &TaskContext,
    ) -> Result<PolicyDecision> {
        // Load applicable policies
        let policies = self.get_applicable_policies(task).await?;

        // Evaluate policies
        for policy in policies {
            let decision = self.evaluate_policy(&policy, task, context).await?;

            match decision {
                PolicyDecision::Allow => continue,
                PolicyDecision::Deny { reason } => {
                    return Ok(PolicyDecision::Deny { reason });
                }
                PolicyDecision::RequireApproval { approver } => {
                    return Ok(PolicyDecision::RequireApproval { approver });
                }
            }
        }

        Ok(PolicyDecision::Allow)
    }

    /// Track costs for budget enforcement
    pub async fn track_cost(
        &self,
        task: &TaskDefinition,
        output: &TaskOutput,
    ) -> Result<()> {
        let cost = output.metadata.cost.unwrap_or(0.0);

        self.client.record_cost(CostRecord {
            task_id: task.id.clone(),
            workflow_id: output.metadata.workflow_id.clone(),
            timestamp: Utc::now(),
            cost,
            currency: "USD".to_string(),
            resource_type: task.executor.clone(),
        }).await?;

        // Check budget limits
        let budget_status = self.client.check_budget(
            output.metadata.workflow_id.clone()
        ).await?;

        if budget_status.exceeded {
            warn!(
                "Budget exceeded for workflow {}: {} / {}",
                output.metadata.workflow_id,
                budget_status.current,
                budget_status.limit
            );
        }

        Ok(())
    }

    async fn evaluate_policy(
        &self,
        policy: &Policy,
        task: &TaskDefinition,
        context: &TaskContext,
    ) -> Result<PolicyDecision> {
        self.client.evaluate_policy(PolicyEvaluationRequest {
            policy_id: policy.id.clone(),
            task: task.clone(),
            context: context.clone(),
        }).await
    }
}

/// Policy executor for inline policy checks
pub struct PolicyExecutor {
    governance: Arc<GovernanceInterceptor>,
}

#[async_trait]
impl TaskExecutor for PolicyExecutor {
    async fn execute(
        &self,
        inputs: HashMap<String, Value>,
        context: TaskContext,
    ) -> Result<TaskOutput> {
        // Extract usage information
        let usage = inputs.get("usage")
            .ok_or(ExecutorError::MissingInput("usage"))?;

        // Calculate cost
        let cost = self.calculate_cost(usage)?;

        // Track in governance system
        self.governance.client.record_cost(CostRecord {
            task_id: context.task_id.clone(),
            workflow_id: context.workflow_id.clone(),
            timestamp: Utc::now(),
            cost,
            currency: "USD".to_string(),
            resource_type: context.executor.clone(),
        }).await?;

        // Check budget
        let budget_status = self.governance.client
            .check_budget(context.workflow_id.clone())
            .await?;

        Ok(TaskOutput {
            values: HashMap::from([
                ("cost".to_string(), json!(cost)),
                ("within_budget".to_string(), json!(!budget_status.exceeded)),
                ("budget_remaining".to_string(), json!(budget_status.remaining)),
            ]),
            metadata: Default::default(),
        })
    }

    fn metadata(&self) -> ExecutorMetadata {
        ExecutorMetadata {
            name: "policy".to_string(),
            version: "1.0.0".to_string(),
            description: "Governance policy executor".to_string(),
        }
    }

    fn validate_inputs(&self, inputs: &HashMap<String, Value>) -> Result<()> {
        if !inputs.contains_key("usage") {
            return Err(ExecutorError::MissingInput("usage"));
        }
        Ok(())
    }

    fn calculate_cost(&self, usage: &Value) -> Result<f64> {
        // Example cost calculation for LLM usage
        let input_tokens = usage.get("prompt_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let output_tokens = usage.get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Pricing: $0.015/1K input tokens, $0.075/1K output tokens (example)
        let cost = (input_tokens as f64 / 1000.0 * 0.015)
            + (output_tokens as f64 / 1000.0 * 0.075);

        Ok(cost)
    }
}
```

---

## 8. Workflow Execution State Model

### 8.1 State Transition Diagram

```
                  ┌─────────┐
                  │ PENDING │
                  └────┬────┘
                       │
                       ▼
                  ┌─────────┐
            ┌────▶│ RUNNING │◀────┐
            │     └────┬────┘     │
            │          │          │
            │          ├──────────┤
            │          │          │
       RESUME        PAUSE    CONTINUE
            │          │          │
            │          ▼          │
            │     ┌─────────┐    │
            └─────│ PAUSED  │────┘
                  └─────────┘
                       │
                       │ CANCEL
                       ▼
            ┌────────────────────┐
            │     CANCELLED      │
            └────────────────────┘

    ┌─────────┐        ┌──────────┐        ┌──────────┐
    │ RUNNING │───────▶│ COMPLETED│        │  FAILED  │
    └─────────┘        └──────────┘        └──────────┘
         │                                       ▲
         │              ┌──────────┐             │
         └─────────────▶│ TIMED_OUT│─────────────┘
                        └──────────┘
```

### 8.2 State Model Implementation

```rust
/// Comprehensive execution state model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub id: WorkflowExecutionId,
    pub workflow: WorkflowDefinition,
    pub state: ExecutionState,
    pub tasks: HashMap<TaskId, TaskExecution>,
    pub outputs: HashMap<String, Value>,
    pub context: ExecutionContext,
    pub metadata: ExecutionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecution {
    pub task_id: TaskId,
    pub state: TaskExecutionState,
    pub attempts: Vec<TaskAttempt>,
    pub output: Option<TaskOutput>,
    pub error: Option<TaskError>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskExecutionState {
    Pending,
    Waiting { dependencies: Vec<TaskId> },
    Ready,
    Running { attempt: u32 },
    Retrying { attempt: u32, next_retry_at: DateTime<Utc> },
    Completed,
    Failed { retryable: bool },
    Skipped,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAttempt {
    pub attempt_number: u32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub output: Option<TaskOutput>,
    pub error: Option<TaskError>,
    pub resource_usage: ResourceUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration: Option<Duration>,
    pub triggered_by: TriggerSource,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerSource {
    Manual { user: String },
    Scheduled { cron: String },
    Event { event_id: String },
    Api { client_id: String },
}
```

---

## 9. Technology Stack Summary

| Component | Technology | Justification |
|-----------|-----------|---------------|
| **Language** | Rust | Memory safety, performance, fearless concurrency |
| **Async Runtime** | Tokio | Production-grade async I/O, excellent ecosystem |
| **Graph Processing** | petgraph | Robust DAG algorithms, cycle detection |
| **Serialization** | serde + JSON/YAML | Type-safe de/serialization, human-readable configs |
| **Database** | PostgreSQL + SQLx | ACID compliance, type-safe queries, async support |
| **Cache** | Redis | High-performance in-memory cache, pub/sub |
| **gRPC** | tonic | Type-safe RPC, bidirectional streaming |
| **REST API** | axum | Fast, ergonomic, built on tokio/tower |
| **Logging** | tracing | Structured logging, distributed tracing |
| **Metrics** | Prometheus client | Industry-standard metrics collection |
| **Testing** | tokio-test, mockall | Async testing, comprehensive mocking |

---

## 10. Deployment Architecture

### 10.1 Single-Node Deployment

```
┌─────────────────────────────────────────┐
│         Single Node (Standalone)        │
├─────────────────────────────────────────┤
│                                         │
│  ┌───────────────────────────────────┐  │
│  │   Orchestrator Process            │  │
│  │   • Scheduler                     │  │
│  │   • Executor Pool                 │  │
│  │   • State Manager                 │  │
│  └───────────────────────────────────┘  │
│                 │                       │
│                 ▼                       │
│  ┌───────────────────────────────────┐  │
│  │   Embedded SQLite                 │  │
│  │   • Execution state               │  │
│  │   • Workflow definitions          │  │
│  └───────────────────────────────────┘  │
│                                         │
│  Interfaces:                            │
│  • CLI commands                         │
│  • Local file-based configs             │
└─────────────────────────────────────────┘
```

### 10.2 Distributed Deployment

```
┌──────────────────────────────────────────────────────────────┐
│                     Load Balancer                            │
└────────────┬──────────────────────────┬──────────────────────┘
             │                          │
             ▼                          ▼
┌────────────────────────┐  ┌────────────────────────┐
│  API Server (Node 1)   │  │  API Server (Node 2)   │
│  • gRPC/REST           │  │  • gRPC/REST           │
│  • Authentication      │  │  • Authentication      │
└────────┬───────────────┘  └────────┬───────────────┘
         │                           │
         └───────────┬───────────────┘
                     ▼
         ┌───────────────────────┐
         │  Message Queue (NATS) │
         │  • Task distribution  │
         └───────────┬───────────┘
                     │
         ┌───────────┴───────────┐
         ▼                       ▼
┌─────────────────┐    ┌─────────────────┐
│  Worker Node 1  │    │  Worker Node 2  │
│  • Executor Pool│    │  • Executor Pool│
│  • Task Runner  │    │  • Task Runner  │
└─────────────────┘    └─────────────────┘
         │                       │
         └───────────┬───────────┘
                     ▼
         ┌───────────────────────┐
         │    PostgreSQL         │
         │    (Primary + Replica)│
         │    • Execution state  │
         └───────────────────────┘
                     │
                     ▼
         ┌───────────────────────┐
         │       Redis           │
         │       • Cache         │
         │       • Pub/Sub       │
         └───────────────────────┘
```

### 10.3 Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: llm-orchestrator-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: llm-orchestrator-api
  template:
    metadata:
      labels:
        app: llm-orchestrator-api
    spec:
      containers:
      - name: api-server
        image: llm-orchestrator:latest
        args: ["server", "--mode=api"]
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 50051
          name: grpc
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: orchestrator-secrets
              key: database-url
        - name: REDIS_URL
          value: redis://redis-service:6379
        resources:
          requests:
            cpu: "500m"
            memory: "512Mi"
          limits:
            cpu: "2000m"
            memory: "2Gi"
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: llm-orchestrator-worker
spec:
  replicas: 5
  selector:
    matchLabels:
      app: llm-orchestrator-worker
  template:
    metadata:
      labels:
        app: llm-orchestrator-worker
    spec:
      containers:
      - name: worker
        image: llm-orchestrator:latest
        args: ["server", "--mode=worker"]
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: orchestrator-secrets
              key: database-url
        - name: WORKER_THREADS
          value: "8"
        resources:
          requests:
            cpu: "1000m"
            memory: "1Gi"
          limits:
            cpu: "4000m"
            memory: "4Gi"
```

---

## 11. Observability and Monitoring

### 11.1 Metrics Collection

```rust
/// Prometheus metrics exporter
pub struct MetricsExporter {
    registry: prometheus::Registry,

    // Workflow metrics
    workflow_executions: prometheus::CounterVec,
    workflow_duration: prometheus::HistogramVec,
    workflow_state: prometheus::GaugeVec,

    // Task metrics
    task_executions: prometheus::CounterVec,
    task_duration: prometheus::HistogramVec,
    task_retries: prometheus::CounterVec,

    // Resource metrics
    cpu_usage: prometheus::Gauge,
    memory_usage: prometheus::Gauge,
    queue_depth: prometheus::Gauge,

    // Integration metrics
    llm_api_calls: prometheus::CounterVec,
    llm_token_usage: prometheus::CounterVec,
    llm_cost: prometheus::CounterVec,
}

impl MetricsExporter {
    pub fn record_workflow_execution(&self, workflow: &WorkflowDefinition, state: &ExecutionState) {
        self.workflow_executions
            .with_label_values(&[&workflow.metadata.name, &state.to_string()])
            .inc();
    }

    pub fn record_task_execution(&self, task: &TaskDefinition, duration: Duration, success: bool) {
        self.task_executions
            .with_label_values(&[
                &task.id.to_string(),
                &task.executor.to_string(),
                if success { "success" } else { "failure" },
            ])
            .inc();

        self.task_duration
            .with_label_values(&[&task.id.to_string()])
            .observe(duration.as_secs_f64());
    }
}
```

### 11.2 Distributed Tracing

```rust
use tracing::{info, warn, error, instrument};
use tracing_subscriber::layer::SubscriberExt;

/// Configure distributed tracing
pub fn setup_tracing() -> Result<()> {
    let jaeger_tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name("llm-orchestrator")
        .install_batch(opentelemetry::runtime::Tokio)?;

    let telemetry = tracing_opentelemetry::layer().with_tracer(jaeger_tracer);

    let subscriber = tracing_subscriber::Registry::default()
        .with(telemetry)
        .with(tracing_subscriber::fmt::layer());

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

#[instrument(skip(orchestrator, workflow))]
pub async fn execute_workflow_traced(
    orchestrator: &Orchestrator,
    workflow: WorkflowDefinition,
    context: ExecutionContext,
) -> Result<WorkflowExecutionId> {
    info!("Starting workflow execution: {}", workflow.metadata.name);

    let result = orchestrator.execute(workflow, context).await;

    match &result {
        Ok(id) => info!("Workflow scheduled: {}", id),
        Err(e) => error!("Workflow scheduling failed: {}", e),
    }

    result
}
```

---

## 12. Security Architecture

```rust
/// Authentication and authorization layer
pub struct SecurityLayer {
    auth_provider: Arc<dyn AuthProvider>,
    policy_engine: Arc<PolicyEngine>,
    audit_log: Arc<AuditLog>,
}

#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn authenticate(&self, credentials: Credentials) -> Result<Principal>;
    async fn authorize(&self, principal: &Principal, action: Action, resource: Resource) -> Result<bool>;
}

impl SecurityLayer {
    pub async fn authorize_workflow_execution(
        &self,
        principal: &Principal,
        workflow: &WorkflowDefinition,
    ) -> Result<()> {
        // Check permissions
        let authorized = self.auth_provider
            .authorize(
                principal,
                Action::Execute,
                Resource::Workflow(workflow.metadata.name.clone()),
            )
            .await?;

        if !authorized {
            return Err(SecurityError::Unauthorized);
        }

        // Audit log
        self.audit_log.record(AuditEvent {
            principal: principal.clone(),
            action: Action::Execute,
            resource: Resource::Workflow(workflow.metadata.name.clone()),
            timestamp: Utc::now(),
            result: AuditResult::Success,
        }).await?;

        Ok(())
    }
}
```

---

## Conclusion

This comprehensive architecture provides LLM-Orchestrator with:

1. **Production-grade reliability** through fault tolerance, checkpointing, and graceful degradation
2. **High performance** via Tokio-based async execution and resource-aware scheduling
3. **Flexible deployment** supporting CLI, microservice, embedded SDK, and hybrid modes
4. **Seamless integration** with the LLM DevOps ecosystem (Forge, Test-Bench, Auto-Optimizer, Governance)
5. **Scalability** through distributed execution and Kubernetes deployment
6. **Observability** with comprehensive metrics, tracing, and logging
7. **Security** through authentication, authorization, and audit logging

The architecture is designed to evolve with the project, supporting future enhancements like:
- GraphQL API for flexible querying
- WebSocket support for real-time updates
- Multi-tenancy and workspace isolation
- Advanced scheduling algorithms (gang scheduling, bin packing)
- ML-based workflow optimization
- Cross-cloud deployment support
