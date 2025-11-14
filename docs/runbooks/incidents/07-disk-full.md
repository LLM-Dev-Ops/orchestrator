# Disk Full

## Overview
Storage exhaustion on PostgreSQL PVC or node disk, preventing writes.

## Symptoms
- "no space left on device" errors
- Database write failures
- Log rotation failing
- Pod evictions

## Step-by-Step Procedure

### Step 1: Identify Full Disk

```bash
# Check PVC usage
kubectl exec -n llm-orchestrator postgres-0 -- df -h

# Check node disk usage
kubectl get nodes -o wide
kubectl describe node <node-name> | grep -A 10 "Allocated resources"
```

### Step 2: Emergency Cleanup

```bash
# Clean up old WAL logs
kubectl exec -n llm-orchestrator postgres-0 -- \
  find /var/lib/postgresql/data/pg_wal -name "*.old" -mtime +7 -delete

# Vacuum database
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "VACUUM FULL;"

# Archive and compress logs
kubectl exec -n llm-orchestrator postgres-0 -- sh -c \
  "find /var/log -name '*.log' -mtime +3 -exec gzip {} \;"
```

### Step 3: Expand PVC

```bash
# Resize PVC (if storage class supports it)
kubectl patch pvc postgres-pvc -n llm-orchestrator -p '{"spec":{"resources":{"requests":{"storage":"20Gi"}}}}'

# Wait for resize
kubectl get pvc postgres-pvc -n llm-orchestrator -w
```

## Validation

```bash
# Disk usage < 80%
kubectl exec -n llm-orchestrator postgres-0 -- df -h | grep "/var/lib/postgresql/data"

# Database writes working
kubectl exec -n llm-orchestrator postgres-0 -- psql -U orchestrator -c "CREATE TABLE test(id int); DROP TABLE test;"
```

## Prevention
- Set up disk usage alerts (> 70%)
- Implement log rotation
- Regular VACUUM
- Automated backup cleanup

## Related Runbooks
- [../maintenance/01-database-maintenance.md](../maintenance/01-database-maintenance.md)
- [../maintenance/02-log-rotation.md](../maintenance/02-log-rotation.md)

---
**Last Updated**: 2025-11-14
**Version**: 1.0
