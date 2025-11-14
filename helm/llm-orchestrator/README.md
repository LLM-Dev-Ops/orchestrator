# LLM Orchestrator Helm Chart

Production-ready Helm chart for deploying the LLM Orchestrator to Kubernetes clusters.

## Overview

The LLM Orchestrator is an enterprise-grade workflow orchestration platform for Large Language Models (LLMs). This Helm chart provides a complete deployment solution with:

- **Multi-provider LLM support**: Anthropic Claude, OpenAI GPT, Cohere
- **State persistence**: PostgreSQL with replication
- **Caching layer**: Redis cluster
- **Vector database integration**: Pinecone, Qdrant, Weaviate
- **High availability**: Horizontal Pod Autoscaling, Pod Disruption Budgets
- **Security**: Network policies, non-root containers, read-only filesystems
- **Observability**: Prometheus metrics, health checks, structured logging
- **Production-ready**: 234+ tests, zero compilation errors

## Prerequisites

- Kubernetes 1.24+
- Helm 3.8+
- PV provisioner support in the underlying infrastructure (for persistence)
- (Optional) Prometheus Operator for ServiceMonitor support
- (Optional) cert-manager for TLS certificate management
- (Optional) Ingress controller (nginx, traefik, etc.)

## Installation

### Quick Start

```bash
# Add the Helm repository (when published)
helm repo add llm-orchestrator https://charts.llm-devops.io
helm repo update

# Install with default values
helm install my-orchestrator llm-orchestrator/llm-orchestrator

# Install with custom values
helm install my-orchestrator llm-orchestrator/llm-orchestrator \
  --set global.domain=orchestrator.mycompany.com \
  --set-string llmProviders.anthropic.apiKey="sk-ant-..." \
  --set-string llmProviders.openai.apiKey="sk-..." \
  --namespace llm-orchestrator \
  --create-namespace
```

### Install from Source

```bash
# Clone the repository
git clone https://github.com/llm-devops/llm-orchestrator.git
cd llm-orchestrator/helm/llm-orchestrator

# Install the chart
helm install my-orchestrator . \
  --namespace llm-orchestrator \
  --create-namespace \
  --set-string llmProviders.anthropic.apiKey="your-key-here" \
  --set-string llmProviders.openai.apiKey="your-key-here"
```

### Development Installation

For development/testing with minimal resources:

```bash
helm install my-orchestrator . \
  --set replicaCount=1 \
  --set autoscaling.enabled=false \
  --set postgresql.primary.persistence.enabled=false \
  --set redis.master.persistence.enabled=false \
  --set monitoring.enabled=false \
  --namespace llm-orchestrator-dev \
  --create-namespace
```

## Configuration

### Key Configuration Options

| Parameter | Description | Default |
|-----------|-------------|---------|
| `replicaCount` | Number of orchestrator replicas | `3` |
| `image.repository` | Container image repository | `ghcr.io/llm-devops/llm-orchestrator` |
| `image.tag` | Container image tag | `Chart appVersion` |
| `ingress.enabled` | Enable ingress controller | `true` |
| `ingress.className` | Ingress class name | `nginx` |
| `ingress.hosts[0].host` | Hostname for ingress | `orchestrator.example.com` |
| `ingress.tls.enabled` | Enable TLS for ingress | `true` |
| `postgresql.enabled` | Enable PostgreSQL deployment | `true` |
| `postgresql.primary.persistence.size` | PostgreSQL storage size | `10Gi` |
| `redis.enabled` | Enable Redis deployment | `true` |
| `autoscaling.enabled` | Enable horizontal pod autoscaling | `true` |
| `autoscaling.minReplicas` | Minimum number of replicas | `2` |
| `autoscaling.maxReplicas` | Maximum number of replicas | `10` |
| `monitoring.serviceMonitor.enabled` | Enable Prometheus ServiceMonitor | `true` |
| `networkPolicy.enabled` | Enable network policies | `true` |

### LLM Provider Configuration

Configure your LLM provider API keys:

```bash
# Using --set-string (recommended for CLI)
helm install my-orchestrator . \
  --set-string llmProviders.anthropic.apiKey="sk-ant-api-..." \
  --set-string llmProviders.openai.apiKey="sk-..." \
  --set-string vectorDB.pinecone.apiKey="..."

# Using values file
cat > my-values.yaml <<EOF
llmProviders:
  anthropic:
    enabled: true
    apiKey: "sk-ant-api-..."
  openai:
    enabled: true
    apiKey: "sk-..."
  cohere:
    enabled: false

vectorDB:
  type: pinecone
  pinecone:
    apiKey: "..."
    environment: "us-east-1-gcp"
    indexName: "llm-orchestrator"
EOF

helm install my-orchestrator . -f my-values.yaml
```

### Using External Secrets

For production deployments, use an external secrets manager:

```yaml
# values-production.yaml
secrets:
  externalSecrets:
    enabled: true
    backendType: secretsManager  # AWS Secrets Manager
    roleArn: "arn:aws:iam::123456789012:role/llm-orchestrator"

llmProviders:
  anthropic:
    enabled: true
    apiKeySecret: "llm-orchestrator-secrets"  # Pre-existing secret
    apiKeySecretKey: "anthropic-api-key"
  openai:
    enabled: true
    apiKeySecret: "llm-orchestrator-secrets"
    apiKeySecretKey: "openai-api-key"
```

### Resource Profiles

Configure resources based on your workload:

**Small** (Development/Testing):
```yaml
resources:
  limits:
    cpu: 500m
    memory: 1Gi
  requests:
    cpu: 250m
    memory: 512Mi
replicaCount: 1
autoscaling:
  enabled: false
```

**Medium** (Default - Production):
```yaml
resources:
  limits:
    cpu: 1000m
    memory: 2Gi
  requests:
    cpu: 500m
    memory: 1Gi
replicaCount: 3
autoscaling:
  enabled: true
  minReplicas: 2
  maxReplicas: 10
```

**Large** (High Traffic):
```yaml
resources:
  limits:
    cpu: 2000m
    memory: 4Gi
  requests:
    cpu: 1000m
    memory: 2Gi
replicaCount: 5
autoscaling:
  enabled: true
  minReplicas: 5
  maxReplicas: 20
```

### PostgreSQL Configuration

**Using built-in PostgreSQL**:
```yaml
postgresql:
  enabled: true
  auth:
    username: orchestrator
    password: ""  # Auto-generated
    database: orchestrator
  primary:
    persistence:
      enabled: true
      size: 20Gi
      storageClass: "fast-ssd"
  readReplicas:
    replicaCount: 2
```

**Using external PostgreSQL**:
```yaml
postgresql:
  enabled: false
  externalHost: "postgres.mycompany.com"
  externalPort: 5432
  externalDatabase: "orchestrator"
  externalUsername: "orchestrator"
  externalSecretName: "postgres-credentials"
```

### High Availability Configuration

```yaml
# Multiple replicas with anti-affinity
replicaCount: 3
affinity:
  podAntiAffinity:
    requiredDuringSchedulingIgnoredDuringExecution:
      - labelSelector:
          matchLabels:
            app.kubernetes.io/name: llm-orchestrator
        topologyKey: kubernetes.io/hostname

# Pod Disruption Budget
podDisruptionBudget:
  enabled: true
  minAvailable: 2

# Autoscaling
autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 10
  targetCPUUtilizationPercentage: 70

# Topology spread
topologySpreadConstraints:
  - maxSkew: 1
    topologyKey: topology.kubernetes.io/zone
    whenUnsatisfiable: DoNotSchedule
    labelSelector:
      matchLabels:
        app.kubernetes.io/name: llm-orchestrator
```

## Upgrading

### Standard Upgrade

```bash
# Update Helm repository
helm repo update

# Upgrade to latest version
helm upgrade my-orchestrator llm-orchestrator/llm-orchestrator \
  --namespace llm-orchestrator \
  --reuse-values

# Upgrade with new values
helm upgrade my-orchestrator llm-orchestrator/llm-orchestrator \
  --namespace llm-orchestrator \
  -f my-values.yaml
```

### Zero-Downtime Upgrade

The chart is configured for rolling updates by default:

```yaml
strategy:
  type: RollingUpdate
  rollingUpdate:
    maxSurge: 1
    maxUnavailable: 0
```

This ensures:
- New pods start before old ones terminate
- No downtime during upgrades
- Automatic rollback on failure

### Rollback

```bash
# View release history
helm history my-orchestrator -n llm-orchestrator

# Rollback to previous version
helm rollback my-orchestrator -n llm-orchestrator

# Rollback to specific revision
helm rollback my-orchestrator 2 -n llm-orchestrator
```

## Monitoring

### Prometheus Integration

If you have Prometheus Operator installed:

```yaml
monitoring:
  enabled: true
  serviceMonitor:
    enabled: true
    interval: 30s
    additionalLabels:
      prometheus: kube-prometheus
```

Access metrics at: `http://<service>:9090/metrics`

### Available Metrics

- `llm_orchestrator_workflows_total` - Total workflows executed
- `llm_orchestrator_workflow_duration_seconds` - Workflow execution time
- `llm_orchestrator_step_duration_seconds` - Step execution time
- `llm_orchestrator_provider_requests_total` - LLM provider requests
- `llm_orchestrator_errors_total` - Total errors
- `llm_orchestrator_state_operations_total` - State persistence operations

### Health Checks

```bash
# Liveness probe
curl http://orchestrator.example.com/health/live

# Readiness probe
curl http://orchestrator.example.com/health/ready

# Detailed health
kubectl port-forward svc/my-orchestrator 8080:8080
curl http://localhost:8080/health
```

## Security

### Network Policies

Network policies are enabled by default and restrict:
- **Ingress**: Only from ingress controller and Prometheus
- **Egress**: Only to PostgreSQL, Redis, DNS, and HTTPS (LLM APIs)

```yaml
networkPolicy:
  enabled: true
  ingress:
    - from:
      - namespaceSelector:
          matchLabels:
            name: ingress-nginx
      ports:
      - protocol: TCP
        port: 8080
```

### Pod Security

Runs as non-root user with restricted permissions:

```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 1000
  allowPrivilegeEscalation: false
  readOnlyRootFilesystem: true
  capabilities:
    drop:
    - ALL
```

### Secrets Management

**Option 1: Kubernetes Secrets** (default)
```bash
helm install my-orchestrator . \
  --set-string llmProviders.anthropic.apiKey="sk-..."
```

**Option 2: External Secrets Operator**
```yaml
secrets:
  externalSecrets:
    enabled: true
    backendType: secretsManager
```

**Option 3: HashiCorp Vault**
```yaml
secrets:
  type: vault
  vault:
    enabled: true
    address: "https://vault.example.com"
    role: "llm-orchestrator"
```

## Troubleshooting

### Pods Not Starting

```bash
# Check pod status
kubectl get pods -n llm-orchestrator

# View pod events
kubectl describe pod <pod-name> -n llm-orchestrator

# Check logs
kubectl logs <pod-name> -n llm-orchestrator

# Common issues:
# - Missing secrets: Ensure API keys are configured
# - Database connection: Check PostgreSQL is running
# - Resource limits: Verify node has sufficient resources
```

### Database Connection Issues

```bash
# Test PostgreSQL connectivity
kubectl run -it --rm psql-test --image=postgres:15 --restart=Never -- \
  psql -h my-orchestrator-postgresql -U orchestrator -d orchestrator

# Check PostgreSQL logs
kubectl logs my-orchestrator-postgresql-0 -n llm-orchestrator
```

### Performance Issues

```bash
# Check resource usage
kubectl top pods -n llm-orchestrator

# View HPA status
kubectl get hpa -n llm-orchestrator

# Check if autoscaling is working
kubectl describe hpa my-orchestrator -n llm-orchestrator
```

### Ingress Not Working

```bash
# Check ingress status
kubectl get ingress -n llm-orchestrator

# Describe ingress
kubectl describe ingress my-orchestrator -n llm-orchestrator

# Check ingress controller logs
kubectl logs -n ingress-nginx -l app.kubernetes.io/name=ingress-nginx
```

## Uninstallation

```bash
# Uninstall the release
helm uninstall my-orchestrator -n llm-orchestrator

# Delete the namespace (optional)
kubectl delete namespace llm-orchestrator

# Note: PVCs are not deleted automatically
# Delete them manually if needed:
kubectl delete pvc -n llm-orchestrator --all
```

## Testing

Run Helm tests to validate the installation:

```bash
helm test my-orchestrator -n llm-orchestrator
```

The test will:
1. Check service connectivity
2. Verify health endpoints are accessible
3. Validate basic functionality

## Values Reference

For a complete list of configuration options, see:
- [values.yaml](values.yaml) - Full configuration with defaults
- [Chart.yaml](Chart.yaml) - Chart metadata and dependencies

### Complete Values Example

```yaml
# values-production.yaml
global:
  domain: orchestrator.production.com
  environment: production

replicaCount: 5

image:
  repository: ghcr.io/mycompany/llm-orchestrator
  tag: "v1.0.0"
  pullPolicy: IfNotPresent

ingress:
  enabled: true
  className: nginx
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
  hosts:
    - host: orchestrator.production.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    enabled: true
    secretName: orchestrator-tls

resources:
  limits:
    cpu: 2000m
    memory: 4Gi
  requests:
    cpu: 1000m
    memory: 2Gi

autoscaling:
  enabled: true
  minReplicas: 5
  maxReplicas: 20
  targetCPUUtilizationPercentage: 70

postgresql:
  enabled: true
  primary:
    persistence:
      enabled: true
      size: 50Gi
      storageClass: fast-ssd
  readReplicas:
    replicaCount: 2

redis:
  enabled: true
  architecture: replication
  replica:
    replicaCount: 3

monitoring:
  enabled: true
  serviceMonitor:
    enabled: true
    interval: 15s

networkPolicy:
  enabled: true

podDisruptionBudget:
  enabled: true
  minAvailable: 3
```

## Support

- **Documentation**: https://docs.llm-devops.io/orchestrator
- **GitHub**: https://github.com/llm-devops/llm-orchestrator
- **Issues**: https://github.com/llm-devops/llm-orchestrator/issues
- **Discussions**: https://github.com/llm-devops/llm-orchestrator/discussions

## Contributing

We welcome contributions! Please see our [Contributing Guide](../../CONTRIBUTING.md) for details.

## License

This Helm chart is licensed under MIT OR Apache-2.0, the same as the LLM Orchestrator project.

## Changelog

### Version 1.0.0 (2025-11-14)

- Initial release
- PostgreSQL state persistence
- Redis caching support
- Multi-provider LLM support (Anthropic, OpenAI, Cohere)
- Vector database integration (Pinecone, Qdrant, Weaviate)
- Horizontal Pod Autoscaling
- Prometheus ServiceMonitor
- Network policies
- Pod Disruption Budgets
- Comprehensive health checks
- Production-ready defaults
