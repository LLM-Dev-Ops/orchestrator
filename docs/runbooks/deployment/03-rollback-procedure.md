# Rollback Procedure

## Overview
Emergency rollback procedure for failed deployments. This runbook covers reverting to a previous working version when a deployment causes issues in production.

## Prerequisites
- Required access/permissions:
  - Kubernetes namespace admin
  - Database admin (for schema rollbacks)
  - Incident commander authority
- Tools needed:
  - `kubectl` v1.24+
  - `psql` for database operations
  - Access to backup files
- Knowledge required:
  - Deployment history
  - Database schema management
  - Kubernetes rollback mechanisms

## Symptoms
- High error rate (> 5%)
- Application crashes or pod crash loops
- Database connection failures
- User-reported issues after deployment
- Failed health checks
- Memory leaks or resource exhaustion

## Impact Assessment
- Severity: Critical (production incident)
- User impact: High (service degradation)
- Business impact: Revenue/reputation at risk

## Step-by-Step Procedure

### Step 1: Declare Incident and Assess Scope

```bash
# 1. Check deployment status
kubectl get deployment orchestrator -n llm-orchestrator

# 2. Check pod status
kubectl get pods -n llm-orchestrator -l app=orchestrator

# 3. Quick health check
curl -s https://orchestrator.example.com/health

# 4. Check error rate (Prometheus)
# Query: rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m]) * 100

# 5. Declare incident in Slack
# Message template:
# "INCIDENT: LLM Orchestrator deployment rollback required
# Symptoms: [High error rate / Crashes / etc]
# Impact: [User-facing / Internal only]
# Initiated by: [Your name]
# Time: [Current time]"
```

### Step 2: Pause Current Rollout (if in progress)

```bash
# If deployment is still rolling out, pause it immediately
kubectl rollout pause deployment/orchestrator -n llm-orchestrator

# Verify paused
kubectl rollout status deployment/orchestrator -n llm-orchestrator

# Expected: "deployment is paused"
```

### Step 3: Identify Target Rollback Version

```bash
# Check deployment history
kubectl rollout history deployment/orchestrator -n llm-orchestrator

# Example output:
# REVISION  CHANGE-CAUSE
# 1         Initial deployment (v0.1.0)
# 2         Update to v0.1.1
# 3         Update to v0.2.0 (current, failing)

# Check which revision was last stable
# Usually: current revision - 1

# View specific revision details
kubectl rollout history deployment/orchestrator -n llm-orchestrator --revision=2

# Expected: Shows image version and configuration
```

### Step 4: Execute Application Rollback

```bash
# Option A: Rollback to previous revision (most common)
kubectl rollout undo deployment/orchestrator -n llm-orchestrator

# Option B: Rollback to specific revision
kubectl rollout undo deployment/orchestrator -n llm-orchestrator --to-revision=2

# Option C: Apply backed-up manifest
kubectl apply -f deployment-backup-20251114-100000.yaml

# Watch rollback progress
kubectl rollout status deployment/orchestrator -n llm-orchestrator --watch

# Monitor pods
watch kubectl get pods -n llm-orchestrator -l app=orchestrator
```

### Step 5: Rollback Database Schema (if migrations were run)

```bash
# ONLY if new version included database migrations

# 1. Check if backup exists
ls -lh backup-*.sql.gz

# 2. Stop application traffic to database (scale to 0)
kubectl scale deployment orchestrator -n llm-orchestrator --replicas=0

# Wait for all pods to terminate
kubectl wait --for=delete pod -l app=orchestrator -n llm-orchestrator --timeout=60s

# 3. Restore database from backup
gunzip < backup-20251114-095959.sql.gz | \
  kubectl exec -i -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator orchestrator

# 4. Verify restoration
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1;"

# Expected: Previous schema version

# 5. Scale application back up
kubectl scale deployment orchestrator -n llm-orchestrator --replicas=3

# Wait for pods to be ready
kubectl wait --for=condition=ready pod -l app=orchestrator -n llm-orchestrator --timeout=300s
```

### Step 6: Rollback Configuration Changes

```bash
# If ConfigMap was updated, restore previous version

# Option A: Apply backed-up ConfigMap
kubectl apply -f configmap-backup-20251114-095959.yaml

# Option B: Manually edit ConfigMap
kubectl edit configmap orchestrator-config -n llm-orchestrator

# After ConfigMap change, restart pods to pick up changes
kubectl rollout restart deployment/orchestrator -n llm-orchestrator

# Wait for restart to complete
kubectl rollout status deployment/orchestrator -n llm-orchestrator
```

### Step 7: Verify Rollback Success

```bash
# 1. Check all pods running previous version
kubectl get pods -n llm-orchestrator -l app=orchestrator \
  -o jsonpath='{.items[*].spec.containers[0].image}' | tr ' ' '\n' | sort -u

# Expected: Previous version image (e.g., v0.1.1)

# 2. Check application version
curl -s https://orchestrator.example.com/health | jq '.version'

# Expected: Previous version number

# 3. Test health endpoints
curl -s https://orchestrator.example.com/health
curl -s https://orchestrator.example.com/health/ready

# Expected: Both return 200 OK

# 4. Check recent logs for errors
kubectl logs -n llm-orchestrator -l app=orchestrator --tail=100 --timestamps | grep -i error

# Expected: No critical errors

# 5. Test functionality
curl -X POST https://orchestrator.example.com/api/v1/workflows \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "rollback-test",
    "steps": [{"type": "llm_call", "provider": "openai"}]
  }'

# Expected: Workflow created successfully
```

### Step 8: Monitor System Stability

```bash
# Monitor for 30 minutes after rollback

# 1. Watch error rate (should drop to < 1%)
# Prometheus query: rate(http_requests_total{status=~"5.."}[5m])

# 2. Watch latency (P99 should drop to < 2s)
# Prometheus query: histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))

# 3. Watch pod stability
watch kubectl get pods -n llm-orchestrator -l app=orchestrator

# Expected: All pods remain in Running state

# 4. Check metrics endpoint
curl https://orchestrator.example.com/metrics | grep -E "(error|latency|requests)"

# 5. Monitor user reports
# Check support channels for issues
```

### Step 9: Resume Autoscaling (if it was disabled)

```bash
# If HPA was paused or scaled manually, restore it

# Check current HPA status
kubectl get hpa orchestrator -n llm-orchestrator

# If HPA was deleted, recreate it
cat <<EOF | kubectl apply -f -
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: orchestrator
  namespace: llm-orchestrator
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: orchestrator
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
EOF

# Verify HPA is active
kubectl get hpa orchestrator -n llm-orchestrator -w
```

### Step 10: Document and Communicate

```bash
# 1. Create incident report
# Document:
# - Failed version
# - Symptoms observed
# - Rollback steps taken
# - Time to recovery
# - Root cause (if known)

# 2. Update incident in Slack
# Message template:
# "RESOLVED: LLM Orchestrator rollback completed
# Rolled back from: v0.2.0 to v0.1.1
# Recovery time: XX minutes
# Status: Monitoring stability
# Next steps: Root cause analysis scheduled"

# 3. Create post-incident ticket
# Assign to team for root cause analysis

# 4. Tag Docker image as broken (prevent future deployments)
# Add label to image in registry
```

## Validation

```bash
# Complete rollback validation checklist:

# ✓ Deployment shows previous revision
kubectl rollout history deployment/orchestrator -n llm-orchestrator

# ✓ All pods running and ready
kubectl get pods -n llm-orchestrator -l app=orchestrator
# All should be Running with 1/1 READY

# ✓ Correct image version
kubectl describe deployment orchestrator -n llm-orchestrator | grep Image:

# ✓ Health endpoints returning 200
curl -w "\n%{http_code}\n" https://orchestrator.example.com/health

# ✓ Error rate < 1% for 30 minutes
# Check Grafana dashboard

# ✓ P99 latency < 2 seconds
# Check Grafana dashboard

# ✓ No errors in logs
kubectl logs -n llm-orchestrator -l app=orchestrator --since=30m | grep -i error | wc -l

# ✓ Database queries executing successfully
kubectl exec -n llm-orchestrator postgres-0 -- psql -U orchestrator -d orchestrator -c "SELECT COUNT(*) FROM workflows;"

# ✓ User-facing functionality working
# Test critical workflows

# ✓ Metrics collection working
curl https://orchestrator.example.com/metrics | head -20
```

## Rollback Procedure
Not applicable (this IS the rollback procedure).

If rollback itself fails, escalate immediately to senior engineering team.

## Post-Incident Actions

1. **Immediate** (Within 1 hour):
   - Update status page
   - Notify stakeholders of resolution
   - Document timeline of events
   - Preserve logs and metrics

2. **Same Day**:
   - Schedule post-incident review (within 24-48 hours)
   - Create root cause analysis ticket
   - Review monitoring gaps
   - Update rollback runbook if needed

3. **Within Week**:
   - Conduct blameless post-mortem
   - Identify preventive measures
   - Update deployment procedures
   - Enhance testing coverage
   - Improve monitoring/alerting

4. **Ongoing**:
   - Track fix implementation
   - Verify fix in staging
   - Plan re-deployment with fixes
   - Update documentation

## Common Pitfalls

1. **Not Backing Up Before Deployment**: Always create backup before any deployment
   - Automate backup creation in deployment pipeline

2. **Forgetting to Rollback Database**: Schema changes need database rollback too
   - Document which deployments include migrations
   - Test migration rollback in staging

3. **Rolling Back Too Late**: Waiting too long can cause data loss
   - Define clear rollback criteria
   - Automate rollback for critical failures

4. **Incomplete Rollback**: Missing ConfigMap or Secret changes
   - Backup all resources, not just Deployment
   - Use Git to track all changes

5. **No Communication**: Team and users unaware of issues
   - Declare incidents early
   - Use status page for transparency

## Related Runbooks

- [02-rolling-update.md](./02-rolling-update.md) - Normal update procedure
- [01-initial-deployment.md](./01-initial-deployment.md) - Initial deployment
- [../incidents/02-service-unavailable.md](../incidents/02-service-unavailable.md) - Service unavailable response
- [../maintenance/01-database-maintenance.md](../maintenance/01-database-maintenance.md) - Database backup/restore

## Escalation

**Escalate IMMEDIATELY if**:
- Rollback fails after 2 attempts
- Database corruption detected
- Data loss occurring
- Rollback causes worse issues than original problem
- Unable to restore service after 15 minutes

**Escalation Path**:
1. **Immediate** - Senior DevOps Engineer - Slack: @devops-lead - Phone: xxx-xxx-xxxx
2. **5 minutes** - Engineering Manager - PagerDuty: LLM Orchestrator Critical
3. **10 minutes** - Database Administrator - Phone: xxx-xxx-xxxx
4. **15 minutes** - VP Engineering - Phone: xxx-xxx-xxxx
5. **20 minutes** - CTO - Phone: xxx-xxx-xxxx

**Crisis Communication**:
- Slack: #incidents (all hands)
- Create Zoom bridge: zoom.us/j/incident
- Enable conference bridge: 1-xxx-xxx-xxxx
- Status page: status.example.com

**Emergency Contacts**:
- AWS Support: 1-xxx-xxx-xxxx (if infrastructure issue)
- Database Vendor: support@postgres.com (if database issue)
- Security Team: security@example.com (if security incident)

---

**Last Updated**: 2025-11-14
**Version**: 1.0
**Owner**: DevOps Team
**Review Schedule**: After each rollback incident
