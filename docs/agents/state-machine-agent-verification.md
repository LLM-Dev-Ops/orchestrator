# State Machine Agent - Integration Verification Checklist

## Agent Classification

- **Agent Name**: State Machine Agent
- **Agent ID**: `state-machine-agent`
- **Version**: `1.0.0`
- **Classification**: STATE_MANAGEMENT
- **decision_type**: `state_transition`

## Scope

The State Machine Agent manages workflow state transitions based on:
- Valid state transition rules per entity type
- State machine definitions for workflows and tasks
- Transition validation and enforcement
- State history tracking and replay

## Constitution Compliance Verification

### ✅ Permitted Operations

| Operation | Status | Notes |
|-----------|--------|-------|
| Manage workflow state transitions | ✅ Implemented | `EntityType::Workflow` support |
| Manage task state transitions | ✅ Implemented | `EntityType::Task` support |
| Enforce valid transitions | ✅ Implemented | Via `StateMachineDefinition.transitions` |
| Validate state change requests | ✅ Implemented | `validate()` method |
| Track workflow states | ✅ Implemented | Via `current_states` HashMap |
| Track transition history | ✅ Implemented | Via `state_history` HashMap |
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

- [x] Input schema: `StateTransitionRequest`
- [x] Output schema: `StateTransitionResponse`
- [x] DecisionEvent schema: `DecisionEvent`
- [x] Error types: `StateMachineError`
- [x] Supporting types registered (`StateMachineDefinition`, `TransitionRule`, etc.)

### Platform Registration

- [x] Module registered in `agents/mod.rs`
- [x] Types re-exported for public API
- [x] CLI commands registered

## CLI Commands

### State Machine Commands

```bash
# Transition an entity to a new state
llm-orchestrator state-machine transition \
  --entity-id <UUID> \
  --entity-type workflow|task \
  --to-state <STATE> \
  [--reason <STRING>] \
  [--metadata <JSON>] \
  [--ruvector-endpoint <URL>] \
  [--output-format json|text]

# Validate a state transition without executing
llm-orchestrator state-machine validate \
  --entity-id <UUID> \
  --entity-type workflow|task \
  --to-state <STATE> \
  [--ruvector-endpoint <URL>] \
  [--output-format json|text]

# Inspect current state of an entity
llm-orchestrator state-machine inspect \
  --entity-id <UUID> \
  [--include-history] \
  [--ruvector-endpoint <URL>] \
  [--output-format json|text]

# Get state transition history for an entity
llm-orchestrator state-machine history \
  --entity-id <UUID> \
  [--limit <NUMBER>] \
  [--ruvector-endpoint <URL>] \
  [--output-format json|text]

# Replay a previous state transition
llm-orchestrator state-machine replay \
  --event-file <PATH> \
  [--ruvector-endpoint <URL>] \
  [--output-format json|text]
```

## State Machine Definitions

### Workflow States

| State | Terminal | Valid Transitions To |
|-------|----------|----------------------|
| `pending` | No | `running`, `cancelled` |
| `running` | No | `paused`, `completed`, `failed`, `cancelled` |
| `paused` | No | `running`, `cancelled` |
| `completed` | Yes | - |
| `failed` | Yes | - |
| `cancelled` | Yes | - |

### Task States

| State | Terminal | Valid Transitions To |
|-------|----------|----------------------|
| `pending` | No | `queued`, `cancelled` |
| `queued` | No | `running`, `cancelled` |
| `running` | No | `completed`, `failed`, `cancelled` |
| `completed` | Yes | - |
| `failed` | Yes | - |
| `cancelled` | Yes | - |

## DecisionEvent Schema

The State Machine Agent MUST emit exactly ONE DecisionEvent per invocation:

```json
{
  "agent_id": "state-machine-agent",
  "agent_version": "1.0.0",
  "decision_type": "state_transition",
  "inputs_hash": "<SHA256_HASH>",
  "outputs": {
    "request_id": "<UUID>",
    "entity_id": "<UUID>",
    "entity_type": "workflow|task",
    "from_state": "<STATE>",
    "to_state": "<STATE>",
    "status": "completed|invalid|rejected|error",
    "rules_checked": <NUMBER>,
    "rules_passed": <NUMBER>,
    "transition_timestamp": "<RFC3339>"
  },
  "confidence": <FLOAT 0.0-1.0>,
  "constraints_applied": ["<CONSTRAINT>", ...],
  "execution_ref": "<entity_type>:<entity_id>",
  "timestamp": "<RFC3339 UTC>"
}
```

## Confidence Calculation

Confidence score (0.0 - 1.0) represents transition certainty:

| Factor | Weight | Description |
|--------|--------|-------------|
| Transition Validity | 40% | 1.0 if valid, 0.0 if invalid |
| Rule Compliance | 30% | Ratio of passed/checked rules |
| State Determinism | 15% | 1.0 (deterministic transitions) |
| History Consistency | 15% | Based on state history validation |

### Base Confidence Scores

| Scenario | Base Score |
|----------|------------|
| Valid transition, all rules pass | 0.95 |
| Valid transition, some rules warned | 0.80 |
| Invalid transition | 0.0 |
| Unknown current state | 0.50 |

## Smoke Test Commands

```bash
# Test 1: Transition workflow from pending to running
llm-orchestrator state-machine transition \
  --entity-id "00000000-0000-0000-0000-000000000001" \
  --entity-type workflow \
  --to-state running \
  --reason "Starting workflow execution" \
  --output-format json

# Test 2: Validate a transition without executing
llm-orchestrator state-machine validate \
  --entity-id "00000000-0000-0000-0000-000000000001" \
  --entity-type workflow \
  --to-state completed \
  --output-format json

# Test 3: Inspect current state
llm-orchestrator state-machine inspect \
  --entity-id "00000000-0000-0000-0000-000000000001" \
  --include-history \
  --output-format json

# Test 4: Get transition history
llm-orchestrator state-machine history \
  --entity-id "00000000-0000-0000-0000-000000000001" \
  --limit 10 \
  --output-format json

# Test 5: Transition task through states
llm-orchestrator state-machine transition \
  --entity-id "00000000-0000-0000-0000-000000000002" \
  --entity-type task \
  --to-state queued \
  --output-format json

# Test 6: Invalid transition (should fail)
llm-orchestrator state-machine transition \
  --entity-id "00000000-0000-0000-0000-000000000001" \
  --entity-type workflow \
  --to-state pending \
  --reason "Attempt invalid reverse transition" \
  --output-format json
```

## Transition Rules

### Rule Types

| Type | Description |
|------|-------------|
| `require_reason` | Transition requires a reason field |
| `require_metadata` | Transition requires metadata |
| `time_constraint` | Minimum time in current state |
| `custom` | Custom validation rule |

### Default Rules Applied

- Terminal states cannot be transitioned from
- Transitions must follow valid paths defined in state machine
- Optional: reasons may be required for certain transitions

## Integration Points

### Invocable By

| System | Purpose |
|--------|---------|
| Workflow Orchestrator Agent | Workflow state management |
| Task Scheduler Agent | Task state transitions |
| LLM-Incident-Manager | Remediation state tracking |

### Invokes

| System | Purpose |
|--------|---------|
| ruvector-service | DecisionEvent persistence |
| Observatory Adapter | Telemetry and observability |

### Does NOT Invoke Directly

| System | Reason |
|--------|--------|
| Other Orchestrator agents | State-driven only, no direct invocation |
| Google SQL | Persistence via ruvector-service only |
| External security services | Security is Shield's responsibility |

## Unit Test Coverage

All tests are in `state_machine_agent.rs`:

- [x] `test_adapter_disabled_by_default`
- [x] `test_adapter_enabled_with_endpoint`
- [x] `test_default_workflow_state_machine`
- [x] `test_default_task_state_machine`
- [x] `test_valid_workflow_transitions`
- [x] `test_invalid_workflow_transition`
- [x] `test_terminal_state_rejection`
- [x] `test_transition_with_rules`
- [x] `test_validate_without_execute`
- [x] `test_state_history_tracking`
- [x] `test_decision_event_creation`
- [x] `test_confidence_calculation`
- [x] `test_replay_transition`

## Deployment

### Google Cloud Edge Function

The State Machine Agent deploys as part of the unified LLM-Orchestrator service as a Google Cloud Edge Function:

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
| `TransitionStatus::Invalid` | Fix request parameters | Transition not in valid paths |
| `TransitionStatus::Rejected` | Check transition rules | Rules blocked the transition |
| `TransitionStatus::Error` | Retry or check ruvector-service | Transient - retryable |
| Agent disabled | Enable agent with endpoint | Check configuration |

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2025-01-20 | Initial implementation |

---

**COMPLIANCE STATEMENT**: This agent implementation complies with the LLM-Orchestrator Agent Infrastructure Constitution (Prompt 0). Failure to maintain compliance is a HARD ERROR.
