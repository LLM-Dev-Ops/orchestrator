// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Data models for secret management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// A secret value with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    /// The secret key/name.
    pub key: String,
    /// The secret value (sensitive data).
    pub value: String,
    /// Version identifier (if supported by backend).
    pub version: Option<String>,
    /// When the secret was created.
    pub created_at: DateTime<Utc>,
    /// Additional metadata associated with the secret.
    pub metadata: HashMap<String, String>,
}

impl Secret {
    /// Create a new secret with minimal information.
    pub fn new(key: String, value: String) -> Self {
        Self {
            key,
            value,
            version: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a new secret with version information.
    pub fn with_version(mut self, version: String) -> Self {
        self.version = Some(version);
        self
    }

    /// Add metadata to the secret.
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Add a single metadata entry.
    pub fn add_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Metadata for creating or updating a secret.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecretMetadata {
    /// Human-readable description of the secret.
    pub description: Option<String>,
    /// Tags for organizing and categorizing secrets.
    pub tags: HashMap<String, String>,
    /// Optional rotation period (for automatic rotation).
    pub rotation_period: Option<Duration>,
}

impl SecretMetadata {
    /// Create new empty metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the description.
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set tags.
    pub fn with_tags(mut self, tags: HashMap<String, String>) -> Self {
        self.tags = tags;
        self
    }

    /// Add a single tag.
    pub fn add_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }

    /// Set rotation period.
    pub fn with_rotation_period(mut self, period: Duration) -> Self {
        self.rotation_period = Some(period);
        self
    }
}

/// Represents a version of a secret (for backends that support versioning).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretVersion {
    /// Version identifier.
    pub version: String,
    /// When this version was created.
    pub created_at: DateTime<Utc>,
    /// Whether this version is the current/active version.
    pub is_current: bool,
    /// Whether this version has been deleted/deactivated.
    pub is_deleted: bool,
}

impl SecretVersion {
    /// Create a new secret version.
    pub fn new(version: String, created_at: DateTime<Utc>) -> Self {
        Self {
            version,
            created_at,
            is_current: false,
            is_deleted: false,
        }
    }

    /// Mark this version as current.
    pub fn mark_current(mut self) -> Self {
        self.is_current = true;
        self
    }
}
