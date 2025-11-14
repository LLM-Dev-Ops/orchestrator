# LLM Orchestrator Helm Chart - Installation Examples

This document provides practical installation examples for various scenarios.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Development Installation](#development-installation)
3. [Production Installation](#production-installation)
4. [Custom Configuration](#custom-configuration)
5. [External Services](#external-services)
6. [Security Best Practices](#security-best-practices)

---

## Quick Start

Minimal installation with default values:

```bash
# Create namespace
kubectl create namespace llm-orchestrator

# Install with API keys
helm install my-orchestrator . \
  --namespace llm-orchestrator \
  --set-string llmProviders.anthropic.apiKey="sk-ant-api-..." \
  --set-string llmProviders.openai.apiKey="sk-..." \
  --set-string vectorDB.pinecone.apiKey="..."

# Wait for deployment
kubectl wait --for=condition=available --timeout=300s \
  deployment/my-orchestrator-llm-orchestrator \
  -n llm-orchestrator

# Run tests
helm test my-orchestrator -n llm-orchestrator
```

---

## Development Installation

Minimal resources for local development:

```bash
# Install with development values
helm install dev-orchestrator . \
  -f values.yaml \
  -f values-development.yaml \
  --namespace llm-orchestrator-dev \
  --create-namespace \
  --set-string llmProviders.anthropic.apiKey="sk-ant-api-..." \
  --set-string llmProviders.openai.apiKey="sk-..."

# Port-forward to access locally
kubectl port-forward -n llm-orchestrator-dev \
  svc/dev-orchestrator-llm-orchestrator 8080:8080

# Test connection
curl http://localhost:8080/health/live
```

**Development Configuration:**
- 1 replica (no autoscaling)
- No persistence (data lost on restart)
- Minimal resources (500m CPU, 1Gi RAM)
- Debug logging enabled
- No network policies
- No monitoring

---

## Production Installation

Full production deployment with high availability:

```bash
# Create production namespace
kubectl create namespace llm-orchestrator-prod

# Create secrets separately (recommended)
kubectl create secret generic llm-orchestrator-secrets \
  --namespace llm-orchestrator-prod \
  --from-literal=anthropic-api-key="sk-ant-api-..." \
  --from-literal=openai-api-key="sk-..." \
  --from-literal=pinecone-api-key="..."

# Install with production values
helm install prod-orchestrator . \
  -f values.yaml \
  -f values-production.yaml \
  --namespace llm-orchestrator-prod \
  --set global.domain=orchestrator.mycompany.com \
  --set ingress.hosts[0].host=orchestrator.mycompany.com \
  --set ingress.tls.secretName=orchestrator-tls

# Verify deployment
kubectl get all -n llm-orchestrator-prod

# Check HPA
kubectl get hpa -n llm-orchestrator-prod

# View logs
kubectl logs -n llm-orchestrator-prod \
  -l app.kubernetes.io/name=llm-orchestrator \
  --tail=100
```

**Production Configuration:**
- 5 replicas (autoscaling 5-20)
- Full persistence (50Gi PostgreSQL, 20Gi Redis)
- High resources (2 CPU, 4Gi RAM per pod)
- Production logging (JSON format)
- Network policies enabled
- Monitoring with Prometheus
- Pod Disruption Budget (min 3 available)
- Multi-zone topology spread

---

## Custom Configuration

### Using Custom values.yaml

Create a custom values file:

```yaml
# my-values.yaml
global:
  domain: orchestrator.acme.com
  environment: staging

replicaCount: 3

image:
  repository: gcr.io/my-project/llm-orchestrator
  tag: "v1.2.3"

resources:
  limits:
    cpu: 1500m
    memory: 3Gi
  requests:
    cpu: 750m
    memory: 1.5Gi

postgresql:
  primary:
    persistence:
      size: 30Gi
      storageClass: "standard-rwo"

redis:
  replica:
    replicaCount: 2

config:
  logLevel: info
  maxConcurrentWorkflows: 150

ingress:
  enabled: true
  hosts:
    - host: orchestrator.acme.com
      paths:
        - path: /
          pathType: Prefix
```

Install with custom values:

```bash
helm install my-orchestrator . \
  -f my-values.yaml \
  --namespace llm-orchestrator \
  --create-namespace
```

### Resource Sizing Examples

**Small (Development/Testing):**
```bash
helm install my-orchestrator . \
  --set replicaCount=1 \
  --set autoscaling.enabled=false \
  --set resources.limits.cpu=500m \
  --set resources.limits.memory=1Gi \
  --set postgresql.primary.persistence.size=5Gi \
  --set redis.master.persistence.size=2Gi
```

**Medium (Staging):**
```bash
helm install my-orchestrator . \
  --set replicaCount=3 \
  --set autoscaling.minReplicas=3 \
  --set autoscaling.maxReplicas=8 \
  --set resources.limits.cpu=1000m \
  --set resources.limits.memory=2Gi \
  --set postgresql.primary.persistence.size=20Gi
```

**Large (Production - High Traffic):**
```bash
helm install my-orchestrator . \
  --set replicaCount=5 \
  --set autoscaling.minReplicas=5 \
  --set autoscaling.maxReplicas=20 \
  --set resources.limits.cpu=2000m \
  --set resources.limits.memory=4Gi \
  --set postgresql.primary.persistence.size=100Gi \
  --set postgresql.readReplicas.replicaCount=3
```

---

## External Services

### Using External PostgreSQL

```bash
helm install my-orchestrator . \
  --set postgresql.enabled=false \
  --set postgresql.externalHost=postgres.mycompany.com \
  --set postgresql.externalPort=5432 \
  --set postgresql.externalDatabase=orchestrator \
  --set postgresql.externalUsername=orchestrator \
  --set postgresql.externalSecretName=postgres-credentials
```

Create the external secret:

```bash
kubectl create secret generic postgres-credentials \
  --namespace llm-orchestrator \
  --from-literal=password="your-postgres-password"
```

### Using External Redis

```bash
helm install my-orchestrator . \
  --set redis.enabled=false \
  --set redis.externalHost=redis.mycompany.com \
  --set redis.externalPort=6379 \
  --set redis.externalSecretName=redis-credentials
```

### Using External Vector Database

**Qdrant:**
```bash
helm install my-orchestrator . \
  --set vectorDB.type=qdrant \
  --set vectorDB.qdrant.url=http://qdrant.mycompany.com:6333
```

**Weaviate:**
```bash
helm install my-orchestrator . \
  --set vectorDB.type=weaviate \
  --set vectorDB.weaviate.url=http://weaviate.mycompany.com:8080
```

---

## Security Best Practices

### Using External Secrets Operator

1. Install External Secrets Operator:
```bash
helm repo add external-secrets https://charts.external-secrets.io
helm install external-secrets \
  external-secrets/external-secrets \
  -n external-secrets-system \
  --create-namespace
```

2. Create SecretStore (AWS Secrets Manager example):
```yaml
apiVersion: external-secrets.io/v1beta1
kind: SecretStore
metadata:
  name: aws-secrets
  namespace: llm-orchestrator
spec:
  provider:
    aws:
      service: SecretsManager
      region: us-east-1
      auth:
        jwt:
          serviceAccountRef:
            name: llm-orchestrator
```

3. Install with external secrets:
```bash
helm install my-orchestrator . \
  --set secrets.externalSecrets.enabled=true \
  --set secrets.externalSecrets.backendType=secretsManager \
  --set serviceAccount.annotations."eks\.amazonaws\.com/role-arn"="arn:aws:iam::123456789012:role/llm-orchestrator"
```

### Using HashiCorp Vault

```bash
helm install my-orchestrator . \
  --set secrets.type=vault \
  --set secrets.vault.enabled=true \
  --set secrets.vault.address=https://vault.mycompany.com \
  --set secrets.vault.role=llm-orchestrator \
  --set secrets.vault.secretPath=secret/data/llm-orchestrator
```

### Network Security

Enable all network security features:

```bash
helm install my-orchestrator . \
  --set networkPolicy.enabled=true \
  --set podSecurityContext.runAsNonRoot=true \
  --set securityContext.readOnlyRootFilesystem=true \
  --set ingress.annotations."nginx\.ingress\.kubernetes\.io/rate-limit"=100
```

### TLS/SSL Configuration

```bash
# Install cert-manager first
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml

# Create ClusterIssuer
kubectl apply -f - <<EOF
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: admin@mycompany.com
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
    - http01:
        ingress:
          class: nginx
EOF

# Install with TLS
helm install my-orchestrator . \
  --set ingress.enabled=true \
  --set ingress.className=nginx \
  --set ingress.tls.enabled=true \
  --set ingress.tls.secretName=orchestrator-tls \
  --set ingress.annotations."cert-manager\.io/cluster-issuer"=letsencrypt-prod \
  --set global.domain=orchestrator.mycompany.com
```

---

## Monitoring and Observability

### Prometheus Integration

```bash
# Install Prometheus Operator
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm install prometheus prometheus-community/kube-prometheus-stack \
  -n monitoring \
  --create-namespace

# Install orchestrator with monitoring
helm install my-orchestrator . \
  --set monitoring.enabled=true \
  --set monitoring.serviceMonitor.enabled=true \
  --set monitoring.serviceMonitor.additionalLabels.release=prometheus
```

Access Prometheus:
```bash
kubectl port-forward -n monitoring svc/prometheus-kube-prometheus-prometheus 9090:9090
# Open http://localhost:9090
```

Access Grafana:
```bash
kubectl port-forward -n monitoring svc/prometheus-grafana 3000:80
# Open http://localhost:3000 (admin/prom-operator)
```

---

## Upgrade Examples

### Zero-Downtime Upgrade

```bash
# Upgrade to new version
helm upgrade my-orchestrator . \
  --namespace llm-orchestrator \
  --reuse-values \
  --wait

# Monitor rollout
kubectl rollout status deployment/my-orchestrator-llm-orchestrator \
  -n llm-orchestrator
```

### Upgrade with New Values

```bash
helm upgrade my-orchestrator . \
  --namespace llm-orchestrator \
  -f values.yaml \
  -f my-new-values.yaml \
  --wait

# Rollback if needed
helm rollback my-orchestrator -n llm-orchestrator
```

---

## Troubleshooting Commands

```bash
# Check all resources
kubectl get all -n llm-orchestrator

# Check pod status
kubectl describe pod <pod-name> -n llm-orchestrator

# View logs
kubectl logs -f deployment/my-orchestrator-llm-orchestrator \
  -n llm-orchestrator

# Check events
kubectl get events -n llm-orchestrator --sort-by='.lastTimestamp'

# Test connectivity
kubectl run -it --rm debug --image=curlimages/curl --restart=Never -- \
  curl http://my-orchestrator-llm-orchestrator:8080/health/live

# Check secrets
kubectl get secrets -n llm-orchestrator
kubectl describe secret llm-orchestrator-secrets -n llm-orchestrator

# Validate Helm release
helm list -n llm-orchestrator
helm status my-orchestrator -n llm-orchestrator
helm get values my-orchestrator -n llm-orchestrator
```

---

## Complete Production Example

Here's a complete production installation with all features:

```bash
#!/bin/bash
set -e

NAMESPACE="llm-orchestrator-prod"
RELEASE="prod-orchestrator"
DOMAIN="orchestrator.mycompany.com"

# 1. Create namespace
kubectl create namespace $NAMESPACE || true

# 2. Create secrets
kubectl create secret generic llm-orchestrator-secrets \
  --namespace $NAMESPACE \
  --from-literal=anthropic-api-key="$ANTHROPIC_API_KEY" \
  --from-literal=openai-api-key="$OPENAI_API_KEY" \
  --from-literal=pinecone-api-key="$PINECONE_API_KEY" \
  --dry-run=client -o yaml | kubectl apply -f -

# 3. Install Helm chart
helm upgrade --install $RELEASE . \
  --namespace $NAMESPACE \
  -f values.yaml \
  -f values-production.yaml \
  --set global.domain=$DOMAIN \
  --set ingress.hosts[0].host=$DOMAIN \
  --set ingress.tls.secretName=orchestrator-tls \
  --set postgresql.primary.persistence.storageClass=fast-ssd \
  --set redis.master.persistence.storageClass=fast-ssd \
  --wait \
  --timeout 10m

# 4. Wait for deployment
kubectl wait --for=condition=available --timeout=300s \
  deployment/$RELEASE-llm-orchestrator \
  -n $NAMESPACE

# 5. Run tests
helm test $RELEASE -n $NAMESPACE

# 6. Show status
echo "Installation complete!"
echo "Access at: https://$DOMAIN"
kubectl get all -n $NAMESPACE
```

---

For more information, see the [main README](README.md).
