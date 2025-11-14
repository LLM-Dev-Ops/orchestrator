# Initial Deployment

## Overview
First-time deployment of the LLM Orchestrator to a Kubernetes cluster. This runbook covers the complete setup from cluster preparation to service verification.

## Prerequisites
- Required access/permissions:
  - Kubernetes cluster admin access (RBAC: `cluster-admin`)
  - Docker registry push access
  - DNS management access
  - Secrets management (Vault/AWS Secrets Manager)
- Tools needed:
  - `kubectl` v1.24+
  - `helm` v3.0+
  - `docker` or `podman`
  - `openssl` for certificate generation
- Knowledge required:
  - Kubernetes fundamentals
  - Container orchestration
  - Basic PostgreSQL administration

## Impact Assessment
- Severity: Low (new deployment, no existing users)
- User impact: None (initial deployment)
- Business impact: Enables new LLM orchestration capabilities

## Step-by-Step Procedure

### Step 1: Verify Cluster Readiness

Check cluster version and resources:

```bash
# Verify Kubernetes version (minimum 1.24)
kubectl version --short

# Expected output:
# Client Version: v1.27.0
# Server Version: v1.27.0

# Check cluster nodes
kubectl get nodes

# Expected: At least 3 nodes in Ready state

# Verify available resources
kubectl top nodes

# Expected: CPU < 70%, Memory < 70% across nodes
```

### Step 2: Create Namespace

```bash
# Create dedicated namespace
kubectl create namespace llm-orchestrator

# Label namespace for monitoring
kubectl label namespace llm-orchestrator monitoring=enabled

# Verify namespace creation
kubectl get namespace llm-orchestrator

# Expected output:
# NAME               STATUS   AGE
# llm-orchestrator   Active   5s
```

### Step 3: Setup Secrets

```bash
# Create secret for database credentials
kubectl create secret generic orchestrator-db-secret \
  --namespace=llm-orchestrator \
  --from-literal=username=orchestrator \
  --from-literal=password=$(openssl rand -base64 32)

# Create secret for LLM provider API keys
kubectl create secret generic orchestrator-api-keys \
  --namespace=llm-orchestrator \
  --from-literal=openai-api-key="${OPENAI_API_KEY}" \
  --from-literal=anthropic-api-key="${ANTHROPIC_API_KEY}"

# Create JWT secret for authentication
kubectl create secret generic orchestrator-jwt-secret \
  --namespace=llm-orchestrator \
  --from-literal=secret=$(openssl rand -base64 64)

# Verify secrets
kubectl get secrets -n llm-orchestrator

# Expected: 3 secrets created
```

### Step 4: Deploy PostgreSQL

```bash
# Create persistent volume claim
cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: postgres-pvc
  namespace: llm-orchestrator
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
  storageClassName: gp3
EOF

# Deploy PostgreSQL StatefulSet
cat <<EOF | kubectl apply -f -
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: postgres
  namespace: llm-orchestrator
spec:
  serviceName: postgres
  replicas: 1
  selector:
    matchLabels:
      app: postgres
  template:
    metadata:
      labels:
        app: postgres
    spec:
      containers:
      - name: postgres
        image: postgres:16-alpine
        env:
        - name: POSTGRES_DB
          value: orchestrator
        - name: POSTGRES_USER
          valueFrom:
            secretKeyRef:
              name: orchestrator-db-secret
              key: username
        - name: POSTGRES_PASSWORD
          valueFrom:
            secretKeyRef:
              name: orchestrator-db-secret
              key: password
        - name: PGDATA
          value: /var/lib/postgresql/data/pgdata
        ports:
        - containerPort: 5432
          name: postgres
        volumeMounts:
        - name: postgres-storage
          mountPath: /var/lib/postgresql/data
        resources:
          requests:
            cpu: 500m
            memory: 1Gi
          limits:
            cpu: 1000m
            memory: 2Gi
  volumeClaimTemplates:
  - metadata:
      name: postgres-storage
    spec:
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 10Gi
      storageClassName: gp3
EOF

# Create PostgreSQL service
cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: Service
metadata:
  name: postgres
  namespace: llm-orchestrator
spec:
  selector:
    app: postgres
  ports:
  - port: 5432
    targetPort: 5432
  clusterIP: None
EOF

# Wait for PostgreSQL to be ready
kubectl wait --for=condition=ready pod -l app=postgres -n llm-orchestrator --timeout=300s

# Expected: pod/postgres-0 condition met
```

### Step 5: Build and Push Container Image

```bash
# Navigate to project root
cd /workspaces/llm-orchestrator

# Build the container image
docker build -t orchestrator:latest .

# Tag for registry
docker tag orchestrator:latest ghcr.io/llm-orchestrator/orchestrator:v0.1.0
docker tag orchestrator:latest ghcr.io/llm-orchestrator/orchestrator:latest

# Push to registry
docker push ghcr.io/llm-orchestrator/orchestrator:v0.1.0
docker push ghcr.io/llm-orchestrator/orchestrator:latest

# Expected: Successfully pushed images
```

### Step 6: Deploy Orchestrator Application

```bash
# Create ConfigMap for application configuration
cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: ConfigMap
metadata:
  name: orchestrator-config
  namespace: llm-orchestrator
data:
  config.yaml: |
    server:
      port: 8080
      log_level: info
    database:
      host: postgres.llm-orchestrator.svc.cluster.local
      port: 5432
      database: orchestrator
      max_connections: 20
    providers:
      openai:
        enabled: true
        timeout_seconds: 30
      anthropic:
        enabled: true
        timeout_seconds: 30
EOF

# Deploy the orchestrator
cat <<EOF | kubectl apply -f -
apiVersion: apps/v1
kind: Deployment
metadata:
  name: orchestrator
  namespace: llm-orchestrator
spec:
  replicas: 3
  selector:
    matchLabels:
      app: orchestrator
  template:
    metadata:
      labels:
        app: orchestrator
        version: v0.1.0
    spec:
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
      containers:
      - name: orchestrator
        image: ghcr.io/llm-orchestrator/orchestrator:v0.1.0
        imagePullPolicy: Always
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 9090
          name: metrics
        env:
        - name: DATABASE_URL
          value: "postgresql://orchestrator:password@postgres.llm-orchestrator.svc.cluster.local:5432/orchestrator"
        - name: OPENAI_API_KEY
          valueFrom:
            secretKeyRef:
              name: orchestrator-api-keys
              key: openai-api-key
        - name: ANTHROPIC_API_KEY
          valueFrom:
            secretKeyRef:
              name: orchestrator-api-keys
              key: anthropic-api-key
        - name: JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: orchestrator-jwt-secret
              key: secret
        - name: RUST_LOG
          value: info
        volumeMounts:
        - name: config
          mountPath: /etc/orchestrator
          readOnly: true
        resources:
          requests:
            cpu: 500m
            memory: 1Gi
          limits:
            cpu: 1000m
            memory: 2Gi
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: orchestrator-config
EOF

# Wait for deployment to be ready
kubectl rollout status deployment/orchestrator -n llm-orchestrator --timeout=5m

# Expected: deployment "orchestrator" successfully rolled out
```

### Step 7: Create Service

```bash
# Create ClusterIP service
cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: Service
metadata:
  name: orchestrator
  namespace: llm-orchestrator
  labels:
    app: orchestrator
spec:
  type: ClusterIP
  selector:
    app: orchestrator
  ports:
  - port: 8080
    targetPort: 8080
    protocol: TCP
    name: http
  - port: 9090
    targetPort: 9090
    protocol: TCP
    name: metrics
EOF

# Verify service
kubectl get service orchestrator -n llm-orchestrator

# Expected: Service with ClusterIP assigned
```

### Step 8: Configure Ingress

```bash
# Create TLS certificate (or use cert-manager)
kubectl create secret tls orchestrator-tls \
  --cert=path/to/cert.pem \
  --key=path/to/key.pem \
  -n llm-orchestrator

# Create Ingress
cat <<EOF | kubectl apply -f -
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: orchestrator
  namespace: llm-orchestrator
  annotations:
    kubernetes.io/ingress.class: nginx
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
spec:
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
            name: orchestrator
            port:
              number: 8080
EOF

# Verify ingress
kubectl get ingress orchestrator -n llm-orchestrator

# Expected: Ingress with address assigned
```

### Step 9: Setup Monitoring

```bash
# Create ServiceMonitor for Prometheus
cat <<EOF | kubectl apply -f -
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: orchestrator
  namespace: llm-orchestrator
  labels:
    app: orchestrator
spec:
  selector:
    matchLabels:
      app: orchestrator
  endpoints:
  - port: metrics
    interval: 30s
    path: /metrics
EOF

# Verify ServiceMonitor
kubectl get servicemonitor orchestrator -n llm-orchestrator

# Expected: ServiceMonitor created
```

### Step 10: Configure Horizontal Pod Autoscaler

```bash
# Create HPA
cat <<EOF | kubectl apply -f -
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: orchestrator
  namespace: llm-orchestrator
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: orchestrator
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
EOF

# Verify HPA
kubectl get hpa orchestrator -n llm-orchestrator

# Expected: HPA with current/target replicas
```

## Validation

Check deployment health:

```bash
# 1. Check all pods are running
kubectl get pods -n llm-orchestrator

# Expected: All pods in Running state

# 2. Check pod logs
kubectl logs -n llm-orchestrator -l app=orchestrator --tail=50

# Expected: No ERROR messages, info logs about startup

# 3. Test health endpoints
kubectl run -it --rm test-pod --image=curlimages/curl --restart=Never -- \
  curl http://orchestrator.llm-orchestrator.svc.cluster.local:8080/health

# Expected: {"status":"healthy","database":"connected"}

# 4. Test readiness
kubectl run -it --rm test-pod --image=curlimages/curl --restart=Never -- \
  curl http://orchestrator.llm-orchestrator.svc.cluster.local:8080/health/ready

# Expected: {"ready":true}

# 5. Verify metrics endpoint
kubectl run -it --rm test-pod --image=curlimages/curl --restart=Never -- \
  curl http://orchestrator.llm-orchestrator.svc.cluster.local:9090/metrics

# Expected: Prometheus metrics output

# 6. Test external access via ingress
curl https://orchestrator.example.com/health

# Expected: {"status":"healthy"}
```

## Rollback Procedure

If deployment fails:

```bash
# Delete all resources
kubectl delete namespace llm-orchestrator

# This will remove:
# - All deployments
# - All services
# - All secrets
# - All ConfigMaps
# - StatefulSets
# - PVCs (may need manual deletion if retain policy)

# Clean up PVCs if they persist
kubectl get pvc -A | grep llm-orchestrator
kubectl delete pvc <pvc-name> -n llm-orchestrator

# Verify cleanup
kubectl get all -n llm-orchestrator

# Expected: No resources found
```

## Post-Deployment Actions

1. **Document Configuration**:
   - Save all applied manifests to Git repository
   - Document custom values and configurations
   - Record database credentials location

2. **Configure Monitoring**:
   - Import Grafana dashboards
   - Configure alert rules in Prometheus
   - Set up PagerDuty/OpsGenie integration

3. **Setup Backups**:
   - Configure automated database backups
   - Test backup restoration
   - Document backup schedule

4. **Security Hardening**:
   - Review and apply network policies
   - Configure pod security policies
   - Enable audit logging

5. **Training**:
   - Train operations team
   - Share runbook location
   - Schedule disaster recovery drill

## Common Pitfalls

1. **Insufficient Resources**: Ensure cluster has enough CPU/memory before deploying
   - Minimum: 6 CPU cores, 12GB RAM across cluster

2. **Secret Not Found**: Ensure secrets are created in correct namespace
   - Verify with: `kubectl get secrets -n llm-orchestrator`

3. **Image Pull Errors**: Ensure image registry credentials are configured
   - Create image pull secret if using private registry

4. **Database Connection Failures**: Check PostgreSQL is fully ready before deploying app
   - Wait for StatefulSet to be ready: `kubectl wait --for=condition=ready pod -l app=postgres`

5. **DNS Resolution Issues**: Ensure CoreDNS is functioning
   - Test: `kubectl run -it --rm debug --image=busybox -- nslookup postgres.llm-orchestrator.svc.cluster.local`

## Related Runbooks

- [02-rolling-update.md](./02-rolling-update.md) - Upgrading to new versions
- [03-rollback-procedure.md](./03-rollback-procedure.md) - Rolling back failed deployments
- [04-scaling.md](./04-scaling.md) - Scaling the deployment
- [../maintenance/01-database-maintenance.md](../maintenance/01-database-maintenance.md) - Database operations

## Escalation

**Escalate if**:
- Deployment fails after 3 attempts
- Database corruption detected
- Critical errors in application logs
- Unable to access cluster

**Escalation Path**:
1. Senior DevOps Engineer - devops-lead@example.com
2. Infrastructure Manager - infra-manager@example.com
3. VP Engineering - vp-eng@example.com

**Communication Channels**:
- Slack: #llm-orchestrator-ops
- PagerDuty: LLM Orchestrator Service
- Incident Management: incident-mgmt@example.com

---

**Last Updated**: 2025-11-14
**Version**: 1.0
**Owner**: DevOps Team
**Review Schedule**: Quarterly
