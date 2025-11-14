# Workflow Definition Schema Reference

**Version:** 1.0
**Last Updated:** 2025-11-14

---

## Table of Contents

1. [Overview](#overview)
2. [Schema Structure](#schema-structure)
3. [Field Reference](#field-reference)
4. [Expression Language](#expression-language)
5. [Built-in Executors](#built-in-executors)
6. [Examples](#examples)

---

## Overview

The LLM-Orchestrator workflow definition schema provides a declarative, type-safe way to define complex LLM workflows with DAG-based task dependencies, conditional branching, and event handling.

### Design Principles

- **Declarative**: Describe what to do, not how to do it
- **Type-safe**: Strong typing with JSON Schema validation
- **Composable**: Reusable workflow templates and task definitions
- **Extensible**: Plugin-based executor system
- **Version-controlled**: Workflow definitions stored as YAML/JSON files

---

## Schema Structure

### Root Schema

```yaml
apiVersion: string             # Required. API version (e.g., "orchestrator.llm/v1")
kind: WorkflowKind            # Required. Type of workflow
metadata: WorkflowMetadata     # Required. Metadata about the workflow
spec: WorkflowSpec            # Required. Workflow specification
```

### WorkflowKind

```yaml
kind: "Workflow" | "Template" | "CronWorkflow" | "EventWorkflow"
```

- **Workflow**: Standard one-time or manually triggered workflow
- **Template**: Reusable workflow template (can be instantiated)
- **CronWorkflow**: Scheduled workflow with cron expression
- **EventWorkflow**: Event-driven workflow triggered by external events

---

## Field Reference

### Metadata

```yaml
metadata:
  name: string                    # Required. Unique workflow name
  version: string                 # Required. Semantic version (e.g., "1.0.0")
  description: string             # Optional. Human-readable description
  labels:                         # Optional. Key-value labels for organization
    <key>: <value>
  annotations:                    # Optional. Additional metadata
    <key>: <value>
```

**Example:**

```yaml
metadata:
  name: llm-inference-pipeline
  version: 1.2.0
  description: Multi-stage LLM inference with evaluation
  labels:
    team: ai-research
    environment: production
    cost-center: ml-ops
  annotations:
    owner: john.doe@example.com
    documentation: https://docs.example.com/workflows/llm-inference
```

### Spec

```yaml
spec:
  config: WorkflowConfig         # Optional. Global workflow configuration
  inputs: InputParameter[]       # Optional. Workflow input parameters
  tasks: TaskDefinition[]        # Required. Task definitions (DAG nodes)
  branches: ConditionalBranch[]  # Optional. Conditional execution branches
  outputs: OutputMapping{}       # Optional. Output value mappings
  events: EventHandlers          # Optional. Event handler configuration
```

### WorkflowConfig

```yaml
config:
  timeout: duration              # Optional. Overall workflow timeout (e.g., "3600s")
  retryPolicy:                   # Optional. Default retry policy for all tasks
    maxAttempts: int
    backoffMultiplier: float
    initialInterval: duration
    maxInterval: duration
    retryableErrors: string[]
  concurrency:                   # Optional. Concurrency limits
    maxParallel: int             # Max parallel tasks
    resourceLimits:
      cpu: float                 # CPU cores
      memory: string             # Memory (e.g., "8Gi")
      gpu: int                   # GPU count
  failurePolicy:                 # Optional. How to handle failures
    mode: "failFast" | "continueOnError"
    deadLetterQueue: boolean
```

### InputParameter

```yaml
inputs:
  - name: string                 # Required. Parameter name
    type: TypeDefinition         # Required. Type specification
    required: boolean            # Optional. Default: false
    default: value               # Optional. Default value if not provided
    validation:                  # Optional. Validation rules
      minLength: int
      maxLength: int
      pattern: regex
      enum: value[]
      custom: expression
    description: string          # Optional. Documentation
```

**Type Definitions:**

```yaml
type: "string" | "int" | "float" | "boolean" | "object" | "array"

# For object types
type: object
schema:
  field1: TypeDefinition
  field2: TypeDefinition

# For array types
type: array
items: TypeDefinition
```

**Example:**

```yaml
inputs:
  - name: user_prompt
    type: string
    required: true
    validation:
      minLength: 1
      maxLength: 10000
    description: User input prompt for LLM

  - name: model_config
    type: object
    required: false
    default:
      temperature: 0.7
      max_tokens: 1000
    schema:
      temperature: float
      max_tokens: int
      top_p: float

  - name: evaluation_metrics
    type: array
    items: string
    default: ["accuracy", "f1_score"]
    validation:
      enum: ["accuracy", "precision", "recall", "f1_score", "bleu", "rouge"]
```

### TaskDefinition

```yaml
tasks:
  - id: TaskId                   # Required. Unique task identifier
    type: TaskType               # Required. Task type
    executor: ExecutorRef        # Required. Executor reference (e.g., "llm-forge:claude-api")
    dependsOn: TaskId[]          # Optional. Task dependencies
    inputs:                      # Required. Task input mappings
      <key>: ValueExpression
    outputs:                     # Required. Expected output schema
      <key>: TypeDefinition
    config:                      # Optional. Task-specific configuration
      timeout: duration
      retryPolicy: RetryPolicy
      resources:
        cpu: float
        memory: string
        gpu: int
    condition: Expression        # Optional. Conditional execution
    metadata: {}                 # Optional. Custom metadata
```

**TaskType Values:**

```yaml
type: "transform" | "llm" | "evaluation" | "policy" | "analytics" | "custom"
```

**Example:**

```yaml
tasks:
  - id: prompt_preprocessing
    type: transform
    executor: llm-forge:text-transform
    inputs:
      prompt: ${{ workflow.inputs.user_prompt }}
      operation: "normalize"
    outputs:
      processed_prompt: string
    config:
      timeout: 30s
      resources:
        cpu: 0.5
        memory: 512Mi

  - id: model_inference
    type: llm
    executor: llm-forge:claude-api
    dependsOn:
      - prompt_preprocessing
    inputs:
      prompt: ${{ tasks.prompt_preprocessing.outputs.processed_prompt }}
      config:
        model: claude-sonnet-4-5-20250929
        temperature: ${{ workflow.inputs.model_config.temperature }}
        max_tokens: ${{ workflow.inputs.model_config.max_tokens }}
    outputs:
      response: string
      usage:
        type: object
        schema:
          prompt_tokens: int
          completion_tokens: int
          total_tokens: int
    retryPolicy:
      maxAttempts: 5
      initialInterval: 1s
      maxInterval: 30s
      backoffMultiplier: 2.0
      retryableErrors:
        - RateLimitError
        - TimeoutError
        - ServiceUnavailable

  - id: response_evaluation
    type: evaluation
    executor: llm-test-bench:semantic-similarity
    dependsOn:
      - model_inference
    inputs:
      response: ${{ tasks.model_inference.outputs.response }}
      ground_truth: ${{ workflow.inputs.expected_response }}
      metrics: ${{ workflow.inputs.evaluation_metrics }}
    outputs:
      score: float
      metrics: object
      passed: boolean
    condition: ${{ workflow.inputs.enable_evaluation == true }}
```

### ConditionalBranch

```yaml
branches:
  - condition: Expression        # Required. Boolean expression
    tasks: TaskDefinition[]      # Required. Tasks to execute if condition is true
    else: TaskDefinition[]       # Optional. Tasks to execute if condition is false
```

**Example:**

```yaml
branches:
  - condition: ${{ tasks.response_evaluation.outputs.score < 0.7 }}
    tasks:
      - id: fallback_inference
        type: llm
        executor: llm-forge:gpt4-api
        inputs:
          prompt: ${{ tasks.prompt_preprocessing.outputs.processed_prompt }}
        outputs:
          response: string
          usage: object

      - id: fallback_notification
        type: custom
        executor: notification:slack
        inputs:
          message: "Fallback model used due to low quality score"
          channel: "#ml-alerts"

  - condition: ${{ tasks.cost_tracking.outputs.within_budget == false }}
    tasks:
      - id: budget_alert
        type: custom
        executor: notification:email
        inputs:
          to: "finance@example.com"
          subject: "Workflow budget exceeded"
          body: ${{ format("Budget exceeded: ${cost}", tasks.cost_tracking.outputs.cost) }}
```

### OutputMapping

```yaml
outputs:
  <output_name>:
    value: Expression            # Required. Output value expression
    fallback: Expression         # Optional. Fallback value if primary fails
    transform: TransformDef      # Optional. Output transformation
```

**Example:**

```yaml
outputs:
  final_response:
    value: ${{ tasks.model_inference.outputs.response }}
    fallback: ${{ tasks.fallback_inference.outputs.response }}

  quality_score:
    value: ${{ tasks.response_evaluation.outputs.score }}
    transform:
      type: round
      decimals: 2

  total_cost:
    value: ${{ tasks.cost_tracking.outputs.cost }}

  execution_summary:
    value: |
      {
        "response": "${{ outputs.final_response }}",
        "quality": ${{ outputs.quality_score }},
        "cost": ${{ outputs.total_cost }},
        "model": "${{ tasks.model_inference.config.model }}"
      }
```

### EventHandlers

```yaml
events:
  onSuccess: EventAction[]       # Optional. Actions on successful completion
  onFailure: EventAction[]       # Optional. Actions on failure
  onTimeout: EventAction[]       # Optional. Actions on timeout
  onCancel: EventAction[]        # Optional. Actions on cancellation
```

**EventAction Types:**

```yaml
# Webhook
- type: webhook
  url: string
  method: "GET" | "POST" | "PUT"
  headers: {}
  body: template

# Alert
- type: alert
  channel: "slack" | "email" | "pagerduty"
  severity: "low" | "medium" | "high" | "critical"
  message: template

# Dead Letter Queue
- type: deadletter
  queue: string

# Custom handler
- type: custom
  executor: ExecutorRef
  inputs: {}
```

**Example:**

```yaml
events:
  onSuccess:
    - type: webhook
      url: https://api.example.com/workflow-complete
      method: POST
      headers:
        Authorization: Bearer ${{ secrets.api_token }}
      body: |
        {
          "workflow_id": "${{ workflow.id }}",
          "status": "success",
          "outputs": ${{ workflow.outputs }}
        }

    - type: alert
      channel: slack
      severity: low
      message: "Workflow ${{ workflow.metadata.name }} completed successfully"

  onFailure:
    - type: alert
      channel: slack
      severity: high
      message: |
        Workflow failed: ${{ workflow.metadata.name }}
        Error: ${{ workflow.error.message }}
        Execution ID: ${{ workflow.execution_id }}

    - type: deadletter
      queue: failed-workflows

  onTimeout:
    - type: alert
      channel: pagerduty
      severity: critical
      message: "Workflow timeout: ${{ workflow.metadata.name }}"
```

---

## Expression Language

### Syntax

LLM-Orchestrator uses a template expression language similar to GitHub Actions:

```
${{ <expression> }}
```

### Context Variables

```yaml
# Workflow context
workflow.id                      # Workflow execution ID
workflow.metadata.name           # Workflow name
workflow.metadata.version        # Workflow version
workflow.inputs.<name>           # Workflow input parameter

# Task context
tasks.<task_id>.outputs.<key>   # Task output value
tasks.<task_id>.state           # Task execution state
tasks.<task_id>.error           # Task error (if failed)

# System context
system.timestamp                # Current timestamp
system.user                     # Current user/principal
system.environment              # Environment name

# Secrets (from external secret store)
secrets.<key>                   # Secret value
```

### Operators

```yaml
# Comparison
==, !=, <, <=, >, >=

# Logical
&&, ||, !

# Arithmetic
+, -, *, /, %

# String
contains(str, substr)
startsWith(str, prefix)
endsWith(str, suffix)
```

### Functions

```yaml
# String functions
format(template, ...args)        # String formatting
lower(str)                       # Lowercase
upper(str)                       # Uppercase
trim(str)                        # Trim whitespace
replace(str, old, new)           # Replace substring

# Math functions
round(num, decimals)             # Round number
floor(num)                       # Floor
ceil(num)                        # Ceiling
min(...nums)                     # Minimum
max(...nums)                     # Maximum

# JSON functions
toJson(obj)                      # Convert to JSON string
fromJson(str)                    # Parse JSON string

# Array functions
length(arr)                      # Array length
join(arr, separator)             # Join array elements
contains(arr, value)             # Check if array contains value

# Conditional
if(condition, trueValue, falseValue)
```

### Examples

```yaml
# Simple reference
${{ workflow.inputs.user_prompt }}

# Nested access
${{ tasks.model_inference.outputs.usage.total_tokens }}

# Conditional
${{ tasks.evaluation.outputs.score >= 0.8 }}

# Function call
${{ format("Total cost: ${}", tasks.cost.outputs.total) }}

# Complex expression
${{ tasks.inference.outputs.usage.prompt_tokens * 0.015 / 1000 + tasks.inference.outputs.usage.completion_tokens * 0.075 / 1000 }}

# String interpolation
${{ "Model: " + tasks.inference.config.model + ", Score: " + round(tasks.evaluation.outputs.score, 2) }}

# Conditional with fallback
${{ if(contains(tasks.inference.outputs.response, "error"), tasks.fallback.outputs.response, tasks.inference.outputs.response) }}
```

---

## Built-in Executors

### Transform Executors

#### llm-forge:text-transform

**Purpose:** Text preprocessing and transformation

**Inputs:**
```yaml
inputs:
  prompt: string                 # Text to transform
  operation: string              # Operation type
  config: object                 # Operation-specific config
```

**Operations:**
- `normalize`: Normalize whitespace and formatting
- `truncate`: Truncate to max length
- `template`: Apply template with variables
- `extract`: Extract content using regex

**Outputs:**
```yaml
outputs:
  result: string                 # Transformed text
```

#### llm-forge:json-transform

**Purpose:** JSON data transformation

**Inputs:**
```yaml
inputs:
  data: object                   # Input JSON
  jsonpath: string               # JSONPath expression
  schema: object                 # Output schema
```

**Outputs:**
```yaml
outputs:
  result: object                 # Transformed JSON
```

### LLM Executors

#### llm-forge:claude-api

**Purpose:** Claude API inference

**Inputs:**
```yaml
inputs:
  prompt: string                 # User prompt
  config:
    model: string                # Model name
    temperature: float           # Temperature (0.0-1.0)
    max_tokens: int              # Max output tokens
    top_p: float                 # Nucleus sampling
    stop_sequences: string[]     # Stop sequences
```

**Outputs:**
```yaml
outputs:
  response: string               # Model response
  usage:
    prompt_tokens: int
    completion_tokens: int
    total_tokens: int
  model: string                  # Model used
```

#### llm-forge:gpt4-api

**Purpose:** GPT-4 API inference

**Inputs:** Same as `claude-api`

**Outputs:** Same as `claude-api`

### Evaluation Executors

#### llm-test-bench:semantic-similarity

**Purpose:** Semantic similarity evaluation

**Inputs:**
```yaml
inputs:
  response: string               # Model response
  ground_truth: string           # Expected response
  threshold: float               # Similarity threshold
```

**Outputs:**
```yaml
outputs:
  score: float                   # Similarity score (0.0-1.0)
  passed: boolean                # Whether score >= threshold
  metrics:
    cosine_similarity: float
    embedding_distance: float
```

#### llm-test-bench:multi-metric

**Purpose:** Multi-metric evaluation

**Inputs:**
```yaml
inputs:
  response: string
  ground_truth: string
  metrics: string[]              # Metric names
```

**Outputs:**
```yaml
outputs:
  overall_score: float
  passed: boolean
  metrics:
    <metric_name>: float
```

### Policy Executors

#### llm-governance:cost-tracker

**Purpose:** Cost tracking and budget enforcement

**Inputs:**
```yaml
inputs:
  usage: object                  # Token usage
  model: string                  # Model name
  pricing: object                # Pricing config (optional)
```

**Outputs:**
```yaml
outputs:
  cost: float                    # Calculated cost (USD)
  within_budget: boolean         # Whether within budget limits
  budget_remaining: float        # Remaining budget
```

#### llm-governance:policy-check

**Purpose:** Policy enforcement

**Inputs:**
```yaml
inputs:
  content: string                # Content to check
  policies: string[]             # Policy IDs
```

**Outputs:**
```yaml
outputs:
  passed: boolean
  violations: object[]
```

### Analytics Executors

#### llm-auto-optimizer:metric-collector

**Purpose:** Performance metric collection

**Inputs:**
```yaml
inputs:
  metrics: object                # Metrics to collect
  context: object                # Execution context
```

**Outputs:**
```yaml
outputs:
  collected: boolean
```

---

## Examples

### Example 1: Simple LLM Inference

```yaml
apiVersion: orchestrator.llm/v1
kind: Workflow
metadata:
  name: simple-inference
  version: 1.0.0
  description: Simple LLM inference workflow

spec:
  inputs:
    - name: prompt
      type: string
      required: true

  tasks:
    - id: inference
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

  outputs:
    response:
      value: ${{ tasks.inference.outputs.response }}
```

### Example 2: Multi-Stage Pipeline with Evaluation

```yaml
apiVersion: orchestrator.llm/v1
kind: Workflow
metadata:
  name: llm-pipeline-with-eval
  version: 2.0.0
  description: Multi-stage LLM pipeline with quality evaluation

spec:
  config:
    timeout: 600s
    concurrency:
      maxParallel: 3

  inputs:
    - name: user_query
      type: string
      required: true
    - name: expected_answer
      type: string
      required: false
    - name: quality_threshold
      type: float
      default: 0.75

  tasks:
    # Stage 1: Query preprocessing
    - id: preprocess
      type: transform
      executor: llm-forge:text-transform
      inputs:
        prompt: ${{ workflow.inputs.user_query }}
        operation: normalize
      outputs:
        processed_query: string

    # Stage 2: Primary inference
    - id: primary_inference
      type: llm
      executor: llm-forge:claude-api
      dependsOn:
        - preprocess
      inputs:
        prompt: ${{ tasks.preprocess.outputs.processed_query }}
        config:
          model: claude-sonnet-4-5-20250929
          temperature: 0.7
          max_tokens: 2000
      outputs:
        response: string
        usage: object
      retryPolicy:
        maxAttempts: 3

    # Stage 3: Parallel evaluation and cost tracking
    - id: evaluate
      type: evaluation
      executor: llm-test-bench:semantic-similarity
      dependsOn:
        - primary_inference
      inputs:
        response: ${{ tasks.primary_inference.outputs.response }}
        ground_truth: ${{ workflow.inputs.expected_answer }}
        threshold: ${{ workflow.inputs.quality_threshold }}
      outputs:
        score: float
        passed: boolean
        metrics: object
      condition: ${{ workflow.inputs.expected_answer != null }}

    - id: track_cost
      type: policy
      executor: llm-governance:cost-tracker
      dependsOn:
        - primary_inference
      inputs:
        usage: ${{ tasks.primary_inference.outputs.usage }}
        model: claude-sonnet-4-5-20250929
      outputs:
        cost: float
        within_budget: boolean

  # Conditional fallback
  branches:
    - condition: ${{ tasks.evaluate.outputs.passed == false }}
      tasks:
        - id: fallback_inference
          type: llm
          executor: llm-forge:gpt4-api
          inputs:
            prompt: ${{ tasks.preprocess.outputs.processed_query }}
            config:
              model: gpt-4-turbo
              temperature: 0.5
          outputs:
            response: string
            usage: object

  outputs:
    final_response:
      value: ${{ tasks.primary_inference.outputs.response }}
      fallback: ${{ tasks.fallback_inference.outputs.response }}
    quality_score:
      value: ${{ tasks.evaluate.outputs.score }}
    cost:
      value: ${{ tasks.track_cost.outputs.cost }}
    used_fallback:
      value: ${{ tasks.fallback_inference.state == "completed" }}

  events:
    onFailure:
      - type: alert
        channel: slack
        severity: high
        message: "LLM pipeline failed for query: ${{ workflow.inputs.user_query }}"
```

### Example 3: Scheduled Batch Processing

```yaml
apiVersion: orchestrator.llm/v1
kind: CronWorkflow
metadata:
  name: daily-batch-summarization
  version: 1.0.0
  description: Daily batch summarization of documents

spec:
  schedule: "0 2 * * *"  # 2 AM daily

  config:
    concurrency:
      maxParallel: 10
    failurePolicy:
      mode: continueOnError

  inputs:
    - name: source_bucket
      type: string
      default: s3://documents/pending

  tasks:
    - id: fetch_documents
      type: custom
      executor: storage:s3-list
      inputs:
        bucket: ${{ workflow.inputs.source_bucket }}
      outputs:
        documents: array

    - id: process_documents
      type: custom
      executor: orchestrator:fan-out
      dependsOn:
        - fetch_documents
      inputs:
        items: ${{ tasks.fetch_documents.outputs.documents }}
        task_template:
          id: summarize_document
          type: llm
          executor: llm-forge:claude-api
          inputs:
            document: ${{ item }}
            config:
              model: claude-sonnet-4-5-20250929
              temperature: 0.3
              max_tokens: 500
          outputs:
            summary: string
      outputs:
        summaries: array

    - id: aggregate_results
      type: custom
      executor: storage:s3-upload
      dependsOn:
        - process_documents
      inputs:
        bucket: s3://summaries/output
        data: ${{ tasks.process_documents.outputs.summaries }}

  events:
    onSuccess:
      - type: webhook
        url: https://api.example.com/batch-complete
    onFailure:
      - type: alert
        channel: email
        severity: medium
```

### Example 4: Event-Driven Workflow

```yaml
apiVersion: orchestrator.llm/v1
kind: EventWorkflow
metadata:
  name: support-ticket-classifier
  version: 1.0.0
  description: Classify and route support tickets

spec:
  trigger:
    eventSource: support-system
    eventType: ticket.created
    filter: ${{ event.priority == "high" }}

  inputs:
    - name: ticket
      type: object
      source: ${{ event.data.ticket }}

  tasks:
    - id: classify_ticket
      type: llm
      executor: llm-forge:claude-api
      inputs:
        prompt: |
          Classify the following support ticket:

          Subject: ${{ workflow.inputs.ticket.subject }}
          Description: ${{ workflow.inputs.ticket.description }}

          Categories: technical, billing, feature-request, bug-report

          Return only the category name.
        config:
          model: claude-sonnet-4-5-20250929
          temperature: 0.1
          max_tokens: 50
      outputs:
        category: string

    - id: route_ticket
      type: custom
      executor: support:assign-ticket
      dependsOn:
        - classify_ticket
      inputs:
        ticket_id: ${{ workflow.inputs.ticket.id }}
        category: ${{ tasks.classify_ticket.outputs.category }}
        routing_rules:
          technical: team-engineering
          billing: team-finance
          feature-request: team-product
          bug-report: team-qa

  events:
    onSuccess:
      - type: webhook
        url: https://support.example.com/api/ticket-routed
        body: |
          {
            "ticket_id": "${{ workflow.inputs.ticket.id }}",
            "category": "${{ tasks.classify_ticket.outputs.category }}"
          }
```

---

## Schema Validation

### JSON Schema

The workflow definition schema is validated using JSON Schema. The full schema definition is available at:

```
https://orchestrator.llm/schemas/v1/workflow.schema.json
```

### Validation Rules

1. **Required Fields:**
   - `apiVersion`, `kind`, `metadata`, `spec`
   - `metadata.name`, `metadata.version`
   - `spec.tasks` (at least one task)

2. **Naming Conventions:**
   - Task IDs: `[a-z0-9_-]+`
   - Workflow names: `[a-z0-9_-]+`
   - Labels/annotations: `[a-z0-9._-]+`

3. **Dependency Validation:**
   - No circular dependencies
   - All referenced tasks must exist
   - Conditional branches cannot create cycles

4. **Type Validation:**
   - Input/output types must match between tasks
   - Expression return types must match expected types
   - Executor inputs must match executor schema

5. **Resource Limits:**
   - CPU: 0.1 - 64.0 cores
   - Memory: 128Mi - 256Gi
   - Timeout: 1s - 86400s (24 hours)

---

## Best Practices

1. **Use explicit versioning**: Always specify workflow versions for reproducibility
2. **Add descriptions**: Document workflows and complex tasks
3. **Set timeouts**: Prevent runaway executions
4. **Use retry policies**: Handle transient failures gracefully
5. **Validate inputs**: Use validation rules to catch errors early
6. **Modular design**: Break complex workflows into smaller, reusable templates
7. **Resource limits**: Set appropriate CPU/memory limits
8. **Error handling**: Use conditional branches for fallback logic
9. **Monitoring**: Add event handlers for observability
10. **Cost tracking**: Always include cost tracking for LLM tasks
