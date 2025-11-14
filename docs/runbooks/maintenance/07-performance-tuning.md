# Performance Tuning

## Overview
Optimize application and database performance based on metrics and profiling.

## Step-by-Step Procedure

### Application Performance

```bash
# Profile application (if profiling enabled)
curl https://orchestrator.example.com/debug/pprof/profile?seconds=30 > cpu-profile.pprof

# Analyze hot paths
# go tool pprof cpu-profile.pprof

# Tune connection pool
kubectl patch configmap orchestrator-config -n llm-orchestrator --type merge -p '
{
  "data": {
    "config.yaml": "database:\n  max_connections: 50\n  min_connections: 10\n  acquire_timeout_seconds: 5\n"
  }
}'
```

### Database Performance

```bash
# Tune PostgreSQL settings
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "
    ALTER SYSTEM SET shared_buffers = '256MB';
    ALTER SYSTEM SET effective_cache_size = '1GB';
    ALTER SYSTEM SET maintenance_work_mem = '128MB';
    ALTER SYSTEM SET work_mem = '32MB';
  "

# Restart PostgreSQL
kubectl delete pod postgres-0 -n llm-orchestrator

# Add indexes for slow queries
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    CREATE INDEX CONCURRENTLY idx_workflows_user_status
    ON workflows(user_id, status)
    WHERE status IN ('pending', 'running');
  "
```

### Resource Optimization

```bash
# Adjust pod resources based on actual usage
kubectl patch deployment orchestrator -n llm-orchestrator -p '
{
  "spec": {
    "template": {
      "spec": {
        "containers": [{
          "name": "orchestrator",
          "resources": {
            "requests": {
              "cpu": "750m",
              "memory": "1.5Gi"
            },
            "limits": {
              "cpu": "1500m",
              "memory": "3Gi"
            }
          }
        }]
      }
    }
  }
}'
```

## Validation

```bash
# Latency improved
# P99 < 2s (from previous baseline)

# Resource utilization optimal
kubectl top pods -n llm-orchestrator
# CPU: 50-70%, Memory: 60-80%

# No performance regressions
# Monitor for 24 hours
```

---
**Last Updated**: 2025-11-14
**Version**: 1.0
