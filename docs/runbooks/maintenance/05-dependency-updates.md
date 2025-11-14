# Dependency Updates

## Overview
Update application dependencies (Rust crates, system packages) for security and features.

## Step-by-Step Procedure

### Check for Updates

```bash
# Check for Rust crate updates
cd /workspaces/llm-orchestrator
cargo outdated

# Check for security vulnerabilities
cargo audit

# List critical vulnerabilities
cargo audit --deny warnings
```

### Update Dependencies

```bash
# Update Cargo.lock
cargo update

# Update specific crate to latest compatible version
cargo update -p tokio

# Update to specific version
# Edit Cargo.toml, then:
cargo update

# Run tests
cargo test --all

# Build and verify
cargo build --release
```

### Update Container Base Image

```bash
# Edit Dockerfile to use newer base image
# FROM rust:1.75-alpine -> rust:1.76-alpine

# Rebuild image
docker build -t orchestrator:latest .

# Tag and push
docker tag orchestrator:latest ghcr.io/llm-orchestrator/orchestrator:v0.2.1
docker push ghcr.io/llm-orchestrator/orchestrator:v0.2.1

# Deploy (using rolling update)
kubectl set image deployment/orchestrator \
  orchestrator=ghcr.io/llm-orchestrator/orchestrator:v0.2.1 \
  -n llm-orchestrator
```

### System Package Updates (PostgreSQL)

```bash
# Check PostgreSQL version
kubectl exec -n llm-orchestrator postgres-0 -- psql -V

# If minor version update available (e.g., 16.0 -> 16.1)
# Update image in StatefulSet
kubectl patch statefulset postgres -n llm-orchestrator -p '
{
  "spec": {
    "template": {
      "spec": {
        "containers": [{
          "name": "postgres",
          "image": "postgres:16.1-alpine"
        }]
      }
    }
  }
}'

# Rolling restart (one pod at a time)
kubectl delete pod postgres-0 -n llm-orchestrator
kubectl wait --for=condition=ready pod/postgres-0 -n llm-orchestrator
```

## Validation

```bash
# All tests pass
cargo test --all

# Application runs correctly
curl https://orchestrator.example.com/health

# No new vulnerabilities
cargo audit
```

---
**Last Updated**: 2025-11-14
**Version**: 1.0
