# Service Unavailable

## Overview
Complete service outage - the LLM Orchestrator is not responding to requests. This is a P0 incident requiring immediate action.

## Prerequisites
- Kubernetes cluster access
- PagerDuty/incident management access
- Communication channels (Slack, status page)

## Symptoms
- All health checks failing (502/503/504 errors)
- Users unable to access service
- All pods crashed or not ready
- Database unreachable
- Load balancer not routing traffic

## Impact Assessment
- Severity: Critical (P0)
- User impact: Total service outage
- Business impact: Revenue loss, SLA breach

## Step-by-Step Procedure

### Step 1: Declare Major Incident

```bash
# 1. Post in Slack #incidents
"P0 INCIDENT: LLM Orchestrator completely unavailable
Start time: $(date -u +"%Y-%m-%d %H:%M:%S UTC")
Incident Commander: [Your Name]
Bridge: zoom.us/j/incident"

# 2. Update status page
# Set status to "Major Outage"

# 3. Page on-call team
# PagerDuty: Escalate to all engineers

# 4. Start incident timeline doc
```

### Step 2: Quick Health Assessment

```bash
# Check if cluster is accessible
kubectl cluster-info

# Check namespace exists
kubectl get namespace llm-orchestrator

# Check pod status
kubectl get pods -n llm-orchestrator -l app=orchestrator

# Possible states:
# - No pods: Deployment deleted/scaled to 0
# - Pending: Resource constraints or scheduling issues
# - CrashLoopBackOff: Application crash
# - ImagePullBackOff: Image not found
# - Error: Various failures

# Check recent events
kubectl get events -n llm-orchestrator --sort-by='.lastTimestamp' | tail -20
```

### Step 3: Diagnose Root Cause

```bash
# If NO PODS exist:
kubectl get deployment orchestrator -n llm-orchestrator
# If deployment not found, someone deleted it
# IMMEDIATE ACTION: Redeploy from Git
kubectl apply -f deployment/

# If PODS PENDING:
kubectl describe pods -n llm-orchestrator | grep -A 10 Events
# Look for:
# - "Insufficient cpu/memory"
# - "No nodes available"
# - "FailedScheduling"
# IMMEDIATE ACTION: Scale cluster or adjust resources

# If PODS CRASHLOOPBACKOFF:
kubectl logs -n llm-orchestrator -l app=orchestrator --tail=100
# Look for:
# - "panic" or "FATAL"
# - Database connection errors
# - Configuration errors
# IMMEDIATE ACTION: See Step 4

# If PODS IMAGEPULLBACKOFF:
kubectl describe pods -n llm-orchestrator -l app=orchestrator | grep -A 5 "Failed to pull"
# IMMEDIATE ACTION: Fix image tag or registry credentials
```

### Step 4: Emergency Recovery

```bash
# SCENARIO A: Deployment deleted
git clone https://github.com/org/llm-orchestrator-deploy
cd llm-orchestrator-deploy
kubectl apply -f kubernetes/

# SCENARIO B: Bad deployment pushed
kubectl rollout undo deployment/orchestrator -n llm-orchestrator

# SCENARIO C: Database down
kubectl get pods -n llm-orchestrator -l app=postgres
kubectl logs -n llm-orchestrator postgres-0 --tail=100
# If corrupted, restore from backup (see ../maintenance/01-database-maintenance.md)

# SCENARIO D: Resource exhaustion
kubectl scale deployment orchestrator -n llm-orchestrator --replicas=1
# Start with 1 pod to reduce resource requirements

# SCENARIO E: ConfigMap/Secret missing
kubectl get configmap -n llm-orchestrator
kubectl get secrets -n llm-orchestrator
# Recreate from backup or Git

# SCENARIO F: Cluster issues
kubectl get nodes
# If nodes NotReady, escalate to infrastructure team
```

### Step 5: Force Restart (Last Resort)

```bash
# Delete all pods to force fresh start
kubectl delete pods -n llm-orchestrator -l app=orchestrator

# Wait for new pods
kubectl wait --for=condition=ready pod -l app=orchestrator -n llm-orchestrator --timeout=300s

# If pods still not coming up, check ReplicaSet
kubectl get rs -n llm-orchestrator
kubectl describe rs -n llm-orchestrator <replicaset-name>
```

### Step 6: Verify Service Recovery

```bash
# Check pods are running
kubectl get pods -n llm-orchestrator -l app=orchestrator
# Expected: All Running with 1/1 READY

# Test internal health endpoint
kubectl run -it --rm test-pod --image=curlimages/curl --restart=Never -- \
  curl http://orchestrator.llm-orchestrator.svc.cluster.local:8080/health

# Test external endpoint
curl https://orchestrator.example.com/health

# Check load balancer
kubectl get ingress orchestrator -n llm-orchestrator
kubectl describe ingress orchestrator -n llm-orchestrator

# Test workflow execution
curl -X POST https://orchestrator.example.com/api/v1/workflows/test
```

### Step 7: Monitor Stability

```bash
# Watch pods for 10 minutes
watch kubectl get pods -n llm-orchestrator -l app=orchestrator

# Monitor error rate
# Prometheus: rate(http_requests_total{status=~"5.."}[5m])

# Check logs continuously
kubectl logs -n llm-orchestrator -l app=orchestrator -f

# Verify no pod restarts
kubectl get pods -n llm-orchestrator -l app=orchestrator -o jsonpath='{.items[*].status.containerStatuses[*].restartCount}'
# Expected: 0 or low number
```

## Validation

```bash
# ✓ All pods Running and Ready
kubectl get pods -n llm-orchestrator

# ✓ Health endpoints return 200
curl -w "%{http_code}\n" https://orchestrator.example.com/health

# ✓ Can execute workflows
curl -X POST https://orchestrator.example.com/api/v1/workflows/execute -d '{...}'

# ✓ No errors in logs
kubectl logs -n llm-orchestrator -l app=orchestrator --since=5m | grep -i error

# ✓ Metrics being collected
curl https://orchestrator.example.com/metrics

# ✓ Database accessible
kubectl exec -n llm-orchestrator postgres-0 -- psql -U orchestrator -c "SELECT 1;"
```

## Post-Incident Actions

1. **Update status page** to "Operational"
2. **Notify stakeholders** of resolution
3. **Document timeline** of incident
4. **Schedule post-mortem** within 24 hours
5. **Preserve logs** for analysis
6. **Update monitoring** to prevent recurrence

## Prevention

- Implement PodDisruptionBudget (min 2 available)
- Set up comprehensive health checks
- Configure resource requests/limits properly
- Enable cluster autoscaling
- Regular disaster recovery drills
- GitOps for all infrastructure

## Related Runbooks
- [../deployment/03-rollback-procedure.md](../deployment/03-rollback-procedure.md)
- [03-database-issues.md](./03-database-issues.md)
- [../maintenance/01-database-maintenance.md](../maintenance/01-database-maintenance.md)

## Escalation

**Escalate IMMEDIATELY**:
- Incident Commander: On-call SRE lead
- Database Team: If database issues
- Infrastructure: If cluster/node issues
- Engineering: If application bugs
- Management: After 15 minutes of outage
- Executive: After 1 hour of outage

**Communication**:
- Slack: #incidents (public)
- Status Page: Update every 15 minutes
- Email: stakeholders@example.com
- Zoom Bridge: zoom.us/j/incident

---
**Last Updated**: 2025-11-14
**Version**: 1.0
**Owner**: SRE Team
