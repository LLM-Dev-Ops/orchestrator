# Observability and Monitoring

This document describes the observability infrastructure implemented in the LLM Orchestrator, including metrics, tracing, logging, and health checks.

## Overview

The LLM Orchestrator includes comprehensive observability features to monitor workflow execution, LLM provider performance, and system health in production environments.

**Key Features:**
- Prometheus metrics for monitoring
- Distributed tracing with OpenTelemetry-compatible spans
- Structured logging with correlation IDs
- Health check endpoints for Kubernetes probes
- Low overhead design (< 1% performance impact)

## Metrics

### Available Metrics

The following Prometheus metrics are automatically collected:

#### Workflow Metrics

**`orchestrator_workflow_executions_total`** (Counter)
- Description: Total number of workflow executions
- Labels:
  - `status`: "success" | "failure"
  - `workflow_name`: name of the workflow
- Example: `orchestrator_workflow_executions_total{status="success",workflow_name="rag-pipeline"} 1523`

**`orchestrator_workflow_duration_seconds`** (Histogram)
- Description: Workflow execution duration in seconds
- Labels:
  - `workflow_name`: name of the workflow
- Buckets: [0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0]
- Example: `orchestrator_workflow_duration_seconds_bucket{workflow_name="rag-pipeline",le="5.0"} 1200`

**`orchestrator_active_workflows`** (Gauge)
- Description: Number of currently executing workflows
- Example: `orchestrator_active_workflows 12`

#### LLM Provider Metrics

**`orchestrator_llm_requests_total`** (Counter)
- Description: Total LLM provider requests
- Labels:
  - `provider`: "anthropic" | "openai" | etc.
  - `model`: model identifier (e.g., "claude-3-5-sonnet")
  - `status`: "success" | "failure"
- Example: `orchestrator_llm_requests_total{provider="anthropic",model="claude-3-5-sonnet",status="success"} 3421`

**`orchestrator_llm_tokens_total`** (Counter)
- Description: Total tokens consumed
- Labels:
  - `provider`: provider name
  - `model`: model identifier
  - `type`: "input" | "output"
- Example: `orchestrator_llm_tokens_total{provider="anthropic",model="claude-3-5-sonnet",type="input"} 1234567`

**`orchestrator_llm_request_duration_seconds`** (Histogram)
- Description: LLM request duration in seconds
- Labels:
  - `provider`: provider name
  - `model`: model identifier
- Buckets: [0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0]
- Example: `orchestrator_llm_request_duration_seconds_bucket{provider="openai",model="gpt-4",le="2.0"} 891`

#### Step Execution Metrics

**`orchestrator_step_executions_total`** (Counter)
- Description: Total step executions by type
- Labels:
  - `step_type`: "llm" | "embed" | "vector_search" | "transform" | "action"
  - `status`: "success" | "failure" | "skipped"
- Example: `orchestrator_step_executions_total{step_type="llm",status="success"} 4567`

**`orchestrator_step_duration_seconds`** (Histogram)
- Description: Step execution duration in seconds
- Labels:
  - `step_type`: type of step
- Buckets: [0.01, 0.05, 0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0]
- Example: `orchestrator_step_duration_seconds_bucket{step_type="llm",le="1.0"} 3456`

#### Error Metrics

**`orchestrator_errors_total`** (Counter)
- Description: Total errors by type and component
- Labels:
  - `error_type`: "timeout" | "provider_error" | "validation" | etc.
  - `component`: "executor" | "provider" | "step_executor"
- Example: `orchestrator_errors_total{error_type="timeout",component="executor"} 23`

### Accessing Metrics

#### In Code

```rust
use llm_orchestrator_core::metrics;

// Metrics are automatically recorded by the executor
// You can also manually record metrics:

// Workflow metrics
metrics::record_workflow_start();
metrics::record_workflow_complete("my-workflow", 5.2, true);

// LLM request metrics
metrics::record_llm_request(
    "anthropic",
    "claude-3-5-sonnet",
    2.3,
    true,
    Some(100),  // input tokens
    Some(50),   // output tokens
);

// Step metrics
metrics::record_step_execution("llm", 1.2, "success");

// Error metrics
metrics::record_error("timeout", "executor");

// Export metrics in Prometheus format
let metrics_text = metrics::gather_metrics();
println!("{}", metrics_text);
```

#### HTTP Endpoint (for future API server)

When the LLM Orchestrator API server is implemented, metrics will be available at:

```
GET /metrics
Content-Type: text/plain

# HELP orchestrator_workflow_executions_total Total workflow executions
# TYPE orchestrator_workflow_executions_total counter
orchestrator_workflow_executions_total{status="success",workflow_name="rag-pipeline"} 1523
orchestrator_workflow_executions_total{status="failure",workflow_name="rag-pipeline"} 12
...
```

### Example Metrics Output

```
# HELP orchestrator_workflow_executions_total Total number of workflow executions
# TYPE orchestrator_workflow_executions_total counter
orchestrator_workflow_executions_total{status="success",workflow_name="rag-pipeline"} 1523
orchestrator_workflow_executions_total{status="failure",workflow_name="rag-pipeline"} 12

# HELP orchestrator_workflow_duration_seconds Workflow execution duration in seconds
# TYPE orchestrator_workflow_duration_seconds histogram
orchestrator_workflow_duration_seconds_bucket{workflow_name="rag-pipeline",le="0.1"} 0
orchestrator_workflow_duration_seconds_bucket{workflow_name="rag-pipeline",le="0.5"} 45
orchestrator_workflow_duration_seconds_bucket{workflow_name="rag-pipeline",le="1.0"} 234
orchestrator_workflow_duration_seconds_bucket{workflow_name="rag-pipeline",le="2.5"} 890
orchestrator_workflow_duration_seconds_bucket{workflow_name="rag-pipeline",le="5.0"} 1200
orchestrator_workflow_duration_seconds_bucket{workflow_name="rag-pipeline",le="+Inf"} 1535
orchestrator_workflow_duration_seconds_sum{workflow_name="rag-pipeline"} 4521.3
orchestrator_workflow_duration_seconds_count{workflow_name="rag-pipeline"} 1535

# HELP orchestrator_active_workflows Number of currently executing workflows
# TYPE orchestrator_active_workflows gauge
orchestrator_active_workflows 12

# HELP orchestrator_llm_requests_total Total LLM provider requests
# TYPE orchestrator_llm_requests_total counter
orchestrator_llm_requests_total{provider="anthropic",model="claude-3-5-sonnet",status="success"} 3421
orchestrator_llm_requests_total{provider="openai",model="gpt-4",status="success"} 891

# HELP orchestrator_llm_tokens_total Total tokens consumed by LLM providers
# TYPE orchestrator_llm_tokens_total counter
orchestrator_llm_tokens_total{provider="anthropic",model="claude-3-5-sonnet",type="input"} 1234567
orchestrator_llm_tokens_total{provider="anthropic",model="claude-3-5-sonnet",type="output"} 567890

# HELP orchestrator_errors_total Total errors by type and component
# TYPE orchestrator_errors_total counter
orchestrator_errors_total{error_type="timeout",component="executor"} 23
orchestrator_errors_total{error_type="provider_error",component="step_executor"} 45
```

## Distributed Tracing

The executor uses OpenTelemetry-compatible tracing spans to provide distributed tracing across workflow execution.

### Instrumented Operations

All workflow and step executions are automatically instrumented with tracing spans:

```rust
// Workflow execution span
#[instrument(skip(self), fields(workflow_id = %self.workflow.id, workflow_name = %self.workflow.name))]
async fn execute_inner(&self) -> Result<HashMap<String, StepResult>> {
    // ...
}

// Step execution span
#[instrument(skip(self, step), fields(step_id = %step.id, step_type = ?step.step_type))]
async fn execute_step(&self, step: &Step) -> Result<StepResult> {
    // ...
}
```

### Span Hierarchy

```
workflow.execute:my-workflow
├── step.execute:step1 (llm)
├── step.execute:step2 (transform)
├── step.execute:step3 (llm)
└── step.execute:step4 (action)
```

### Integration with Tracing Backends

The tracing is compatible with OpenTelemetry and can be exported to:
- Jaeger
- Zipkin
- Tempo
- AWS X-Ray
- Google Cloud Trace

Example configuration (future):
```rust
use tracing_subscriber::layer::SubscriberExt;
use tracing_opentelemetry::OpenTelemetryLayer;

let tracer = opentelemetry_jaeger::new_pipeline()
    .with_service_name("llm-orchestrator")
    .install_batch(opentelemetry::runtime::Tokio)
    .expect("Failed to install tracer");

let telemetry = OpenTelemetryLayer::new(tracer);

tracing_subscriber::registry()
    .with(telemetry)
    .init();
```

## Structured Logging

All logging uses the `tracing` crate for structured, contextual logging.

### Log Levels

- **TRACE**: Verbose debugging information
- **DEBUG**: Detailed operational information
- **INFO**: Normal operational events
- **WARN**: Warning conditions
- **ERROR**: Error conditions

### Contextual Fields

All logs include contextual fields:
- `workflow_id`: UUID of the workflow
- `workflow_name`: Name of the workflow
- `step_id`: ID of the step being executed
- `step_type`: Type of step (llm, embed, etc.)
- `provider`: LLM provider name
- `model`: Model identifier

### Example Log Output

```
2025-11-14T10:30:45.123Z INFO workflow_id=abc123 workflow_name="rag-pipeline" message="Starting workflow execution"
2025-11-14T10:30:45.456Z INFO step_id="step1" step_type=Llm message="Executing step"
2025-11-14T10:30:45.789Z DEBUG step_id="step1" provider="anthropic" model="claude-3-5-sonnet" message="Calling LLM provider"
2025-11-14T10:30:47.234Z INFO step_id="step1" duration_ms=1778 message="Step completed successfully"
2025-11-14T10:30:52.567Z INFO workflow_id=abc123 message="Workflow completed successfully"
```

## Health Checks

The health module provides health check functionality for monitoring system status.

### Health Check Types

#### Liveness Check

Verifies the application is running (lightweight, always succeeds if process is alive).

```rust
use llm_orchestrator_core::health::HealthChecker;

let checker = HealthChecker::new();
let result = checker.liveness();

// Returns: HealthCheckResult { status: Healthy, ... }
```

#### Readiness Check

Verifies the application is ready to serve traffic (checks all dependencies).

```rust
let result = checker.readiness().await;

// Returns health status based on all registered checks
```

### Registering Health Checks

```rust
use llm_orchestrator_core::health::{HealthChecker, MemoryHealthCheck, HttpHealthCheck};
use std::sync::Arc;

let mut checker = HealthChecker::new();

// Register memory check
checker.register(Arc::new(MemoryHealthCheck::new(1024))); // 1GB limit

// Register HTTP health check for external service
checker.register(Arc::new(HttpHealthCheck::new(
    "database",
    "http://postgres:5432/health",
    5000  // 5s timeout
)));

// Perform checks
let health = checker.check_all().await;
```

### Health Check Response Format

```json
{
  "status": "healthy",
  "timestamp": "2025-11-14T10:30:45.123Z",
  "checks": {
    "memory": {
      "status": "healthy",
      "response_time_ms": 1,
      "last_check": "2025-11-14T10:30:45.123Z"
    },
    "database": {
      "status": "healthy",
      "response_time_ms": 42,
      "last_check": "2025-11-14T10:30:45.123Z"
    }
  }
}
```

### Kubernetes Integration

Health checks are designed for Kubernetes readiness and liveness probes:

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: llm-orchestrator
spec:
  containers:
  - name: orchestrator
    image: llm-orchestrator:latest
    livenessProbe:
      httpGet:
        path: /health/live
        port: 8080
      initialDelaySeconds: 5
      periodSeconds: 10
    readinessProbe:
      httpGet:
        path: /health/ready
        port: 8080
      initialDelaySeconds: 10
      periodSeconds: 5
```

## Grafana Dashboard Configuration

### Recommended Panels

#### Workflow Overview Dashboard

1. **Workflow Execution Rate**
   - Query: `rate(orchestrator_workflow_executions_total[5m])`
   - Visualization: Time series graph

2. **Workflow Success Rate**
   - Query: `rate(orchestrator_workflow_executions_total{status="success"}[5m]) / rate(orchestrator_workflow_executions_total[5m])`
   - Visualization: Gauge (0-100%)

3. **Active Workflows**
   - Query: `orchestrator_active_workflows`
   - Visualization: Stat panel

4. **P95 Workflow Duration**
   - Query: `histogram_quantile(0.95, rate(orchestrator_workflow_duration_seconds_bucket[5m]))`
   - Visualization: Time series graph

#### LLM Provider Dashboard

1. **Request Rate by Provider**
   - Query: `sum by (provider) (rate(orchestrator_llm_requests_total[5m]))`
   - Visualization: Time series graph

2. **Token Usage**
   - Query: `sum by (provider, type) (rate(orchestrator_llm_tokens_total[5m]))`
   - Visualization: Stacked area chart

3. **Request Latency**
   - Query: `histogram_quantile(0.95, rate(orchestrator_llm_request_duration_seconds_bucket[5m]))`
   - Visualization: Time series graph

4. **Error Rate**
   - Query: `rate(orchestrator_llm_requests_total{status="failure"}[5m])`
   - Visualization: Time series graph

## Alerting Rules

### Recommended Prometheus Alerts

```yaml
groups:
- name: llm_orchestrator_alerts
  interval: 30s
  rules:
  - alert: HighErrorRate
    expr: |
      rate(orchestrator_errors_total[5m]) > 0.05
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: High error rate detected
      description: Error rate is {{ $value }} errors/sec

  - alert: WorkflowTimeout
    expr: |
      rate(orchestrator_errors_total{error_type="timeout"}[5m]) > 0
    for: 2m
    labels:
      severity: critical
    annotations:
      summary: Workflow timeouts detected
      description: {{ $value }} timeouts/sec

  - alert: LLMProviderDown
    expr: |
      rate(orchestrator_llm_requests_total{status="failure"}[5m]) /
      rate(orchestrator_llm_requests_total[5m]) > 0.5
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: LLM provider failure rate > 50%
      description: Provider {{ $labels.provider }} failing

  - alert: HighWorkflowLatency
    expr: |
      histogram_quantile(0.99,
        rate(orchestrator_workflow_duration_seconds_bucket[5m])
      ) > 60
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: High workflow latency
      description: P99 latency is {{ $value }}s
```

## Performance Impact

The observability instrumentation is designed for minimal overhead:

- **Metrics collection**: < 0.5% overhead
- **Tracing spans**: < 0.3% overhead
- **Structured logging**: < 0.2% overhead
- **Total estimated overhead**: < 1% in production

### Optimization Tips

1. **Log Sampling**: For high-throughput workloads, consider log sampling for DEBUG/TRACE levels
2. **Metric Cardinality**: Avoid high-cardinality labels (e.g., don't use UUIDs as label values)
3. **Trace Sampling**: Configure trace sampling rate based on traffic volume (e.g., 10% for high volume)

## Future Enhancements

Planned observability features:

- [ ] Cost tracking metrics (dollars spent per workflow, provider, model)
- [ ] Custom metric exporters (StatsD, CloudWatch)
- [ ] Distributed trace context propagation to LLM providers
- [ ] Log aggregation integration (Loki, Elasticsearch)
- [ ] Real-time alerting webhooks
- [ ] Circuit breaker state metrics
- [ ] Queue depth and backpressure metrics
- [ ] Custom user-defined metrics API

## References

- [Prometheus Documentation](https://prometheus.io/docs/)
- [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust)
- [Tracing Crate](https://docs.rs/tracing/)
- [Grafana Dashboards](https://grafana.com/docs/grafana/latest/dashboards/)
