# Authentication Failures

## Overview
Users unable to authenticate or authorization failures preventing access to workflows.

## Symptoms
- 401 Unauthorized errors
- 403 Forbidden errors
- JWT validation failures
- API key rejected
- RBAC permission denied

## Impact Assessment
- Severity: High (P1)
- User impact: Unable to use service
- Business impact: User frustration, support tickets

## Step-by-Step Procedure

### Step 1: Identify Authentication Type

```bash
# Check error logs
kubectl logs -n llm-orchestrator -l app=orchestrator --tail=500 | grep -E "(401|403|auth|jwt)"

# Common patterns:
# - "JWT signature invalid" -> JWT secret rotated
# - "API key not found" -> API key deleted/expired
# - "Insufficient permissions" -> RBAC issue
```

### Step 2: JWT Issues

```bash
# Check JWT secret exists
kubectl get secret orchestrator-jwt-secret -n llm-orchestrator

# Verify secret value hasn't changed
kubectl get secret orchestrator-jwt-secret -n llm-orchestrator -o jsonpath='{.data.secret}' | base64 -d | wc -c
# Expected: 64+ characters

# If secret changed, old JWTs won't validate
# Users need to re-authenticate

# Restart pods to pick up secret
kubectl rollout restart deployment/orchestrator -n llm-orchestrator
```

### Step 3: API Key Issues

```bash
# Check API key in database
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT id, user_id, name, created_at, last_used_at
    FROM api_keys
    WHERE revoked_at IS NULL
    LIMIT 10;
  "

# Recreate API key for user
curl -X POST https://orchestrator.example.com/api/v1/auth/api-keys \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"user_id": "user123", "name": "recovery-key"}'
```

### Step 4: RBAC Permission Issues

```bash
# Check user roles
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT u.id, u.email, r.name as role
    FROM users u
    JOIN user_roles ur ON u.id = ur.user_id
    JOIN roles r ON ur.role_id = r.id
    WHERE u.email = 'user@example.com';
  "

# Grant missing permission
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    INSERT INTO user_roles (user_id, role_id)
    SELECT u.id, r.id
    FROM users u, roles r
    WHERE u.email = 'user@example.com' AND r.name = 'workflow_execute';
  "
```

### Step 5: Check Auth Service Health

```bash
# Test auth endpoint
curl -X POST https://orchestrator.example.com/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"test","password":"test"}'

# Check audit logs for auth failures
kubectl logs -n llm-orchestrator -l app=orchestrator | grep "authentication_failed"
```

## Validation

```bash
# User can login
curl -X POST https://orchestrator.example.com/api/v1/auth/login -d '{...}'

# JWT validates
curl -H "Authorization: Bearer $TOKEN" https://orchestrator.example.com/api/v1/workflows

# API key works
curl -H "X-API-Key: $API_KEY" https://orchestrator.example.com/api/v1/workflows
```

## Related Runbooks
- [../security/01-security-incident.md](../security/01-security-incident.md)
- [../security/02-user-lockout.md](../security/02-user-lockout.md)
- [../security/03-api-key-compromise.md](../security/03-api-key-compromise.md)

---
**Last Updated**: 2025-11-14
**Version**: 1.0
