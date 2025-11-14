# Grafana Dashboard Guide

## Overview
Guide to understanding and using Grafana dashboards for the LLM Orchestrator.

## Main Dashboards

### 1. Orchestrator Overview Dashboard

**Purpose**: High-level system health and performance

**Key Panels**:
- Request rate (QPS)
- Error rate %
- P50/P95/P99 latency
- Active workflows
- Database connection pool
- Pod CPU/Memory usage

**URL**: `https://grafana.example.com/d/orchestrator-overview`

### 2. Database Performance Dashboard

**Purpose**: PostgreSQL monitoring and tuning

**Key Panels**:
- Query performance (slow queries)
- Connection pool utilization
- Database size and growth
- Transaction rate
- Lock contention
- Replication lag

### 3. LLM Provider Dashboard

**Purpose**: Monitor external LLM API usage

**Key Panels**:
- Requests by provider (OpenAI, Anthropic)
- Provider API latency
- Token usage and costs
- Error rates by provider
- Rate limiting events

### 4. Kubernetes Infrastructure Dashboard

**Purpose**: Cluster and pod health

**Key Panels**:
- Pod status (Running/Pending/Failed)
- Node resource utilization
- Pod restarts
- Network I/O
- PVC usage

## How to Use Dashboards

### Investigate Performance Issue
1. Open "Orchestrator Overview" dashboard
2. Set time range to when issue occurred
3. Check error rate and latency panels
4. Identify which endpoint is slow
5. Switch to "Database Performance" dashboard
6. Check for slow queries or lock contention
7. Use findings to determine root cause

### Monitor Deployment
1. Set time range to deployment window
2. Watch error rate (should stay < 1%)
3. Monitor latency (should not spike)
4. Check pod status (should transition smoothly)
5. Verify new version deployed (check labels)

### Capacity Planning
1. Set time range to last 30 days
2. Check resource usage trends
3. Note growth patterns
4. Extrapolate future needs
5. Plan scaling actions

## Custom Dashboards

Create custom dashboard:

```json
{
  "dashboard": {
    "title": "Custom Workflow Dashboard",
    "panels": [
      {
        "title": "Workflow Completion Rate",
        "targets": [{
          "expr": "rate(workflow_completed_total[5m])"
        }]
      }
    ]
  }
}
```

---
**Last Updated**: 2025-11-14
**Version**: 1.0
