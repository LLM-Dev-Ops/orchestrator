# Scaling Operations

## Overview
Procedures for scaling the LLM Orchestrator horizontally (more pods) and vertically (more resources per pod) to handle increased load or optimize resource usage.

## Prerequisites
- Kubernetes namespace admin access
- Grafana/Prometheus access
- Understanding of application resource requirements
- Capacity planning data

## Impact Assessment
- Severity: Low to Medium
- User impact: None (if proactive), High (if reactive)
- Business impact: Ensures service availability under load

## Step-by-Step Procedure

### Horizontal Scaling (Add/Remove Pods)

#### Manual Scaling

```bash
# Scale up to 5 replicas
kubectl scale deployment orchestrator -n llm-orchestrator --replicas=5

# Watch scaling progress
kubectl rollout status deployment/orchestrator -n llm-orchestrator

# Verify new pod count
kubectl get pods -n llm-orchestrator -l app=orchestrator

# Monitor resource utilization
kubectl top pods -n llm-orchestrator -l app=orchestrator
```

#### Configure Horizontal Pod Autoscaler (HPA)

```bash
# Update HPA parameters
kubectl patch hpa orchestrator -n llm-orchestrator -p '
{
  "spec": {
    "minReplicas": 5,
    "maxReplicas": 20,
    "metrics": [
      {
        "type": "Resource",
        "resource": {
          "name": "cpu",
          "target": {
            "type": "Utilization",
            "averageUtilization": 60
          }
        }
      }
    ]
  }
}'

# Monitor HPA status
kubectl get hpa orchestrator -n llm-orchestrator -w
```

### Vertical Scaling (Increase Pod Resources)

```bash
# Update resource requests/limits
kubectl patch deployment orchestrator -n llm-orchestrator -p '
{
  "spec": {
    "template": {
      "spec": {
        "containers": [{
          "name": "orchestrator",
          "resources": {
            "requests": {
              "cpu": "1000m",
              "memory": "2Gi"
            },
            "limits": {
              "cpu": "2000m",
              "memory": "4Gi"
            }
          }
        }]
      }
    }
  }
}'

# This triggers a rolling restart
kubectl rollout status deployment/orchestrator -n llm-orchestrator
```

### Database Scaling

```bash
# Increase PostgreSQL resources
kubectl patch statefulset postgres -n llm-orchestrator -p '
{
  "spec": {
    "template": {
      "spec": {
        "containers": [{
          "name": "postgres",
          "resources": {
            "requests": {
              "cpu": "1000m",
              "memory": "2Gi"
            },
            "limits": {
              "cpu": "2000m",
              "memory": "4Gi"
            }
          }
        }]
      }
    }
  }
}'

# Increase connection pool size in application
kubectl patch configmap orchestrator-config -n llm-orchestrator --type merge -p '
{
  "data": {
    "config.yaml": "database:\n  max_connections: 50\n"
  }
}'
```

## Validation

```bash
# Verify scaling completed
kubectl get deployment orchestrator -n llm-orchestrator
kubectl get hpa orchestrator -n llm-orchestrator
kubectl top pods -n llm-orchestrator

# Check application responds normally
curl https://orchestrator.example.com/health
```

## Related Runbooks
- [02-rolling-update.md](./02-rolling-update.md)
- [../maintenance/07-performance-tuning.md](../maintenance/07-performance-tuning.md)
- [../maintenance/08-capacity-planning.md](../maintenance/08-capacity-planning.md)

---
**Last Updated**: 2025-11-14
**Version**: 1.0
