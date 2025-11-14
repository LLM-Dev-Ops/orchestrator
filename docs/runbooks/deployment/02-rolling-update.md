# Rolling Update

## Overview
Perform a zero-downtime rolling update of the LLM Orchestrator to a new version. This procedure ensures continuous service availability while deploying new code.

## Prerequisites
- Required access/permissions:
  - Kubernetes namespace admin (RBAC: `edit` role in `llm-orchestrator` namespace)
  - Docker registry access
  - Grafana/Prometheus access for monitoring
- Tools needed:
  - `kubectl` v1.24+
  - `helm` (if using Helm)
  - Access to deployment manifests
- Knowledge required:
  - Kubernetes rolling update strategy
  - Application monitoring
  - Rollback procedures

## Impact Assessment
- Severity: Medium (production change)
- User impact: None (zero-downtime deployment)
- Business impact: Enables new features/bug fixes

## Step-by-Step Procedure

### Step 1: Pre-Update Verification

```bash
# 1. Check current deployment status
kubectl get deployment orchestrator -n llm-orchestrator

# Expected output showing current version:
# NAME           READY   UP-TO-DATE   AVAILABLE   AGE
# orchestrator   3/3     3            3           5d

# 2. Verify all pods are healthy
kubectl get pods -n llm-orchestrator -l app=orchestrator

# Expected: All pods in Running state with 1/1 READY

# 3. Check current version
kubectl get deployment orchestrator -n llm-orchestrator -o jsonpath='{.spec.template.spec.containers[0].image}'

# Expected output: ghcr.io/llm-orchestrator/orchestrator:v0.1.0

# 4. Check application health
curl -s https://orchestrator.example.com/health | jq

# Expected: {"status":"healthy","version":"0.1.0"}

# 5. Review current error rates in Grafana
# Navigate to Grafana dashboard
# Check error rate < 1% over last hour
```

### Step 2: Review Changelog and Breaking Changes

```bash
# Review changelog for the new version
curl https://github.com/llm-orchestrator/orchestrator/blob/main/CHANGELOG.md

# Key items to check:
# - Database migrations required?
# - Configuration changes needed?
# - API breaking changes?
# - Dependency updates?
# - Security patches?

# Check for database migrations
ls -la migrations/

# If migrations exist, plan migration execution
```

### Step 3: Backup Current State

```bash
# 1. Backup current deployment manifest
kubectl get deployment orchestrator -n llm-orchestrator -o yaml > deployment-backup-$(date +%Y%m%d-%H%M%S).yaml

# 2. Backup current ConfigMap
kubectl get configmap orchestrator-config -n llm-orchestrator -o yaml > configmap-backup-$(date +%Y%m%d-%H%M%S).yaml

# 3. Create database backup (if migrations required)
kubectl exec -n llm-orchestrator postgres-0 -- pg_dump -U orchestrator orchestrator | gzip > backup-$(date +%Y%m%d-%H%M%S).sql.gz

# Verify backup file created
ls -lh backup-*.sql.gz

# Expected: Backup file with reasonable size (> 0 bytes)
```

### Step 4: Run Database Migrations (if required)

```bash
# Only run if new version includes schema changes

# 1. Create migration job
cat <<EOF | kubectl apply -f -
apiVersion: batch/v1
kind: Job
metadata:
  name: orchestrator-migrate-$(date +%s)
  namespace: llm-orchestrator
spec:
  template:
    spec:
      containers:
      - name: migrate
        image: ghcr.io/llm-orchestrator/orchestrator:v0.2.0
        command: ["./orchestrator", "migrate"]
        env:
        - name: DATABASE_URL
          value: "postgresql://orchestrator:password@postgres.llm-orchestrator.svc.cluster.local:5432/orchestrator"
      restartPolicy: Never
  backoffLimit: 3
EOF

# 2. Wait for migration to complete
kubectl wait --for=condition=complete job/orchestrator-migrate-* -n llm-orchestrator --timeout=300s

# 3. Check migration logs
kubectl logs -n llm-orchestrator job/orchestrator-migrate-*

# Expected: "Migrations completed successfully"

# 4. Verify database schema
kubectl exec -n llm-orchestrator postgres-0 -- psql -U orchestrator -d orchestrator -c "\dt"

# Expected: New tables/columns present
```

### Step 5: Update Configuration (if needed)

```bash
# Update ConfigMap with new configuration
cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: ConfigMap
metadata:
  name: orchestrator-config
  namespace: llm-orchestrator
data:
  config.yaml: |
    server:
      port: 8080
      log_level: info
    database:
      host: postgres.llm-orchestrator.svc.cluster.local
      port: 5432
      database: orchestrator
      max_connections: 20
    providers:
      openai:
        enabled: true
        timeout_seconds: 30
      anthropic:
        enabled: true
        timeout_seconds: 30
    # New feature flags for v0.2.0
    features:
      state_persistence: true
      audit_logging: true
EOF

# Verify ConfigMap updated
kubectl get configmap orchestrator-config -n llm-orchestrator -o yaml
```

### Step 6: Initiate Rolling Update

```bash
# Update deployment to new image version
kubectl set image deployment/orchestrator \
  orchestrator=ghcr.io/llm-orchestrator/orchestrator:v0.2.0 \
  -n llm-orchestrator

# Alternative: Apply updated manifest
# kubectl apply -f deployment-v0.2.0.yaml

# Expected output:
# deployment.apps/orchestrator image updated
```

### Step 7: Monitor Rollout Progress

```bash
# Watch rollout status
kubectl rollout status deployment/orchestrator -n llm-orchestrator --watch

# Expected output:
# Waiting for deployment "orchestrator" rollout to finish: 1 out of 3 new replicas have been updated...
# Waiting for deployment "orchestrator" rollout to finish: 2 out of 3 new replicas have been updated...
# Waiting for deployment "orchestrator" rollout to finish: 3 old replicas are pending termination...
# deployment "orchestrator" successfully rolled out

# In another terminal, watch pods
watch kubectl get pods -n llm-orchestrator -l app=orchestrator

# You should see:
# - Old pods terminating one by one
# - New pods creating and becoming ready
# - Total always >= minAvailable (2)
```

### Step 8: Monitor Application Metrics

```bash
# While rollout is in progress, monitor:

# 1. Error rate (should stay < 1%)
# Check Prometheus query:
# rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m]) * 100

# 2. Latency (P99 should stay < 5s)
# histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))

# 3. Request success rate (should stay > 99%)
# rate(http_requests_total{status="200"}[5m]) / rate(http_requests_total[5m]) * 100

# 4. Active connections
# database_connections_active

# If any metric degrades significantly, immediately trigger rollback (see Step 11)
```

### Step 9: Verify New Version

```bash
# 1. Check all pods running new version
kubectl get pods -n llm-orchestrator -l app=orchestrator -o jsonpath='{.items[*].spec.containers[0].image}' | tr ' ' '\n' | sort -u

# Expected: ghcr.io/llm-orchestrator/orchestrator:v0.2.0 (only)

# 2. Verify application version
curl -s https://orchestrator.example.com/health | jq '.version'

# Expected: "0.2.0"

# 3. Check pod logs for errors
kubectl logs -n llm-orchestrator -l app=orchestrator --tail=100 | grep -i error

# Expected: No critical errors

# 4. Test key functionality
curl -X POST https://orchestrator.example.com/api/v1/workflows \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "test-workflow",
    "steps": [{"type": "llm_call", "provider": "openai"}]
  }'

# Expected: Workflow created successfully
```

### Step 10: Canary Testing (Optional but Recommended)

```bash
# If using canary deployment strategy:

# 1. Deploy canary with 1 replica first
kubectl patch deployment orchestrator -n llm-orchestrator -p '
{
  "spec": {
    "replicas": 1,
    "template": {
      "spec": {
        "containers": [{
          "name": "orchestrator",
          "image": "ghcr.io/llm-orchestrator/orchestrator:v0.2.0"
        }]
      }
    }
  }
}'

# 2. Monitor canary for 15 minutes
# Check error rates, latency, success rates

# 3. If canary healthy, gradually increase replicas
kubectl scale deployment orchestrator -n llm-orchestrator --replicas=3

# 4. Continue monitoring until full rollout
```

### Step 11: Post-Deployment Verification

```bash
# 1. Run smoke tests
kubectl run -it --rm smoke-test --image=curlimages/curl --restart=Never -- sh -c '
  curl -f http://orchestrator.llm-orchestrator.svc.cluster.local:8080/health &&
  curl -f http://orchestrator.llm-orchestrator.svc.cluster.local:8080/health/ready &&
  curl -f http://orchestrator.llm-orchestrator.svc.cluster.local:9090/metrics
'

# Expected: All curl commands succeed

# 2. Verify HPA is functioning
kubectl get hpa orchestrator -n llm-orchestrator

# Expected: Current replicas matching desired

# 3. Check deployment revision
kubectl rollout history deployment/orchestrator -n llm-orchestrator

# Expected: New revision listed

# 4. Monitor for 30 minutes post-deployment
# Watch dashboards for any anomalies
```

## Validation

```bash
# Complete validation checklist:

# 1. All pods running and ready
kubectl get pods -n llm-orchestrator -l app=orchestrator
# ✓ All pods in Running state
# ✓ All pods show 1/1 READY

# 2. Correct image version deployed
kubectl describe deployment orchestrator -n llm-orchestrator | grep Image:
# ✓ Image matches target version

# 3. Health endpoints responding
curl https://orchestrator.example.com/health
# ✓ Returns 200 OK

# 4. No errors in logs
kubectl logs -n llm-orchestrator -l app=orchestrator --tail=200 | grep -i error | wc -l
# ✓ Error count is 0 or within acceptable threshold

# 5. Metrics endpoint working
curl https://orchestrator.example.com/metrics
# ✓ Returns Prometheus metrics

# 6. Database connectivity confirmed
kubectl exec -n llm-orchestrator -it deployment/orchestrator -- sh -c 'echo "SELECT 1" | psql $DATABASE_URL'
# ✓ Returns: 1

# 7. Error rate < 1% over last 30 minutes
# Check Grafana dashboard
# ✓ Error rate acceptable

# 8. P99 latency < 5 seconds
# Check Grafana dashboard
# ✓ Latency acceptable
```

## Rollback Procedure

If update fails or issues detected:

```bash
# IMMEDIATE ROLLBACK

# Option 1: Rollback to previous revision
kubectl rollout undo deployment/orchestrator -n llm-orchestrator

# Watch rollback progress
kubectl rollout status deployment/orchestrator -n llm-orchestrator

# Option 2: Rollback to specific revision
# First, check revision history
kubectl rollout history deployment/orchestrator -n llm-orchestrator

# Rollback to specific revision (e.g., revision 5)
kubectl rollout undo deployment/orchestrator -n llm-orchestrator --to-revision=5

# Option 3: Apply previous manifest
kubectl apply -f deployment-backup-*.yaml

# Verify rollback successful
kubectl get pods -n llm-orchestrator -l app=orchestrator
kubectl get deployment orchestrator -n llm-orchestrator -o jsonpath='{.spec.template.spec.containers[0].image}'

# Rollback database migrations if needed
kubectl exec -n llm-orchestrator postgres-0 -- psql -U orchestrator orchestrator < backup-*.sql

# For complete rollback procedure, see: ../deployment/03-rollback-procedure.md
```

## Prevention

1. **Automated Testing**:
   - Run integration tests in staging before production
   - Implement automated smoke tests
   - Configure CI/CD gates

2. **Gradual Rollout**:
   - Start with canary deployment (1 pod)
   - Monitor metrics for 15-30 minutes
   - Gradually increase replicas
   - Use traffic splitting if available

3. **Monitoring Alerts**:
   - Set up alerts for error rate spikes
   - Alert on high latency (P99 > 5s)
   - Alert on deployment failures
   - Configure PagerDuty integration

4. **Scheduled Maintenance Windows**:
   - Perform updates during low-traffic periods
   - Notify users of maintenance windows
   - Have rollback plan ready

## Common Pitfalls

1. **Insufficient Database Backup**: Always backup before migrations
   - Verify backup is restorable before proceeding

2. **Breaking Configuration Changes**: ConfigMap updates may require pod restart
   - Update ConfigMap before deployment update
   - Consider pod disruption budget

3. **Resource Exhaustion During Rollout**: Surge pods may exceed cluster capacity
   - Ensure cluster has 20% spare capacity
   - Adjust maxSurge/maxUnavailable if needed

4. **Rollback Database After Code Rollback**: Schema changes may not be backward compatible
   - Test rollback procedure in staging
   - Consider migration versioning

5. **Ignoring Canary Metrics**: Small error rate increases can indicate major issues
   - Set strict thresholds for canary acceptance
   - Automate canary analysis

## Related Runbooks

- [01-initial-deployment.md](./01-initial-deployment.md) - Initial deployment setup
- [03-rollback-procedure.md](./03-rollback-procedure.md) - Detailed rollback procedures
- [04-scaling.md](./04-scaling.md) - Scaling operations
- [05-multi-region-deployment.md](./05-multi-region-deployment.md) - Multi-region updates
- [../incidents/01-high-latency.md](../incidents/01-high-latency.md) - High latency troubleshooting

## Escalation

**Escalate if**:
- Rollout stuck for > 10 minutes
- Error rate > 5% during rollout
- Rollback fails
- Data corruption suspected
- User-reported production issues

**Escalation Path**:
1. Senior DevOps Engineer - devops-lead@example.com - Slack: @devops-lead
2. Application Owner - app-owner@example.com - Slack: @app-owner
3. Engineering Manager - eng-manager@example.com - PagerDuty
4. VP Engineering - vp-eng@example.com

**Immediate Actions**:
1. Halt rollout: `kubectl rollout pause deployment/orchestrator -n llm-orchestrator`
2. Notify team in Slack #llm-orchestrator-ops
3. Create incident in incident management system
4. Execute rollback if degradation confirmed

---

**Last Updated**: 2025-11-14
**Version**: 1.0
**Owner**: DevOps Team
**Review Schedule**: After each deployment
