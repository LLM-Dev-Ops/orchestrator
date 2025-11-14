# Log Analysis

## Overview
Common log patterns and how to analyze them for troubleshooting.

## Log Locations

```bash
# Application logs
kubectl logs -n llm-orchestrator -l app=orchestrator --tail=100

# Database logs
kubectl logs -n llm-orchestrator postgres-0 --tail=100

# All logs from last hour
kubectl logs -n llm-orchestrator -l app=orchestrator --since=1h
```

## Common Log Patterns

### Success Patterns
```
INFO  workflow_executed workflow_id=abc123 duration=1.2s status=completed
INFO  database_query query="SELECT * FROM workflows" duration=15ms
INFO  llm_provider_call provider=openai model=gpt-4 tokens=150 duration=2.1s
```

### Error Patterns
```
ERROR workflow_failed workflow_id=abc123 error="database connection timeout"
ERROR database_connection_failed error="connection refused" retry_attempt=3
ERROR llm_provider_error provider=anthropic status=429 error="rate limit exceeded"
```

### Warning Patterns
```
WARN  high_latency endpoint=/api/v1/workflows duration=4.5s threshold=2.0s
WARN  connection_pool_high usage=85% max=100 available=15
WARN  retry_attempted operation=llm_call attempt=2 max=3
```

## Log Queries

### Find errors in last hour
```bash
kubectl logs -n llm-orchestrator -l app=orchestrator --since=1h | grep ERROR
```

### Find slow queries
```bash
kubectl logs -n llm-orchestrator -l app=orchestrator --since=1h | \
  grep "duration" | awk '$NF > 2' | sort -k NF -n
```

### Count errors by type
```bash
kubectl logs -n llm-orchestrator -l app=orchestrator --since=1h | \
  grep ERROR | awk '{print $5}' | sort | uniq -c | sort -rn
```

### Track specific workflow
```bash
kubectl logs -n llm-orchestrator -l app=orchestrator | \
  grep "workflow_id=abc123"
```

---
**Last Updated**: 2025-11-14
**Version**: 1.0
