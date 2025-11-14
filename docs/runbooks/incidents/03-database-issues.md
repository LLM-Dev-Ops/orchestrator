# Database Issues

## Overview
PostgreSQL connection failures, slow queries, or corruption affecting the LLM Orchestrator.

## Symptoms
- "connection refused" errors
- "too many connections" errors
- Slow query performance
- Database locks
- Replication lag

## Impact Assessment
- Severity: Critical (P0)
- User impact: Service degraded or unavailable
- Business impact: Data integrity at risk

## Step-by-Step Procedure

### Step 1: Check Database Connectivity

```bash
# Test connection from application pod
kubectl exec -n llm-orchestrator deployment/orchestrator -- \
  psql "$DATABASE_URL" -c "SELECT 1;"

# Check PostgreSQL pod status
kubectl get pods -n llm-orchestrator -l app=postgres

# Check logs
kubectl logs -n llm-orchestrator postgres-0 --tail=100
```

### Step 2: Connection Pool Exhaustion

```bash
# Check active connections
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT count(*) FROM pg_stat_activity;
  "

# Check max connections
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "SHOW max_connections;"

# If at limit, kill idle connections
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT pg_terminate_backend(pid)
    FROM pg_stat_activity
    WHERE state = 'idle' AND state_change < now() - interval '10 minutes';
  "
```

### Step 3: Slow Queries

```bash
# Identify slow queries
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT pid, now() - query_start AS duration, query
    FROM pg_stat_activity
    WHERE state != 'idle' AND now() - query_start > interval '5 seconds'
    ORDER BY duration DESC;
  "

# Kill problematic query (if needed)
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "SELECT pg_terminate_backend(<pid>);"
```

### Step 4: Database Corruption

```bash
# Check for corruption
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT pg_database.datname, pg_database_size(pg_database.datname)
    FROM pg_database;
  "

# If corruption detected, restore from backup
# See ../maintenance/01-database-maintenance.md
```

## Validation

```bash
# Connection working
psql "$DATABASE_URL" -c "SELECT 1;"

# No locks
kubectl exec postgres-0 -n llm-orchestrator -- psql -U orchestrator -c "SELECT * FROM pg_locks;"

# Queries fast
# Check P95 query time < 100ms
```

## Related Runbooks
- [../maintenance/01-database-maintenance.md](../maintenance/01-database-maintenance.md)
- [02-service-unavailable.md](./02-service-unavailable.md)

---
**Last Updated**: 2025-11-14
**Version**: 1.0
