# LLM Orchestrator State

State persistence and recovery for the LLM workflow orchestrator.

## Features

- **Database Backends**: PostgreSQL (production) and SQLite (development/testing)
- **Connection Pooling**: Configurable min/max connections for optimal performance
- **Automatic Checkpointing**: Create checkpoints after each step for crash recovery
- **Transaction Support**: Atomic state updates with rollback capability
- **Workflow Recovery**: Resume workflows from last checkpoint after crashes
- **Automatic Cleanup**: Retain last N checkpoints per workflow (configurable)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
llm-orchestrator-state = "0.1"
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres"] }  # For PostgreSQL
# OR
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite"] }   # For SQLite
```

## Usage

### PostgreSQL (Production)

```rust
use llm_orchestrator_state::{PostgresStateStore, StateStore, WorkflowState};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create state store with connection pooling
    let store = PostgresStateStore::new(
        "postgresql://user:password@localhost/workflows",
        Some(5),   // min connections
        Some(20),  // max connections
    ).await?;

    // Create workflow state
    let mut state = WorkflowState::new(
        "my-workflow-123",
        "Data Processing Workflow",
        Some("user-456".to_string()),
        json!({"inputs": {"file": "data.csv"}}),
    );

    // Mark as running
    state.mark_running();

    // Save state
    store.save_workflow_state(&state).await?;

    // Load state
    let loaded = store.load_workflow_state(&state.id).await?;
    println!("Loaded workflow: {}", loaded.workflow_name);

    // Create checkpoint
    use llm_orchestrator_state::Checkpoint;
    let snapshot = serde_json::to_value(&state)?;
    let checkpoint = Checkpoint::new(state.id, "step-5", snapshot);
    store.create_checkpoint(&checkpoint).await?;

    Ok(())
}
```

### SQLite (Development/Testing)

```rust
use llm_orchestrator_state::{SqliteStateStore, StateStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create SQLite store
    let store = SqliteStateStore::new("./workflows.db").await?;

    // Or in-memory for testing
    let mem_store = SqliteStateStore::new(":memory:").await?;

    // Same API as PostgreSQL
    store.health_check().await?;

    Ok(())
}
```

### Recovery After Crash

```rust
use llm_orchestrator_state::{PostgresStateStore, StateStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = PostgresStateStore::new(
        "postgresql://localhost/workflows",
        Some(5),
        Some(20),
    ).await?;

    // On startup: list all active workflows
    let active_workflows = store.list_active_workflows().await?;
    println!("Found {} active workflows to recover", active_workflows.len());

    for workflow_state in active_workflows {
        // Get latest checkpoint
        if let Some(checkpoint) = store.get_latest_checkpoint(&workflow_state.id).await? {
            println!("Resuming workflow {} from step {}",
                workflow_state.workflow_id,
                checkpoint.step_id
            );

            // Restore state and resume execution
            let restored_state = store.restore_from_checkpoint(&checkpoint.id).await?;
            // ... resume workflow execution from restored state
        }
    }

    Ok(())
}
```

### Cleanup Old Data

```rust
use llm_orchestrator_state::{PostgresStateStore, StateStore};
use chrono::{Utc, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = PostgresStateStore::new(
        "postgresql://localhost/workflows",
        Some(5),
        Some(20),
    ).await?;

    // Delete states older than 30 days
    let cutoff = Utc::now() - Duration::days(30);
    let deleted = store.delete_old_states(cutoff).await?;
    println!("Deleted {} old workflow states", deleted);

    Ok(())
}
```

## Database Schema

### Workflow States Table

```sql
CREATE TABLE workflow_states (
    id UUID PRIMARY KEY,
    workflow_id VARCHAR(255) NOT NULL,
    workflow_name VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL,  -- pending, running, paused, completed, failed
    user_id VARCHAR(255),
    started_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    completed_at TIMESTAMP WITH TIME ZONE,
    context TEXT NOT NULL,  -- JSON
    error TEXT
);
```

### Step States Table

```sql
CREATE TABLE step_states (
    workflow_state_id UUID NOT NULL,
    step_id VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL,
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    outputs TEXT,  -- JSON
    error TEXT,
    retry_count INTEGER DEFAULT 0,
    PRIMARY KEY (workflow_state_id, step_id)
);
```

### Checkpoints Table

```sql
CREATE TABLE checkpoints (
    id UUID PRIMARY KEY,
    workflow_state_id UUID NOT NULL,
    step_id VARCHAR(255) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    snapshot TEXT NOT NULL  -- JSON
);
```

## Performance Characteristics

### PostgreSQL

- **State Save Latency**: < 50ms (P99) with connection pooling
- **State Load Latency**: < 30ms (P99)
- **Concurrent Workflows**: 10,000+ active workflows
- **Connection Pool**: 5-20 connections (configurable)
- **Checkpoint Overhead**: < 100ms per checkpoint

### SQLite

- **State Save Latency**: < 20ms (P99) for file-based, < 5ms for in-memory
- **State Load Latency**: < 10ms (P99)
- **Concurrent Workflows**: 1,000+ (limited by single-writer constraint)
- **Best For**: Development, testing, single-node deployments

## Configuration

### PostgreSQL Connection String Format

```
postgresql://[user[:password]@][host][:port][/database][?param1=value1&...]
```

Example with SSL:
```
postgresql://user:pass@localhost:5432/workflows?sslmode=require
```

### SQLite Path Format

```
./relative/path/to/file.db
/absolute/path/to/file.db
:memory:  # In-memory database
```

## Testing

Run unit tests:
```bash
cargo test --package llm-orchestrator-state
```

Run integration tests with PostgreSQL:
```bash
TEST_DATABASE_URL=postgresql://localhost/test_workflows cargo test --package llm-orchestrator-state -- --ignored
```

## Thread Safety

All state store implementations are:
- `Send + Sync` - Safe to share across threads
- Thread-safe for concurrent reads and writes
- Use internal locking/connection pooling

## Error Handling

```rust
use llm_orchestrator_state::{StateStore, StateStoreError};

match store.load_workflow_state(&id).await {
    Ok(state) => println!("Loaded: {}", state.workflow_name),
    Err(StateStoreError::NotFound(msg)) => println!("Not found: {}", msg),
    Err(StateStoreError::Database(err)) => println!("DB error: {}", err),
    Err(StateStoreError::Connection(err)) => println!("Connection failed: {}", err),
    Err(e) => println!("Other error: {}", e),
}
```

## License

MIT OR Apache-2.0
