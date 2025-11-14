# LLM-Orchestrator Deployment Guide

**Version:** 1.0
**Last Updated:** 2025-11-14

---

## Table of Contents

1. [Deployment Modes](#deployment-modes)
2. [Infrastructure Requirements](#infrastructure-requirements)
3. [Configuration](#configuration)
4. [Single-Node Deployment](#single-node-deployment)
5. [Distributed Deployment](#distributed-deployment)
6. [Kubernetes Deployment](#kubernetes-deployment)
7. [Monitoring and Observability](#monitoring-and-observability)
8. [Security](#security)
9. [Scaling Guidelines](#scaling-guidelines)
10. [Troubleshooting](#troubleshooting)

---

## Deployment Modes

### Mode Comparison

| Mode | Use Case | Complexity | Scalability | Cost |
|------|----------|------------|-------------|------|
| **CLI** | Development, testing, small-scale | Low | Limited | Minimal |
| **Single-Node** | Small production, edge deployment | Medium | Vertical | Low |
| **Distributed** | Production, high availability | High | Horizontal | Medium |
| **Kubernetes** | Cloud-native, enterprise | High | Auto-scaling | Variable |
| **Hybrid** | Edge + cloud coordination | Very High | Multi-tier | High |

### Decision Matrix

```
Choose CLI if:
  ✓ Running workflows locally
  ✓ Development and testing
  ✓ One-off workflow executions
  ✓ Minimal infrastructure

Choose Single-Node if:
  ✓ Small-scale production (<100 workflows/day)
  ✓ Edge deployment
  ✓ Limited infrastructure
  ✓ Simple operations

Choose Distributed if:
  ✓ High throughput (>1000 workflows/day)
  ✓ High availability required
  ✓ Multiple workflow types
  ✓ Team collaboration

Choose Kubernetes if:
  ✓ Cloud-native infrastructure
  ✓ Auto-scaling needed
  ✓ Multi-tenancy
  ✓ Enterprise deployment
```

---

## Infrastructure Requirements

### Minimum Requirements (CLI/Single-Node)

```yaml
Hardware:
  CPU: 2 cores
  Memory: 4 GB RAM
  Storage: 20 GB SSD
  Network: 100 Mbps

Software:
  OS: Linux (Ubuntu 22.04+, RHEL 8+) / macOS 12+ / Windows 11+
  Runtime: N/A (standalone binary)
  Database: SQLite (bundled)
```

### Recommended Requirements (Distributed)

```yaml
API Servers (3 nodes):
  CPU: 4 cores
  Memory: 8 GB RAM
  Storage: 50 GB SSD
  Network: 1 Gbps

Worker Nodes (5+ nodes):
  CPU: 8 cores
  Memory: 16 GB RAM
  Storage: 100 GB SSD
  Network: 1 Gbps

Database:
  PostgreSQL 14+
  CPU: 4 cores
  Memory: 16 GB RAM
  Storage: 500 GB SSD (RAID 10)

Cache (Redis):
  CPU: 2 cores
  Memory: 8 GB RAM
  Storage: 50 GB SSD

Message Queue (NATS):
  CPU: 2 cores
  Memory: 4 GB RAM
  Storage: 100 GB SSD
```

### Kubernetes Requirements

```yaml
Cluster:
  Nodes: 3+ (HA control plane)
  Node Size: 4 CPU, 16 GB RAM minimum
  Total Capacity: 20+ CPU, 64+ GB RAM
  Storage Class: SSD-backed (for databases)

Kubernetes Version: 1.26+

Required Add-ons:
  - Ingress Controller (nginx/traefik)
  - Cert Manager (for TLS)
  - Metrics Server
  - Prometheus Operator (monitoring)
```

---

## Configuration

### Configuration File Structure

```yaml
# config/orchestrator.yaml

# Server configuration
server:
  mode: api  # api, worker, standalone
  host: 0.0.0.0
  port:
    http: 8080
    grpc: 50051
  tls:
    enabled: true
    cert_file: /etc/certs/tls.crt
    key_file: /etc/certs/tls.key

# Database configuration
database:
  type: postgresql  # postgresql, sqlite
  url: postgresql://user:pass@localhost:5432/orchestrator
  pool:
    max_connections: 20
    min_connections: 5
    connection_timeout: 30s
  migrations:
    auto_run: true

# Cache configuration
cache:
  type: redis  # redis, memory
  url: redis://localhost:6379
  pool_size: 10
  ttl: 3600s

# Execution configuration
execution:
  scheduler:
    max_parallel_workflows: 100
    task_queue_size: 1000
    scheduling_interval: 1s

  executor:
    worker_threads: 8
    max_blocking_threads: 100
    task_timeout: 3600s

  resources:
    cpu_limit: 8.0
    memory_limit: 16Gi
    gpu_limit: 0

# Fault tolerance
fault_tolerance:
  checkpoint:
    enabled: true
    interval: 60s
    storage: s3://checkpoints/

  retry:
    default_max_attempts: 3
    default_backoff_multiplier: 2.0
    default_initial_interval: 1s
    default_max_interval: 30s

  circuit_breaker:
    enabled: true
    failure_threshold: 5
    timeout: 60s

# Integrations
integrations:
  llm_forge:
    endpoint: http://llm-forge:8080
    api_keys:
      anthropic: ${ANTHROPIC_API_KEY}
      openai: ${OPENAI_API_KEY}
    timeout: 60s

  llm_test_bench:
    endpoint: http://test-bench:8081
    auto_evaluate: true

  llm_auto_optimizer:
    endpoint: http://auto-optimizer:8082
    batch_size: 100
    flush_interval: 30s

  llm_governance:
    endpoint: http://governance:8083
    enforce_policies: true

# Observability
observability:
  logging:
    level: info  # trace, debug, info, warn, error
    format: json
    output: stdout

  metrics:
    enabled: true
    endpoint: /metrics
    port: 9090

  tracing:
    enabled: true
    exporter: jaeger
    endpoint: http://jaeger:14268/api/traces
    sample_rate: 0.1

# Security
security:
  authentication:
    enabled: true
    provider: jwt  # jwt, oauth2, api-key
    jwt_secret: ${JWT_SECRET}

  authorization:
    enabled: true
    policy_file: /etc/policies/rbac.yaml

  audit:
    enabled: true
    level: full  # minimal, standard, full
    storage: database
```

### Environment Variables

```bash
# Database
export DATABASE_URL="postgresql://user:pass@localhost:5432/orchestrator"

# Cache
export REDIS_URL="redis://localhost:6379"

# API Keys (Integrations)
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-..."

# Security
export JWT_SECRET="your-secret-key"
export TLS_CERT_FILE="/etc/certs/tls.crt"
export TLS_KEY_FILE="/etc/certs/tls.key"

# Observability
export JAEGER_ENDPOINT="http://jaeger:14268/api/traces"
export PROMETHEUS_ENDPOINT="http://prometheus:9090"

# Deployment
export ORCHESTRATOR_MODE="api"  # api, worker, standalone
export RUST_LOG="info"
export RUST_BACKTRACE=1
```

---

## Single-Node Deployment

### Quick Start

```bash
# 1. Download binary
wget https://github.com/llm-orchestrator/releases/latest/llm-orchestrator
chmod +x llm-orchestrator

# 2. Initialize database
./llm-orchestrator db init

# 3. Start server
./llm-orchestrator server --config config.yaml
```

### Systemd Service

```ini
# /etc/systemd/system/llm-orchestrator.service

[Unit]
Description=LLM Orchestrator
After=network.target

[Service]
Type=simple
User=orchestrator
Group=orchestrator
WorkingDirectory=/opt/llm-orchestrator
ExecStart=/opt/llm-orchestrator/bin/llm-orchestrator server --config /etc/llm-orchestrator/config.yaml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Environment
Environment="DATABASE_URL=postgresql://localhost/orchestrator"
Environment="RUST_LOG=info"

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/llm-orchestrator

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start service
sudo systemctl enable llm-orchestrator
sudo systemctl start llm-orchestrator
sudo systemctl status llm-orchestrator
```

### Docker Deployment

```dockerfile
# Dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/llm-orchestrator /usr/local/bin/

EXPOSE 8080 50051 9090

ENTRYPOINT ["/usr/local/bin/llm-orchestrator"]
CMD ["server", "--config", "/etc/config/orchestrator.yaml"]
```

```yaml
# docker-compose.yaml
version: '3.8'

services:
  orchestrator:
    build: .
    ports:
      - "8080:8080"   # HTTP API
      - "50051:50051" # gRPC API
      - "9090:9090"   # Metrics
    environment:
      - DATABASE_URL=postgresql://postgres:password@db:5432/orchestrator
      - REDIS_URL=redis://redis:6379
      - RUST_LOG=info
    volumes:
      - ./config:/etc/config
      - ./workflows:/var/lib/orchestrator/workflows
    depends_on:
      - db
      - redis
    restart: unless-stopped

  db:
    image: postgres:15
    environment:
      - POSTGRES_DB=orchestrator
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: unless-stopped

  redis:
    image: redis:7
    volumes:
      - redis_data:/data
    restart: unless-stopped

volumes:
  postgres_data:
  redis_data:
```

```bash
# Deploy with Docker Compose
docker-compose up -d

# View logs
docker-compose logs -f orchestrator

# Scale workers
docker-compose up -d --scale worker=5
```

---

## Distributed Deployment

### Architecture

```
                    ┌─────────────────┐
                    │  Load Balancer  │
                    │   (HAProxy)     │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
         ┌────▼────┐    ┌────▼────┐   ┌────▼────┐
         │ API     │    │ API     │   │ API     │
         │ Server 1│    │ Server 2│   │ Server 3│
         └────┬────┘    └────┬────┘   └────┬────┘
              │              │              │
              └──────────────┼──────────────┘
                             │
                    ┌────────▼────────┐
                    │  Message Queue  │
                    │     (NATS)      │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┬──────────────┐
              │              │              │              │
         ┌────▼────┐    ┌────▼────┐   ┌────▼────┐   ┌────▼────┐
         │ Worker  │    │ Worker  │   │ Worker  │   │ Worker  │
         │  Node 1 │    │  Node 2 │   │  Node 3 │   │  Node 4 │
         └────┬────┘    └────┬────┘   └────┬────┘   └────┬────┘
              │              │              │              │
              └──────────────┼──────────────┴──────────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
         ┌────▼────┐    ┌────▼────┐   ┌────▼────┐
         │ Postgres│    │  Redis  │   │   S3    │
         │(Primary)│    │         │   │(Storage)│
         └────┬────┘    └─────────┘   └─────────┘
              │
         ┌────▼────┐
         │ Postgres│
         │(Replica)│
         └─────────┘
```

### HAProxy Configuration

```
# /etc/haproxy/haproxy.cfg

global
    log /dev/log local0
    maxconn 4096
    daemon

defaults
    log global
    mode http
    option httplog
    timeout connect 5000
    timeout client 50000
    timeout server 50000

# HTTP API Load Balancing
frontend http_front
    bind *:80
    bind *:443 ssl crt /etc/ssl/certs/orchestrator.pem
    redirect scheme https if !{ ssl_fc }
    default_backend http_back

backend http_back
    balance roundrobin
    option httpchk GET /health
    http-check expect status 200
    server api1 10.0.1.10:8080 check
    server api2 10.0.1.11:8080 check
    server api3 10.0.1.12:8080 check

# gRPC Load Balancing
frontend grpc_front
    bind *:50051 ssl crt /etc/ssl/certs/orchestrator.pem alpn h2
    default_backend grpc_back

backend grpc_back
    balance leastconn
    option httpchk GET /health
    server api1 10.0.1.10:50051 check proto h2
    server api2 10.0.1.11:50051 check proto h2
    server api3 10.0.1.12:50051 check proto h2

# Metrics
listen stats
    bind *:9000
    stats enable
    stats uri /stats
    stats refresh 30s
```

### PostgreSQL HA Setup

```yaml
# Patroni configuration for PostgreSQL HA
scope: orchestrator
namespace: /service/
name: postgres1

restapi:
  listen: 0.0.0.0:8008
  connect_address: 10.0.2.10:8008

etcd:
  hosts: 10.0.3.10:2379,10.0.3.11:2379,10.0.3.12:2379

bootstrap:
  dcs:
    ttl: 30
    loop_wait: 10
    retry_timeout: 10
    maximum_lag_on_failover: 1048576
    postgresql:
      use_pg_rewind: true
      parameters:
        max_connections: 100
        shared_buffers: 4GB
        effective_cache_size: 12GB
        maintenance_work_mem: 1GB
        checkpoint_completion_target: 0.9
        wal_buffers: 16MB
        default_statistics_target: 100
        random_page_cost: 1.1
        effective_io_concurrency: 200

  initdb:
    - encoding: UTF8
    - data-checksums

  pg_hba:
    - host replication replicator 10.0.2.0/24 md5
    - host all all 0.0.0.0/0 md5

postgresql:
  listen: 0.0.0.0:5432
  connect_address: 10.0.2.10:5432
  data_dir: /var/lib/postgresql/15/main
  pgpass: /tmp/pgpass0
  authentication:
    replication:
      username: replicator
      password: rep-pass
    superuser:
      username: postgres
      password: postgres-pass
```

---

## Kubernetes Deployment

### Namespace and RBAC

```yaml
# namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: llm-orchestrator

---
# rbac.yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: orchestrator
  namespace: llm-orchestrator

---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: orchestrator
  namespace: llm-orchestrator
rules:
  - apiGroups: [""]
    resources: ["configmaps", "secrets"]
    verbs: ["get", "list", "watch"]
  - apiGroups: [""]
    resources: ["pods"]
    verbs: ["get", "list", "watch", "create", "delete"]

---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: orchestrator
  namespace: llm-orchestrator
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: orchestrator
subjects:
  - kind: ServiceAccount
    name: orchestrator
```

### ConfigMap

```yaml
# configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: orchestrator-config
  namespace: llm-orchestrator
data:
  orchestrator.yaml: |
    server:
      mode: api
      host: 0.0.0.0
      port:
        http: 8080
        grpc: 50051

    database:
      type: postgresql
      url: postgresql://$(DB_USER):$(DB_PASSWORD)@postgres:5432/orchestrator

    cache:
      type: redis
      url: redis://redis:6379

    execution:
      scheduler:
        max_parallel_workflows: 100
      executor:
        worker_threads: 8

    observability:
      logging:
        level: info
        format: json
      metrics:
        enabled: true
        port: 9090
      tracing:
        enabled: true
        exporter: jaeger
        endpoint: http://jaeger-collector:14268/api/traces
```

### Secrets

```yaml
# secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: orchestrator-secrets
  namespace: llm-orchestrator
type: Opaque
stringData:
  db-user: orchestrator
  db-password: changeme
  anthropic-api-key: sk-ant-...
  openai-api-key: sk-...
  jwt-secret: your-secret-key
```

### API Server Deployment

```yaml
# api-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: orchestrator-api
  namespace: llm-orchestrator
spec:
  replicas: 3
  selector:
    matchLabels:
      app: orchestrator-api
  template:
    metadata:
      labels:
        app: orchestrator-api
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
        prometheus.io/path: "/metrics"
    spec:
      serviceAccountName: orchestrator
      containers:
        - name: api-server
          image: llm-orchestrator:latest
          args:
            - server
            - --mode=api
            - --config=/etc/config/orchestrator.yaml
          ports:
            - containerPort: 8080
              name: http
            - containerPort: 50051
              name: grpc
            - containerPort: 9090
              name: metrics
          env:
            - name: DB_USER
              valueFrom:
                secretKeyRef:
                  name: orchestrator-secrets
                  key: db-user
            - name: DB_PASSWORD
              valueFrom:
                secretKeyRef:
                  name: orchestrator-secrets
                  key: db-password
            - name: ANTHROPIC_API_KEY
              valueFrom:
                secretKeyRef:
                  name: orchestrator-secrets
                  key: anthropic-api-key
            - name: RUST_LOG
              value: info
          volumeMounts:
            - name: config
              mountPath: /etc/config
          resources:
            requests:
              cpu: 500m
              memory: 512Mi
            limits:
              cpu: 2000m
              memory: 2Gi
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
            initialDelaySeconds: 30
            periodSeconds: 10
          readinessProbe:
            httpGet:
              path: /ready
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 5
      volumes:
        - name: config
          configMap:
            name: orchestrator-config

---
# api-service.yaml
apiVersion: v1
kind: Service
metadata:
  name: orchestrator-api
  namespace: llm-orchestrator
spec:
  selector:
    app: orchestrator-api
  ports:
    - name: http
      port: 80
      targetPort: 8080
    - name: grpc
      port: 50051
      targetPort: 50051
  type: ClusterIP

---
# api-hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: orchestrator-api
  namespace: llm-orchestrator
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: orchestrator-api
  minReplicas: 3
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
```

### Worker Deployment

```yaml
# worker-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: orchestrator-worker
  namespace: llm-orchestrator
spec:
  replicas: 5
  selector:
    matchLabels:
      app: orchestrator-worker
  template:
    metadata:
      labels:
        app: orchestrator-worker
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
    spec:
      serviceAccountName: orchestrator
      containers:
        - name: worker
          image: llm-orchestrator:latest
          args:
            - server
            - --mode=worker
            - --config=/etc/config/orchestrator.yaml
          env:
            - name: DB_USER
              valueFrom:
                secretKeyRef:
                  name: orchestrator-secrets
                  key: db-user
            - name: DB_PASSWORD
              valueFrom:
                secretKeyRef:
                  name: orchestrator-secrets
                  key: db-password
            - name: WORKER_THREADS
              value: "8"
            - name: RUST_LOG
              value: info
          volumeMounts:
            - name: config
              mountPath: /etc/config
          resources:
            requests:
              cpu: 1000m
              memory: 2Gi
            limits:
              cpu: 4000m
              memory: 8Gi
      volumes:
        - name: config
          configMap:
            name: orchestrator-config

---
# worker-hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: orchestrator-worker
  namespace: llm-orchestrator
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: orchestrator-worker
  minReplicas: 5
  maxReplicas: 20
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 80
```

### Ingress

```yaml
# ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: orchestrator-ingress
  namespace: llm-orchestrator
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/backend-protocol: "GRPC"
spec:
  ingressClassName: nginx
  tls:
    - hosts:
        - orchestrator.example.com
      secretName: orchestrator-tls
  rules:
    - host: orchestrator.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: orchestrator-api
                port:
                  number: 80
          - path: /grpc
            pathType: Prefix
            backend:
              service:
                name: orchestrator-api
                port:
                  number: 50051
```

### Deploy to Kubernetes

```bash
# Apply manifests
kubectl apply -f namespace.yaml
kubectl apply -f rbac.yaml
kubectl apply -f configmap.yaml
kubectl apply -f secrets.yaml
kubectl apply -f api-deployment.yaml
kubectl apply -f worker-deployment.yaml
kubectl apply -f ingress.yaml

# Verify deployment
kubectl get pods -n llm-orchestrator
kubectl get svc -n llm-orchestrator
kubectl get ingress -n llm-orchestrator

# View logs
kubectl logs -f deployment/orchestrator-api -n llm-orchestrator
kubectl logs -f deployment/orchestrator-worker -n llm-orchestrator
```

---

## Monitoring and Observability

### Prometheus Configuration

```yaml
# prometheus-config.yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'orchestrator-api'
    kubernetes_sd_configs:
      - role: pod
        namespaces:
          names:
            - llm-orchestrator
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
        action: keep
        regex: true
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_path]
        action: replace
        target_label: __metrics_path__
        regex: (.+)
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_port]
        action: replace
        target_label: __address__
        regex: (.+)
        replacement: $1:9090

  - job_name: 'orchestrator-worker'
    # Similar configuration for workers
```

### Grafana Dashboards

Key metrics to monitor:

```yaml
Workflow Metrics:
  - workflow_executions_total (counter)
  - workflow_duration_seconds (histogram)
  - workflow_success_rate (gauge)
  - workflow_active_count (gauge)

Task Metrics:
  - task_executions_total (counter)
  - task_duration_seconds (histogram)
  - task_retry_count (counter)
  - task_failure_rate (gauge)

Resource Metrics:
  - cpu_usage_percent (gauge)
  - memory_usage_bytes (gauge)
  - queue_depth (gauge)
  - worker_utilization (gauge)

Integration Metrics:
  - llm_api_calls_total (counter)
  - llm_token_usage_total (counter)
  - llm_api_latency_seconds (histogram)
  - llm_cost_usd_total (counter)
```

---

## Security

### TLS Configuration

```bash
# Generate self-signed certificate (development)
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# Use Let's Encrypt (production with cert-manager)
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml
```

### Authentication

```yaml
# JWT authentication
security:
  authentication:
    enabled: true
    provider: jwt
    jwt_secret: ${JWT_SECRET}
    jwt_expiry: 3600s

# Example: Generate JWT token
curl -X POST https://orchestrator.example.com/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "password"}'
```

### Network Policies

```yaml
# network-policy.yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: orchestrator-network-policy
  namespace: llm-orchestrator
spec:
  podSelector:
    matchLabels:
      app: orchestrator-api
  policyTypes:
    - Ingress
    - Egress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              name: ingress-nginx
      ports:
        - protocol: TCP
          port: 8080
        - protocol: TCP
          port: 50051
  egress:
    - to:
        - podSelector:
            matchLabels:
              app: postgres
      ports:
        - protocol: TCP
          port: 5432
    - to:
        - podSelector:
            matchLabels:
              app: redis
      ports:
        - protocol: TCP
          port: 6379
```

---

## Scaling Guidelines

### Vertical Scaling

```yaml
# Increase resources for existing pods
resources:
  requests:
    cpu: 2000m      # From 500m
    memory: 4Gi     # From 512Mi
  limits:
    cpu: 8000m      # From 2000m
    memory: 16Gi    # From 2Gi
```

### Horizontal Scaling

```bash
# Manual scaling
kubectl scale deployment orchestrator-worker --replicas=10 -n llm-orchestrator

# Auto-scaling (HPA already configured)
# Scales based on CPU/memory utilization
```

### Database Scaling

```yaml
# PostgreSQL connection pooling
database:
  pool:
    max_connections: 50  # Increase from 20
    min_connections: 10  # Increase from 5

# Read replicas for read-heavy workloads
# Configure in PostgreSQL HA setup
```

---

## Troubleshooting

### Common Issues

#### Issue: High API latency

```bash
# Check API pod logs
kubectl logs -f deployment/orchestrator-api -n llm-orchestrator

# Check database connections
kubectl exec -it deployment/orchestrator-api -n llm-orchestrator -- \
  psql $DATABASE_URL -c "SELECT count(*) FROM pg_stat_activity;"

# Solution: Scale up API servers or database connection pool
```

#### Issue: Worker tasks stuck

```bash
# Check worker logs
kubectl logs -f deployment/orchestrator-worker -n llm-orchestrator

# Check task queue depth
curl http://orchestrator.example.com/metrics | grep queue_depth

# Solution: Scale up workers or investigate task failures
```

#### Issue: Database connection errors

```bash
# Test database connectivity
kubectl run -it --rm debug --image=postgres:15 --restart=Never -- \
  psql postgresql://user:pass@postgres:5432/orchestrator

# Check PostgreSQL logs
kubectl logs -f statefulset/postgres -n llm-orchestrator

# Solution: Increase connection pool or scale PostgreSQL
```

### Debug Commands

```bash
# Get all resources
kubectl get all -n llm-orchestrator

# Describe pod issues
kubectl describe pod <pod-name> -n llm-orchestrator

# Execute commands in pod
kubectl exec -it <pod-name> -n llm-orchestrator -- /bin/bash

# Port forward for local debugging
kubectl port-forward svc/orchestrator-api 8080:80 -n llm-orchestrator

# View recent events
kubectl get events -n llm-orchestrator --sort-by='.lastTimestamp'
```

---

This deployment guide provides comprehensive instructions for deploying LLM-Orchestrator across all deployment modes with production-ready configurations.
