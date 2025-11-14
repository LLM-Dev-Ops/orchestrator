# API Key Compromise Response

## Overview
Steps to take when an API key has been compromised or exposed.

## Step-by-Step Procedure

### Immediate Revocation

```bash
# Revoke compromised key
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    UPDATE api_keys
    SET revoked_at = now(),
        revocation_reason = 'compromised'
    WHERE id = '<compromised-key-id>';
  "

# Audit recent usage
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      timestamp,
      endpoint,
      ip_address,
      user_agent,
      response_status
    FROM audit_events
    WHERE api_key_id = '<compromised-key-id>'
      AND timestamp > now() - interval '7 days'
    ORDER BY timestamp DESC;
  "
```

### Issue New Key

```bash
# Generate new API key for user
curl -X POST https://orchestrator.example.com/api/v1/auth/api-keys \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "user_id": "user123",
    "name": "replacement-key",
    "scopes": ["workflow:execute", "workflow:read"]
  }'

# Securely deliver new key to user
# Use encrypted channel (not email!)
```

### Investigate Abuse

```bash
# Check for suspicious activity
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      endpoint,
      COUNT(*) as request_count,
      COUNT(DISTINCT ip_address) as unique_ips
    FROM audit_events
    WHERE api_key_id = '<compromised-key-id>'
      AND timestamp > now() - interval '7 days'
    GROUP BY endpoint
    ORDER BY request_count DESC;
  "

# Check for data exfiltration
# Look for bulk downloads or exports
```

## Validation

```bash
# Old key revoked
curl -H "X-API-Key: $OLD_KEY" https://orchestrator.example.com/api/v1/workflows
# Expected: 401 Unauthorized

# New key works
curl -H "X-API-Key: $NEW_KEY" https://orchestrator.example.com/api/v1/workflows
# Expected: 200 OK
```

## Prevention
- Rotate API keys every 90 days
- Monitor API key usage patterns
- Set expiration dates on keys
- Use IP allowlisting where possible
- Audit key permissions regularly

---
**Last Updated**: 2025-11-14
**Version**: 1.0
