# Troubleshooting Guide

## Overview
Common issues, diagnostic commands, and quick fixes for the LLM Orchestrator.

## Quick Diagnostics

### System Health Check

```bash
# One-command health check
kubectl get pods,svc,ingress -n llm-orchestrator && \
curl -s https://orchestrator.example.com/health | jq && \
kubectl top pods -n llm-orchestrator
```

## Common Issues and Solutions

### 1. Pods Not Starting

**Symptoms**: Pods stuck in Pending, ContainerCreating, or CrashLoopBackOff

**Diagnostic Commands**:
```bash
kubectl get pods -n llm-orchestrator
kubectl describe pod <pod-name> -n llm-orchestrator
kubectl logs <pod-name> -n llm-orchestrator --previous
```

**Common Causes**:

#### Insufficient Resources
```bash
# Check node resources
kubectl describe nodes | grep -A 5 "Allocated resources"

# Solution: Scale cluster or reduce resource requests
kubectl patch deployment orchestrator -n llm-orchestrator -p '
{
  "spec": {
    "template": {
      "spec": {
        "containers": [{
          "name": "orchestrator",
          "resources": {
            "requests": {"cpu": "250m", "memory": "512Mi"}
          }
        }]
      }
    }
  }
}'
```

#### Image Pull Errors
```bash
# Check image pull status
kubectl describe pod <pod-name> -n llm-orchestrator | grep -A 10 "Events"

# Solution: Fix image tag or add image pull secret
kubectl create secret docker-registry regcred \
  --docker-server=ghcr.io \
  --docker-username=<username> \
  --docker-password=<token> \
  -n llm-orchestrator
```

#### Configuration Errors
```bash
# Check ConfigMap
kubectl get configmap orchestrator-config -n llm-orchestrator -o yaml

# Verify secrets exist
kubectl get secrets -n llm-orchestrator

# Solution: Fix or recreate missing resources
```

### 2. High Latency

**Symptoms**: Slow response times, timeouts, user complaints

**Diagnostic Commands**:
```bash
# Check current latency
curl -w "@curl-format.txt" -o /dev/null -s https://orchestrator.example.com/health

# Prometheus query for P99 latency
curl -G 'http://prometheus:9090/api/v1/query' \
  --data-urlencode 'query=histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))'

# Check pod resources
kubectl top pods -n llm-orchestrator
```

**Common Causes**:

#### Database Slow Queries
```bash
# Identify slow queries
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT query, calls, mean_time, max_time
    FROM pg_stat_statements
    ORDER BY mean_time DESC LIMIT 10;
  "

# Solution: Add indexes, optimize queries
# See: docs/runbooks/incidents/01-high-latency.md
```

#### High CPU/Memory
```bash
# Check resource usage
kubectl top pods -n llm-orchestrator

# Solution: Scale horizontally
kubectl scale deployment orchestrator -n llm-orchestrator --replicas=6
```

#### External API Slow
```bash
# Check LLM provider latency
kubectl logs -n llm-orchestrator -l app=orchestrator | \
  grep -E "(openai|anthropic)" | grep "duration"

# Solution: Increase timeouts, use circuit breaker
```

### 3. Database Connection Errors

**Symptoms**: "connection refused", "too many connections", database unavailable

**Diagnostic Commands**:
```bash
# Check PostgreSQL pod
kubectl get pods -n llm-orchestrator -l app=postgres

# Test connection
kubectl exec -n llm-orchestrator deployment/orchestrator -- \
  psql "$DATABASE_URL" -c "SELECT 1;"

# Check active connections
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT count(*), state FROM pg_stat_activity GROUP BY state;
  "
```

**Solutions**:

#### Connection Pool Exhausted
```bash
# Kill idle connections
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT pg_terminate_backend(pid)
    FROM pg_stat_activity
    WHERE state = 'idle' AND state_change < now() - interval '10 minutes';
  "

# Increase pool size
kubectl patch configmap orchestrator-config -n llm-orchestrator --type merge -p '
{
  "data": {
    "config.yaml": "database:\n  max_connections: 40\n"
  }
}'
```

#### PostgreSQL Down
```bash
# Check PostgreSQL logs
kubectl logs -n llm-orchestrator postgres-0 --tail=100

# Restart PostgreSQL
kubectl delete pod postgres-0 -n llm-orchestrator

# See: docs/runbooks/incidents/03-database-issues.md
```

### 4. Authentication Failures

**Symptoms**: 401/403 errors, users can't login, JWT validation failures

**Diagnostic Commands**:
```bash
# Check auth logs
kubectl logs -n llm-orchestrator -l app=orchestrator | \
  grep -E "(401|403|auth|jwt)"

# Verify JWT secret
kubectl get secret orchestrator-jwt-secret -n llm-orchestrator

# Test authentication
curl -X POST https://orchestrator.example.com/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"test","password":"test"}'
```

**Solutions**: See [incidents/04-authentication-failures.md](./incidents/04-authentication-failures.md)

### 5. Workflows Stuck

**Symptoms**: Workflows not progressing, stuck in "running" state

**Diagnostic Commands**:
```bash
# Find stuck workflows
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT id, status, created_at, updated_at,
           now() - updated_at as stuck_duration
    FROM workflows
    WHERE status = 'running'
      AND updated_at < now() - interval '30 minutes';
  "

# Check workflow logs
kubectl logs -n llm-orchestrator -l app=orchestrator | \
  grep "workflow_id: <id>"
```

**Solutions**: See [incidents/05-workflow-stuck.md](./incidents/05-workflow-stuck.md)

### 6. Memory Issues

**Symptoms**: OOMKilled pods, high memory usage, swap usage

**Diagnostic Commands**:
```bash
# Check memory usage
kubectl top pods -n llm-orchestrator

# Check for OOM kills
kubectl get events -n llm-orchestrator | grep OOMKilled

# Monitor memory trend
# Grafana: container_memory_usage_bytes{pod=~"orchestrator-.*"}
```

**Solutions**: See [incidents/06-memory-leak.md](./incidents/06-memory-leak.md)

### 7. Disk Space Issues

**Symptoms**: "no space left on device", database write failures

**Diagnostic Commands**:
```bash
# Check disk usage
kubectl exec -n llm-orchestrator postgres-0 -- df -h

# Check PVC size
kubectl get pvc -n llm-orchestrator
```

**Solutions**: See [incidents/07-disk-full.md](./incidents/07-disk-full.md)

### 8. Network Connectivity

**Symptoms**: Connection timeouts, DNS failures, network errors

**Diagnostic Commands**:
```bash
# Test internal DNS
kubectl run -it --rm nettest --image=nicolaka/netshoot --restart=Never -- \
  nslookup postgres.llm-orchestrator.svc.cluster.local

# Test pod-to-pod connectivity
kubectl run -it --rm nettest --image=nicolaka/netshoot --restart=Never -- \
  ping orchestrator.llm-orchestrator.svc.cluster.local

# Test external connectivity
kubectl run -it --rm nettest --image=nicolaka/netshout --restart=Never -- \
  curl -v https://api.openai.com
```

**Solutions**: See [incidents/08-network-issues.md](./incidents/08-network-issues.md)

## Configuration File Locations

### Kubernetes Resources
```bash
# Deployment
kubectl get deployment orchestrator -n llm-orchestrator -o yaml

# ConfigMap
kubectl get configmap orchestrator-config -n llm-orchestrator -o yaml

# Secrets
kubectl get secrets -n llm-orchestrator

# Ingress
kubectl get ingress orchestrator -n llm-orchestrator -o yaml

# HPA
kubectl get hpa orchestrator -n llm-orchestrator -o yaml
```

### PostgreSQL
```bash
# Config file
kubectl exec -n llm-orchestrator postgres-0 -- cat /var/lib/postgresql/data/postgresql.conf

# Data directory
kubectl exec -n llm-orchestrator postgres-0 -- ls -la /var/lib/postgresql/data/
```

## Log Locations

```bash
# Application logs (stdout)
kubectl logs -n llm-orchestrator -l app=orchestrator

# Application logs (follow/tail)
kubectl logs -n llm-orchestrator -l app=orchestrator -f --tail=100

# Previous container logs (if crashed)
kubectl logs -n llm-orchestrator <pod-name> --previous

# PostgreSQL logs
kubectl logs -n llm-orchestrator postgres-0

# All logs from specific time
kubectl logs -n llm-orchestrator -l app=orchestrator --since=1h

# Export logs to file
kubectl logs -n llm-orchestrator -l app=orchestrator --since=24h > debug-logs.txt
```

## Enable Debug Logging

```bash
# Set log level to debug
kubectl patch configmap orchestrator-config -n llm-orchestrator --type merge -p '
{
  "data": {
    "config.yaml": "server:\n  log_level: debug\n"
  }
}'

# Or set via environment variable
kubectl set env deployment/orchestrator -n llm-orchestrator RUST_LOG=debug

# Restart pods to apply
kubectl rollout restart deployment/orchestrator -n llm-orchestrator

# Watch debug logs
kubectl logs -n llm-orchestrator -l app=orchestrator -f | grep DEBUG
```

## Performance Profiling

### CPU Profiling

```bash
# Enable profiling endpoint (if not already)
kubectl patch deployment orchestrator -n llm-orchestrator -p '
{
  "spec": {
    "template": {
      "spec": {
        "containers": [{
          "name": "orchestrator",
          "env": [{
            "name": "ENABLE_PROFILING",
            "value": "true"
          }]
        }]
      }
    }
  }
}'

# Capture CPU profile (30 seconds)
kubectl port-forward -n llm-orchestrator deployment/orchestrator 6060:6060
curl http://localhost:6060/debug/pprof/profile?seconds=30 > cpu.pprof

# Analyze profile
# go tool pprof cpu.pprof
```

### Memory Profiling

```bash
# Capture heap profile
curl http://localhost:6060/debug/pprof/heap > heap.pprof

# Analyze
# go tool pprof heap.pprof
```

### Database Query Profiling

```bash
# Enable slow query logging
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "
    ALTER SYSTEM SET log_min_duration_statement = 100;
    SELECT pg_reload_conf();
  "

# View slow queries in logs
kubectl logs -n llm-orchestrator postgres-0 | grep "duration:"

# Disable after debugging
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "
    ALTER SYSTEM RESET log_min_duration_statement;
    SELECT pg_reload_conf();
  "
```

## Emergency Commands

### Force Restart Everything

```bash
# WARNING: Causes downtime!
kubectl delete pods -n llm-orchestrator --all

# Wait for pods to restart
kubectl wait --for=condition=ready pod -l app=orchestrator -n llm-orchestrator --timeout=300s
```

### Quick Rollback

```bash
# Rollback to previous version
kubectl rollout undo deployment/orchestrator -n llm-orchestrator

# See: docs/runbooks/deployment/03-rollback-procedure.md
```

### Emergency Scaling

```bash
# Scale down (to reduce load)
kubectl scale deployment orchestrator -n llm-orchestrator --replicas=1

# Scale up (to handle load)
kubectl scale deployment orchestrator -n llm-orchestrator --replicas=10
```

### Database Emergency Access

```bash
# Direct PostgreSQL access
kubectl exec -it -n llm-orchestrator postgres-0 -- psql -U orchestrator -d orchestrator

# Backup database immediately
kubectl exec -n llm-orchestrator postgres-0 -- \
  pg_dump -U orchestrator orchestrator | gzip > emergency-backup-$(date +%Y%m%d-%H%M%S).sql.gz
```

## Getting Help

### Internal Resources
- **Slack**: #llm-orchestrator-ops
- **On-call**: PagerDuty rotation
- **Documentation**: /docs/runbooks/
- **Monitoring**: https://grafana.example.com

### External Resources
- **Kubernetes**: https://kubernetes.io/docs/
- **PostgreSQL**: https://www.postgresql.org/docs/
- **Rust**: https://doc.rust-lang.org/
- **Prometheus**: https://prometheus.io/docs/

### Escalation
1. Team Lead - Slack: @team-lead
2. Senior SRE - PagerDuty
3. Engineering Manager - Phone: xxx-xxx-xxxx
4. VP Engineering (critical issues)

---
**Last Updated**: 2025-11-14
**Version**: 1.0
**Owner**: SRE Team
