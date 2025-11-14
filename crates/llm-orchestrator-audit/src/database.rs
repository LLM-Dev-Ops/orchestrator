#[cfg(feature = "database")]
use crate::models::{AuditEvent, AuditEventType, AuditFilter, AuditResult, ResourceType};
#[cfg(feature = "database")]
use crate::storage::{AuditStorage, Result, StorageError};
#[cfg(feature = "database")]
use async_trait::async_trait;
#[cfg(feature = "database")]
use chrono::{DateTime, Utc};
#[cfg(feature = "database")]
use sqlx::postgres::PgPoolOptions;
#[cfg(feature = "database")]
use sqlx::{PgPool, Row};
#[cfg(feature = "database")]
use std::time::Duration;
#[cfg(feature = "database")]
use uuid::Uuid;

#[cfg(feature = "database")]
/// PostgreSQL-backed audit storage
pub struct DatabaseAuditStorage {
    pool: PgPool,
}

#[cfg(feature = "database")]
impl DatabaseAuditStorage {
    /// Create a new database audit storage
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .min_connections(5)
            .max_connections(20)
            .acquire_timeout(Duration::from_secs(5))
            .idle_timeout(Duration::from_secs(300))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        Ok(Self { pool })
    }

    /// Create a new database audit storage with an existing pool
    pub fn with_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_events (
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
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_timestamp ON audit_events(timestamp DESC)")
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_user_id ON audit_events(user_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_event_type ON audit_events(event_type)")
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_resource ON audit_events(resource_type, resource_id)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(feature = "database")]
#[async_trait]
impl AuditStorage for DatabaseAuditStorage {
    async fn store(&self, event: &AuditEvent) -> Result<()> {
        let result_str = event.result.as_str();
        let result_error = event.result.error_message();

        sqlx::query(
            r#"
            INSERT INTO audit_events (
                id, timestamp, event_type, user_id, action,
                resource_type, resource_id, result, result_error, details,
                ip_address, user_agent, request_id, previous_hash, event_hash
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            "#,
        )
        .bind(event.id)
        .bind(event.timestamp)
        .bind(event.event_type.as_str())
        .bind(&event.user_id)
        .bind(&event.action)
        .bind(event.resource_type.as_str())
        .bind(&event.resource_id)
        .bind(result_str)
        .bind(result_error)
        .bind(&event.details)
        .bind(&event.ip_address)
        .bind(&event.user_agent)
        .bind(&event.request_id)
        .bind(&event.previous_hash)
        .bind(&event.event_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn query(&self, filter: AuditFilter) -> Result<Vec<AuditEvent>> {
        let mut query = String::from("SELECT * FROM audit_events WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        if let Some(user_id) = &filter.user_id {
            params.push(user_id.clone());
            query.push_str(&format!(" AND user_id = ${}", params.len()));
        }

        if let Some(event_type) = &filter.event_type {
            params.push(event_type.as_str().to_string());
            query.push_str(&format!(" AND event_type = ${}", params.len()));
        }

        if let Some(resource_type) = &filter.resource_type {
            params.push(resource_type.as_str().to_string());
            query.push_str(&format!(" AND resource_type = ${}", params.len()));
        }

        if let Some(resource_id) = &filter.resource_id {
            params.push(resource_id.clone());
            query.push_str(&format!(" AND resource_id = ${}", params.len()));
        }

        if let Some(start_time) = filter.start_time {
            params.push(start_time.to_rfc3339());
            query.push_str(&format!(" AND timestamp >= ${}", params.len()));
        }

        if let Some(end_time) = filter.end_time {
            params.push(end_time.to_rfc3339());
            query.push_str(&format!(" AND timestamp <= ${}", params.len()));
        }

        if let Some(result) = &filter.result {
            params.push(result.as_str().to_string());
            query.push_str(&format!(" AND result = ${}", params.len()));
        }

        query.push_str(" ORDER BY timestamp DESC");
        query.push_str(&format!(" LIMIT {} OFFSET {}", filter.limit, filter.offset));

        let mut sql_query = sqlx::query(&query);
        for param in params {
            sql_query = sql_query.bind(param);
        }

        let rows = sql_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        let events = rows
            .into_iter()
            .map(|row| {
                let result_str: String = row.get("result");
                let result_error: Option<String> = row.get("result_error");
                let result = match result_str.as_str() {
                    "success" => AuditResult::Success,
                    "failure" => AuditResult::Failure(result_error.unwrap_or_default()),
                    "partial_success" => AuditResult::PartialSuccess,
                    _ => AuditResult::Failure("Unknown result".to_string()),
                };

                let event_type_str: String = row.get("event_type");
                let event_type = match event_type_str.as_str() {
                    "authentication" => AuditEventType::Authentication,
                    "authorization" => AuditEventType::Authorization,
                    "workflow_execution" => AuditEventType::WorkflowExecution,
                    "workflow_create" => AuditEventType::WorkflowCreate,
                    "workflow_update" => AuditEventType::WorkflowUpdate,
                    "workflow_delete" => AuditEventType::WorkflowDelete,
                    "secret_access" => AuditEventType::SecretAccess,
                    "config_change" => AuditEventType::ConfigChange,
                    "api_key_create" => AuditEventType::ApiKeyCreate,
                    "api_key_revoke" => AuditEventType::ApiKeyRevoke,
                    "step_execution" => AuditEventType::StepExecution,
                    _ => AuditEventType::SystemEvent,
                };

                let resource_type_str: String = row.get("resource_type");
                let resource_type = match resource_type_str.as_str() {
                    "workflow" => ResourceType::Workflow,
                    "user" => ResourceType::User,
                    "api_key" => ResourceType::ApiKey,
                    "secret" => ResourceType::Secret,
                    "configuration" => ResourceType::Configuration,
                    "step" => ResourceType::Step,
                    _ => ResourceType::System,
                };

                AuditEvent {
                    id: row.get("id"),
                    timestamp: row.get("timestamp"),
                    event_type,
                    user_id: row.get("user_id"),
                    action: row.get("action"),
                    resource_type,
                    resource_id: row.get("resource_id"),
                    result,
                    details: row.get("details"),
                    ip_address: row.get("ip_address"),
                    user_agent: row.get("user_agent"),
                    request_id: row.get("request_id"),
                    previous_hash: row.get("previous_hash"),
                    event_hash: row.get("event_hash"),
                }
            })
            .collect();

        Ok(events)
    }

    async fn get(&self, id: Uuid) -> Result<Option<AuditEvent>> {
        let row = sqlx::query("SELECT * FROM audit_events WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        if let Some(row) = row {
            let result_str: String = row.get("result");
            let result_error: Option<String> = row.get("result_error");
            let result = match result_str.as_str() {
                "success" => AuditResult::Success,
                "failure" => AuditResult::Failure(result_error.unwrap_or_default()),
                "partial_success" => AuditResult::PartialSuccess,
                _ => AuditResult::Failure("Unknown result".to_string()),
            };

            let event_type_str: String = row.get("event_type");
            let event_type = match event_type_str.as_str() {
                "authentication" => AuditEventType::Authentication,
                "authorization" => AuditEventType::Authorization,
                "workflow_execution" => AuditEventType::WorkflowExecution,
                "workflow_create" => AuditEventType::WorkflowCreate,
                "workflow_update" => AuditEventType::WorkflowUpdate,
                "workflow_delete" => AuditEventType::WorkflowDelete,
                "secret_access" => AuditEventType::SecretAccess,
                "config_change" => AuditEventType::ConfigChange,
                "api_key_create" => AuditEventType::ApiKeyCreate,
                "api_key_revoke" => AuditEventType::ApiKeyRevoke,
                "step_execution" => AuditEventType::StepExecution,
                _ => AuditEventType::SystemEvent,
            };

            let resource_type_str: String = row.get("resource_type");
            let resource_type = match resource_type_str.as_str() {
                "workflow" => ResourceType::Workflow,
                "user" => ResourceType::User,
                "api_key" => ResourceType::ApiKey,
                "secret" => ResourceType::Secret,
                "configuration" => ResourceType::Configuration,
                "step" => ResourceType::Step,
                _ => ResourceType::System,
            };

            Ok(Some(AuditEvent {
                id: row.get("id"),
                timestamp: row.get("timestamp"),
                event_type,
                user_id: row.get("user_id"),
                action: row.get("action"),
                resource_type,
                resource_id: row.get("resource_id"),
                result,
                details: row.get("details"),
                ip_address: row.get("ip_address"),
                user_agent: row.get("user_agent"),
                request_id: row.get("request_id"),
                previous_hash: row.get("previous_hash"),
                event_hash: row.get("event_hash"),
            }))
        } else {
            Ok(None)
        }
    }

    async fn delete_older_than(&self, cutoff: DateTime<Utc>) -> Result<u64> {
        let result = sqlx::query("DELETE FROM audit_events WHERE timestamp < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(result.rows_affected())
    }

    async fn count(&self, filter: AuditFilter) -> Result<u64> {
        let mut query = String::from("SELECT COUNT(*) FROM audit_events WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        if let Some(user_id) = &filter.user_id {
            params.push(user_id.clone());
            query.push_str(&format!(" AND user_id = ${}", params.len()));
        }

        if let Some(event_type) = &filter.event_type {
            params.push(event_type.as_str().to_string());
            query.push_str(&format!(" AND event_type = ${}", params.len()));
        }

        if let Some(resource_type) = &filter.resource_type {
            params.push(resource_type.as_str().to_string());
            query.push_str(&format!(" AND resource_type = ${}", params.len()));
        }

        if let Some(start_time) = filter.start_time {
            params.push(start_time.to_rfc3339());
            query.push_str(&format!(" AND timestamp >= ${}", params.len()));
        }

        if let Some(end_time) = filter.end_time {
            params.push(end_time.to_rfc3339());
            query.push_str(&format!(" AND timestamp <= ${}", params.len()));
        }

        let mut sql_query = sqlx::query(&query);
        for param in params {
            sql_query = sql_query.bind(param);
        }

        let row = sql_query
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        let count: i64 = row.get(0);
        Ok(count as u64)
    }

    async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        Ok(())
    }
}
