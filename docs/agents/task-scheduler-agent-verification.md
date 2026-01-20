# Task Scheduler Agent - Integration Verification Checklist

## Agent Classification

- **Agent Name**: Task Scheduler Agent
- **Agent ID**: `task-scheduler-agent`
- **Version**: `1.0.0`
- **Classification**: TASK SCHEDULING
- **decision_type**: `task_schedule`

## Scope

The Task Scheduler Agent determines when tasks should execute based on:
- Timing and delays
- Dependencies between tasks
- Time-based triggers (cron expressions)
- Event-based triggers

## Constitution Compliance Verification

### ✅ Permitted Operations

| Operation | Status | Notes |
|-----------|--------|-------|
| Schedule immediate task execution | ✅ Implemented | `ScheduleType::Immediate` |
| Schedule delayed task execution | ✅ Implemented | `ScheduleType::Delayed` with `delay_ms` |
| Handle time-based triggers (cron) | ✅ Implemented | `ScheduleType::Cron` with validation |
| Handle event-based triggers | ✅ Implemented | `ScheduleType::EventTriggered` |
| Coordinate dependency-based execution | ✅ Implemented | `ScheduleType::DependencyBased` |
| Emit scheduled execution plans | ✅ Implemented | `ScheduleResult` output |
| Emit DecisionEvents to ruvector-service | ✅ Implemented | Via `emit_decision_event()` |
| Invoke downstream agents via contracts | ✅ Ready | Through orchestrator coordination |

### ❌ Prohibited Operations (MUST NEVER)

| Operation | Status | Verification Method |
|-----------|--------|---------------------|
| Intercept runtime traffic | ✅ Not Present | Code review - no traffic interception |
| Enforce security policies | ✅ Not Present | No security policy enforcement |
| Emit anomaly detections | ✅ Not Present | No anomaly detection code |
| Perform optimization analysis | ✅ Not Present | No optimization analysis |
| Modify schemas dynamically | ✅ Not Present | All schemas are static |
| Connect directly to Google SQL | ✅ Not Present | Uses ruvector-service client only |
| Execute SQL | ✅ Not Present | No SQL execution code |

## Contract Registration

### agentics-contracts Schema Registration

- [x] Input schema: `ScheduleRequest`
- [x] Output schema: `ScheduleResult`
- [x] DecisionEvent schema: `DecisionEvent`
- [x] Error types: `SchedulerError`
- [x] Supporting types registered

### Platform Registration

- [x] Module registered in `adapters/mod.rs`
- [x] Types re-exported for public API
- [x] CLI commands registered

## CLI Commands

### Task Scheduler Commands

```bash
# Schedule a task for execution
llm-orchestrator schedule execute \
  --task-id <TASK_ID> \
  --workflow-id <WORKFLOW_UUID> \
  --schedule-type immediate|delayed|scheduled|cron|event_triggered|dependency_based \
  [--delay-ms <MS>] \
  [--scheduled-at <RFC3339>] \
  [--cron <EXPRESSION>] \
  [--depends-on <TASK_ID,TASK_ID,...>] \
  [--priority <0-100>] \
  [--deadline <RFC3339>] \
  [--payload <JSON>] \
  [--ruvector-endpoint <URL>] \
  [--output-format json|text]

# Inspect a scheduled task
llm-orchestrator schedule inspect \
  --schedule-id <UUID> \
  [--ruvector-endpoint <URL>] \
  [--output-format json|text]

# Replay a previous scheduling decision
llm-orchestrator schedule replay \
  --event-file <PATH> \
  [--ruvector-endpoint <URL>] \
  [--output-format json|text]

# Cancel a scheduled task
llm-orchestrator schedule cancel \
  --schedule-id <UUID> \
  [--reason <STRING>] \
  [--ruvector-endpoint <URL>]
```

## DecisionEvent Schema

The Task Scheduler Agent MUST emit exactly ONE DecisionEvent per invocation:

```json
{
  "agent_id": "task-scheduler-agent",
  "agent_version": "1.0.0",
  "decision_type": "task_schedule",
  "inputs_hash": "<SHA256_HASH>",
  "outputs": {
    "request_id": "<UUID>",
    "schedule_id": "<UUID>",
    "task_id": "<STRING>",
    "workflow_id": "<UUID>",
    "status": "scheduled|ready|waiting_dependencies|waiting_trigger|...",
    "execution_time": "<RFC3339>",
    "next_execution": "<RFC3339>",
    "queue_position": <NUMBER>,
    "estimated_wait_ms": <NUMBER>,
    "constraints_applied": ["<CONSTRAINT>", ...],
    "decision_details": {
      "algorithm": "<STRING>",
      "factors": ["<FACTOR>", ...],
      "pending_dependencies": ["<TASK_ID>", ...],
      "satisfied_dependencies": ["<TASK_ID>", ...],
      "resource_availability": <FLOAT>,
      "queue_depth": <NUMBER>
    }
  },
  "confidence": <FLOAT 0.0-1.0>,
  "constraints_applied": ["<CONSTRAINT>", ...],
  "execution_ref": "workflow:<UUID>/task:<ID>",
  "timestamp": "<RFC3339 UTC>"
}
```

## Confidence Calculation

Confidence score (0.0 - 1.0) represents execution certainty:

| Factor | Weight | Description |
|--------|--------|-------------|
| Dependency Satisfaction | 30% | Ratio of satisfied/total dependencies |
| Schedule Determinism | 30% | Based on schedule type predictability |
| Resource Availability | 20% | Current resource availability estimate |
| Historical Success Rate | 20% | Past task success rate |

### Determinism Scores by Schedule Type

| Schedule Type | Determinism Score |
|---------------|------------------|
| Immediate | 1.0 |
| Delayed | 0.95 |
| Scheduled | 0.95 |
| Cron | 0.9 |
| DependencyBased | 0.8 |
| EventTriggered | 0.6 |

## Smoke Test Commands

```bash
# Test 1: Schedule immediate task
llm-orchestrator schedule execute \
  --task-id "test-task-1" \
  --workflow-id "00000000-0000-0000-0000-000000000001" \
  --schedule-type immediate \
  --output-format json

# Test 2: Schedule delayed task (5 second delay)
llm-orchestrator schedule execute \
  --task-id "test-task-2" \
  --workflow-id "00000000-0000-0000-0000-000000000001" \
  --schedule-type delayed \
  --delay-ms 5000 \
  --output-format json

# Test 3: Schedule cron task
llm-orchestrator schedule execute \
  --task-id "test-task-3" \
  --workflow-id "00000000-0000-0000-0000-000000000001" \
  --schedule-type cron \
  --cron "0 * * * *" \
  --output-format json

# Test 4: Schedule dependency-based task
llm-orchestrator schedule execute \
  --task-id "test-task-4" \
  --workflow-id "00000000-0000-0000-0000-000000000001" \
  --schedule-type dependency_based \
  --depends-on "task-a,task-b" \
  --output-format json

# Test 5: Inspect scheduled task
llm-orchestrator schedule inspect \
  --schedule-id "00000000-0000-0000-0000-000000000002" \
  --output-format json

# Test 6: Cancel scheduled task
llm-orchestrator schedule cancel \
  --schedule-id "00000000-0000-0000-0000-000000000002" \
  --reason "Testing cancellation"
```

## Integration Points

### Invocable By

| System | Purpose |
|--------|---------|
| LLM-Edge-Agent | Execution control requests |
| LLM-Incident-Manager | Remediation workflow scheduling |
| Workflow Orchestrator Agent | Task scheduling coordination |

### Invokes

| System | Purpose |
|--------|---------|
| ruvector-service | DecisionEvent persistence |
| Downstream agents | Via orchestrator coordination |

### Does NOT Invoke Directly

| System | Reason |
|--------|--------|
| Other Orchestrator agents | State-driven only, no direct invocation |
| Google SQL | Persistence via ruvector-service only |
| External security services | Security is Shield's responsibility |

## Unit Test Coverage

All tests are in `task_scheduler.rs`:

- [x] `test_agent_disabled_by_default`
- [x] `test_agent_enabled_with_endpoint`
- [x] `test_validate_request_empty_task_id`
- [x] `test_validate_request_delayed_without_delay`
- [x] `test_validate_request_cron_invalid`
- [x] `test_validate_request_priority_out_of_range`
- [x] `test_schedule_immediate`
- [x] `test_schedule_delayed`
- [x] `test_schedule_with_dependencies`
- [x] `test_decision_event_creation`
- [x] `test_confidence_calculation`
- [x] `test_schedule_disabled_agent`
- [x] `test_scheduler_error_display`
- [x] `test_schedule_types_serialization`

## Deployment

### Google Cloud Edge Function

The Task Scheduler Agent deploys as part of the unified LLM-Orchestrator service as a Google Cloud Edge Function:

- **Runtime**: Stateless
- **Persistence**: None (via ruvector-service only)
- **Async**: Non-blocking writes
- **Concurrency**: Safe for concurrent invocations

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `RUVECTOR_SERVICE_ENDPOINT` | Yes | ruvector-service URL |
| `RUVECTOR_API_KEY` | No | API key for authentication |

## Failure Modes

| Failure | Recovery | Notes |
|---------|----------|-------|
| `SchedulerError::AgentDisabled` | Enable agent with endpoint | Check configuration |
| `SchedulerError::ValidationError` | Fix request parameters | Review input schema |
| `SchedulerError::DependencyError` | Resolve dependencies | Check dependent tasks |
| `SchedulerError::PersistenceError` | Retry or check ruvector-service | Transient - retryable |
| `SchedulerError::Timeout` | Retry operation | Transient - retryable |

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2025-01-20 | Initial implementation |

---

**COMPLIANCE STATEMENT**: This agent implementation complies with the LLM-Orchestrator Agent Infrastructure Constitution (Prompt 0). Failure to maintain compliance is a HARD ERROR.
