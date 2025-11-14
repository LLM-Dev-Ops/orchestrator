# Capacity Planning

## Overview
Plan and prepare for capacity growth based on usage trends and forecasts.

## Step-by-Step Procedure

### Analyze Current Capacity

```bash
# Check current resource utilization (30-day average)
# Prometheus query:
# avg_over_time(container_cpu_usage_seconds_total{pod=~"orchestrator-.*"}[30d])
# avg_over_time(container_memory_usage_bytes{pod=~"orchestrator-.*"}[30d])

# Check request volume trend
# rate(http_requests_total[30d])

# Check database size growth
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "
    SELECT pg_size_pretty(pg_database_size('orchestrator'));
  "
```

### Forecast Future Needs

```bash
# Calculate growth rate
# Current: 1000 req/min, Growth: 20%/month
# Forecast: Month 1: 1200, Month 2: 1440, Month 3: 1728

# Estimate resource needs
# Current: 3 pods @ 1 CPU each = 3 CPU total
# At 2x traffic: 6 pods or 3 pods @ 2 CPU each

# Database storage growth
# Current: 10GB, Growth: 2GB/month
# Forecast: Need 20GB in 5 months
```

### Plan Scaling Actions

```bash
# Horizontal scaling (add pods)
# Update HPA max replicas
kubectl patch hpa orchestrator -n llm-orchestrator -p '
{
  "spec": {
    "maxReplicas": 20
  }
}'

# Vertical scaling (bigger nodes)
# Add node pool with larger instance types
# AWS: c5.2xlarge -> c5.4xlarge

# Database scaling
# Increase PVC size
kubectl patch pvc postgres-pvc -n llm-orchestrator -p '
{
  "spec": {
    "resources": {
      "requests": {
        "storage": "50Gi"
      }
    }
  }
}'
```

### Budget Planning

```bash
# Current costs (estimate)
# - Compute: 3 pods × $50/month = $150
# - Database: 10GB × $0.10/GB = $1
# - Load balancer: $20/month
# Total: ~$171/month

# Forecasted costs (6 months)
# - Compute: 6 pods × $50/month = $300
# - Database: 20GB × $0.10/GB = $2
# - Load balancer: $20/month
# Total: ~$322/month

# Budget approval needed: $322 - $171 = $151 increase
```

## Documentation

Create capacity plan document with:
- Current utilization metrics
- Growth trends
- Forecasts (3, 6, 12 months)
- Scaling actions required
- Budget implications
- Risk assessment

---
**Last Updated**: 2025-11-14
**Version**: 1.0
