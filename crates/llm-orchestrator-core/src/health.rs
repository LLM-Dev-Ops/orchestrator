// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Health check functionality for monitoring system status.
//!
//! This module provides health check endpoints for Kubernetes readiness/liveness
//! probes and general system health monitoring.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;

/// Overall health status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// System is healthy and operational.
    Healthy,
    /// System is degraded but operational.
    Degraded,
    /// System is unhealthy and not operational.
    Unhealthy,
}

/// Health check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Overall status.
    pub status: HealthStatus,
    /// Timestamp of the check.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Individual component checks.
    pub checks: HashMap<String, ComponentHealth>,
    /// Additional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Health status of an individual component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component status.
    pub status: HealthStatus,
    /// Response time in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_time_ms: Option<u64>,
    /// Error message if unhealthy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Last check timestamp.
    pub last_check: chrono::DateTime<chrono::Utc>,
}

impl ComponentHealth {
    /// Creates a healthy component status.
    pub fn healthy() -> Self {
        Self {
            status: HealthStatus::Healthy,
            response_time_ms: None,
            error: None,
            last_check: chrono::Utc::now(),
        }
    }

    /// Creates a healthy component status with response time.
    pub fn healthy_with_time(response_time_ms: u64) -> Self {
        Self {
            status: HealthStatus::Healthy,
            response_time_ms: Some(response_time_ms),
            error: None,
            last_check: chrono::Utc::now(),
        }
    }

    /// Creates a degraded component status.
    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Degraded,
            response_time_ms: None,
            error: Some(message.into()),
            last_check: chrono::Utc::now(),
        }
    }

    /// Creates an unhealthy component status.
    pub fn unhealthy(error: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            response_time_ms: None,
            error: Some(error.into()),
            last_check: chrono::Utc::now(),
        }
    }
}

/// Trait for components that can be health-checked.
#[async_trait]
pub trait HealthCheck: Send + Sync {
    /// Performs a health check on the component.
    async fn check_health(&self) -> ComponentHealth;

    /// Returns the component name.
    fn component_name(&self) -> &str;
}

/// System health checker.
pub struct HealthChecker {
    /// Registered health checks.
    checks: Vec<Arc<dyn HealthCheck>>,
}

impl HealthChecker {
    /// Creates a new health checker.
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
        }
    }

    /// Registers a health check component.
    pub fn register(&mut self, check: Arc<dyn HealthCheck>) {
        self.checks.push(check);
    }

    /// Performs all health checks.
    ///
    /// Returns an overall health status based on all component checks.
    pub async fn check_all(&self) -> HealthCheckResult {
        let mut checks = HashMap::new();
        let mut overall_status = HealthStatus::Healthy;

        // Run all checks in parallel
        let futures: Vec<_> = self.checks.iter().map(|check| {
            async move {
                let name = check.component_name().to_string();
                let result = check.check_health().await;
                (name, result)
            }
        }).collect();

        let results = futures::future::join_all(futures).await;

        for (name, result) in results {
            // Update overall status based on component status
            match result.status {
                HealthStatus::Unhealthy => {
                    overall_status = HealthStatus::Unhealthy;
                }
                HealthStatus::Degraded => {
                    if overall_status != HealthStatus::Unhealthy {
                        overall_status = HealthStatus::Degraded;
                    }
                }
                HealthStatus::Healthy => {}
            }

            checks.insert(name, result);
        }

        HealthCheckResult {
            status: overall_status,
            timestamp: chrono::Utc::now(),
            checks,
            message: None,
        }
    }

    /// Performs a simple liveness check.
    ///
    /// This is a lightweight check that verifies the application is running.
    /// Unlike `check_all()`, it doesn't check dependencies.
    pub fn liveness(&self) -> HealthCheckResult {
        HealthCheckResult {
            status: HealthStatus::Healthy,
            timestamp: chrono::Utc::now(),
            checks: HashMap::new(),
            message: Some("Application is alive".to_string()),
        }
    }

    /// Performs a readiness check.
    ///
    /// Alias for `check_all()` - verifies the application is ready to serve traffic.
    pub async fn readiness(&self) -> HealthCheckResult {
        self.check_all().await
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory usage health check.
pub struct MemoryHealthCheck {
    /// Maximum memory usage threshold (bytes).
    #[allow(dead_code)]  // TODO: Integrate with sysinfo crate for actual memory monitoring
    max_memory_bytes: u64,
}

impl MemoryHealthCheck {
    /// Creates a new memory health check.
    ///
    /// # Arguments
    /// * `max_memory_mb` - Maximum memory usage in megabytes
    pub fn new(max_memory_mb: u64) -> Self {
        Self {
            max_memory_bytes: max_memory_mb * 1024 * 1024,
        }
    }
}

#[async_trait]
impl HealthCheck for MemoryHealthCheck {
    async fn check_health(&self) -> ComponentHealth {
        // Use a simple heuristic: check if we're using more than the threshold
        // In a real implementation, you'd use a crate like `sysinfo` to get actual memory usage

        // For now, return healthy as a placeholder
        // TODO: Integrate with sysinfo crate for actual memory monitoring
        ComponentHealth::healthy()
    }

    fn component_name(&self) -> &str {
        "memory"
    }
}

/// Simple HTTP-based health check.
pub struct HttpHealthCheck {
    name: String,
    url: String,
    timeout_ms: u64,
}

impl HttpHealthCheck {
    /// Creates a new HTTP health check.
    pub fn new(name: impl Into<String>, url: impl Into<String>, timeout_ms: u64) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            timeout_ms,
        }
    }
}

#[async_trait]
impl HealthCheck for HttpHealthCheck {
    async fn check_health(&self) -> ComponentHealth {
        let start = std::time::Instant::now();

        let client = reqwest::Client::new();
        let timeout = std::time::Duration::from_millis(self.timeout_ms);

        match tokio::time::timeout(
            timeout,
            client.get(&self.url).send()
        ).await {
            Ok(Ok(response)) => {
                let elapsed = start.elapsed().as_millis() as u64;

                if response.status().is_success() {
                    ComponentHealth::healthy_with_time(elapsed)
                } else {
                    ComponentHealth::degraded(
                        format!("HTTP {} returned status {}", self.url, response.status())
                    )
                }
            }
            Ok(Err(e)) => {
                ComponentHealth::unhealthy(format!("HTTP request failed: {}", e))
            }
            Err(_) => {
                ComponentHealth::unhealthy(format!("HTTP request timed out after {}ms", self.timeout_ms))
            }
        }
    }

    fn component_name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_health_constructors() {
        let healthy = ComponentHealth::healthy();
        assert_eq!(healthy.status, HealthStatus::Healthy);
        assert!(healthy.error.is_none());

        let degraded = ComponentHealth::degraded("slow response");
        assert_eq!(degraded.status, HealthStatus::Degraded);
        assert!(degraded.error.is_some());

        let unhealthy = ComponentHealth::unhealthy("connection failed");
        assert_eq!(unhealthy.status, HealthStatus::Unhealthy);
        assert!(unhealthy.error.is_some());
    }

    #[tokio::test]
    async fn test_health_checker_liveness() {
        let checker = HealthChecker::new();
        let result = checker.liveness();

        assert_eq!(result.status, HealthStatus::Healthy);
        assert!(result.checks.is_empty());
    }

    #[tokio::test]
    async fn test_health_checker_with_memory_check() {
        let mut checker = HealthChecker::new();
        checker.register(Arc::new(MemoryHealthCheck::new(1024))); // 1GB limit

        let result = checker.check_all().await;

        assert!(result.checks.contains_key("memory"));
        // Memory check should be healthy in test environment
        assert_eq!(result.checks["memory"].status, HealthStatus::Healthy);
    }

    #[test]
    fn test_health_check_result_serialization() {
        let mut checks = HashMap::new();
        checks.insert(
            "test".to_string(),
            ComponentHealth::healthy_with_time(42),
        );

        let result = HealthCheckResult {
            status: HealthStatus::Healthy,
            timestamp: chrono::Utc::now(),
            checks,
            message: Some("All systems operational".to_string()),
        };

        let json = serde_json::to_string_pretty(&result).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("test"));
    }
}
