# Network Connectivity Issues

## Overview
Network connectivity problems between services or to external APIs.

## Symptoms
- "connection refused" or "timeout" errors
- Intermittent connectivity
- DNS resolution failures
- High packet loss

## Step-by-Step Procedure

### Step 1: Test Connectivity

```bash
# Test pod-to-pod connectivity
kubectl run -it --rm nettest --image=nicolaka/netshoot --restart=Never -- \
  ping orchestrator.llm-orchestrator.svc.cluster.local

# Test DNS resolution
kubectl run -it --rm nettest --image=nicolaka/netshoot --restart=Never -- \
  nslookup postgres.llm-orchestrator.svc.cluster.local

# Test external connectivity
kubectl run -it --rm nettest --image=nicolaka/netshoot --restart=Never -- \
  curl -v https://api.openai.com/v1/models
```

### Step 2: Check Network Policies

```bash
# List network policies
kubectl get networkpolicies -n llm-orchestrator

# Check if blocking traffic
kubectl describe networkpolicy -n llm-orchestrator
```

### Step 3: Check Service/Endpoints

```bash
# Verify service has endpoints
kubectl get endpoints orchestrator -n llm-orchestrator

# If no endpoints, pods may not be ready
kubectl get pods -n llm-orchestrator -l app=orchestrator -o wide
```

### Step 4: Check CoreDNS

```bash
# Check CoreDNS pods
kubectl get pods -n kube-system -l k8s-app=kube-dns

# Check CoreDNS logs
kubectl logs -n kube-system -l k8s-app=kube-dns --tail=100
```

## Validation

```bash
# All connectivity tests pass
# Services have endpoints
# DNS resolving correctly
```

## Related Runbooks
- [02-service-unavailable.md](./02-service-unavailable.md)
- [05-multi-region-deployment.md](../deployment/05-multi-region-deployment.md)

---
**Last Updated**: 2025-11-14
**Version**: 1.0
