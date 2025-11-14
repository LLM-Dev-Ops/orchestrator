# Metrics Guide

## Overview
Understanding and interpreting Prometheus metrics exposed by the LLM Orchestrator.

## Key Metrics

### Application Metrics

#### Request Metrics
```promql
# Request rate (requests per second)
rate(http_requests_total[5m])

# Error rate
rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m]) * 100

# Success rate
rate(http_requests_total{status="200"}[5m]) / rate(http_requests_total[5m]) * 100

# Request duration (P50, P95, P99)
histogram_quantile(0.50, rate(http_request_duration_seconds_bucket[5m]))
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))
histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))
```

#### Workflow Metrics
```promql
# Active workflows
workflow_active_total

# Workflow completion rate
rate(workflow_completed_total[5m])

# Workflow failure rate
rate(workflow_failed_total[5m]) / rate(workflow_started_total[5m]) * 100

# Workflow duration
histogram_quantile(0.95, rate(workflow_duration_seconds_bucket[5m]))
```

#### Database Metrics
```promql
# Connection pool utilization
database_connections_active / database_connections_max * 100

# Connection wait time
database_connection_wait_seconds

# Query duration
histogram_quantile(0.95, rate(database_query_duration_seconds_bucket[5m]))

# Active transactions
database_transactions_active
```

#### LLM Provider Metrics
```promql
# Provider API calls
rate(llm_provider_requests_total{provider="openai"}[5m])
rate(llm_provider_requests_total{provider="anthropic"}[5m])

# Provider API latency
histogram_quantile(0.95, rate(llm_provider_duration_seconds_bucket[5m]))

# Provider errors
rate(llm_provider_errors_total[5m])

# Token usage
rate(llm_tokens_used_total[5m])
```

### Infrastructure Metrics

#### Pod Metrics
```promql
# CPU usage
rate(container_cpu_usage_seconds_total{pod=~"orchestrator-.*"}[5m])

# Memory usage
container_memory_usage_bytes{pod=~"orchestrator-.*"}

# Memory usage percentage
container_memory_usage_bytes{pod=~"orchestrator-.*"} /
container_spec_memory_limit_bytes{pod=~"orchestrator-.*"} * 100

# Network I/O
rate(container_network_receive_bytes_total{pod=~"orchestrator-.*"}[5m])
rate(container_network_transmit_bytes_total{pod=~"orchestrator-.*"}[5m])
```

#### Database Infrastructure
```promql
# PostgreSQL metrics
pg_stat_database_tup_inserted
pg_stat_database_tup_updated
pg_stat_database_tup_deleted
pg_stat_database_conflicts
pg_database_size_bytes
```

## Metric Interpretation

### Healthy Baselines
- **Error rate**: < 1%
- **P99 latency**: < 2 seconds
- **Success rate**: > 99%
- **CPU utilization**: 50-70%
- **Memory utilization**: 60-80%
- **Database connections**: < 80% of max

### Warning Thresholds
- **Error rate**: 1-5%
- **P99 latency**: 2-5 seconds
- **Success rate**: 95-99%
- **CPU utilization**: 70-85%
- **Memory utilization**: 80-90%
- **Database connections**: 80-90%

### Critical Thresholds
- **Error rate**: > 5%
- **P99 latency**: > 5 seconds
- **Success rate**: < 95%
- **CPU utilization**: > 85%
- **Memory utilization**: > 90%
- **Database connections**: > 90%

## Common Queries

### Troubleshooting

```promql
# Find slowest endpoints
topk(10, histogram_quantile(0.99,
  rate(http_request_duration_seconds_bucket[5m])))

# Identify error sources
sum by (endpoint, status) (rate(http_requests_total{status=~"5.."}[5m]))

# Database slow queries
topk(10, rate(database_query_duration_seconds_sum[5m]) /
  rate(database_query_duration_seconds_count[5m]))

# Pod restarts
increase(kube_pod_container_status_restarts_total{pod=~"orchestrator-.*"}[1h])
```

### Capacity Planning

```promql
# Request volume trend (30 days)
avg_over_time(rate(http_requests_total[1h])[30d:1h])

# Resource usage trend
avg_over_time(container_cpu_usage_seconds_total{pod=~"orchestrator-.*"}[30d])

# Database growth rate
deriv(pg_database_size_bytes[7d])
```

## Related Runbooks
- [02-alert-definitions.md](./02-alert-definitions.md)
- [03-dashboard-guide.md](./03-dashboard-guide.md)
- [../incidents/01-high-latency.md](../incidents/01-high-latency.md)

---
**Last Updated**: 2025-11-14
**Version**: 1.0
