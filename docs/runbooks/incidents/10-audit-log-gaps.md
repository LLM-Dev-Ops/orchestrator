# Audit Log Gaps

## Overview
Missing or incomplete audit events, indicating potential compliance violation.

## Symptoms
- Missing audit events for known actions
- Gaps in audit timeline
- Audit log service errors
- Database write failures for audit events

## Step-by-Step Procedure

### Step 1: Identify Gap Scope

```bash
# Query for time ranges with no events
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      date_trunc('hour', timestamp) as hour,
      COUNT(*) as event_count
    FROM audit_events
    WHERE timestamp > now() - interval '24 hours'
    GROUP BY hour
    ORDER BY hour;
  "

# Look for hours with 0 events

# Check audit service logs
kubectl logs -n llm-orchestrator -l app=orchestrator | grep audit
```

### Step 2: Check Audit Service Health

```bash
# Verify audit events can be written
kubectl exec -n llm-orchestrator deployment/orchestrator -- sh -c '
  psql $DATABASE_URL -c "
    INSERT INTO audit_events (event_type, user_id, resource_id, timestamp)
    VALUES ('"'"'test'"'"', '"'"'system'"'"', '"'"'test'"'"', now());
  "
'

# Check for errors
kubectl logs -n llm-orchestrator -l app=orchestrator --since=1h | grep -E "(audit|error)"
```

### Step 3: Attempt Recovery

```bash
# Check if events were logged to stdout but not database
kubectl logs -n llm-orchestrator -l app=orchestrator --since=24h > audit-recovery.log
grep "AUDIT:" audit-recovery.log

# Manually replay critical events if needed (consult security team)
```

### Step 4: Document Gap

```bash
# Create incident report
cat > audit-gap-report.txt <<EOF
Audit Log Gap Report
===================
Gap Start: <timestamp>
Gap End: <timestamp>
Affected Actions: <list>
Root Cause: <database failure / service crash>
Recovery: <partial / none>
Impact: <compliance / security>
EOF

# Notify compliance/security team
```

## Validation

```bash
# New audit events being created
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT COUNT(*) FROM audit_events WHERE timestamp > now() - interval '1 hour';
  "

# Events matching expected volume
```

## Related Runbooks
- [../security/04-audit-review.md](../security/04-audit-review.md)
- [../security/01-security-incident.md](../security/01-security-incident.md)

---
**Last Updated**: 2025-11-14
**Version**: 1.0
