# LLM Orchestrator Audit Logging

Comprehensive audit logging for security events, workflow executions, and configuration changes in the LLM Orchestrator system.

## Features

- **Multiple Storage Backends**
  - PostgreSQL for production deployments
  - File-based storage for development and testing

- **Comprehensive Event Types**
  - Authentication attempts
  - Authorization checks
  - Workflow executions
  - Workflow lifecycle (create, update, delete)
  - Secret access tracking
  - Configuration changes
  - API key management

- **Security Features**
  - Tamper detection via hash chaining
  - Immutable audit log design
  - IP address and user agent tracking
  - Request correlation IDs

- **Retention Management**
  - Configurable retention policies
  - Automatic cleanup of old events
  - Background cleanup tasks

- **Query Interface**
  - Flexible filtering by user, event type, resource, time range
  - Pagination support
  - Count queries for reporting

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
llm-orchestrator-audit = { path = "../llm-orchestrator-audit" }

# For database support
llm-orchestrator-audit = { path = "../llm-orchestrator-audit", features = ["database"] }
```

## Quick Start

### File-Based Storage (Development)

```rust
use llm_orchestrator_audit::{
    AuditLogger, FileAuditStorage, RotationPolicy, AuditResult,
};
use std::sync::Arc;
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create file-based storage
    let storage = Arc::new(FileAuditStorage::new(
        PathBuf::from("/var/log/orchestrator/audit.log"),
        RotationPolicy::Daily,
    )?);

    // Create audit logger
    let logger = AuditLogger::new(storage);

    // Log a workflow execution
    logger.log_workflow_execution(
        "workflow-123",
        "user-456",
        AuditResult::Success,
        Duration::from_millis(500),
    ).await?;

    Ok(())
}
```

### Database Storage (Production)

```rust
use llm_orchestrator_audit::{AuditLogger, DatabaseAuditStorage};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create database storage
    let storage = Arc::new(
        DatabaseAuditStorage::new("postgresql://localhost/audit").await?
    );

    // Run migrations
    storage.migrate().await?;

    // Create audit logger
    let logger = AuditLogger::new(storage);

    // Log an authentication attempt
    logger.log_auth_attempt(
        "user@example.com",
        true,
        Some("192.168.1.1".to_string()),
    ).await?;

    Ok(())
}
```

## Event Types

### Authentication Events

```rust
// Successful authentication
logger.log_auth_attempt(
    "user@example.com",
    true,
    Some("192.168.1.1".to_string()),
).await?;

// Failed authentication
logger.log_auth_attempt(
    "user@example.com",
    false,
    Some("192.168.1.1".to_string()),
).await?;
```

### Authorization Events

```rust
logger.log_authorization(
    "user-123",
    "workflow:execute",
    "workflow-456",
    true,  // allowed
).await?;
```

### Workflow Events

```rust
// Workflow execution
logger.log_workflow_execution(
    "workflow-123",
    "user-456",
    AuditResult::Success,
    Duration::from_millis(500),
).await?;

// Workflow creation
logger.log_workflow_create(
    "workflow-123",
    "My Workflow",
    "user-456",
).await?;

// Workflow update
logger.log_workflow_update(
    "workflow-123",
    "user-456",
    serde_json::json!({"field": "new_value"}),
).await?;

// Workflow deletion
logger.log_workflow_delete(
    "workflow-123",
    "user-456",
).await?;
```

### Secret Access Events

```rust
logger.log_secret_access(
    "api_key",
    "user-123",
    chrono::Utc::now(),
).await?;
```

### Configuration Change Events

```rust
logger.log_config_change(
    "max_concurrent_workflows",
    Some("10"),
    "20",
    "admin-user",
).await?;
```

### API Key Events

```rust
// API key creation
logger.log_api_key_create(
    "key-123",
    "user-456",
    vec!["workflow:read".to_string(), "workflow:execute".to_string()],
).await?;

// API key revocation
logger.log_api_key_revoke(
    "key-123",
    "admin-user",
    "Security policy update",
).await?;
```

## Querying Audit Events

```rust
use llm_orchestrator_audit::AuditFilter;

// Query all events for a user
let filter = AuditFilter::new()
    .with_user_id("user-123".to_string())
    .with_limit(100);

let events = storage.query(filter).await?;

// Query workflow execution events
let filter = AuditFilter::new()
    .with_event_type(AuditEventType::WorkflowExecution)
    .with_time_range(
        Utc::now() - Duration::days(7),
        Utc::now(),
    );

let events = storage.query(filter).await?;

// Get a specific event by ID
let event = storage.get(event_id).await?;

// Count events
let count = storage.count(filter).await?;
```

## Retention Management

### Manual Cleanup

```rust
use llm_orchestrator_audit::AuditRetentionManager;

let manager = AuditRetentionManager::new(storage, 90);  // 90 days retention

// Run cleanup manually
let deleted = manager.cleanup().await?;
println!("Deleted {} old audit events", deleted);
```

### Background Cleanup

```rust
use std::time::Duration;

let manager = Arc::new(AuditRetentionManager::new(storage, 90));

// Run cleanup daily
let _handle = manager.start_background_cleanup(Duration::from_secs(86400));

// The cleanup task runs in the background until the handle is dropped
```

## File Rotation

### Daily Rotation

```rust
let storage = FileAuditStorage::new(
    PathBuf::from("/var/log/audit.log"),
    RotationPolicy::Daily,
)?;
```

### Size-Based Rotation

```rust
// Rotate when file reaches 100 MB
let storage = FileAuditStorage::new(
    PathBuf::from("/var/log/audit.log"),
    RotationPolicy::SizeBased(100 * 1024 * 1024),
)?;
```

### No Rotation

```rust
let storage = FileAuditStorage::new(
    PathBuf::from("/var/log/audit.log"),
    RotationPolicy::Never,
)?;
```

## Database Schema

The audit events table is created with the following structure:

```sql
CREATE TABLE audit_events (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    user_id VARCHAR(255),
    action VARCHAR(255) NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    resource_id VARCHAR(255) NOT NULL,
    result VARCHAR(50) NOT NULL,
    result_error TEXT,
    details JSONB,
    ip_address INET,
    user_agent TEXT,
    request_id VARCHAR(255),
    previous_hash VARCHAR(64),
    event_hash VARCHAR(64)
);
```

### Indexes

- `idx_audit_timestamp`: Time-based queries (most common)
- `idx_audit_user_id`: User-based queries
- `idx_audit_event_type`: Event type filtering
- `idx_audit_resource`: Resource lookups
- `idx_audit_result`: Result-based queries
- `idx_audit_request_id`: Request correlation
- `idx_audit_details`: JSONB queries (GIN index)

## Performance Considerations

### Database Storage

- **Query Performance**: < 100ms P99 for filtered queries with proper indexes
- **Write Performance**: < 5ms P99 for single event insertion
- **Connection Pool**: 5-20 connections (configurable)
- **Async Operations**: All operations are async for non-blocking I/O

### File Storage

- **Write Performance**: < 1ms for single event (append-only)
- **Query Performance**: O(n) scan of file (suitable for development only)
- **Rotation Overhead**: < 10ms for file rotation

## Security Features

### Hash Chain

Each audit event includes a hash of the previous event, creating a tamper-evident chain:

```rust
event.previous_hash = previous_event.event_hash;
event.event_hash = compute_hash(event);
```

Any modification to a past event will break the chain, making tampering detectable.

### Immutable Design

The storage backends are designed to be append-only. The query interface does not provide update or delete methods for individual events, only bulk deletion for retention management.

## Compliance

This audit logging implementation supports compliance with:

- SOC 2 Type II (audit trail requirements)
- HIPAA (access logging)
- GDPR (data access tracking)
- PCI DSS (security event logging)

## Testing

Run the test suite:

```bash
cargo test -p llm-orchestrator-audit
```

Run tests with database support:

```bash
cargo test -p llm-orchestrator-audit --features database
```

## Examples

See the `examples/` directory for complete working examples:

- `basic_file_storage.rs`: Simple file-based logging
- `database_storage.rs`: PostgreSQL-backed logging
- `retention_management.rs`: Automatic cleanup
- `query_examples.rs`: Filtering and querying events

## License

Licensed under either of:

- MIT License
- Apache License, Version 2.0

at your option.
