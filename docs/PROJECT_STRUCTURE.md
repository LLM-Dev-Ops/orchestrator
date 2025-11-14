# LLM-Orchestrator Project Structure

**Version:** 1.0
**Last Updated:** 2025-11-14

---

## Directory Structure

```
llm-orchestrator/
├── Cargo.toml                      # Root workspace manifest
├── Cargo.lock
├── README.md
├── LICENSE
├── .gitignore
├── .github/
│   └── workflows/
│       ├── ci.yml                  # Continuous integration
│       ├── release.yml             # Release automation
│       └── security.yml            # Security scanning
│
├── docs/
│   ├── ARCHITECTURE.md             # System architecture (this document)
│   ├── SCHEMA.md                   # Workflow schema reference
│   ├── INTEGRATION_GUIDE.md        # Integration guide
│   ├── DEPLOYMENT.md               # Deployment guide
│   ├── API_REFERENCE.md            # API documentation
│   └── CONTRIBUTING.md             # Contribution guidelines
│
├── crates/
│   ├── orchestrator-core/          # Core orchestration engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── workflow/
│   │       │   ├── mod.rs
│   │       │   ├── definition.rs   # Workflow definition types
│   │       │   ├── parser.rs       # YAML/JSON parsing
│   │       │   ├── validator.rs    # Schema validation
│   │       │   └── registry.rs     # Workflow registry
│   │       ├── scheduler/
│   │       │   ├── mod.rs
│   │       │   ├── scheduler.rs    # Main scheduler
│   │       │   ├── priority_queue.rs
│   │       │   ├── backpressure.rs
│   │       │   └── resource_manager.rs
│   │       ├── executor/
│   │       │   ├── mod.rs
│   │       │   ├── pool.rs         # Executor pool
│   │       │   ├── task_executor.rs # Task executor trait
│   │       │   ├── registry.rs     # Executor registry
│   │       │   └── builtin/
│   │       │       ├── transform.rs
│   │       │       ├── llm.rs
│   │       │       ├── evaluation.rs
│   │       │       └── policy.rs
│   │       ├── state/
│   │       │   ├── mod.rs
│   │       │   ├── manager.rs      # State manager
│   │       │   ├── machine.rs      # State machine FSM
│   │       │   ├── storage.rs      # Storage trait
│   │       │   └── cache.rs        # State cache
│   │       ├── graph/
│   │       │   ├── mod.rs
│   │       │   ├── dag.rs          # DAG implementation
│   │       │   ├── traversal.rs    # Graph traversal
│   │       │   └── cycle_detection.rs
│   │       ├── expression/
│   │       │   ├── mod.rs
│   │       │   ├── parser.rs       # Expression parser
│   │       │   ├── evaluator.rs    # Expression evaluator
│   │       │   └── context.rs      # Execution context
│   │       └── error.rs            # Error types
│   │
│   ├── orchestrator-fault-tolerance/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── checkpoint/
│   │       │   ├── mod.rs
│   │       │   ├── manager.rs      # Checkpoint manager
│   │       │   ├── wal.rs          # Write-ahead log
│   │       │   └── storage.rs      # Checkpoint storage
│   │       ├── retry/
│   │       │   ├── mod.rs
│   │       │   ├── coordinator.rs  # Retry coordinator
│   │       │   ├── policy.rs       # Retry policies
│   │       │   └── backoff.rs      # Backoff strategies
│   │       ├── circuit_breaker/
│   │       │   ├── mod.rs
│   │       │   ├── breaker.rs      # Circuit breaker
│   │       │   └── state.rs        # Breaker state machine
│   │       ├── dlq/
│   │       │   ├── mod.rs
│   │       │   ├── queue.rs        # Dead letter queue
│   │       │   └── storage.rs      # DLQ storage
│   │       └── degradation/
│   │           ├── mod.rs
│   │           ├── manager.rs      # Degradation manager
│   │           └── strategy.rs     # Fallback strategies
│   │
│   ├── orchestrator-storage/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── postgres.rs         # PostgreSQL implementation
│   │       ├── sqlite.rs           # SQLite implementation
│   │       ├── redis.rs            # Redis cache
│   │       ├── s3.rs               # S3 storage
│   │       ├── migrations/         # Database migrations
│   │       │   ├── mod.rs
│   │       │   └── *.sql
│   │       └── models.rs           # Database models
│   │
│   ├── orchestrator-api/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── grpc/
│   │       │   ├── mod.rs
│   │       │   ├── server.rs       # gRPC server
│   │       │   └── service.rs      # Service implementation
│   │       ├── rest/
│   │       │   ├── mod.rs
│   │       │   ├── server.rs       # REST API server
│   │       │   ├── handlers.rs     # Request handlers
│   │       │   └── middleware.rs   # Middleware
│   │       └── proto/
│   │           └── orchestrator.proto # gRPC proto definitions
│   │
│   ├── orchestrator-cli/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/
│   │       │   ├── mod.rs
│   │       │   ├── run.rs          # Run workflow
│   │       │   ├── validate.rs     # Validate workflow
│   │       │   ├── list.rs         # List executions
│   │       │   ├── status.rs       # Get status
│   │       │   └── cancel.rs       # Cancel execution
│   │       └── output.rs           # Output formatting
│   │
│   ├── orchestrator-sdk/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs           # SDK client
│   │       ├── builder.rs          # Workflow builder
│   │       └── handle.rs           # Execution handle
│   │
│   ├── orchestrator-integrations/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── llm_forge/
│   │       │   ├── mod.rs
│   │       │   ├── adapter.rs      # LLM-Forge adapter
│   │       │   ├── client.rs       # Forge client
│   │       │   └── executors.rs    # Forge executors
│   │       ├── test_bench/
│   │       │   ├── mod.rs
│   │       │   ├── hooks.rs        # Test-Bench hooks
│   │       │   ├── client.rs       # Test-Bench client
│   │       │   └── executors.rs    # Evaluation executors
│   │       ├── auto_optimizer/
│   │       │   ├── mod.rs
│   │       │   ├── collector.rs    # Metrics collector
│   │       │   ├── client.rs       # Optimizer client
│   │       │   └── recommendations.rs
│   │       └── governance/
│   │           ├── mod.rs
│   │           ├── interceptor.rs  # Policy interceptor
│   │           ├── client.rs       # Governance client
│   │           └── cost_tracker.rs # Cost tracking
│   │
│   ├── orchestrator-observability/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── metrics/
│   │       │   ├── mod.rs
│   │       │   ├── collector.rs    # Metrics collector
│   │       │   └── exporter.rs     # Prometheus exporter
│   │       ├── tracing/
│   │       │   ├── mod.rs
│   │       │   └── setup.rs        # Tracing setup
│   │       └── logging/
│   │           ├── mod.rs
│   │           └── formatter.rs    # Log formatting
│   │
│   └── orchestrator-security/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── auth/
│           │   ├── mod.rs
│           │   ├── provider.rs     # Auth provider trait
│           │   ├── jwt.rs          # JWT authentication
│           │   └── api_key.rs      # API key auth
│           ├── authz/
│           │   ├── mod.rs
│           │   ├── policy.rs       # Authorization policies
│           │   └── rbac.rs         # RBAC implementation
│           └── audit/
│               ├── mod.rs
│               └── logger.rs       # Audit logger
│
├── examples/
│   ├── simple_workflow.yaml        # Simple workflow example
│   ├── multi_stage_pipeline.yaml   # Multi-stage pipeline
│   ├── event_driven.yaml            # Event-driven workflow
│   ├── conditional_branches.yaml    # Conditional branching
│   └── embedded_sdk.rs              # SDK usage example
│
├── tests/
│   ├── integration/
│   │   ├── workflow_execution_test.rs
│   │   ├── fault_tolerance_test.rs
│   │   ├── integration_test.rs
│   │   └── e2e_test.rs
│   └── fixtures/
│       ├── workflows/
│       └── data/
│
├── benches/
│   ├── scheduler_benchmark.rs
│   ├── executor_benchmark.rs
│   └── graph_benchmark.rs
│
├── config/
│   ├── orchestrator.yaml            # Default configuration
│   ├── orchestrator.dev.yaml        # Development config
│   └── orchestrator.prod.yaml       # Production config
│
├── deploy/
│   ├── docker/
│   │   ├── Dockerfile
│   │   └── docker-compose.yaml
│   ├── kubernetes/
│   │   ├── namespace.yaml
│   │   ├── configmap.yaml
│   │   ├── secrets.yaml
│   │   ├── api-deployment.yaml
│   │   ├── worker-deployment.yaml
│   │   ├── service.yaml
│   │   ├── ingress.yaml
│   │   └── hpa.yaml
│   └── terraform/
│       ├── main.tf
│       ├── variables.tf
│       └── outputs.tf
│
└── scripts/
    ├── build.sh                     # Build script
    ├── test.sh                      # Test script
    ├── deploy.sh                    # Deployment script
    └── migrate.sh                   # Database migration
```

---

## Crate Dependencies

### Root Cargo.toml

```toml
[workspace]
members = [
    "crates/orchestrator-core",
    "crates/orchestrator-fault-tolerance",
    "crates/orchestrator-storage",
    "crates/orchestrator-api",
    "crates/orchestrator-cli",
    "crates/orchestrator-sdk",
    "crates/orchestrator-integrations",
    "crates/orchestrator-observability",
    "crates/orchestrator-security",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["LLM Orchestrator Team"]
license = "MIT"
repository = "https://github.com/llm-orchestrator/llm-orchestrator"

[workspace.dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }
tokio-stream = "0.1"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"

# Graph algorithms
petgraph = "0.6"

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "sqlite"] }
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }

# API frameworks
tonic = "0.10"
tonic-build = "0.10"
prost = "0.12"
axum = "0.7"
tower = "0.4"

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
opentelemetry = "0.21"
opentelemetry-jaeger = "0.20"
prometheus = "0.13"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# CLI
clap = { version = "4.4", features = ["derive"] }

# Utilities
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
futures = "0.3"
async-trait = "0.1"
dashmap = "5.5"

# Testing
mockall = "0.12"
tokio-test = "0.4"
criterion = "0.5"
```

---

## Implementation Roadmap

### Phase 1: Core Foundation (Weeks 1-4)

**Week 1-2: Core Data Structures**
```
✓ Workflow definition types
✓ Task execution state machine
✓ DAG implementation with petgraph
✓ Expression parser and evaluator
✓ Basic error handling
```

**Week 3-4: Scheduler and Executor**
```
✓ Basic scheduler with priority queue
✓ Executor pool with Tokio
✓ Task executor trait
✓ Built-in executors (transform, mock LLM)
✓ Resource manager
```

**Deliverables:**
- `orchestrator-core` crate functional
- Can execute simple workflows locally
- Unit tests for core components

---

### Phase 2: State Management and Storage (Weeks 5-6)

**Week 5: State Persistence**
```
✓ State manager implementation
✓ PostgreSQL storage backend
✓ SQLite storage backend
✓ Database migrations
✓ Checkpoint mechanism
```

**Week 6: Caching and WAL**
```
✓ Redis cache integration
✓ Write-ahead log (WAL)
✓ State recovery from checkpoints
✓ Integration tests
```

**Deliverables:**
- `orchestrator-storage` crate complete
- Persistent workflow execution
- Checkpoint/restore functionality

---

### Phase 3: Fault Tolerance (Weeks 7-8)

**Week 7: Retry and Circuit Breaker**
```
✓ Retry coordinator
✓ Exponential backoff strategies
✓ Circuit breaker implementation
✓ Retryable error classification
```

**Week 8: DLQ and Degradation**
```
✓ Dead letter queue
✓ Graceful degradation manager
✓ Fallback strategies
✓ Integration with core
```

**Deliverables:**
- `orchestrator-fault-tolerance` crate complete
- Production-grade error handling
- Comprehensive fault tolerance tests

---

### Phase 4: API Layer (Weeks 9-10)

**Week 9: gRPC API**
```
✓ Proto definitions
✓ gRPC server implementation
✓ Service methods (execute, status, cancel)
✓ Streaming support
```

**Week 10: REST API**
```
✓ Axum REST server
✓ REST endpoints
✓ OpenAPI/Swagger documentation
✓ Authentication middleware
```

**Deliverables:**
- `orchestrator-api` crate complete
- Both gRPC and REST APIs functional
- API documentation

---

### Phase 5: Integration Layer (Weeks 11-13)

**Week 11: LLM-Forge and Test-Bench**
```
✓ LLM-Forge adapter
✓ LLM executors (Claude, GPT-4)
✓ Test-Bench hooks
✓ Evaluation executors
```

**Week 12: Auto-Optimizer and Governance**
```
✓ Metrics collector
✓ Optimizer client
✓ Policy interceptor
✓ Cost tracker
```

**Week 13: Integration Testing**
```
✓ End-to-end integration tests
✓ Mock services for external systems
✓ Performance benchmarks
```

**Deliverables:**
- `orchestrator-integrations` crate complete
- Full ecosystem integration
- Integration test suite

---

### Phase 6: Observability and Security (Weeks 14-15)

**Week 14: Observability**
```
✓ Prometheus metrics exporter
✓ Distributed tracing setup
✓ Structured logging
✓ Grafana dashboards
```

**Week 15: Security**
```
✓ JWT authentication
✓ RBAC authorization
✓ Audit logging
✓ TLS support
```

**Deliverables:**
- `orchestrator-observability` crate complete
- `orchestrator-security` crate complete
- Production-ready monitoring and security

---

### Phase 7: CLI and SDK (Week 16)

```
✓ CLI commands (run, validate, list, status, cancel)
✓ SDK client library
✓ Workflow builder API
✓ Examples and documentation
```

**Deliverables:**
- `orchestrator-cli` crate complete
- `orchestrator-sdk` crate complete
- Developer-friendly interfaces

---

### Phase 8: Advanced Features (Weeks 17-18)

**Week 17: Advanced Orchestration**
```
✓ Event-driven workflows
✓ Streaming pipelines
✓ Hybrid orchestration
✓ Dynamic workflow modification
```

**Week 18: Optimization**
```
✓ Performance tuning
✓ Memory optimization
✓ Concurrency improvements
✓ Benchmark suite
```

**Deliverables:**
- Advanced orchestration patterns
- Performance optimizations
- Comprehensive benchmarks

---

### Phase 9: Deployment and Operations (Weeks 19-20)

**Week 19: Containerization**
```
✓ Dockerfile
✓ Docker Compose setup
✓ Multi-stage builds
✓ Image optimization
```

**Week 20: Kubernetes**
```
✓ Kubernetes manifests
✓ Helm charts
✓ Auto-scaling configuration
✓ Deployment documentation
```

**Deliverables:**
- Production-ready containers
- Kubernetes deployment
- Complete deployment guide

---

### Phase 10: Documentation and Release (Weeks 21-22)

**Week 21: Documentation**
```
✓ Architecture documentation
✓ API reference
✓ Integration guide
✓ Deployment guide
✓ Examples and tutorials
```

**Week 22: Testing and Release**
```
✓ Comprehensive test suite
✓ Security audit
✓ Performance validation
✓ v1.0.0 release
```

**Deliverables:**
- Complete documentation
- Production release
- Release announcement

---

## Development Guidelines

### Code Organization

```rust
// Module structure example
pub mod workflow {
    mod definition;   // Public API types
    mod parser;       // Internal implementation
    mod validator;    // Internal implementation

    pub use definition::*;  // Re-export public types
}
```

### Error Handling

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),

    #[error("Task execution failed: {source}")]
    TaskExecutionFailed {
        #[from]
        source: anyhow::Error,
    },

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}
```

### Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_execution() {
        // Arrange
        let workflow = create_test_workflow();
        let orchestrator = Orchestrator::new_test();

        // Act
        let result = orchestrator.execute(workflow).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fault_tolerance() {
        // Test retry logic, circuit breaker, etc.
    }
}
```

### Performance Benchmarks

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn scheduler_benchmark(c: &mut Criterion) {
    c.bench_function("schedule_workflow", |b| {
        b.iter(|| {
            // Benchmark code
        });
    });
}

criterion_group!(benches, scheduler_benchmark);
criterion_main!(benches);
```

---

## Build and CI/CD

### Build Script

```bash
#!/bin/bash
# scripts/build.sh

set -e

echo "Building LLM-Orchestrator..."

# Format check
cargo fmt --all -- --check

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Build
cargo build --release

# Test
cargo test --all-features

echo "Build complete!"
```

### CI Configuration

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - uses: Swatinem/rust-cache@v2
      - name: Run tests
        run: cargo test --all-features

  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy
      - name: Check formatting
        run: cargo fmt -- --check
      - name: Run clippy
        run: cargo clippy -- -D warnings

  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Build release
        run: cargo build --release
```

---

This project structure provides a comprehensive blueprint for implementing the LLM-Orchestrator with a clear roadmap and development guidelines.
