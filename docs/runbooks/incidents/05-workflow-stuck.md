# Workflow Stuck

## Overview
Workflows not progressing, stuck in "running" state indefinitely.

## Symptoms
- Workflows in "running" state for > 30 minutes
- No progress in workflow steps
- State not updating in database
- Workflow queue backing up

## Impact Assessment
- Severity: Medium (P2)
- User impact: Workflows not completing
- Business impact: Processing delays

## Step-by-Step Procedure

### Step 1: Identify Stuck Workflows

```bash
# Query for long-running workflows
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT id, name, status, created_at, updated_at,
           now() - updated_at as stuck_duration
    FROM workflows
    WHERE status = 'running'
      AND updated_at < now() - interval '30 minutes'
    ORDER BY updated_at ASC;
  "

# Check workflow executor logs
kubectl logs -n llm-orchestrator -l app=orchestrator | grep -A 5 "workflow_id: <stuck-id>"
```

### Step 2: Diagnose Root Cause

```bash
# Check for deadlocks
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT * FROM pg_locks WHERE NOT granted;
  "

# Check LLM provider issues
kubectl logs -n llm-orchestrator -l app=orchestrator | grep -E "(openai|anthropic|timeout)"

# Check for retry exhaustion
kubectl logs -n llm-orchestrator -l app=orchestrator | grep "max_retries_exceeded"
```

### Step 3: Manual Intervention

```bash
# Option 1: Cancel stuck workflow
curl -X POST https://orchestrator.example.com/api/v1/workflows/<id>/cancel \
  -H "Authorization: Bearer $TOKEN"

# Option 2: Update state in database (emergency)
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    UPDATE workflows
    SET status = 'failed',
        error = 'Manually failed due to stuck state',
        updated_at = now()
    WHERE id = '<workflow-id>';
  "

# Option 3: Restart workflow executor pod
kubectl delete pod -n llm-orchestrator -l app=orchestrator,role=executor
```

### Step 4: Resume Queue Processing

```bash
# Check queue depth
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT status, COUNT(*) FROM workflows GROUP BY status;
  "

# If queue backed up, scale executors
kubectl scale deployment orchestrator -n llm-orchestrator --replicas=6
```

## Validation

```bash
# No stuck workflows
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT COUNT(*) FROM workflows
    WHERE status = 'running' AND updated_at < now() - interval '30 minutes';
  "
# Expected: 0

# Queue processing
# Check workflows completing successfully
```

## Prevention
- Implement workflow timeout mechanisms
- Add heartbeat updates during execution
- Monitor workflow duration metrics
- Circuit breakers for external APIs

## Related Runbooks
- [01-high-latency.md](./01-high-latency.md)
- [../maintenance/01-database-maintenance.md](../maintenance/01-database-maintenance.md)

---
**Last Updated**: 2025-11-14
**Version**: 1.0
