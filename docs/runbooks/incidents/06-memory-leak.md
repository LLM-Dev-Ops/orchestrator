# Memory Leak Investigation

## Overview
Memory consumption growing over time, eventually leading to OOMKilled pods.

## Symptoms
- Pod memory usage steadily increasing
- Pods being OOMKilled
- Swap usage increasing
- Slower performance over time

## Impact Assessment
- Severity: Medium (P2)
- User impact: Eventual service degradation
- Business impact: Requires frequent restarts

## Step-by-Step Procedure

### Step 1: Confirm Memory Leak

```bash
# Check current memory usage
kubectl top pods -n llm-orchestrator -l app=orchestrator

# Compare with historical data (Grafana)
# Query: container_memory_usage_bytes{pod=~"orchestrator-.*"}
# Look for steady increase over hours/days

# Check OOMKill events
kubectl get events -n llm-orchestrator | grep OOMKilled
```

### Step 2: Identify Leaking Component

```bash
# Get detailed memory stats
kubectl exec -n llm-orchestrator <pod-name> -- cat /proc/1/status | grep -E "VmSize|VmRSS|VmData"

# If Rust app, check allocator stats
kubectl logs -n llm-orchestrator <pod-name> | grep "memory"

# Check for growing data structures
# Review application metrics for:
# - Connection pool size
# - Cache size
# - Workflow queue depth
```

### Step 3: Collect Diagnostic Data

```bash
# Take heap snapshot (if supported)
kubectl exec -n llm-orchestrator <pod-name> -- kill -USR1 1

# Download core dump
kubectl cp llm-orchestrator/<pod-name>:/tmp/core.dump ./core.dump

# Analyze with debugger
# rust-gdb core.dump
```

### Step 4: Immediate Mitigation

```bash
# Restart leaking pod
kubectl delete pod <pod-name> -n llm-orchestrator

# Scale horizontally to spread load
kubectl scale deployment orchestrator -n llm-orchestrator --replicas=6

# Reduce memory limits if OOMKilling too frequently
kubectl patch deployment orchestrator -n llm-orchestrator -p '
{
  "spec": {
    "template": {
      "spec": {
        "containers": [{
          "name": "orchestrator",
          "resources": {
            "limits": {
              "memory": "4Gi"
            }
          }
        }]
      }
    }
  }
}'
```

### Step 5: Implement Workaround

```bash
# Set up automatic pod restart when memory high
# Create monitoring alert to restart pods at 80% memory

# Schedule periodic pod restarts (temporary)
# Add CronJob to restart deployment daily
cat <<EOF | kubectl apply -f -
apiVersion: batch/v1
kind: CronJob
metadata:
  name: orchestrator-restart
  namespace: llm-orchestrator
spec:
  schedule: "0 2 * * *"  # 2 AM daily
  jobTemplate:
    spec:
      template:
        spec:
          serviceAccountName: pod-restarter
          containers:
          - name: kubectl
            image: bitnami/kubectl
            command:
            - /bin/sh
            - -c
            - kubectl rollout restart deployment/orchestrator -n llm-orchestrator
          restartPolicy: OnFailure
EOF
```

## Validation

```bash
# Memory stable after restart
kubectl top pods -n llm-orchestrator -l app=orchestrator
# Monitor for 1 hour - should stay flat

# No OOMKills
kubectl get events -n llm-orchestrator | grep OOMKilled
```

## Prevention
- Profile memory usage in development
- Implement connection/cache limits
- Regular load testing
- Memory leak detection in CI/CD

## Related Runbooks
- [01-high-latency.md](./01-high-latency.md)
- [../maintenance/07-performance-tuning.md](../maintenance/07-performance-tuning.md)

---
**Last Updated**: 2025-11-14
**Version**: 1.0
