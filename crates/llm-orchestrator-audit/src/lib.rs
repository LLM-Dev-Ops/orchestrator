// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Audit logging for LLM workflow orchestrator.
//!
//! This crate provides comprehensive audit logging capabilities,
//! including event tracking, storage backends, and compliance features.

pub mod database;
pub mod file;
pub mod logger;
pub mod models;
pub mod retention;
pub mod storage;

// Re-export commonly used types
pub use file::{FileAuditStorage, RotationPolicy};
pub use logger::AuditLogger;
pub use models::{AuditEvent, AuditEventType, AuditFilter, AuditResult, ResourceType};
pub use retention::AuditRetentionManager;
pub use storage::{AuditStorage, Result, StorageError};

#[cfg(feature = "database")]
pub use database::DatabaseAuditStorage;
