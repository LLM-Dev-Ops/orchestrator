# Audit Logger Integration Guide

This document describes how to integrate the audit logger with the LLM Orchestrator workflow executor.

## Integration Points

The audit logger should be integrated at the following key points in the orchestrator:

### 1. Workflow Executor Integration

Add audit logging to the `WorkflowExecutor` to track all workflow and step executions.

#### Add Audit Logger Field

```rust
use llm_orchestrator_audit::AuditLogger;

pub struct WorkflowExecutor {
    // ... existing fields ...

    /// Optional audit logger
    audit_logger: Option<Arc<AuditLogger>>,
}
```

#### Update Constructor

```rust
impl WorkflowExecutor {
    pub fn new(workflow: Workflow) -> Self {
        // ... existing initialization ...

        Self {
            // ... existing fields ...
            audit_logger: None,
        }
    }

    /// Enable audit logging
    pub fn with_audit_logger(mut self, logger: Arc<AuditLogger>) -> Self {
        self.audit_logger = Some(logger);
        self
    }
}
```

#### Log Workflow Execution

```rust
impl WorkflowExecutor {
    pub async fn execute(&self) -> Result<HashMap<String, StepResult>> {
        let start_time = std::time::Instant::now();

        // Log workflow start
        if let Some(logger) = &self.audit_logger {
            let _ = logger.log_workflow_create(
                &self.workflow.id,
                &self.workflow.name,
                "system", // or actual user ID
            ).await;
        }

        // Execute workflow
        let result = self.execute_inner().await;

        let duration = start_time.elapsed();

        // Log workflow completion
        if let Some(logger) = &self.audit_logger {
            let audit_result = match &result {
                Ok(_) => llm_orchestrator_audit::AuditResult::Success,
                Err(e) => llm_orchestrator_audit::AuditResult::Failure(e.to_string()),
            };

            let _ = logger.log_workflow_execution(
                &self.workflow.id,
                "system", // or actual user ID
                audit_result,
                duration,
            ).await;
        }

        result
    }
}
```

#### Log Step Execution

```rust
impl WorkflowExecutor {
    async fn execute_step(&self, step: &Step) -> Result<HashMap<String, Value>> {
        let start_time = std::time::Instant::now();

        // Execute step
        let result = self.execute_step_inner(step).await;

        let duration = start_time.elapsed();

        // Log step execution
        if let Some(logger) = &self.audit_logger {
            let audit_result = match &result {
                Ok(_) => llm_orchestrator_audit::AuditResult::Success,
                Err(e) => llm_orchestrator_audit::AuditResult::Failure(e.to_string()),
            };

            let _ = logger.log_step_execution(
                &self.workflow.id,
                &step.id,
                "system", // or actual user ID
                audit_result,
                duration,
            ).await;
        }

        result
    }
}
```

### 2. Authentication Integration

If you have an authentication layer, integrate audit logging there:

```rust
use llm_orchestrator_audit::AuditLogger;

pub struct AuthService {
    audit_logger: Arc<AuditLogger>,
    // ... other fields ...
}

impl AuthService {
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<Session> {
        let result = self.verify_credentials(username, password).await;

        // Log authentication attempt
        let success = result.is_ok();
        self.audit_logger.log_auth_attempt(
            username,
            success,
            None, // IP address can be added here
        ).await?;

        result
    }

    pub async fn check_permission(&self, user: &User, permission: &str, resource: &str) -> Result<bool> {
        let allowed = self.has_permission(user, permission).await;

        // Log authorization check
        self.audit_logger.log_authorization(
            &user.id,
            permission,
            resource,
            allowed,
        ).await?;

        Ok(allowed)
    }
}
```

### 3. Secret Manager Integration

Track all secret access:

```rust
use llm_orchestrator_audit::AuditLogger;

pub struct SecretManager {
    audit_logger: Arc<AuditLogger>,
    // ... other fields ...
}

impl SecretManager {
    pub async fn get_secret(&self, key: &str, user_id: &str) -> Result<String> {
        let secret = self.retrieve_secret(key).await?;

        // Log secret access
        self.audit_logger.log_secret_access(
            key,
            user_id,
            chrono::Utc::now(),
        ).await?;

        Ok(secret)
    }
}
```

### 4. Configuration Service Integration

Track configuration changes:

```rust
use llm_orchestrator_audit::AuditLogger;

pub struct ConfigService {
    audit_logger: Arc<AuditLogger>,
    config: Arc<RwLock<HashMap<String, String>>>,
}

impl ConfigService {
    pub async fn set_config(&self, key: &str, value: &str, user_id: &str) -> Result<()> {
        let old_value = self.config.read().await.get(key).cloned();

        // Update configuration
        self.config.write().await.insert(key.to_string(), value.to_string());

        // Log configuration change
        self.audit_logger.log_config_change(
            key,
            old_value.as_deref(),
            value,
            user_id,
        ).await?;

        Ok(())
    }
}
```

## Complete Example

Here's a complete example showing how to set up audit logging in your application:

```rust
use llm_orchestrator_audit::{
    AuditLogger, FileAuditStorage, RotationPolicy, AuditRetentionManager,
};
use std::sync::Arc;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Set up audit storage
    let storage = Arc::new(FileAuditStorage::new(
        PathBuf::from("/var/log/orchestrator/audit.log"),
        RotationPolicy::Daily,
    )?);

    // 2. Create audit logger
    let audit_logger = Arc::new(AuditLogger::new(storage.clone()));

    // 3. Set up retention management (90 days)
    let retention_manager = Arc::new(AuditRetentionManager::new(
        storage.clone(),
        90,
    ));

    // 4. Start background cleanup (daily)
    let _cleanup_handle = retention_manager.start_background_cleanup(
        std::time::Duration::from_secs(86400)
    );

    // 5. Create workflow executor with audit logging
    let workflow = Workflow::from_yaml_file("workflow.yaml")?;
    let executor = WorkflowExecutor::new(workflow)
        .with_audit_logger(audit_logger.clone())
        .build();

    // 6. Execute workflow (audit logs will be automatically generated)
    let result = executor.execute().await?;

    println!("Workflow completed: {:?}", result);

    Ok(())
}
```

## Production Database Setup

For production deployments, use PostgreSQL:

```rust
use llm_orchestrator_audit::{AuditLogger, DatabaseAuditStorage, AuditRetentionManager};
use std::sync::Arc;

async fn setup_production_audit() -> Result<Arc<AuditLogger>, Box<dyn std::error::Error>> {
    // 1. Create database storage
    let storage = Arc::new(
        DatabaseAuditStorage::new("postgresql://user:pass@localhost/audit").await?
    );

    // 2. Run migrations
    storage.migrate().await?;

    // 3. Create audit logger
    let logger = Arc::new(AuditLogger::new(storage.clone()));

    // 4. Set up retention (90 days)
    let retention = Arc::new(AuditRetentionManager::new(storage, 90));

    // 5. Start daily cleanup at 2 AM
    tokio::spawn(async move {
        loop {
            // Wait until 2 AM
            let now = chrono::Utc::now();
            let target = (now + chrono::Duration::days(1))
                .date_naive()
                .and_hms_opt(2, 0, 0)
                .unwrap()
                .and_utc();

            let wait_duration = (target - now).to_std().unwrap();
            tokio::time::sleep(wait_duration).await;

            // Run cleanup
            match retention.cleanup().await {
                Ok(deleted) => {
                    tracing::info!("Audit cleanup: deleted {} events", deleted);
                }
                Err(e) => {
                    tracing::error!("Audit cleanup failed: {}", e);
                }
            }
        }
    });

    Ok(logger)
}
```

## Environment Configuration

Use environment variables for configuration:

```rust
use std::env;

fn create_audit_logger() -> Result<Arc<AuditLogger>, Box<dyn std::error::Error>> {
    let storage_type = env::var("AUDIT_STORAGE").unwrap_or_else(|_| "file".to_string());

    let storage: Arc<dyn AuditStorage> = match storage_type.as_str() {
        "database" => {
            let db_url = env::var("AUDIT_DATABASE_URL")?;
            Arc::new(DatabaseAuditStorage::new(&db_url).await?)
        }
        "file" => {
            let log_path = env::var("AUDIT_LOG_PATH")
                .unwrap_or_else(|_| "/var/log/orchestrator/audit.log".to_string());
            Arc::new(FileAuditStorage::new(
                PathBuf::from(log_path),
                RotationPolicy::Daily,
            )?)
        }
        _ => return Err("Invalid AUDIT_STORAGE type".into()),
    };

    Ok(Arc::new(AuditLogger::new(storage)))
}
```

## Testing with Audit Logging

For tests, you can disable audit logging or use a mock:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_execution() {
        // Create executor without audit logging
        let executor = WorkflowExecutor::new(test_workflow())
            .build();

        let result = executor.execute().await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_workflow_with_audit() {
        // Create temp audit log for testing
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let storage = Arc::new(FileAuditStorage::new(
            temp_file.path().to_path_buf(),
            RotationPolicy::Never,
        ).unwrap());

        let logger = Arc::new(AuditLogger::new(storage.clone()));

        let executor = WorkflowExecutor::new(test_workflow())
            .with_audit_logger(logger)
            .build();

        executor.execute().await.unwrap();

        // Verify audit events were created
        let events = storage.query(AuditFilter::new()).await.unwrap();
        assert!(!events.is_empty());
    }
}
```

## Performance Considerations

1. **Async Logging**: All audit operations are async and non-blocking
2. **Buffered Writes**: File storage uses buffered I/O
3. **Connection Pooling**: Database storage uses connection pooling (5-20 connections)
4. **Error Handling**: Audit failures should not crash the main application

```rust
// Good: Log audit events without failing the main operation
if let Some(logger) = &self.audit_logger {
    if let Err(e) = logger.log_workflow_execution(...).await {
        tracing::warn!("Failed to log audit event: {}", e);
    }
}
```

## Monitoring Audit Logging

Add metrics to monitor audit logging health:

```rust
use prometheus::{Counter, IntCounter};

lazy_static! {
    static ref AUDIT_EVENTS_TOTAL: IntCounter =
        register_int_counter!("audit_events_total", "Total audit events logged").unwrap();

    static ref AUDIT_ERRORS_TOTAL: IntCounter =
        register_int_counter!("audit_errors_total", "Total audit logging errors").unwrap();
}

// In your audit logging code
if let Err(e) = logger.log_event(event).await {
    AUDIT_ERRORS_TOTAL.inc();
    tracing::error!("Audit logging failed: {}", e);
} else {
    AUDIT_EVENTS_TOTAL.inc();
}
```

## Security Best Practices

1. **Never log secrets**: Audit events should not contain actual secret values
2. **Log access patterns**: Track who accessed what and when
3. **Tamper detection**: Use hash chaining to detect modifications
4. **Separate storage**: Consider using a separate database for audit logs
5. **Retention policies**: Implement appropriate retention based on compliance requirements
6. **Access control**: Restrict access to audit logs to authorized personnel only

## Compliance Queries

Example queries for compliance reporting:

```rust
// Failed authentication attempts in the last 24 hours
let filter = AuditFilter::new()
    .with_event_type(AuditEventType::Authentication)
    .with_result(AuditResult::Failure("".to_string()))
    .with_time_range(
        Utc::now() - chrono::Duration::hours(24),
        Utc::now(),
    );

let failed_auths = storage.query(filter).await?;

// All secret access by a specific user
let filter = AuditFilter::new()
    .with_user_id("user-123".to_string())
    .with_event_type(AuditEventType::SecretAccess);

let secret_access = storage.query(filter).await?;

// Configuration changes in the last 30 days
let filter = AuditFilter::new()
    .with_event_type(AuditEventType::ConfigChange)
    .with_time_range(
        Utc::now() - chrono::Duration::days(30),
        Utc::now(),
    );

let config_changes = storage.query(filter).await?;
```
