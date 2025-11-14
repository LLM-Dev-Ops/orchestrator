# Secret Rotation Failure

## Overview
Failure during secret rotation causing authentication issues or service disruption.

## Symptoms
- Authentication failures after secret rotation
- Pods failing to start
- Database connection errors
- API calls failing

## Step-by-Step Procedure

### Step 1: Identify Failed Rotation

```bash
# Check secret exists and is correct
kubectl get secret orchestrator-jwt-secret -n llm-orchestrator -o yaml

# Check pod logs for secret errors
kubectl logs -n llm-orchestrator -l app=orchestrator | grep -i "secret\|auth\|key"

# Compare secret version across pods
kubectl get pods -n llm-orchestrator -l app=orchestrator -o jsonpath='{range .items[*]}{.metadata.name}{"\t"}{.spec.containers[0].env[?(@.name=="JWT_SECRET")].valueFrom.secretKeyRef.name}{"\n"}{end}'
```

### Step 2: Rollback Secret

```bash
# Restore previous secret value from backup
kubectl apply -f secret-backup-<timestamp>.yaml

# Force pod restart to pick up old secret
kubectl rollout restart deployment/orchestrator -n llm-orchestrator
```

### Step 3: Revoke Compromised Keys

```bash
# If secret was exposed, revoke all active sessions
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    UPDATE sessions SET revoked_at = now() WHERE revoked_at IS NULL;
  "

# Invalidate all JWT tokens by rotating secret properly this time
```

## Validation

```bash
# Authentication working
curl -X POST https://orchestrator.example.com/api/v1/auth/login -d '{...}'

# All pods using correct secret
kubectl get pods -n llm-orchestrator -l app=orchestrator
```

## Related Runbooks
- [../maintenance/03-secret-rotation.md](../maintenance/03-secret-rotation.md)
- [04-authentication-failures.md](./04-authentication-failures.md)

---
**Last Updated**: 2025-11-14
**Version**: 1.0
