# LLM-Orchestrator Production Readiness Plan

**Version:** 1.0.0
**Date:** 2025-11-14
**Framework:** SPARC (Specification, Pseudocode, Architecture, Refinement, Completion)
**Objective:** Achieve enterprise-grade, production-ready status
**Timeline:** 12-16 weeks (3-4 development phases)

---

## Executive Summary

This document outlines a comprehensive plan to transform the LLM-Orchestrator from a high-quality MVP (85% complete) to a production-ready, enterprise-grade platform. The plan addresses six critical gaps identified in the implementation assessment:

1. **Authentication & Security** - Implement comprehensive security controls
2. **State Persistence** - Add database-backed state management
3. **Production Monitoring** - Instrument with metrics and observability
4. **CI/CD Automation** - Establish automated build/test/deploy pipeline
5. **RAG Pipeline Completion** - Implement embedding and vector search steps
6. **Critical Bug Fixes** - Resolve high-severity issues

**Current Status:** B+ foundation (85% core complete, 53% total features)
**Target Status:** A+ production system (99% complete, enterprise-ready)
**Estimated Effort:** 840-1,060 hours over 12-16 weeks

---

## S - SPECIFICATION

### 1. Authentication & Security Controls

#### 1.1 Requirements

**SEC-1: API Authentication**
- Support multiple authentication methods (JWT, API Keys, OAuth2)
- Stateless authentication for horizontal scaling
- Token expiration and refresh mechanism
- Secure token storage and transmission (HTTPS only)

**SEC-2: Authorization (RBAC)**
- Role-Based Access Control for workflow operations
- Permissions: `workflow:read`, `workflow:write`, `workflow:execute`, `workflow:delete`
- Roles: `viewer`, `executor`, `developer`, `admin`
- Policy engine for fine-grained access control

**SEC-3: Secret Management**
- Integration with HashiCorp Vault
- Integration with AWS Secrets Manager
- Support for Kubernetes Secrets
- Secret rotation without downtime
- Audit trail for secret access

**SEC-4: Input Validation & Sanitization**
- JSON schema validation for workflow definitions
- Prompt injection prevention
- Maximum length limits for inputs
- Forbidden pattern detection (SQL injection, XSS)
- Rate limiting per user/API key

**SEC-5: Audit Logging**
- Log all authentication attempts (success/failure)
- Log all workflow executions with user context
- Log all configuration changes
- Tamper-proof audit log storage
- Retention policy enforcement (configurable, default 90 days)

**SEC-6: Encryption**
- TLS 1.3 for all network communication
- Encryption at rest for sensitive data (API keys, workflow inputs)
- Secure key derivation (Argon2 for passwords)
- Certificate management and rotation

#### 1.2 Success Criteria

- [ ] Pass OWASP Top 10 security audit
- [ ] Support 1000+ concurrent authenticated users
- [ ] Zero secrets in logs or error messages
- [ ] 100% audit coverage for security events
- [ ] < 10ms authentication overhead per request

---

### 2. State Persistence

#### 2.1 Requirements

**STATE-1: Database Schema**
- Workflow execution state table
- Step execution state table
- Checkpoint table for resumability
- Audit log table
- Configuration table

**STATE-2: State Store Interface**
```rust
#[async_trait]
pub trait StateStore: Send + Sync {
    async fn save_workflow_state(&self, state: &WorkflowState) -> Result<()>;
    async fn load_workflow_state(&self, workflow_id: &str) -> Result<WorkflowState>;
    async fn create_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()>;
    async fn restore_from_checkpoint(&self, checkpoint_id: &str) -> Result<WorkflowState>;
    async fn delete_old_states(&self, older_than: DateTime<Utc>) -> Result<u64>;
}
```

**STATE-3: Database Support**
- PostgreSQL (primary, multi-node production)
- SQLite (single-node, development)
- Migration system (diesel or sqlx migrations)
- Connection pooling (min: 5, max: 20 connections)
- Transaction support for atomic updates

**STATE-4: Checkpoint Strategy**
- Auto-checkpoint after each step completion
- Manual checkpoint API for long-running workflows
- Checkpoint retention: last 10 checkpoints or 24 hours
- Background cleanup of old checkpoints

**STATE-5: Recovery Mechanisms**
- Automatic recovery on restart
- Resume from last checkpoint
- Partial replay from specific step
- State migration for schema changes

#### 2.2 Success Criteria

- [ ] Zero data loss on orchestrator crash
- [ ] Resume workflows within 5 seconds of restart
- [ ] Support 10,000+ concurrent workflow states
- [ ] < 50ms state save latency (P99)
- [ ] Successful migration from v1 to v2 schema

---

### 3. Production Monitoring

#### 3.1 Requirements

**MON-1: Prometheus Metrics**

**Workflow Metrics:**
- `orchestrator_workflow_executions_total{status, workflow_name}` (counter)
- `orchestrator_workflow_duration_seconds{workflow_name}` (histogram)
- `orchestrator_workflow_steps_total{workflow_name, step_id}` (counter)
- `orchestrator_workflow_active{workflow_name}` (gauge)

**Provider Metrics:**
- `orchestrator_llm_requests_total{provider, model, status}` (counter)
- `orchestrator_llm_request_duration_seconds{provider, model}` (histogram)
- `orchestrator_llm_tokens_total{provider, model, type}` (counter) - type: input/output
- `orchestrator_llm_cost_usd{provider, model}` (counter)

**System Metrics:**
- `orchestrator_active_workflows` (gauge)
- `orchestrator_queue_depth` (gauge)
- `orchestrator_memory_bytes` (gauge)
- `orchestrator_cpu_usage_percent` (gauge)
- `orchestrator_db_connections_active` (gauge)

**Error Metrics:**
- `orchestrator_errors_total{error_type, component}` (counter)
- `orchestrator_retries_total{reason}` (counter)
- `orchestrator_circuit_breaker_state{provider}` (gauge) - 0=closed, 1=open, 2=half-open

**MON-2: OpenTelemetry Tracing**
- Distributed tracing for multi-step workflows
- Span for each workflow execution
- Span for each step execution
- Span for each LLM API call
- Trace context propagation across async boundaries
- Integration with Jaeger/Tempo

**MON-3: Structured Logging**
- JSON format for production
- Log levels: TRACE, DEBUG, INFO, WARN, ERROR
- Correlation ID for request tracking
- Contextual fields: `workflow_id`, `step_id`, `user_id`, `trace_id`
- No PII in logs
- Log sampling for high-volume events

**MON-4: Health Checks**
- `/health` - Overall health (HTTP 200/503)
- `/health/ready` - Readiness probe (K8s)
- `/health/live` - Liveness probe (K8s)
- `/metrics` - Prometheus metrics endpoint

Health check components:
- Database connectivity
- LLM provider reachability
- Memory usage < 90% limit
- Disk space > 10% free

**MON-5: Alerting Rules**
- Error rate > 5% for 5 minutes
- Workflow duration > P99 + 50%
- Circuit breaker open for > 1 minute
- Database connection pool exhausted
- Memory usage > 85%
- Disk usage > 90%

#### 3.2 Success Criteria

- [ ] < 1% metrics overhead on throughput
- [ ] 100% workflow execution tracing
- [ ] < 100ms P99 latency for metrics export
- [ ] Alert response time < 2 minutes
- [ ] Grafana dashboards for all key metrics

---

### 4. CI/CD Automation

#### 4.1 Requirements

**CICD-1: GitHub Actions Workflows**

**Build & Test Pipeline (`ci.yml`):**
```yaml
on: [push, pull_request]
jobs:
  test:
    - cargo build --all
    - cargo test --workspace
    - cargo clippy -- -D warnings
    - cargo fmt -- --check
  security:
    - cargo audit
    - cargo deny check
  coverage:
    - cargo tarpaulin --workspace --out Xml
    - Upload to codecov.io
```

**Release Pipeline (`release.yml`):**
```yaml
on:
  push:
    tags: ['v*']
jobs:
  build-binaries:
    - Build for linux-x86_64
    - Build for macos-aarch64
    - Build for windows-x86_64
  publish:
    - Publish to crates.io
    - Create GitHub release
    - Upload binaries as assets
```

**Docker Build Pipeline (`docker.yml`):**
```yaml
on:
  push:
    branches: [main]
    tags: ['v*']
jobs:
  build-push:
    - Build multi-arch Docker image
    - Tag with git SHA and version
    - Push to GitHub Container Registry
```

**CICD-2: Quality Gates**
- All tests must pass (no flaky tests)
- Code coverage ≥ 80%
- Zero clippy warnings
- Zero security vulnerabilities (cargo audit)
- No outdated dependencies with known CVEs

**CICD-3: Deployment Artifacts**

**Dockerfile:**
```dockerfile
FROM rust:1.75 as builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /build/target/release/llm-orchestrator /usr/local/bin/
ENTRYPOINT ["llm-orchestrator"]
```

**Helm Chart Structure:**
```
charts/llm-orchestrator/
├── Chart.yaml
├── values.yaml
├── templates/
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── configmap.yaml
│   ├── secret.yaml
│   ├── ingress.yaml
│   ├── serviceaccount.yaml
│   └── hpa.yaml (Horizontal Pod Autoscaler)
```

**CICD-4: Environment Strategy**
- Development: Auto-deploy on merge to `main`
- Staging: Auto-deploy on tag `v*-rc*`
- Production: Manual approval + deploy on tag `v*` (no suffix)

#### 4.2 Success Criteria

- [ ] < 10 minute build + test time
- [ ] 100% test pass rate (no flaky tests)
- [ ] Zero manual deployment steps
- [ ] Rollback capability within 2 minutes
- [ ] Blue-green deployment support

---

### 5. RAG Pipeline Completion

#### 5.1 Requirements

**RAG-1: Embedding Step Type**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedStepConfig {
    pub provider: String,      // "openai", "cohere", "local"
    pub model: String,          // "text-embedding-3-small", "embed-english-v3.0"
    pub input: String,          // Template for text to embed
    pub dimensions: Option<usize>, // Optional dimension reduction
    pub batch_size: Option<usize>, // Batch multiple embeddings
}
```

**Embedding Providers:**
- OpenAI (text-embedding-3-small, text-embedding-3-large)
- Cohere (embed-english-v3.0, embed-multilingual-v3.0)
- Local (sentence-transformers via HTTP API)

**RAG-2: Vector Search Step Type**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchStepConfig {
    pub database: String,       // "pinecone", "weaviate", "qdrant", "chroma"
    pub index: String,          // Index/collection name
    pub query: String,          // Template for query vector or text
    pub top_k: usize,           // Number of results (default: 5)
    pub filter: Option<Value>,  // Metadata filters
    pub namespace: Option<String>, // Namespace/partition
    pub include_metadata: bool, // Include metadata in results (default: true)
    pub include_vectors: bool,  // Include vectors in results (default: false)
}
```

**Vector Database Support:**
- Pinecone (cloud-hosted, serverless)
- Weaviate (self-hosted or cloud)
- Qdrant (self-hosted or cloud)
- Chroma (embedded or client-server)

**RAG-3: Document Processing Utilities**

```rust
pub struct DocumentChunker {
    pub chunk_size: usize,      // Characters per chunk
    pub chunk_overlap: usize,   // Overlap between chunks
    pub strategy: ChunkStrategy, // Sentence, Paragraph, Fixed
}

pub enum ChunkStrategy {
    FixedSize,                  // Fixed character count
    Sentence,                   // Split on sentence boundaries
    Paragraph,                  // Split on paragraph boundaries
    Semantic,                   // Semantic similarity-based chunking
}
```

**RAG-4: RAG Pipeline Patterns**

**Pattern 1: Simple RAG**
```yaml
steps:
  - id: embed_query
    type: embed
    provider: openai
    model: text-embedding-3-small
    input: "{{inputs.query}}"
    output: [query_embedding]

  - id: search_docs
    type: vector_search
    depends_on: [embed_query]
    database: pinecone
    index: knowledge_base
    query: "{{outputs.query_embedding}}"
    top_k: 5
    output: [search_results]

  - id: generate_answer
    type: llm
    depends_on: [search_docs]
    provider: anthropic
    model: claude-3-5-sonnet-20241022
    prompt: |
      Context: {{outputs.search_results}}
      Question: {{inputs.query}}
      Answer:
    output: [answer]
```

**Pattern 2: Hybrid Search (Vector + Keyword)**
- Parallel vector search and keyword search
- Result merging with Reciprocal Rank Fusion (RRF)
- Re-ranking with cross-encoder

**Pattern 3: Multi-Index RAG**
- Search across multiple vector indexes in parallel
- Aggregate and deduplicate results
- Score normalization and fusion

#### 5.2 Success Criteria

- [ ] Support 3+ vector databases (Pinecone, Weaviate, Qdrant)
- [ ] Support 2+ embedding providers (OpenAI, Cohere)
- [ ] < 500ms P99 latency for vector search (top 10)
- [ ] Batch embedding support (up to 100 texts)
- [ ] Complete RAG pipeline examples (3+)

---

### 6. Critical Bug Fixes

#### 6.1 High-Severity Bugs

**BUG-1: Template Variable Access for Nested Outputs**

**Issue:** Templates using `{{steps.step1.result}}` fail because context stores outputs as flat keys.

**Current (Broken):**
```rust
// Context stores: { "outputs": { "step1": { "greeting": "Hello" } } }
// Template expects: {{steps.step1.greeting}}
// Handlebars receives: { "outputs": { "step1": { ... } } }
// Result: Template renders empty or fails
```

**Fix Strategy:**
```rust
// Option 1: Flatten outputs with dot notation
context_data.insert("steps.step1.greeting", value);

// Option 2: Restructure context for Handlebars
let context = json!({
    "inputs": inputs,
    "steps": {
        "step1": { "greeting": "Hello" },
        "step2": { "analysis": "Positive" }
    }
});
```

**Implementation:**
- Update `ExecutionContext::render_template()` in `context.rs`
- Add `steps` helper to context data
- Update documentation for template syntax
- Add integration tests for nested variable access

---

**BUG-2: Provider Initialization Panics**

**Issue:** `.expect()` in HTTP client creation causes panics instead of returning errors.

**Current (Broken):**
```rust
// openai.rs:135, anthropic.rs:143
let client = Client::builder()
    .timeout(Duration::from_secs(120))
    .build()
    .expect("Failed to create HTTP client");  // ❌ PANICS!
```

**Fix:**
```rust
pub fn with_base_url(api_key: String, base_url: String) -> Result<Self, ProviderError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| ProviderError::InitializationError(e.to_string()))?;

    Ok(Self {
        api_key,
        base_url,
        client,
    })
}

// Update all constructor callsites to handle Result
```

**Implementation:**
- Add `ProviderError::InitializationError` variant
- Update `with_base_url()` return type to `Result<Self, ProviderError>`
- Update `from_env()` to propagate errors
- Update tests to handle new error type

---

**BUG-3: Polling-Based Dependency Waiting**

**Issue:** Busy-loop polling wastes CPU (100 polls/sec per waiting step).

**Current (Inefficient):**
```rust
loop {
    let completed_guard = completed.read().await;
    let all_deps_complete = step.depends_on.iter()
        .all(|dep| completed_guard.contains(dep));
    drop(completed_guard);

    if all_deps_complete { break; }

    tokio::time::sleep(Duration::from_millis(10)).await;  // ❌ Polling!
}
```

**Fix (Event-Driven):**
```rust
use tokio::sync::Notify;

struct ExecutorState {
    completed: Arc<RwLock<HashSet<String>>>,
    notify: Arc<Notify>,  // Notify when any step completes
}

// When step completes:
completed.write().await.insert(step_id);
self.notify.notify_waiters();  // Wake all waiting tasks

// When waiting for dependencies:
loop {
    let completed_guard = completed.read().await;
    let all_deps_complete = step.depends_on.iter()
        .all(|dep| completed_guard.contains(dep));

    if all_deps_complete { break; }

    drop(completed_guard);
    self.notify.notified().await;  // Sleep until notified
}
```

**Implementation:**
- Add `tokio::sync::Notify` to `WorkflowExecutor`
- Update step completion logic to notify waiters
- Update dependency waiting logic to use notifications
- Benchmark CPU usage improvement (expect 80-90% reduction)

---

**BUG-4: No Workflow-Level Timeout**

**Issue:** Workflows can hang indefinitely if a step stalls.

**Fix:**
```rust
pub async fn execute(&self) -> Result<HashMap<String, StepResult>> {
    let timeout_duration = Duration::from_secs(
        self.workflow.timeout_seconds.unwrap_or(3600) // Default: 1 hour
    );

    match tokio::time::timeout(timeout_duration, self.execute_inner()).await {
        Ok(result) => result,
        Err(_) => Err(OrchestratorError::Timeout {
            duration: timeout_duration,
        }),
    }
}
```

**Implementation:**
- Add `timeout_seconds` field to `Workflow` struct
- Add `execute_inner()` method with actual execution logic
- Wrap in `tokio::time::timeout()`
- Add integration test for timeout behavior

---

**BUG-5: Concurrency Limiter Non-Optimal**

**Issue:** Waits for arbitrary first task instead of fastest completion.

**Fix:**
```rust
use futures::future::select_all;

if self.max_concurrency > 0 && tasks.len() >= self.max_concurrency {
    // Wait for first task to complete (not specific task)
    let (result, _index, remaining) = select_all(tasks).await;
    tasks = remaining;

    // Handle result
    match result {
        Ok(output) => { /* ... */ },
        Err(e) => { /* ... */ }
    }
}
```

**Implementation:**
- Replace `tasks.remove(0)` with `select_all()`
- Handle result properly
- Update remaining tasks vector
- Add test for concurrent execution fairness

---

**BUG-6: Multi-Output Steps Not Supported**

**Issue:** Only first output variable is populated, silently ignoring others.

**Fix:**
```rust
// Validate output configuration
if step.output.is_empty() {
    return Err(OrchestratorError::InvalidStepConfig {
        step_id: step.id.clone(),
        reason: "Step must specify at least one output variable".to_string(),
    });
}

// Store main response in first output
outputs.insert(step.output[0].clone(), Value::String(response.text));

// Store metadata in additional outputs
if step.output.len() > 1 {
    outputs.insert(step.output[1].clone(), json!({
        "model": response.model,
        "tokens": response.usage.total_tokens,
        "finish_reason": response.finish_reason,
    }));
}

// Support custom output mappings in config
if let Some(output_map) = &step.output_mapping {
    for (key, path) in output_map {
        let value = extract_json_path(&response, path)?;
        outputs.insert(key.clone(), value);
    }
}
```

**Implementation:**
- Add output validation in `execute_step()`
- Support metadata outputs
- Add `output_mapping` config option
- Add integration tests

---

#### 6.2 Success Criteria

- [ ] Zero panics in production (no `.expect()` in non-test code)
- [ ] < 1% CPU overhead for dependency waiting
- [ ] 100% workflow timeout coverage
- [ ] Fair task scheduling (max 10% variance in completion order)
- [ ] Multi-output support in all step types

---

## P - PSEUDOCODE

### 1. Authentication Middleware

```rust
// Authentication middleware pseudocode

pub struct AuthMiddleware {
    jwt_secret: Vec<u8>,
    api_key_store: Arc<ApiKeyStore>,
    cache: Arc<AuthCache>,
}

impl AuthMiddleware {
    pub async fn authenticate(&self, req: &Request) -> Result<AuthContext> {
        // 1. Extract credentials from request
        let credentials = self.extract_credentials(req)?;

        // 2. Check cache first (avoid DB lookup)
        if let Some(ctx) = self.cache.get(&credentials.token).await {
            if !ctx.is_expired() {
                return Ok(ctx);
            }
        }

        // 3. Validate credentials
        let auth_context = match credentials.auth_type {
            AuthType::Bearer(token) => {
                // Verify JWT
                let claims = self.verify_jwt(&token)?;
                AuthContext {
                    user_id: claims.sub,
                    roles: claims.roles,
                    permissions: self.load_permissions(&claims.roles).await?,
                    expires_at: claims.exp,
                }
            }
            AuthType::ApiKey(key) => {
                // Lookup API key
                let key_info = self.api_key_store.lookup(&key).await?;
                AuthContext {
                    user_id: key_info.user_id,
                    roles: key_info.roles,
                    permissions: key_info.permissions,
                    expires_at: key_info.expires_at,
                }
            }
            AuthType::None => {
                return Err(AuthError::MissingCredentials);
            }
        };

        // 4. Cache the result
        self.cache.set(&credentials.token, &auth_context).await;

        Ok(auth_context)
    }

    pub fn authorize(&self, ctx: &AuthContext, permission: &str) -> Result<()> {
        if ctx.permissions.contains(permission) {
            Ok(())
        } else {
            Err(AuthError::InsufficientPermissions {
                required: permission.to_string(),
                available: ctx.permissions.clone(),
            })
        }
    }
}

// RBAC authorization
pub async fn execute_workflow(
    auth: &AuthContext,
    workflow_id: &str,
    inputs: Value,
) -> Result<WorkflowOutput> {
    // Check permission
    auth.require_permission("workflow:execute")?;

    // Check resource-level access
    if !auth.can_access_workflow(workflow_id).await? {
        return Err(AuthError::ResourceAccessDenied);
    }

    // Audit log
    audit_log(AuditEvent {
        user_id: auth.user_id.clone(),
        action: "workflow:execute",
        resource_id: workflow_id.to_string(),
        timestamp: Utc::now(),
        result: "success",
    }).await;

    // Execute with user context
    orchestrator.execute_as(auth, workflow_id, inputs).await
}
```

---

### 2. State Persistence

```rust
// State store implementation pseudocode

pub struct PostgresStateStore {
    pool: Pool<Postgres>,
}

impl StateStore for PostgresStateStore {
    async fn save_workflow_state(&self, state: &WorkflowState) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Upsert workflow state
        sqlx::query!(
            r#"
            INSERT INTO workflow_states (
                id, workflow_id, status, started_at, updated_at, context
            ) VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO UPDATE SET
                status = EXCLUDED.status,
                updated_at = EXCLUDED.updated_at,
                context = EXCLUDED.context
            "#,
            state.id,
            state.workflow_id,
            state.status.to_string(),
            state.started_at,
            Utc::now(),
            serde_json::to_value(&state.context)?
        )
        .execute(&mut *tx)
        .await?;

        // Save step states
        for (step_id, step_state) in &state.steps {
            sqlx::query!(
                r#"
                INSERT INTO step_states (
                    workflow_state_id, step_id, status, outputs, error
                ) VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (workflow_state_id, step_id) DO UPDATE SET
                    status = EXCLUDED.status,
                    outputs = EXCLUDED.outputs,
                    error = EXCLUDED.error
                "#,
                state.id,
                step_id,
                step_state.status.to_string(),
                serde_json::to_value(&step_state.outputs)?,
                step_state.error.as_ref().map(|e| e.to_string())
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn create_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO checkpoints (
                id, workflow_state_id, step_id, timestamp, snapshot
            ) VALUES ($1, $2, $3, $4, $5)
            "#,
            checkpoint.id,
            checkpoint.workflow_state_id,
            checkpoint.step_id,
            checkpoint.timestamp,
            serde_json::to_value(&checkpoint.snapshot)?
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn restore_from_checkpoint(&self, checkpoint_id: &str) -> Result<WorkflowState> {
        let checkpoint = sqlx::query_as!(
            CheckpointRow,
            r#"
            SELECT id, workflow_state_id, step_id, timestamp, snapshot
            FROM checkpoints
            WHERE id = $1
            "#,
            checkpoint_id
        )
        .fetch_one(&self.pool)
        .await?;

        let state: WorkflowState = serde_json::from_value(checkpoint.snapshot)?;
        Ok(state)
    }
}

// Checkpointing during execution
pub async fn execute_step_with_checkpoint(&self, step: &Step) -> Result<StepOutput> {
    // Execute step
    let output = self.execute_step(step).await?;

    // Create checkpoint
    let checkpoint = Checkpoint {
        id: Uuid::new_v4().to_string(),
        workflow_state_id: self.state.id.clone(),
        step_id: step.id.clone(),
        timestamp: Utc::now(),
        snapshot: serde_json::to_value(&self.state)?,
    };

    self.state_store.create_checkpoint(&checkpoint).await?;

    Ok(output)
}
```

---

### 3. Prometheus Metrics

```rust
// Metrics instrumentation pseudocode

use prometheus::{
    Counter, Histogram, Gauge, IntGauge,
    register_counter, register_histogram, register_gauge,
};

lazy_static! {
    // Workflow metrics
    static ref WORKFLOW_EXECUTIONS: Counter = register_counter!(
        "orchestrator_workflow_executions_total",
        "Total workflow executions",
        &["status", "workflow_name"]
    ).unwrap();

    static ref WORKFLOW_DURATION: Histogram = register_histogram!(
        "orchestrator_workflow_duration_seconds",
        "Workflow execution duration",
        &["workflow_name"],
        vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0]
    ).unwrap();

    // LLM metrics
    static ref LLM_REQUESTS: Counter = register_counter!(
        "orchestrator_llm_requests_total",
        "Total LLM requests",
        &["provider", "model", "status"]
    ).unwrap();

    static ref LLM_TOKENS: Counter = register_counter!(
        "orchestrator_llm_tokens_total",
        "Total tokens used",
        &["provider", "model", "type"]  // type: input/output
    ).unwrap();

    static ref ACTIVE_WORKFLOWS: IntGauge = register_int_gauge!(
        "orchestrator_active_workflows",
        "Number of active workflows"
    ).unwrap();
}

pub async fn execute_with_metrics(&self, workflow: &Workflow) -> Result<WorkflowOutput> {
    let timer = WORKFLOW_DURATION
        .with_label_values(&[&workflow.name])
        .start_timer();

    ACTIVE_WORKFLOWS.inc();

    let result = self.execute_inner(workflow).await;

    ACTIVE_WORKFLOWS.dec();
    timer.observe_duration();

    match &result {
        Ok(_) => {
            WORKFLOW_EXECUTIONS
                .with_label_values(&["success", &workflow.name])
                .inc();
        }
        Err(e) => {
            WORKFLOW_EXECUTIONS
                .with_label_values(&["failure", &workflow.name])
                .inc();

            ERROR_COUNTER
                .with_label_values(&[e.error_type(), "executor"])
                .inc();
        }
    }

    result
}

// Metrics endpoint
pub async fn metrics_handler() -> Response {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();

    Response::builder()
        .status(200)
        .header("Content-Type", encoder.format_type())
        .body(buffer.into())
        .unwrap()
}
```

---

### 4. OpenTelemetry Tracing

```rust
// Distributed tracing pseudocode

use opentelemetry::{trace::{Tracer, Span}, Context};
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub async fn execute_with_tracing(&self, workflow: &Workflow) -> Result<WorkflowOutput> {
    let tracer = global::tracer("llm-orchestrator");
    let mut span = tracer
        .span_builder(format!("workflow.execute:{}", workflow.name))
        .with_kind(SpanKind::Internal)
        .start(&tracer);

    span.set_attribute(KeyValue::new("workflow.id", workflow.id.clone()));
    span.set_attribute(KeyValue::new("workflow.name", workflow.name.clone()));
    span.set_attribute(KeyValue::new("step.count", workflow.steps.len() as i64));

    let cx = Context::current_with_span(span);
    let _guard = cx.attach();

    let result = self.execute_inner(workflow).await;

    match &result {
        Ok(output) => {
            cx.span().set_status(StatusCode::Ok, "".to_string());
            cx.span().set_attribute(KeyValue::new("steps.completed", output.steps.len() as i64));
        }
        Err(e) => {
            cx.span().set_status(StatusCode::Error, e.to_string());
            cx.span().record_error(e);
        }
    }

    result
}

pub async fn execute_step_with_tracing(&self, step: &Step) -> Result<StepOutput> {
    let tracer = global::tracer("llm-orchestrator");
    let mut span = tracer
        .span_builder(format!("step.execute:{}", step.id))
        .with_kind(SpanKind::Internal)
        .start(&tracer);

    span.set_attribute(KeyValue::new("step.id", step.id.clone()));
    span.set_attribute(KeyValue::new("step.type", format!("{:?}", step.step_type)));

    if let StepConfig::Llm(config) = &step.config {
        span.set_attribute(KeyValue::new("llm.provider", config.provider.clone()));
        span.set_attribute(KeyValue::new("llm.model", config.model.clone()));
    }

    let cx = Context::current_with_span(span);
    let result = self.execute_step_inner(step).with_context(cx).await;

    result
}
```

---

### 5. RAG Pipeline Implementation

```rust
// Embedding step execution pseudocode

pub async fn execute_embed_step(&self, step: &Step) -> Result<HashMap<String, Value>> {
    let config = match &step.config {
        StepConfig::Embed(c) => c,
        _ => return Err(OrchestratorError::InvalidStepConfig),
    };

    // Render input template
    let text = self.context.render_template(&config.input)?;

    // Get embedding provider
    let provider = self.embedding_providers
        .get(&config.provider)
        .ok_or_else(|| OrchestratorError::ProviderNotFound(config.provider.clone()))?;

    // Generate embedding
    let request = EmbedRequest {
        model: config.model.clone(),
        input: vec![text],
        dimensions: config.dimensions,
    };

    let response = provider.embed(request).await?;

    // Store outputs
    let mut outputs = HashMap::new();
    outputs.insert(
        step.output[0].clone(),
        serde_json::to_value(&response.embeddings[0])?
    );

    // Store metadata if multiple outputs
    if step.output.len() > 1 {
        outputs.insert(
            step.output[1].clone(),
            json!({
                "model": response.model,
                "dimensions": response.embeddings[0].len(),
                "usage": response.usage,
            })
        );
    }

    Ok(outputs)
}

// Vector search step execution pseudocode

pub async fn execute_vector_search_step(&self, step: &Step) -> Result<HashMap<String, Value>> {
    let config = match &step.config {
        StepConfig::VectorSearch(c) => c,
        _ => return Err(OrchestratorError::InvalidStepConfig),
    };

    // Get vector database
    let db = self.vector_dbs
        .get(&config.database)
        .ok_or_else(|| OrchestratorError::VectorDBNotFound(config.database.clone()))?;

    // Render query (either vector or text)
    let query = self.context.render_template(&config.query)?;
    let query_vector: Vec<f32> = serde_json::from_str(&query)?;

    // Build search request
    let request = SearchRequest {
        index: config.index.clone(),
        query_vector,
        top_k: config.top_k,
        filter: config.filter.clone(),
        namespace: config.namespace.clone(),
        include_metadata: config.include_metadata,
        include_vectors: config.include_vectors,
    };

    // Execute search
    let response = db.search(request).await?;

    // Format results
    let results: Vec<Value> = response.results.iter().map(|r| {
        json!({
            "id": r.id,
            "score": r.score,
            "metadata": r.metadata,
            "text": r.metadata.get("text"),
        })
    }).collect();

    let mut outputs = HashMap::new();
    outputs.insert(step.output[0].clone(), Value::Array(results));

    Ok(outputs)
}
```

---

## A - ARCHITECTURE

### 1. Security Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Security Layer                          │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  Ingress (HTTPS/TLS 1.3)                                    │
│  ├── Rate Limiter (Token Bucket)                            │
│  ├── Auth Middleware                                         │
│  │   ├── JWT Validator                                      │
│  │   ├── API Key Lookup                                     │
│  │   └── Auth Cache (Redis)                                 │
│  ├── Authorization (RBAC)                                    │
│  │   ├── Permission Check                                   │
│  │   └── Resource Access Control                            │
│  └── Input Validator                                         │
│      ├── JSON Schema Validation                             │
│      ├── Size Limits                                         │
│      └── Pattern Blacklist                                   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Orchestration Engine (Authenticated)                       │
│  ├── Audit Logger (All Operations)                          │
│  ├── Secret Resolver                                         │
│  │   ├── Vault Client                                       │
│  │   ├── AWS Secrets Manager                                │
│  │   └── K8s Secrets                                        │
│  └── Encrypted State Store                                   │
└─────────────────────────────────────────────────────────────┘
```

**Security Components:**

1. **Auth Service** (`crates/llm-orchestrator-auth/`)
   - JWT token generation and validation
   - API key management with scopes
   - RBAC policy engine
   - Session management

2. **Secret Manager** (`crates/llm-orchestrator-secrets/`)
   - Vault integration
   - AWS Secrets Manager integration
   - K8s secret mounting
   - In-memory secret cache with TTL

3. **Audit Logger** (`crates/llm-orchestrator-audit/`)
   - Append-only log storage
   - Tamper-proof hashing
   - Log rotation and archival
   - Compliance report generation

---

### 2. State Persistence Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   Workflow Executor                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    State Manager                             │
│  ├── Checkpoint Strategy                                     │
│  │   ├── After Each Step (Auto)                             │
│  │   ├── Manual Checkpoint API                              │
│  │   └── Periodic Timer (5 min)                             │
│  ├── State Serializer                                        │
│  │   ├── Workflow State → JSON                              │
│  │   └── Compression (gzip)                                 │
│  └── State Store Trait                                       │
│      ├── PostgresStateStore                                  │
│      ├── SQLiteStateStore                                    │
│      └── RedisStateStore (Cache)                             │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                Database Schema (PostgreSQL)                  │
├─────────────────────────────────────────────────────────────┤
│  workflow_states                                             │
│    - id (PK, UUID)                                           │
│    - workflow_id (FK)                                        │
│    - status (enum)                                           │
│    - started_at (timestamp)                                  │
│    - updated_at (timestamp)                                  │
│    - context (jsonb)                                         │
│    - user_id (FK)                                            │
│                                                              │
│  step_states                                                 │
│    - workflow_state_id (FK)                                  │
│    - step_id (PK)                                            │
│    - status (enum)                                           │
│    - started_at (timestamp)                                  │
│    - completed_at (timestamp)                                │
│    - outputs (jsonb)                                         │
│    - error (text, nullable)                                  │
│                                                              │
│  checkpoints                                                 │
│    - id (PK, UUID)                                           │
│    - workflow_state_id (FK)                                  │
│    - step_id (string)                                        │
│    - timestamp (timestamp)                                   │
│    - snapshot (jsonb, compressed)                            │
│                                                              │
│  Indexes:                                                    │
│    - workflow_states.workflow_id + status                    │
│    - step_states.workflow_state_id                           │
│    - checkpoints.workflow_state_id + timestamp DESC          │
└─────────────────────────────────────────────────────────────┘
```

**Recovery Flow:**

```
Orchestrator Crash
       │
       ▼
Restart Orchestrator
       │
       ▼
Load Active Workflows from DB
  (status = 'Running' or 'Paused')
       │
       ▼
For Each Workflow:
  ├── Load Latest Checkpoint
  ├── Restore Execution Context
  ├── Identify Completed Steps
  ├── Resume from Next Pending Step
  └── Continue Execution
```

---

### 3. Observability Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  Instrumented Application                    │
├─────────────────────────────────────────────────────────────┤
│  Workflow Executor                                           │
│    │                                                         │
│    ├─── Metrics (Prometheus)                                │
│    │     └── /metrics endpoint                              │
│    │                                                         │
│    ├─── Traces (OpenTelemetry)                              │
│    │     └── OTLP Exporter → Tempo/Jaeger                   │
│    │                                                         │
│    └─── Logs (tracing + tracing-subscriber)                 │
│          └── JSON formatter → stdout                         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  Observability Stack                         │
├─────────────────────────────────────────────────────────────┤
│  Prometheus                                                  │
│    ├── Scrape /metrics every 15s                            │
│    ├── Store time-series data                               │
│    └── Alert Manager (for alerts)                           │
│                                                              │
│  Grafana                                                     │
│    ├── Dashboards (Workflows, LLMs, System)                 │
│    ├── Query Prometheus                                      │
│    └── Alert Visualization                                   │
│                                                              │
│  Tempo/Jaeger (Tracing)                                      │
│    ├── Receive OTLP traces                                   │
│    ├── Store trace data                                      │
│    └── Query API for Grafana                                │
│                                                              │
│  Loki (Logs)                                                 │
│    ├── Collect JSON logs                                     │
│    ├── Index and store                                       │
│    └── Query API for Grafana                                │
└─────────────────────────────────────────────────────────────┘
```

**Key Dashboards:**

1. **Workflow Dashboard**
   - Execution rate (req/s)
   - Success/failure rate
   - P50/P95/P99 duration
   - Active workflows gauge

2. **LLM Provider Dashboard**
   - Requests by provider/model
   - Token usage and cost
   - Error rate by provider
   - Latency distribution

3. **System Dashboard**
   - CPU/Memory usage
   - DB connection pool
   - Queue depth
   - Error rate by type

---

### 4. CI/CD Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     GitHub Repository                        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   GitHub Actions (CI/CD)                     │
├─────────────────────────────────────────────────────────────┤
│  On Push/PR:                                                 │
│    ├── ci.yml                                                │
│    │   ├── cargo build --all                                │
│    │   ├── cargo test --workspace                           │
│    │   ├── cargo clippy -- -D warnings                      │
│    │   ├── cargo fmt -- --check                             │
│    │   ├── cargo audit                                       │
│    │   └── cargo tarpaulin (coverage)                       │
│    │                                                         │
│    └── Quality Gates:                                        │
│        ├── All tests pass                                    │
│        ├── Coverage ≥ 80%                                    │
│        ├── Zero clippy warnings                             │
│        └── Zero security vulnerabilities                     │
│                                                              │
│  On Tag (v*):                                                │
│    ├── release.yml                                           │
│    │   ├── Build binaries (Linux, macOS, Windows)           │
│    │   ├── Run integration tests                            │
│    │   ├── Create GitHub release                            │
│    │   ├── Upload binaries                                  │
│    │   └── Publish to crates.io                             │
│    │                                                         │
│    └── docker.yml                                            │
│        ├── Build multi-arch image                           │
│        ├── Tag: latest, v1.2.3, v1.2, v1                    │
│        └── Push to ghcr.io                                   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Deployment Targets                        │
├─────────────────────────────────────────────────────────────┤
│  Development: Auto-deploy on main                            │
│    └── Kubernetes (dev namespace)                            │
│                                                              │
│  Staging: Auto-deploy on v*-rc*                             │
│    └── Kubernetes (staging namespace)                        │
│                                                              │
│  Production: Manual approval + deploy on v*                 │
│    ├── Kubernetes (prod namespace)                           │
│    ├── Blue-Green deployment                                 │
│    └── Automated rollback on failure                         │
└─────────────────────────────────────────────────────────────┘
```

---

### 5. RAG Pipeline Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    RAG Workflow Steps                        │
└─────────────────────────────────────────────────────────────┘

Step 1: Embedding
┌──────────────┐
│ User Query   │
│ "What is..." │
└──────┬───────┘
       │
       ▼
┌──────────────────────┐
│ Embedding Provider   │
│ ├── OpenAI          │
│ ├── Cohere          │
│ └── Local Model     │
└──────┬───────────────┘
       │
       ▼
┌──────────────┐
│ Vector       │
│ [0.23, ...]  │
└──────┬───────┘

Step 2: Vector Search
       │
       ▼
┌──────────────────────────────┐
│ Vector Database              │
│ ├── Pinecone                 │
│ ├── Weaviate                 │
│ ├── Qdrant                   │
│ └── Chroma                   │
└──────┬───────────────────────┘
       │
       ▼
┌──────────────────────┐
│ Top-K Results        │
│ [                    │
│   {id, score, text}, │
│   {id, score, text}, │
│   ...                │
│ ]                    │
└──────┬───────────────┘

Step 3: Context Assembly
       │
       ▼
┌──────────────────────┐
│ Context Builder      │
│ ├── Result Ranking   │
│ ├── Deduplication    │
│ ├── Text Extraction  │
│ └── Prompt Template  │
└──────┬───────────────┘
       │
       ▼
┌──────────────────────┐
│ Assembled Context    │
│ "Context:\n"         │
│ "- Doc 1: ...\n"     │
│ "- Doc 2: ...\n"     │
└──────┬───────────────┘

Step 4: LLM Generation
       │
       ▼
┌──────────────────────┐
│ LLM Provider         │
│ (Claude, GPT-4)      │
└──────┬───────────────┘
       │
       ▼
┌──────────────────────┐
│ Final Answer         │
└──────────────────────┘
```

**Vector Database Trait:**

```rust
#[async_trait]
pub trait VectorDatabase: Send + Sync {
    async fn upsert(&self, request: UpsertRequest) -> Result<UpsertResponse>;
    async fn search(&self, request: SearchRequest) -> Result<SearchResponse>;
    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResponse>;
    async fn create_index(&self, config: IndexConfig) -> Result<()>;
    async fn index_stats(&self, index_name: &str) -> Result<IndexStats>;
    async fn health_check(&self) -> Result<()>;
}
```

---

### 6. Deployment Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Kubernetes Cluster                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌────────────────────────────────────────────────┐         │
│  │            Ingress (NGINX)                     │         │
│  │  ├── TLS Termination                           │         │
│  │  ├── Rate Limiting                             │         │
│  │  └── /api/v1/* → orchestrator-service          │         │
│  └────────────────────────────────────────────────┘         │
│                         │                                    │
│                         ▼                                    │
│  ┌────────────────────────────────────────────────┐         │
│  │     Service: llm-orchestrator                  │         │
│  │       (ClusterIP)                              │         │
│  └────────────────────────────────────────────────┘         │
│                         │                                    │
│                         ▼                                    │
│  ┌────────────────────────────────────────────────┐         │
│  │   Deployment: llm-orchestrator                 │         │
│  │   ├── Replicas: 3 (HA)                         │         │
│  │   ├── Resources:                                │         │
│  │   │   ├── Requests: 256Mi RAM, 250m CPU        │         │
│  │   │   └── Limits: 1Gi RAM, 1000m CPU           │         │
│  │   ├── Liveness Probe: /health/live             │         │
│  │   ├── Readiness Probe: /health/ready           │         │
│  │   └── Env: DB_URL, REDIS_URL, VAULT_ADDR       │         │
│  └────────────────────────────────────────────────┘         │
│         │              │              │                      │
│         ▼              ▼              ▼                      │
│    ┌────────┐    ┌────────┐    ┌────────┐                  │
│    │ Pod 1  │    │ Pod 2  │    │ Pod 3  │                  │
│    └───┬────┘    └───┬────┘    └───┬────┘                  │
│        │             │             │                        │
│        └─────────────┼─────────────┘                        │
│                      │                                      │
│                      ▼                                      │
│  ┌────────────────────────────────────────────────┐         │
│  │     StatefulSet: PostgreSQL                    │         │
│  │     ├── Replicas: 1 (or 3 with replication)    │         │
│  │     ├── PVC: 100Gi                             │         │
│  │     └── Service: postgres-service              │         │
│  └────────────────────────────────────────────────┘         │
│                                                              │
│  ┌────────────────────────────────────────────────┐         │
│  │     Deployment: Redis                          │         │
│  │     ├── Replicas: 1 (cache)                    │         │
│  │     ├── Memory Limit: 2Gi                      │         │
│  │     └── Service: redis-service                 │         │
│  └────────────────────────────────────────────────┘         │
│                                                              │
│  ┌────────────────────────────────────────────────┐         │
│  │     Observability Stack                        │         │
│  │     ├── Prometheus (metrics)                   │         │
│  │     ├── Grafana (dashboards)                   │         │
│  │     ├── Tempo (traces)                         │         │
│  │     └── Loki (logs)                            │         │
│  └────────────────────────────────────────────────┘         │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Horizontal Pod Autoscaler (HPA):**

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: llm-orchestrator-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: llm-orchestrator
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Pods
    pods:
      metric:
        name: orchestrator_queue_depth
      target:
        type: AverageValue
        averageValue: "50"
```

---

## R - REFINEMENT

### 1. Design Decisions & Trade-offs

#### 1.1 Authentication: JWT vs Session Tokens

| Aspect | JWT | Session Tokens |
|--------|-----|----------------|
| **Scalability** | Stateless, no DB lookup | Requires centralized session store |
| **Security** | Cannot revoke before expiry | Can revoke immediately |
| **Performance** | Fast (no DB hit) | Slower (Redis lookup) |
| **Token Size** | Large (1-2KB) | Small (32 bytes) |
| **Use Case** | Microservices, distributed | Monolithic, centralized |

**Decision:** **JWT with short expiry (15 min) + refresh tokens**
- Balances scalability and security
- Refresh tokens stored in DB for revocation
- Access tokens cached in Redis for validation speed

---

#### 1.2 State Store: PostgreSQL vs SQLite vs Redis

| Aspect | PostgreSQL | SQLite | Redis |
|--------|------------|--------|-------|
| **Durability** | Excellent | Excellent | Good (AOF/RDB) |
| **Scalability** | Multi-node | Single-node | Multi-node (cluster) |
| **Querying** | Full SQL | Full SQL | Limited (key-value) |
| **Performance** | Medium | Fast (local) | Very Fast (in-memory) |
| **Use Case** | Production | Dev/Small | Cache/temporary |

**Decision:** **Hybrid approach**
- PostgreSQL: Primary state store (production)
- SQLite: Development and single-node deployments
- Redis: Cache layer for hot state (active workflows)

---

#### 1.3 Metrics: Push vs Pull Model

| Aspect | Push (Prometheus Pushgateway) | Pull (Prometheus Scrape) |
|--------|-------------------------------|--------------------------|
| **Reliability** | Can lose metrics if gateway down | More reliable (retries) |
| **Simplicity** | Simple (no endpoint exposure) | More complex (needs /metrics) |
| **Firewall** | Works behind firewall | Requires reachable endpoint |
| **Standard** | Non-standard | Prometheus standard |

**Decision:** **Pull model (Prometheus scrape)**
- Industry standard for Kubernetes
- Better reliability with retries
- Easier to debug (can curl /metrics)
- Works well with service discovery

---

#### 1.4 Tracing: Jaeger vs Tempo vs X-Ray

| Aspect | Jaeger | Tempo | AWS X-Ray |
|--------|--------|-------|-----------|
| **Storage** | Cassandra/Elasticsearch | S3/GCS (cheap) | AWS-managed |
| **Cost** | Medium | Low | Medium-High |
| **Complexity** | Medium | Low | Low (managed) |
| **Vendor Lock-in** | None | None | AWS only |

**Decision:** **Tempo (primary), Jaeger (optional)**
- Tempo for production (low cost, simple)
- Jaeger for local development (feature-rich UI)
- OpenTelemetry SDK for vendor neutrality

---

### 2. Performance Optimization Strategies

#### 2.1 Database Connection Pooling

```rust
// Optimized connection pool configuration
let pool = PgPoolOptions::new()
    .min_connections(5)        // Keep 5 warm connections
    .max_connections(20)       // Max 20 concurrent
    .acquire_timeout(Duration::from_secs(5))
    .idle_timeout(Duration::from_secs(300))
    .max_lifetime(Duration::from_secs(1800))
    .connect(&database_url)
    .await?;
```

**Expected Improvement:** 30-40% reduction in query latency

---

#### 2.2 Redis Caching Strategy

```rust
pub async fn get_workflow_state(&self, workflow_id: &str) -> Result<WorkflowState> {
    // Try cache first (hot path)
    if let Some(cached) = self.redis.get(&format!("state:{}", workflow_id)).await? {
        return Ok(cached);
    }

    // Fallback to database (cold path)
    let state = self.db.load_workflow_state(workflow_id).await?;

    // Warm cache for next access
    self.redis.set(
        &format!("state:{}", workflow_id),
        &state,
        Some(Duration::from_secs(300))  // 5 min TTL
    ).await?;

    Ok(state)
}
```

**Expected Improvement:** 80-90% reduction in state read latency for active workflows

---

#### 2.3 Batch Embedding Requests

```rust
pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
    // OpenAI supports up to 2048 inputs per request
    const BATCH_SIZE: usize = 100;

    let mut all_embeddings = Vec::new();

    for chunk in texts.chunks(BATCH_SIZE) {
        let request = EmbedRequest {
            model: self.model.clone(),
            input: chunk.to_vec(),
            dimensions: self.dimensions,
        };

        let response = self.client.embed(request).await?;
        all_embeddings.extend(response.embeddings);
    }

    Ok(all_embeddings)
}
```

**Expected Improvement:** 70-80% reduction in embedding time for large document sets

---

#### 2.4 Metrics Sampling for High-Volume Events

```rust
// Sample logs at high QPS to reduce overhead
if should_sample(0.1) {  // 10% sampling rate
    info!(
        workflow_id = %workflow.id,
        duration_ms = %duration.as_millis(),
        "Workflow completed"
    );
}

fn should_sample(rate: f64) -> bool {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    rng.gen::<f64>() < rate
}
```

**Expected Improvement:** 90% reduction in logging overhead at high QPS

---

### 3. Security Hardening

#### 3.1 Input Validation Schema

```rust
pub struct InputValidator {
    max_prompt_length: usize,        // Default: 50,000 chars
    max_workflow_steps: usize,       // Default: 100 steps
    max_input_size: usize,           // Default: 1MB
    forbidden_patterns: Vec<Regex>,  // SQL injection, XSS
    required_fields: HashSet<String>,
}

impl InputValidator {
    pub fn validate_workflow(&self, workflow: &Workflow) -> Result<()> {
        // Check step count
        if workflow.steps.len() > self.max_workflow_steps {
            return Err(ValidationError::TooManySteps);
        }

        // Validate each step
        for step in &workflow.steps {
            self.validate_step(step)?;
        }

        Ok(())
    }

    pub fn validate_step(&self, step: &Step) -> Result<()> {
        match &step.config {
            StepConfig::Llm(config) => {
                // Check prompt length
                if config.prompt.len() > self.max_prompt_length {
                    return Err(ValidationError::PromptTooLong);
                }

                // Check for injection patterns
                for pattern in &self.forbidden_patterns {
                    if pattern.is_match(&config.prompt) {
                        return Err(ValidationError::ForbiddenPattern);
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}
```

---

#### 3.2 Rate Limiting Implementation

```rust
use governor::{Quota, RateLimiter};

pub struct RateLimiterMiddleware {
    // Per-user rate limiter
    user_limiters: DashMap<String, RateLimiter<String, InMemoryState, DefaultClock>>,

    // Global rate limiter
    global_limiter: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
}

impl RateLimiterMiddleware {
    pub fn new() -> Self {
        Self {
            user_limiters: DashMap::new(),
            global_limiter: RateLimiter::direct(
                Quota::per_second(nonzero!(1000u32))  // 1000 req/s global
            ),
        }
    }

    pub async fn check_rate_limit(&self, user_id: &str) -> Result<()> {
        // Check global limit
        self.global_limiter.check().map_err(|_| Error::GlobalRateLimitExceeded)?;

        // Check per-user limit
        let limiter = self.user_limiters
            .entry(user_id.to_string())
            .or_insert_with(|| {
                RateLimiter::keyed(
                    Quota::per_minute(nonzero!(100u32))  // 100 req/min per user
                )
            });

        limiter.check().map_err(|_| Error::UserRateLimitExceeded)?;

        Ok(())
    }
}
```

---

### 4. Testing Strategy

#### 4.1 Test Coverage Targets

| Component | Unit Tests | Integration Tests | E2E Tests | Target Coverage |
|-----------|------------|-------------------|-----------|-----------------|
| **Auth** | RBAC logic, JWT validation | Token refresh flow | Login → execute workflow | 90% |
| **State** | Serialization, checkpoint | DB transactions | Crash recovery | 85% |
| **Monitoring** | Metric recording | Prometheus scrape | Full observability stack | 70% |
| **RAG** | Embedding logic | Vector DB integration | Full RAG pipeline | 85% |
| **Bug Fixes** | Edge cases | Multi-step workflows | Production scenarios | 95% |

#### 4.2 Integration Test Examples

```rust
#[tokio::test]
async fn test_workflow_crash_recovery() {
    // Setup: Start workflow
    let executor = create_executor().await;
    let workflow = load_test_workflow("long-running.yaml");

    // Execute 3 steps, then simulate crash
    let handle = tokio::spawn(async move {
        executor.execute(&workflow).await
    });

    tokio::time::sleep(Duration::from_secs(5)).await;
    drop(handle);  // Simulate crash

    // Recovery: Restart executor
    let new_executor = create_executor().await;
    let recovered = new_executor.recover_workflows().await.unwrap();

    // Verify: Workflow resumed from checkpoint
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].completed_steps, 3);

    // Complete execution
    let result = new_executor.resume(&recovered[0]).await.unwrap();
    assert!(result.is_success());
}

#[tokio::test]
async fn test_rag_pipeline_end_to_end() {
    // Setup: Seed vector DB
    let vector_db = setup_test_vector_db().await;
    seed_documents(&vector_db, "test-index", test_documents()).await;

    // Execute RAG workflow
    let workflow = load_test_workflow("rag-pipeline.yaml");
    let executor = create_executor()
        .with_vector_db("test", vector_db)
        .build();

    let result = executor.execute(&workflow, json!({
        "query": "What is Rust's ownership system?"
    })).await.unwrap();

    // Verify: Answer contains relevant information
    let answer = result.get("answer").unwrap().as_str().unwrap();
    assert!(answer.contains("ownership"));
    assert!(answer.contains("borrowing"));

    // Verify: Metrics recorded
    let metrics = prometheus::gather();
    assert!(metrics.iter().any(|m| m.get_name() == "orchestrator_vector_search_duration_seconds"));
}
```

---

## C - COMPLETION

### 1. Implementation Phases

#### **Phase 1: Security & Authentication (Weeks 1-4)**

**Week 1-2: Auth System**
- [ ] Create `llm-orchestrator-auth` crate
- [ ] Implement JWT generation and validation
- [ ] Implement API key management
- [ ] Add auth middleware for REST API
- [ ] Create user/role/permission tables
- [ ] Write unit tests (target: 90% coverage)

**Week 3: Secret Management**
- [ ] Create `llm-orchestrator-secrets` crate
- [ ] Implement Vault integration
- [ ] Implement AWS Secrets Manager integration
- [ ] Add K8s secrets support
- [ ] Secret caching with TTL
- [ ] Integration tests with mock Vault

**Week 4: Audit & Input Validation**
- [ ] Create `llm-orchestrator-audit` crate
- [ ] Implement audit logger
- [ ] Add input validator with JSON schema
- [ ] Add rate limiting middleware
- [ ] Security audit and penetration testing
- [ ] Documentation update

**Deliverables:**
- Production-ready authentication system
- Secure secret management
- Comprehensive audit logging
- 90%+ test coverage for security components

**Exit Criteria:**
- [ ] Pass OWASP Top 10 security checklist
- [ ] Zero secrets in logs
- [ ] All auth tests passing
- [ ] Security audit report completed

---

#### **Phase 2: State Persistence & Recovery (Weeks 5-7)**

**Week 5: Database Schema & State Store**
- [ ] Design PostgreSQL schema (workflows, steps, checkpoints)
- [ ] Create migration scripts (sqlx migrations)
- [ ] Implement `PostgresStateStore`
- [ ] Implement `SQLiteStateStore`
- [ ] Add connection pooling
- [ ] Unit tests for state serialization

**Week 6: Checkpointing & Recovery**
- [ ] Implement auto-checkpoint after each step
- [ ] Implement manual checkpoint API
- [ ] Implement recovery on startup
- [ ] Add resume-from-checkpoint logic
- [ ] Background checkpoint cleanup job
- [ ] Integration tests for recovery

**Week 7: Production Hardening**
- [ ] Add Redis caching layer
- [ ] Optimize state queries (indexes)
- [ ] Add state retention policies
- [ ] Load testing (10,000 concurrent states)
- [ ] Crash recovery testing
- [ ] Documentation update

**Deliverables:**
- Production-grade state persistence
- Zero data loss on crash
- Sub-50ms state save latency
- Complete recovery mechanism

**Exit Criteria:**
- [ ] Resume workflows within 5 seconds of restart
- [ ] Zero data loss in crash tests
- [ ] 85%+ test coverage for state management
- [ ] Load test: 10,000 concurrent states

---

#### **Phase 3: Observability & Monitoring (Weeks 8-9)**

**Week 8: Metrics & Tracing**
- [ ] Add Prometheus metrics instrumentation
- [ ] Implement OpenTelemetry tracing
- [ ] Create Grafana dashboards (3 dashboards)
- [ ] Add health check endpoints
- [ ] Metrics unit tests
- [ ] Tracing integration tests

**Week 9: Logging & Alerting**
- [ ] Implement structured JSON logging
- [ ] Add correlation IDs to all logs
- [ ] Create Prometheus alerting rules (5+ alerts)
- [ ] Loki integration for log aggregation
- [ ] Alert response playbooks
- [ ] Documentation update

**Deliverables:**
- Complete observability stack
- Production-ready dashboards
- Automated alerting
- < 1% overhead from instrumentation

**Exit Criteria:**
- [ ] All workflows traced end-to-end
- [ ] < 100ms P99 latency for metrics export
- [ ] Grafana dashboards operational
- [ ] Alert test: fire and resolve alerts

---

#### **Phase 4: CI/CD Automation (Week 10)**

**Week 10: Build & Deploy Pipelines**
- [ ] Create GitHub Actions workflows (ci.yml, release.yml, docker.yml)
- [ ] Add quality gates (coverage, clippy, audit)
- [ ] Create Dockerfile (multi-stage build)
- [ ] Create Helm chart
- [ ] Test deployment to dev/staging/prod
- [ ] Blue-green deployment setup
- [ ] Rollback testing
- [ ] CI/CD documentation

**Deliverables:**
- Fully automated CI/CD pipeline
- Zero manual deployment steps
- Deployment documentation

**Exit Criteria:**
- [ ] < 10 minute build + test time
- [ ] 100% test pass rate
- [ ] Successful blue-green deployment
- [ ] Rollback within 2 minutes

---

#### **Phase 5: RAG Pipeline Completion (Weeks 11-12)**

**Week 11: Embedding & Vector Search**
- [ ] Implement embedding step execution
- [ ] Add OpenAI embeddings provider
- [ ] Add Cohere embeddings provider
- [ ] Implement vector search step
- [ ] Add Pinecone integration
- [ ] Add Weaviate integration
- [ ] Add Qdrant integration
- [ ] Unit tests for embedding/search

**Week 12: RAG Utilities & Examples**
- [ ] Implement document chunker
- [ ] Add hybrid search (vector + keyword)
- [ ] Create 3+ RAG example workflows
- [ ] Performance optimization (batching)
- [ ] Integration tests with real vector DBs
- [ ] RAG pipeline documentation

**Deliverables:**
- Complete RAG pipeline support
- 3+ vector database integrations
- Production-ready examples

**Exit Criteria:**
- [ ] < 500ms P99 latency for vector search
- [ ] Batch embedding support (100 texts)
- [ ] 3+ RAG examples validated
- [ ] 85%+ test coverage

---

#### **Phase 6: Critical Bug Fixes (Week 13)**

**Week 13: Bug Resolution**
- [ ] Fix BUG-1: Template variable access
- [ ] Fix BUG-2: Provider initialization panics
- [ ] Fix BUG-3: Polling-based dependency waiting
- [ ] Fix BUG-4: Workflow-level timeout
- [ ] Fix BUG-5: Concurrency limiter
- [ ] Fix BUG-6: Multi-output steps
- [ ] Regression testing for all fixes
- [ ] Performance validation

**Deliverables:**
- Zero high-severity bugs
- Performance improvements
- Comprehensive regression tests

**Exit Criteria:**
- [ ] Zero panics in production code
- [ ] < 1% CPU overhead for dependency waiting
- [ ] All integration tests passing
- [ ] Performance benchmarks improved

---

#### **Phase 7: Production Hardening & Launch (Weeks 14-16)**

**Week 14: Load Testing & Optimization**
- [ ] Load test: 1000 concurrent workflows for 1 hour
- [ ] Stress test: Maximum throughput until failure
- [ ] Memory profiling and optimization
- [ ] Query optimization (database indexes)
- [ ] Performance tuning documentation

**Week 15: Documentation & Polish**
- [ ] API reference (OpenAPI spec)
- [ ] Troubleshooting guide
- [ ] Operational runbooks
- [ ] Migration guides (v0.1 → v1.0)
- [ ] Video tutorials (optional)

**Week 16: Final Validation & Launch**
- [ ] 24-hour soak test
- [ ] Security audit (third-party)
- [ ] Compliance certification prep (SOC2)
- [ ] Final regression testing
- [ ] Production deployment
- [ ] Launch announcement

**Deliverables:**
- Production-ready v1.0 release
- Complete documentation
- Validated performance targets

**Exit Criteria:**
- [ ] 99.9% uptime in 24-hour soak test
- [ ] Security audit passed (zero critical issues)
- [ ] All performance targets met
- [ ] Production deployment successful

---

### 2. Success Metrics & Validation

#### 2.1 Performance Targets

| Metric | Target | Validation Method |
|--------|--------|-------------------|
| **Orchestration Overhead** | < 50ms P99 | Benchmark suite |
| **State Save Latency** | < 50ms P99 | Load test |
| **Auth Latency** | < 10ms P99 | Auth middleware benchmark |
| **Metrics Export** | < 100ms P99 | Prometheus scrape time |
| **Vector Search** | < 500ms P99 | RAG integration test |
| **Recovery Time** | < 5 seconds | Crash recovery test |
| **Throughput** | 1000 workflows/min | Load test (single instance) |

#### 2.2 Reliability Targets

| Metric | Target | Validation Method |
|--------|--------|-------------------|
| **Uptime** | 99.9% | 24-hour soak test |
| **Error Rate** | < 1% | Error rate monitoring |
| **Test Coverage** | ≥ 80% | cargo tarpaulin |
| **Security Audit** | Pass | Third-party audit |
| **Zero Data Loss** | 100% | Crash recovery tests |

#### 2.3 Quality Gates

**Pre-Merge:**
- [ ] All tests pass (no flaky tests)
- [ ] Code coverage ≥ 80%
- [ ] Zero clippy warnings
- [ ] Zero security vulnerabilities
- [ ] Code review approved (2+ reviewers)

**Pre-Release:**
- [ ] All performance targets met
- [ ] Security audit passed
- [ ] Load testing completed
- [ ] Documentation complete
- [ ] Migration guide tested

**Production Readiness:**
- [ ] 24-hour soak test passed
- [ ] Rollback procedure validated
- [ ] On-call runbooks created
- [ ] Monitoring dashboards operational
- [ ] Incident response plan documented

---

### 3. Risk Mitigation

#### 3.1 Technical Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **Database performance degrades at scale** | High | Medium | Load testing, query optimization, caching |
| **Vector DB integration issues** | Medium | Medium | Early integration testing, fallback to mock |
| **Auth system vulnerabilities** | High | Low | Security audit, penetration testing |
| **State recovery failures** | High | Low | Comprehensive crash testing, multiple checkpoints |
| **Metrics overhead impacts performance** | Medium | Medium | Sampling, async export, benchmarking |

#### 3.2 Schedule Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Security audit delays release** | High | Start audit early (Week 10), parallel work |
| **Integration testing takes longer than expected** | Medium | Buffer time in Phase 5, prioritize critical paths |
| **Dependency on external services (Vault, AWS)** | Medium | Mock integrations, local testing alternatives |

---

### 4. Resource Requirements

#### 4.1 Team Composition

| Role | Allocation | Responsibilities |
|------|------------|------------------|
| **Backend Engineer (Senior)** | 100% | Auth, state persistence, core features |
| **Backend Engineer (Mid)** | 100% | RAG pipeline, vector DB integrations |
| **DevOps Engineer** | 50% | CI/CD, Kubernetes, observability |
| **Security Engineer** | 25% | Security audit, consultation |
| **QA Engineer** | 50% | Test automation, load testing |

**Total: 3.25 FTE**

#### 4.2 Infrastructure Requirements

**Development:**
- GitHub Actions runners (included with GitHub)
- Development Kubernetes cluster (local minikube or cloud dev cluster)
- Development databases (Docker containers)

**Testing:**
- Load testing infrastructure (cloud VMs for load generation)
- Vector database instances (Pinecone free tier, Weaviate/Qdrant self-hosted)
- Observability stack (Prometheus/Grafana self-hosted)

**Production (Post-Launch):**
- Kubernetes cluster (3 nodes minimum for HA)
- PostgreSQL (managed service, 2 cores, 8GB RAM)
- Redis (managed service, 2GB RAM)
- Observability stack (Prometheus, Grafana, Tempo, Loki)

**Estimated Monthly Cost (AWS/GCP):** $500-800 for production infrastructure

---

### 5. Documentation Deliverables

#### 5.1 User Documentation

- [ ] **Getting Started Guide** - 15-minute quickstart
- [ ] **Workflow Syntax Reference** - Complete YAML spec
- [ ] **Provider Configuration** - All LLM and vector DB providers
- [ ] **RAG Pipeline Guide** - End-to-end examples
- [ ] **Authentication Guide** - JWT, API keys, RBAC setup
- [ ] **Deployment Guide** - Kubernetes, Docker Compose, standalone

#### 5.2 Operator Documentation

- [ ] **Installation Guide** - Production installation steps
- [ ] **Configuration Reference** - All config options
- [ ] **Monitoring Guide** - Metrics, dashboards, alerts
- [ ] **Troubleshooting Guide** - Common issues and solutions
- [ ] **Backup & Recovery** - Procedures for disaster recovery
- [ ] **Security Guide** - Best practices, hardening

#### 5.3 Developer Documentation

- [ ] **API Reference** - OpenAPI specification
- [ ] **Architecture Overview** - System design document
- [ ] **Contributing Guide** - Development setup, PR process
- [ ] **Testing Guide** - How to run and write tests
- [ ] **Migration Guides** - Upgrade procedures between versions

---

### 6. Post-Launch Roadmap

#### 6.1 v1.1 (Month 4-6)

- [ ] Additional LLM providers (Cohere, Google AI, Azure OpenAI)
- [ ] Streaming response support
- [ ] Workflow templates library
- [ ] Enhanced monitoring (distributed tracing improvements)
- [ ] Performance optimizations (based on production metrics)

#### 6.2 v1.2 (Month 7-9)

- [ ] Circuit breaker pattern implementation
- [ ] Dead letter queue for failed tasks
- [ ] Multi-tenancy support
- [ ] Enhanced RBAC (resource-level permissions)
- [ ] Cost optimization features

#### 6.3 v2.0 (Month 10-12)

- [ ] Visual workflow designer (Web UI)
- [ ] Workflow marketplace/templates
- [ ] Advanced patterns (map-reduce, conditional loops)
- [ ] ML-based workflow optimization
- [ ] Enterprise support tier

---

## Appendix A: File Structure

```
llm-orchestrator/
├── .github/
│   └── workflows/
│       ├── ci.yml                    # Build, test, lint, security
│       ├── release.yml               # Binary builds, crates.io publish
│       └── docker.yml                # Container image build/push
│
├── crates/
│   ├── llm-orchestrator-core/        # Existing (improvements)
│   ├── llm-orchestrator-providers/   # Existing (improvements)
│   ├── llm-orchestrator-cli/         # Existing (improvements)
│   │
│   ├── llm-orchestrator-auth/        # NEW - Authentication & authorization
│   │   ├── src/
│   │   │   ├── jwt.rs
│   │   │   ├── api_keys.rs
│   │   │   ├── rbac.rs
│   │   │   ├── middleware.rs
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   │
│   ├── llm-orchestrator-secrets/     # NEW - Secret management
│   │   ├── src/
│   │   │   ├── vault.rs
│   │   │   ├── aws.rs
│   │   │   ├── k8s.rs
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   │
│   ├── llm-orchestrator-state/       # NEW - State persistence
│   │   ├── src/
│   │   │   ├── postgres.rs
│   │   │   ├── sqlite.rs
│   │   │   ├── redis.rs
│   │   │   ├── checkpoint.rs
│   │   │   └── lib.rs
│   │   ├── migrations/
│   │   │   ├── 001_initial_schema.sql
│   │   │   └── 002_checkpoints.sql
│   │   └── Cargo.toml
│   │
│   ├── llm-orchestrator-observability/ # NEW - Metrics, tracing, logging
│   │   ├── src/
│   │   │   ├── metrics.rs
│   │   │   ├── tracing.rs
│   │   │   ├── logging.rs
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   │
│   ├── llm-orchestrator-vectordb/    # NEW - Vector database integrations
│   │   ├── src/
│   │   │   ├── traits.rs
│   │   │   ├── pinecone.rs
│   │   │   ├── weaviate.rs
│   │   │   ├── qdrant.rs
│   │   │   ├── chroma.rs
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   │
│   └── llm-orchestrator-audit/       # NEW - Audit logging
│       ├── src/
│       │   ├── logger.rs
│       │   ├── events.rs
│       │   └── lib.rs
│       └── Cargo.toml
│
├── charts/
│   └── llm-orchestrator/             # NEW - Helm chart
│       ├── Chart.yaml
│       ├── values.yaml
│       └── templates/
│           ├── deployment.yaml
│           ├── service.yaml
│           ├── ingress.yaml
│           ├── configmap.yaml
│           ├── secret.yaml
│           └── hpa.yaml
│
├── deploy/
│   ├── docker-compose.yml            # NEW - Development environment
│   └── Dockerfile                     # NEW - Production container
│
├── docs/
│   ├── authentication.md              # NEW - Auth guide
│   ├── state-persistence.md           # NEW - State management
│   ├── monitoring.md                  # NEW - Observability
│   ├── rag-pipelines.md              # NEW - RAG guide
│   └── deployment.md                  # UPDATED - K8s deployment
│
└── examples/
    ├── rag-simple.yaml               # NEW - Simple RAG example
    ├── rag-hybrid.yaml               # NEW - Hybrid search RAG
    └── authenticated-workflow.yaml    # NEW - Auth example
```

---

## Appendix B: Database Schema

```sql
-- PostgreSQL schema for state persistence

CREATE TABLE workflow_states (
    id UUID PRIMARY KEY,
    workflow_id VARCHAR(255) NOT NULL,
    workflow_name VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL,  -- pending, running, paused, completed, failed
    user_id VARCHAR(255),
    started_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    completed_at TIMESTAMP WITH TIME ZONE,
    context JSONB NOT NULL,
    error TEXT,

    INDEX idx_workflow_id_status (workflow_id, status),
    INDEX idx_user_id (user_id),
    INDEX idx_status (status)
);

CREATE TABLE step_states (
    workflow_state_id UUID NOT NULL REFERENCES workflow_states(id) ON DELETE CASCADE,
    step_id VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL,  -- pending, running, completed, failed, skipped
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    outputs JSONB,
    error TEXT,
    retry_count INTEGER DEFAULT 0,

    PRIMARY KEY (workflow_state_id, step_id),
    INDEX idx_workflow_state_id (workflow_state_id)
);

CREATE TABLE checkpoints (
    id UUID PRIMARY KEY,
    workflow_state_id UUID NOT NULL REFERENCES workflow_states(id) ON DELETE CASCADE,
    step_id VARCHAR(255) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    snapshot JSONB NOT NULL,  -- Compressed workflow state

    INDEX idx_workflow_state_timestamp (workflow_state_id, timestamp DESC)
);

CREATE TABLE audit_events (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    user_id VARCHAR(255) NOT NULL,
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    resource_id VARCHAR(255) NOT NULL,
    details JSONB,
    ip_address INET,
    user_agent TEXT,

    INDEX idx_timestamp (timestamp DESC),
    INDEX idx_user_id (user_id),
    INDEX idx_resource (resource_type, resource_id)
);

-- Cleanup old checkpoints (keep last 10 per workflow)
CREATE FUNCTION cleanup_old_checkpoints()
RETURNS TRIGGER AS $$
BEGIN
    DELETE FROM checkpoints
    WHERE workflow_state_id = NEW.workflow_state_id
    AND id NOT IN (
        SELECT id FROM checkpoints
        WHERE workflow_state_id = NEW.workflow_state_id
        ORDER BY timestamp DESC
        LIMIT 10
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_cleanup_checkpoints
AFTER INSERT ON checkpoints
FOR EACH ROW
EXECUTE FUNCTION cleanup_old_checkpoints();
```

---

## Document Change Log

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-11-14 | Claude Code | Initial production readiness plan (SPARC framework) |

---

**END OF PRODUCTION READINESS PLAN**
