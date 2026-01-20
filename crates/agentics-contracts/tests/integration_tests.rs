// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for agentics-contracts crate.
//!
//! These tests verify the contract schemas work correctly together
//! and can be serialized/deserialized as expected.

use agentics_contracts::*;
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashMap;

// =============================================================================
// Agent Schema Tests
// =============================================================================

#[test]
fn test_workflow_orchestrator_agent_metadata() {
    let metadata = AgentMetadata::workflow_orchestrator();

    assert!(metadata.id.0.starts_with("workflow-orchestrator-"));
    assert_eq!(metadata.version.major, 0);
    assert_eq!(metadata.version.minor, 1);
    assert_eq!(metadata.version.patch, 0);
    assert_eq!(metadata.agent_type, AgentType::WorkflowOrchestrator);
    assert_eq!(metadata.classification, AgentClassification::WorkflowExecution);
    assert_eq!(metadata.cli_endpoint, "agent");

    // Verify capabilities
    assert!(metadata.capabilities.execute_workflows);
    assert!(metadata.capabilities.invoke_downstream_agents);
    assert!(metadata.capabilities.manage_task_dependencies);
    assert!(metadata.capabilities.trigger_retries);
    assert!(metadata.capabilities.coordinate_parallel_execution);
    assert!(metadata.capabilities.transition_workflow_states);

    // Verify allowed invokers
    assert!(metadata.allowed_invokers.contains("llm-edge-agent"));
    assert!(metadata.allowed_invokers.contains("llm-incident-manager"));
    assert!(metadata.allowed_invokers.contains("governance-systems"));

    // Verify non-responsibilities are defined
    assert!(!metadata.non_responsibilities.is_empty());
}

#[test]
fn test_recovery_manager_agent_metadata() {
    let metadata = AgentMetadata::recovery_manager();

    assert!(metadata.id.0.starts_with("retry-recovery-"));
    assert_eq!(metadata.agent_type, AgentType::RecoveryManager);
    assert_eq!(metadata.classification, AgentClassification::TaskCoordination);
    assert_eq!(metadata.cli_endpoint, "recovery");

    // Verify capabilities (recovery manager has limited capabilities)
    assert!(!metadata.capabilities.execute_workflows);
    assert!(!metadata.capabilities.invoke_downstream_agents);
    assert!(metadata.capabilities.manage_task_dependencies);
    assert!(metadata.capabilities.trigger_retries);
    assert!(!metadata.capabilities.coordinate_parallel_execution);
    assert!(metadata.capabilities.transition_workflow_states);
}

#[test]
fn test_agent_version_serialization() {
    let version = AgentVersion::new(1, 2, 3);
    let json = serde_json::to_string(&version).unwrap();
    let deserialized: AgentVersion = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.major, 1);
    assert_eq!(deserialized.minor, 2);
    assert_eq!(deserialized.patch, 3);
}

#[test]
fn test_agent_version_with_prerelease() {
    let version = AgentVersion::parse("1.0.0-alpha").unwrap();
    assert_eq!(version.prerelease, Some("alpha".to_string()));
    assert_eq!(version.to_string(), "1.0.0-alpha");
}

// =============================================================================
// Decision Event Tests
// =============================================================================

#[test]
fn test_decision_event_serialization() {
    let event = DecisionEvent {
        agent_id: AgentId::from_string("workflow-orchestrator-test"),
        agent_version: AgentVersion::new(0, 1, 0),
        decision_type: DecisionType::Execute,
        inputs_hash: "abc123".to_string(),
        outputs: DecisionOutputs {
            workflow_id: Some(Uuid::new_v4()),
            task_id: Some("task-1".to_string()),
            state_transition: Some("pending -> running".to_string()),
            result_summary: Some("Workflow started".to_string()),
            artifacts: vec![],
        },
        confidence: 0.95,
        constraints_applied: ConstraintsApplied {
            workflow_constraints: vec!["max_parallel_tasks: 5".to_string()],
            dependency_constraints: vec![],
            retry_constraints: vec![],
            resource_constraints: vec![],
        },
        execution_ref: ExecutionRef {
            execution_id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            task_id: Some("task-1".to_string()),
            trace_id: Some("trace-123".to_string()),
            span_id: Some("span-456".to_string()),
        },
        timestamp: Utc::now(),
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&event).unwrap();
    assert!(json.contains("workflow-orchestrator-test"));
    assert!(json.contains("execute"));

    // Deserialize back
    let deserialized: DecisionEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.agent_id.0, "workflow-orchestrator-test");
    assert_eq!(deserialized.decision_type, DecisionType::Execute);
    assert_eq!(deserialized.confidence, 0.95);
}

#[test]
fn test_decision_types() {
    let types = vec![
        DecisionType::Execute,
        DecisionType::Transition,
        DecisionType::Schedule,
        DecisionType::Retry,
        DecisionType::Skip,
        DecisionType::Cancel,
        DecisionType::Complete,
        DecisionType::Fail,
    ];

    for dt in types {
        let json = serde_json::to_string(&dt).unwrap();
        let deserialized: DecisionType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, dt);
    }
}

// =============================================================================
// Workflow Envelope Tests
// =============================================================================

#[test]
fn test_workflow_envelope_serialization() {
    let envelope = WorkflowEnvelope {
        workflow_id: Uuid::new_v4(),
        workflow_name: "test-workflow".to_string(),
        version: "1.0.0".to_string(),
        state: WorkflowState::Pending,
        tasks: vec![
            TaskEnvelope {
                task_id: "task-1".to_string(),
                task_name: "First Task".to_string(),
                task_type: "llm_call".to_string(),
                state: TaskState::Pending,
                dependencies: vec![],
                input: serde_json::json!({"prompt": "Hello"}),
                output: None,
                error: None,
                retry_count: 0,
                max_retries: 3,
                created_at: Utc::now(),
                started_at: None,
                completed_at: None,
            },
        ],
        config: WorkflowExecutionConfig {
            max_parallel_tasks: 5,
            timeout_ms: 300000,
            retry_config: RetryConfiguration {
                max_retries: 3,
                initial_delay_ms: 1000,
                max_delay_ms: 30000,
                backoff_multiplier: 2.0,
            },
            strategy: ExecutionStrategy::Parallel,
        },
        metadata: HashMap::new(),
        created_at: Utc::now(),
        started_at: None,
        completed_at: None,
    };

    let json = serde_json::to_string_pretty(&envelope).unwrap();
    assert!(json.contains("test-workflow"));
    assert!(json.contains("task-1"));

    let deserialized: WorkflowEnvelope = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.workflow_name, "test-workflow");
    assert_eq!(deserialized.tasks.len(), 1);
}

#[test]
fn test_workflow_states() {
    let states = vec![
        WorkflowState::Pending,
        WorkflowState::Running,
        WorkflowState::Paused,
        WorkflowState::Completed,
        WorkflowState::Failed,
        WorkflowState::Cancelled,
    ];

    for state in states {
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: WorkflowState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, state);
    }
}

#[test]
fn test_task_states() {
    let states = vec![
        TaskState::Pending,
        TaskState::Running,
        TaskState::Completed,
        TaskState::Failed,
        TaskState::Skipped,
        TaskState::Cancelled,
    ];

    for state in states {
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: TaskState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, state);
    }
}

// =============================================================================
// Dependency Resolution Tests
// =============================================================================

#[test]
fn test_dependency_resolution_request() {
    let request = DependencyResolutionRequest {
        request_id: Uuid::new_v4(),
        workflow_id: Uuid::new_v4(),
        tasks: vec![
            TaskDependencyNode {
                task_id: "task-1".to_string(),
                name: "First Task".to_string(),
                task_type: Some("compute".to_string()),
                dependencies: vec![],
                estimated_duration_ms: Some(5000),
                priority: 10,
                metadata: HashMap::new(),
            },
            TaskDependencyNode {
                task_id: "task-2".to_string(),
                name: "Second Task".to_string(),
                task_type: Some("compute".to_string()),
                dependencies: vec![
                    DependencyConstraint {
                        task_id: "task-1".to_string(),
                        required_status: DependencyStatus::Success,
                        timeout_ms: Some(60000),
                    },
                ],
                estimated_duration_ms: Some(3000),
                priority: 5,
                metadata: HashMap::new(),
            },
        ],
        config: DependencyResolutionConfig::default(),
        timestamp: Utc::now(),
    };

    let json = serde_json::to_string_pretty(&request).unwrap();
    assert!(json.contains("task-1"));
    assert!(json.contains("task-2"));

    let deserialized: DependencyResolutionRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.tasks.len(), 2);
    assert_eq!(deserialized.tasks[1].dependencies.len(), 1);
}

#[test]
fn test_dependency_resolution_result() {
    let result = DependencyResolutionResult {
        request_id: Uuid::new_v4(),
        workflow_id: Uuid::new_v4(),
        status: ResolutionStatus::Success,
        execution_order: vec!["task-1".to_string(), "task-2".to_string(), "task-3".to_string()],
        parallel_groups: vec![
            ParallelExecutionGroup {
                group_index: 0,
                task_ids: vec!["task-1".to_string()],
                depends_on_groups: vec![],
            },
            ParallelExecutionGroup {
                group_index: 1,
                task_ids: vec!["task-2".to_string(), "task-3".to_string()],
                depends_on_groups: vec![0],
            },
        ],
        graph_summary: DependencyGraphSummary {
            total_tasks: 3,
            total_dependencies: 2,
            max_depth: 2,
            root_tasks: 1,
            leaf_tasks: 2,
            parallel_groups: 2,
            has_cycles: false,
        },
        critical_path: Some(CriticalPath {
            task_ids: vec!["task-1".to_string(), "task-2".to_string()],
            total_duration_ms: 8000,
            task_count: 2,
        }),
        issues: vec![],
        resolved_at: Utc::now(),
    };

    let json = serde_json::to_string_pretty(&result).unwrap();

    let deserialized: DependencyResolutionResult = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.status, ResolutionStatus::Success);
    assert_eq!(deserialized.parallel_groups.len(), 2);
    assert!(deserialized.critical_path.is_some());
}

#[test]
fn test_dependency_issues() {
    let issue = DependencyIssue {
        severity: IssueSeverity::Error,
        issue_type: IssueType::CyclicDependency,
        affected_tasks: vec!["task-1".to_string(), "task-2".to_string()],
        description: "Circular dependency detected between task-1 and task-2".to_string(),
        suggestion: Some("Remove one of the dependencies to break the cycle".to_string()),
    };

    let json = serde_json::to_string(&issue).unwrap();
    let deserialized: DependencyIssue = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.severity, IssueSeverity::Error);
    assert_eq!(deserialized.issue_type, IssueType::CyclicDependency);
}

#[test]
fn test_resolution_statuses() {
    let statuses = vec![
        ResolutionStatus::Success,
        ResolutionStatus::SuccessWithWarnings,
        ResolutionStatus::Failed,
        ResolutionStatus::Timeout,
        ResolutionStatus::Cancelled,
    ];

    for status in statuses {
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: ResolutionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, status);
    }
}

#[test]
fn test_dependency_resolution_decision_event() {
    let event = DependencyResolutionDecisionEvent {
        agent_id: AgentId::from_string(DEPENDENCY_RESOLVER_AGENT_ID),
        agent_version: AgentVersion::parse(DEPENDENCY_RESOLVER_AGENT_VERSION).unwrap(),
        decision_type: DependencyDecisionType::Resolve,
        inputs_hash: "sha256:abc123".to_string(),
        outputs: DependencyResolutionOutputs {
            status: ResolutionStatus::Success,
            tasks_resolved: 5,
            parallel_groups: 3,
            issues_count: 0,
            critical_path_calculated: true,
        },
        confidence: 1.0,
        constraints_applied: DependencyConstraintsApplied {
            max_depth: 100,
            cycle_detection: true,
            parallelism_optimization: true,
            timeout_ms: None,
        },
        execution_ref: DependencyExecutionRef {
            execution_id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            trace_id: Some("trace-123".to_string()),
            span_id: None,
        },
        timestamp: Utc::now(),
    };

    let json = serde_json::to_string_pretty(&event).unwrap();
    assert!(json.contains(DEPENDENCY_RESOLVER_AGENT_ID));

    let deserialized: DependencyResolutionDecisionEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.confidence, 1.0);
    assert_eq!(deserialized.outputs.tasks_resolved, 5);
}

// =============================================================================
// Error Tests
// =============================================================================

#[test]
fn test_contract_error() {
    let err = ContractError::Validation("Invalid input".to_string());
    assert!(err.to_string().contains("Invalid input"));

    let err = ContractError::Serialization("JSON error".to_string());
    assert!(err.to_string().contains("JSON error"));
}

// =============================================================================
// Cross-Type Integration Tests
// =============================================================================

#[test]
fn test_workflow_with_dependencies() {
    // Create a workflow envelope with tasks that have dependencies
    let workflow_id = Uuid::new_v4();

    let envelope = WorkflowEnvelope {
        workflow_id,
        workflow_name: "dependency-test-workflow".to_string(),
        version: "1.0.0".to_string(),
        state: WorkflowState::Pending,
        tasks: vec![
            TaskEnvelope {
                task_id: "prepare".to_string(),
                task_name: "Prepare Data".to_string(),
                task_type: "data_prep".to_string(),
                state: TaskState::Pending,
                dependencies: vec![],
                input: serde_json::json!({}),
                output: None,
                error: None,
                retry_count: 0,
                max_retries: 3,
                created_at: Utc::now(),
                started_at: None,
                completed_at: None,
            },
            TaskEnvelope {
                task_id: "process".to_string(),
                task_name: "Process Data".to_string(),
                task_type: "processing".to_string(),
                state: TaskState::Pending,
                dependencies: vec!["prepare".to_string()],
                input: serde_json::json!({}),
                output: None,
                error: None,
                retry_count: 0,
                max_retries: 3,
                created_at: Utc::now(),
                started_at: None,
                completed_at: None,
            },
            TaskEnvelope {
                task_id: "finalize".to_string(),
                task_name: "Finalize Results".to_string(),
                task_type: "finalization".to_string(),
                state: TaskState::Pending,
                dependencies: vec!["process".to_string()],
                input: serde_json::json!({}),
                output: None,
                error: None,
                retry_count: 0,
                max_retries: 3,
                created_at: Utc::now(),
                started_at: None,
                completed_at: None,
            },
        ],
        config: WorkflowExecutionConfig::default(),
        metadata: HashMap::new(),
        created_at: Utc::now(),
        started_at: None,
        completed_at: None,
    };

    // Create dependency resolution request from workflow
    let dep_request = DependencyResolutionRequest {
        request_id: Uuid::new_v4(),
        workflow_id,
        tasks: envelope.tasks.iter().map(|t| {
            TaskDependencyNode {
                task_id: t.task_id.clone(),
                name: t.task_name.clone(),
                task_type: Some(t.task_type.clone()),
                dependencies: t.dependencies.iter().map(|dep_id| {
                    DependencyConstraint {
                        task_id: dep_id.clone(),
                        required_status: DependencyStatus::Success,
                        timeout_ms: None,
                    }
                }).collect(),
                estimated_duration_ms: Some(5000),
                priority: 0,
                metadata: HashMap::new(),
            }
        }).collect(),
        config: DependencyResolutionConfig::default(),
        timestamp: Utc::now(),
    };

    assert_eq!(dep_request.tasks.len(), 3);
    assert!(dep_request.tasks[0].dependencies.is_empty());
    assert_eq!(dep_request.tasks[1].dependencies.len(), 1);
    assert_eq!(dep_request.tasks[2].dependencies.len(), 1);
}

#[test]
fn test_constants() {
    assert_eq!(DEPENDENCY_RESOLVER_AGENT_ID, "dependency-resolver");
    assert_eq!(DEPENDENCY_RESOLVER_AGENT_VERSION, "0.1.0");

    // Verify version can be parsed
    let version = AgentVersion::parse(DEPENDENCY_RESOLVER_AGENT_VERSION).unwrap();
    assert_eq!(version.major, 0);
    assert_eq!(version.minor, 1);
    assert_eq!(version.patch, 0);
}
