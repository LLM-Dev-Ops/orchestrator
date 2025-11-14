use crate::models::{AuditEvent, AuditFilter};
use crate::storage::{AuditStorage, Result, StorageError};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// Rotation policy for file-based audit logs
#[derive(Debug, Clone)]
pub enum RotationPolicy {
    /// Rotate daily at midnight
    Daily,

    /// Rotate when file reaches specified size in bytes
    SizeBased(u64),

    /// Never rotate
    Never,
}

/// File-based audit storage for development and testing
pub struct FileAuditStorage {
    path: PathBuf,
    rotation: RotationPolicy,
    current_file: Arc<RwLock<Option<File>>>,
}

impl FileAuditStorage {
    /// Create a new file-based audit storage
    pub fn new(path: PathBuf, rotation: RotationPolicy) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let storage = Self {
            path,
            rotation,
            current_file: Arc::new(RwLock::new(None)),
        };

        // Open initial file
        storage.open_file()?;

        Ok(storage)
    }

    /// Open or reopen the log file
    fn open_file(&self) -> Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        let mut current_file = self.current_file.write();
        *current_file = Some(file);

        Ok(())
    }

    /// Check if rotation is needed and perform it
    fn rotate_if_needed(&self) -> Result<()> {
        match self.rotation {
            RotationPolicy::Never => Ok(()),
            RotationPolicy::Daily => self.rotate_daily(),
            RotationPolicy::SizeBased(max_size) => self.rotate_if_size_exceeded(max_size),
        }
    }

    /// Rotate the log file daily
    fn rotate_daily(&self) -> Result<()> {
        let metadata = std::fs::metadata(&self.path)?;
        let modified = metadata.modified()?;
        let modified_date = chrono::DateTime::<Utc>::from(modified).date_naive();
        let today = Utc::now().date_naive();

        if modified_date < today {
            self.perform_rotation()?;
        }

        Ok(())
    }

    /// Rotate if file size exceeds the limit
    fn rotate_if_size_exceeded(&self, max_size: u64) -> Result<()> {
        let metadata = std::fs::metadata(&self.path)?;

        if metadata.len() >= max_size {
            self.perform_rotation()?;
        }

        Ok(())
    }

    /// Perform the actual file rotation
    fn perform_rotation(&self) -> Result<()> {
        // Close current file
        {
            let mut current_file = self.current_file.write();
            *current_file = None;
        }

        // Rotate file by appending timestamp
        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        let mut rotated_path = self.path.clone();
        let extension = rotated_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("log");
        rotated_path.set_extension(format!("{}.{}", extension, timestamp));

        std::fs::rename(&self.path, &rotated_path)?;

        // Open new file
        self.open_file()?;

        tracing::info!(
            "Rotated audit log from {} to {}",
            self.path.display(),
            rotated_path.display()
        );

        Ok(())
    }

    /// Read all events from the log file
    fn read_events(&self) -> Result<Vec<AuditEvent>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<AuditEvent>(&line) {
                Ok(event) => events.push(event),
                Err(e) => {
                    tracing::warn!("Failed to parse audit event: {}", e);
                    continue;
                }
            }
        }

        Ok(events)
    }

    /// Filter events based on the provided filter
    fn filter_events(&self, events: Vec<AuditEvent>, filter: &AuditFilter) -> Vec<AuditEvent> {
        let mut filtered: Vec<AuditEvent> = events
            .into_iter()
            .filter(|event| {
                if let Some(ref user_id) = filter.user_id {
                    if event.user_id.as_ref() != Some(user_id) {
                        return false;
                    }
                }

                if let Some(ref event_type) = filter.event_type {
                    if &event.event_type != event_type {
                        return false;
                    }
                }

                if let Some(ref resource_type) = filter.resource_type {
                    if &event.resource_type != resource_type {
                        return false;
                    }
                }

                if let Some(ref resource_id) = filter.resource_id {
                    if &event.resource_id != resource_id {
                        return false;
                    }
                }

                if let Some(start_time) = filter.start_time {
                    if event.timestamp < start_time {
                        return false;
                    }
                }

                if let Some(end_time) = filter.end_time {
                    if event.timestamp > end_time {
                        return false;
                    }
                }

                if let Some(ref result) = filter.result {
                    if event.result.as_str() != result.as_str() {
                        return false;
                    }
                }

                true
            })
            .collect();

        // Sort by timestamp descending
        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply offset and limit
        filtered
            .into_iter()
            .skip(filter.offset)
            .take(filter.limit)
            .collect()
    }
}

#[async_trait]
impl AuditStorage for FileAuditStorage {
    async fn store(&self, event: &AuditEvent) -> Result<()> {
        // Check if rotation is needed
        self.rotate_if_needed()?;

        // Serialize event to JSON
        let json = serde_json::to_string(event)?;

        // Write to file
        let mut current_file = self.current_file.write();
        if let Some(file) = current_file.as_mut() {
            writeln!(file, "{}", json)?;
            file.flush()?;
        } else {
            return Err(StorageError::IoError(std::io::Error::other(
                "File not open",
            )));
        }

        Ok(())
    }

    async fn query(&self, filter: AuditFilter) -> Result<Vec<AuditEvent>> {
        let events = self.read_events()?;
        Ok(self.filter_events(events, &filter))
    }

    async fn get(&self, id: Uuid) -> Result<Option<AuditEvent>> {
        let events = self.read_events()?;
        Ok(events.into_iter().find(|e| e.id == id))
    }

    async fn delete_older_than(&self, cutoff: DateTime<Utc>) -> Result<u64> {
        let events = self.read_events()?;
        let (keep, delete): (Vec<_>, Vec<_>) = events
            .into_iter()
            .partition(|e| e.timestamp >= cutoff);

        let deleted_count = delete.len() as u64;

        // Rewrite file with remaining events
        if deleted_count > 0 {
            // Close current file
            {
                let mut current_file = self.current_file.write();
                *current_file = None;
            }

            // Write remaining events
            let file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(&self.path)?;

            let mut writer = std::io::BufWriter::new(file);
            for event in keep {
                let json = serde_json::to_string(&event)?;
                writeln!(writer, "{}", json)?;
            }
            writer.flush()?;

            // Reopen file for appending
            self.open_file()?;
        }

        Ok(deleted_count)
    }

    async fn count(&self, filter: AuditFilter) -> Result<u64> {
        let events = self.read_events()?;
        let filtered = self.filter_events(events, &filter);
        Ok(filtered.len() as u64)
    }

    async fn health_check(&self) -> Result<()> {
        // Check if file is writable
        let current_file = self.current_file.read();
        if current_file.is_none() {
            return Err(StorageError::IoError(std::io::Error::other(
                "File not open",
            )));
        }

        // Check if path exists
        if !self.path.exists() {
            return Err(StorageError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Audit log file not found",
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AuditEventType, AuditResult, ResourceType};
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_file_storage_store_and_query() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        let storage = FileAuditStorage::new(path.clone(), RotationPolicy::Never).unwrap();

        let event = AuditEvent::new(
            AuditEventType::WorkflowExecution,
            "Test workflow".to_string(),
            ResourceType::Workflow,
            "workflow-123".to_string(),
            AuditResult::Success,
        )
        .with_user_id("user-456".to_string());

        // Store event
        storage.store(&event).await.unwrap();

        // Query events
        let filter = AuditFilter::new();
        let events = storage.query(filter).await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, event.id);
        assert_eq!(events[0].user_id, Some("user-456".to_string()));
    }

    #[tokio::test]
    async fn test_file_storage_filter() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        let storage = FileAuditStorage::new(path.clone(), RotationPolicy::Never).unwrap();

        // Store multiple events
        for i in 0..5 {
            let event = AuditEvent::new(
                AuditEventType::WorkflowExecution,
                format!("Workflow {}", i),
                ResourceType::Workflow,
                format!("workflow-{}", i),
                AuditResult::Success,
            )
            .with_user_id(format!("user-{}", i % 2));

            storage.store(&event).await.unwrap();
        }

        // Filter by user_id
        let filter = AuditFilter::new().with_user_id("user-0".to_string());
        let events = storage.query(filter).await.unwrap();

        assert_eq!(events.len(), 3); // user-0 appears at indices 0, 2, 4
    }

    #[tokio::test]
    async fn test_file_storage_delete_older_than() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        let storage = FileAuditStorage::new(path.clone(), RotationPolicy::Never).unwrap();

        // Store an event
        let event = AuditEvent::new(
            AuditEventType::WorkflowExecution,
            "Test workflow".to_string(),
            ResourceType::Workflow,
            "workflow-123".to_string(),
            AuditResult::Success,
        );

        storage.store(&event).await.unwrap();

        // Delete events older than now (should delete all)
        let cutoff = Utc::now() + chrono::Duration::seconds(1);
        let deleted = storage.delete_older_than(cutoff).await.unwrap();

        assert_eq!(deleted, 1);

        // Verify no events remain
        let filter = AuditFilter::new();
        let events = storage.query(filter).await.unwrap();
        assert_eq!(events.len(), 0);
    }
}
