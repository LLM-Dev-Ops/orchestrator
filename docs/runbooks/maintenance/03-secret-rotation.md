# Secret Rotation

## Overview
Regularly rotate secrets (JWT keys, API keys, database passwords) for security.

## Prerequisites
- Secrets management access (Vault/AWS Secrets Manager)
- Ability to update Kubernetes secrets
- Maintenance window (for database password)

## Step-by-Step Procedure

### Rotate JWT Secret

```bash
# Generate new secret
NEW_SECRET=$(openssl rand -base64 64)

# Create new secret version
kubectl create secret generic orchestrator-jwt-secret-new \
  --from-literal=secret=$NEW_SECRET \
  -n llm-orchestrator

# Update deployment to use new secret
kubectl patch deployment orchestrator -n llm-orchestrator -p '
{
  "spec": {
    "template": {
      "spec": {
        "containers": [{
          "name": "orchestrator",
          "env": [{
            "name": "JWT_SECRET",
            "valueFrom": {
              "secretKeyRef": {
                "name": "orchestrator-jwt-secret-new",
                "key": "secret"
              }
            }
          }]
        }]
      }
    }
  }
}'

# Rolling restart
kubectl rollout restart deployment/orchestrator -n llm-orchestrator

# Wait for all pods to update
kubectl rollout status deployment/orchestrator -n llm-orchestrator

# After grace period (7 days), delete old secret
kubectl delete secret orchestrator-jwt-secret -n llm-orchestrator

# Rename new secret to standard name
kubectl get secret orchestrator-jwt-secret-new -n llm-orchestrator -o yaml | \
  sed 's/orchestrator-jwt-secret-new/orchestrator-jwt-secret/' | \
  kubectl apply -f -
```

### Rotate Database Password

```bash
# Generate new password
NEW_DB_PASSWORD=$(openssl rand -base64 32)

# Update password in PostgreSQL
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "ALTER USER orchestrator WITH PASSWORD '$NEW_DB_PASSWORD';"

# Update secret
kubectl create secret generic orchestrator-db-secret-new \
  --from-literal=username=orchestrator \
  --from-literal=password=$NEW_DB_PASSWORD \
  -n llm-orchestrator

# Update application to use new secret
kubectl patch deployment orchestrator -n llm-orchestrator -p '{...}'

# Restart pods
kubectl rollout restart deployment/orchestrator -n llm-orchestrator
```

### Rotate LLM Provider API Keys

```bash
# Obtain new API keys from providers
# OpenAI: https://platform.openai.com/api-keys
# Anthropic: https://console.anthropic.com/settings/keys

# Update secret
kubectl create secret generic orchestrator-api-keys-new \
  --from-literal=openai-api-key="$NEW_OPENAI_KEY" \
  --from-literal=anthropic-api-key="$NEW_ANTHROPIC_KEY" \
  -n llm-orchestrator

# Rolling update
kubectl patch deployment orchestrator -n llm-orchestrator -p '{...}'
kubectl rollout restart deployment/orchestrator -n llm-orchestrator

# Verify new keys work
curl -X POST https://orchestrator.example.com/api/v1/workflows/execute -d '{...}'

# Revoke old keys in provider dashboards after grace period
```

## Validation

```bash
# Application healthy with new secrets
kubectl get pods -n llm-orchestrator -l app=orchestrator

# Authentication working
curl -X POST https://orchestrator.example.com/api/v1/auth/login -d '{...}'

# Database connection working
kubectl exec -n llm-orchestrator deployment/orchestrator -- \
  psql $DATABASE_URL -c "SELECT 1;"
```

## Rotation Schedule
- **JWT Secret**: Every 90 days
- **Database Password**: Every 90 days
- **API Keys**: Every 180 days or on compromise
- **TLS Certificates**: 30 days before expiry

## Related Runbooks
- [04-certificate-renewal.md](./04-certificate-renewal.md)
- [../incidents/09-secret-rotation-failure.md](../incidents/09-secret-rotation-failure.md)
- [../security/03-api-key-compromise.md](../security/03-api-key-compromise.md)

---
**Last Updated**: 2025-11-14
**Version**: 1.0
