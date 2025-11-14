# High Latency Investigation

## Overview
Investigate and resolve high response time issues when workflow execution becomes slow. This runbook addresses P99 latency > 5 seconds or user-reported slowness.

## Prerequisites
- Kubernetes access
- Prometheus/Grafana access
- Database access
- Application logs access
- Basic performance profiling knowledge

## Symptoms
- P99 latency > 5 seconds (alert triggered)
- User complaints about slow workflows
- Timeout errors increasing
- Request queue building up
- Slow database queries

## Impact Assessment
- Severity: High (P1)
- User impact: Degraded user experience
- Business impact: Reduced throughput, potential SLA breach

## Step-by-Step Procedure

### Step 1: Confirm and Quantify Issue

```bash
# Check current latency metrics
curl https://orchestrator.example.com/metrics | grep http_request_duration

# Query Prometheus for P99 latency
# histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))

# Check request rate
# rate(http_requests_total[5m])

# Identify slow endpoints
# topk(5, histogram_quantile(0.99, rate(http_request_duration_seconds_bucket{endpoint!=""}[5m])))

# Expected findings:
# - Which endpoints are slow
# - How slow (baseline vs current)
# - When it started
```

### Step 2: Check System Resources

```bash
# Check pod CPU/memory usage
kubectl top pods -n llm-orchestrator -l app=orchestrator

# Expected issues:
# - CPU > 80%: CPU bottleneck
# - Memory > 80%: Memory pressure
# - Pods being throttled

# Check node resources
kubectl top nodes

# Check for resource exhaustion
kubectl describe nodes | grep -A 5 "Allocated resources"

# If resources exhausted, see Step 8 (Scale horizontally)
```

### Step 3: Analyze Database Performance

```bash
# Check database connection pool
kubectl logs -n llm-orchestrator -l app=orchestrator --tail=100 | grep -i "connection pool"

# Check slow queries
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      pid,
      now() - query_start AS duration,
      state,
      query
    FROM pg_stat_activity
    WHERE state != 'idle'
      AND now() - query_start > interval '1 second'
    ORDER BY duration DESC
    LIMIT 10;
  "

# Expected: Identify slow queries

# Check for locks
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      blocked_locks.pid AS blocked_pid,
      blocking_locks.pid AS blocking_pid,
      blocked_activity.query AS blocked_query,
      blocking_activity.query AS blocking_query
    FROM pg_catalog.pg_locks blocked_locks
    JOIN pg_catalog.pg_stat_activity blocked_activity ON blocked_activity.pid = blocked_locks.pid
    JOIN pg_catalog.pg_locks blocking_locks ON blocking_locks.locktype = blocked_locks.locktype
    JOIN pg_catalog.pg_stat_activity blocking_activity ON blocking_activity.pid = blocking_locks.pid
    WHERE NOT blocked_locks.granted;
  "

# If locks found, investigate and potentially kill blocking queries
```

### Step 4: Check External API Latency

```bash
# Check LLM provider response times
kubectl logs -n llm-orchestrator -l app=orchestrator --tail=500 | \
  grep -E "(openai|anthropic)" | grep -E "duration|latency"

# Look for patterns:
# - OpenAI API slow: > 3s response time
# - Anthropic API slow: > 5s response time
# - Rate limiting: 429 errors

# Check provider metrics in Grafana
# Metrics: provider_api_duration_seconds

# If provider is slow:
# - Check provider status page
# - Consider failing over to alternate provider
# - Implement circuit breaker
```

### Step 5: Analyze Application Logs

```bash
# Check for errors
kubectl logs -n llm-orchestrator -l app=orchestrator --tail=500 | grep -i error

# Check for warnings
kubectl logs -n llm-orchestrator -l app=orchestrator --tail=500 | grep -i warn

# Look for patterns:
# - Timeout errors
# - Connection errors
# - Memory allocation failures
# - Thread pool exhaustion

# Get detailed logs from slowest pod
SLOWEST_POD=$(kubectl top pods -n llm-orchestrator -l app=orchestrator | \
  tail -n +2 | sort -k3 -hr | head -1 | awk '{print $1}')

kubectl logs -n llm-orchestrator $SLOWEST_POD --tail=1000
```

### Step 6: Check Network Latency

```bash
# Test network latency to database
kubectl run -it --rm nettest --image=nicolaka/netshoot --restart=Never -- \
  ping -c 10 postgres.llm-orchestrator.svc.cluster.local

# Expected: < 1ms latency within cluster

# Test network latency to external APIs
kubectl run -it --rm nettest --image=nicolaka/netshoot --restart=Never -- \
  curl -w "@curl-format.txt" -o /dev/null -s https://api.openai.com/v1/models

# Check for packet loss or high latency
```

### Step 7: Check for Memory Leaks

```bash
# Check memory growth over time
kubectl top pods -n llm-orchestrator -l app=orchestrator

# Compare with historical data in Grafana
# Query: container_memory_usage_bytes{pod=~"orchestrator-.*"}

# If memory steadily increasing:
# - Memory leak suspected
# - See ../incidents/06-memory-leak.md for investigation

# Get heap dump (if Rust app supports it)
kubectl exec -n llm-orchestrator $SLOWEST_POD -- kill -USR1 1

# Download and analyze heap dump
```

### Step 8: Scale Horizontally (If Resource Constrained)

```bash
# If CPU/Memory at capacity, scale up immediately
kubectl scale deployment orchestrator -n llm-orchestrator --replicas=6

# Wait for new pods to be ready
kubectl wait --for=condition=ready pod -l app=orchestrator -n llm-orchestrator --timeout=300s

# Monitor if latency improves
watch 'curl -s https://orchestrator.example.com/metrics | grep -E "http_request_duration.*0.99"'

# If improved, update HPA min replicas
kubectl patch hpa orchestrator -n llm-orchestrator -p '{"spec":{"minReplicas":6}}'
```

### Step 9: Optimize Database Queries (If DB Bottleneck)

```bash
# Identify missing indexes
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      schemaname,
      tablename,
      seq_scan,
      seq_tup_read,
      idx_scan,
      seq_tup_read / seq_scan AS avg_seq_tup_read
    FROM pg_stat_user_tables
    WHERE seq_scan > 0
    ORDER BY seq_tup_read DESC
    LIMIT 10;
  "

# If seq_scan high and no idx_scan, index may be needed

# Analyze slow query (get from Step 3)
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    EXPLAIN ANALYZE
    SELECT * FROM workflows WHERE status = 'running';
  "

# Look for Seq Scan instead of Index Scan

# Create index if needed (example)
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    CREATE INDEX CONCURRENTLY idx_workflows_status ON workflows(status);
  "
```

### Step 10: Tune Connection Pool

```bash
# If connection pool exhausted
kubectl logs -n llm-orchestrator -l app=orchestrator | grep "connection pool"

# Increase pool size
kubectl patch configmap orchestrator-config -n llm-orchestrator --type merge -p '
{
  "data": {
    "config.yaml": "database:\n  max_connections: 40\n  min_connections: 10\n"
  }
}'

# Restart pods to apply
kubectl rollout restart deployment/orchestrator -n llm-orchestrator

# Increase PostgreSQL max_connections if needed
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "ALTER SYSTEM SET max_connections = 200;"

# Restart PostgreSQL
kubectl delete pod postgres-0 -n llm-orchestrator
```

## Validation

```bash
# 1. Check P99 latency returned to normal (< 2s)
# Prometheus: histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))

# 2. No timeout errors
kubectl logs -n llm-orchestrator -l app=orchestrator --since=10m | grep -i timeout | wc -l
# Expected: 0 or minimal

# 3. Database queries fast
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT MAX(now() - query_start)
    FROM pg_stat_activity
    WHERE state != 'idle';
  "
# Expected: < 1 second

# 4. Resource utilization healthy
kubectl top pods -n llm-orchestrator
# Expected: CPU < 70%, Memory < 80%

# 5. User-facing test
time curl -X POST https://orchestrator.example.com/api/v1/workflows/execute -d '{...}'
# Expected: < 2 seconds
```

## Rollback Procedure
If changes made things worse:

```bash
# Rollback configuration changes
kubectl rollout undo deployment/orchestrator -n llm-orchestrator

# Rollback database changes (drop index)
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "DROP INDEX CONCURRENTLY idx_workflows_status;"

# Scale back to previous replica count
kubectl scale deployment orchestrator -n llm-orchestrator --replicas=3
```

## Prevention

1. **Implement Query Caching**: Cache frequent database queries
2. **Add Database Indexes**: Regular index analysis and optimization
3. **Set Up Auto-Scaling**: HPA based on latency metrics
4. **Connection Pool Monitoring**: Alert on pool exhaustion
5. **Regular Performance Testing**: Load test before deployments
6. **Circuit Breakers**: For external API calls

## Common Pitfalls

1. **Scaling Too Late**: Scale preemptively based on trends
2. **Ignoring Database**: Often the root cause
3. **Not Checking External APIs**: LLM providers can be slow
4. **Missing Indexes**: Causes sequential scans
5. **Memory Leaks**: Gradual performance degradation

## Related Runbooks
- [../deployment/04-scaling.md](../deployment/04-scaling.md)
- [06-memory-leak.md](./06-memory-leak.md)
- [../maintenance/01-database-maintenance.md](../maintenance/01-database-maintenance.md)
- [../maintenance/07-performance-tuning.md](../maintenance/07-performance-tuning.md)

## Escalation

**Escalate if**:
- Latency not improving after 15 minutes
- User impact growing
- Database corruption suspected
- Unable to identify root cause

**Escalation Path**:
1. Senior SRE - Slack: @sre-lead - PagerDuty
2. Database Admin - Slack: @dba-team
3. Engineering Lead - Slack: @eng-lead
4. VP Engineering (if SLA breach imminent)

---
**Last Updated**: 2025-11-14
**Version**: 1.0
**Owner**: SRE Team
