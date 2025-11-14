// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Disaster recovery test integration.
//!
//! Run all DR tests with: cargo test --test disaster_recovery_tests

mod disaster_recovery;

// Re-export for convenience
pub use disaster_recovery::common::*;

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test that DR metrics can be serialized and deserialized.
    #[test]
    fn test_dr_metrics_serialization() {
        let metrics = DrMetrics::new(
            "test_scenario",
            std::time::Duration::from_secs(300),
            std::time::Duration::from_secs(60),
        );

        // Serialize to JSON
        let json = serde_json::to_string(&metrics).expect("Failed to serialize");

        // Deserialize back
        let deserialized: DrMetrics =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(metrics.scenario, deserialized.scenario);
        assert_eq!(metrics.target_rto, deserialized.target_rto);
        assert_eq!(metrics.target_rpo, deserialized.target_rpo);
    }

    /// Test DR timer functionality.
    #[test]
    fn test_dr_timer() {
        let timer = DrTimer::start("test_operation");

        std::thread::sleep(std::time::Duration::from_millis(100));

        let elapsed = timer.stop();

        assert!(
            elapsed >= std::time::Duration::from_millis(100),
            "Timer should measure at least 100ms"
        );
    }

    /// Test workflow state generation.
    #[test]
    fn test_generate_test_workflows() {
        let workflows = generate_test_workflows(10);

        assert_eq!(workflows.len(), 10);

        for (i, workflow) in workflows.iter().enumerate() {
            assert_eq!(workflow.workflow_id, format!("test-workflow-{}", i));
            assert!(workflow.is_recoverable());
        }
    }

    /// Test health check helper with successful check.
    #[tokio::test]
    async fn test_wait_for_health_success() {
        let health_check = || {
            Box::pin(async { true })
                as std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>>
        };

        let result = wait_for_health(health_check, std::time::Duration::from_secs(5)).await;

        assert!(result, "Health check should succeed immediately");
    }

    /// Test health check helper with timeout.
    #[tokio::test]
    async fn test_wait_for_health_timeout() {
        let health_check = || {
            Box::pin(async { false })
                as std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>>
        };

        let result = wait_for_health(health_check, std::time::Duration::from_millis(100)).await;

        assert!(!result, "Health check should timeout");
    }
}
