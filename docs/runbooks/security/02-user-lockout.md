# User Account Lockout and Recovery

## Overview
Procedures for handling locked user accounts and account recovery.

## Step-by-Step Procedure

### Unlock User Account

```bash
# Check if user is locked
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT id, email, locked_at, failed_login_attempts
    FROM users
    WHERE email = 'user@example.com';
  "

# Unlock account
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    UPDATE users
    SET locked_at = NULL,
        failed_login_attempts = 0
    WHERE email = 'user@example.com';
  "

# Send password reset email
curl -X POST https://orchestrator.example.com/api/v1/auth/reset-password \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"email":"user@example.com"}'
```

### Force Password Reset

```bash
# Mark password as expired
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    UPDATE users
    SET password_expires_at = now()
    WHERE email = 'user@example.com';
  "
```

### Revoke User Sessions

```bash
# Revoke all active sessions for user
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    UPDATE sessions
    SET revoked_at = now()
    WHERE user_id = (SELECT id FROM users WHERE email = 'user@example.com')
      AND revoked_at IS NULL;
  "
```

## Validation

```bash
# User can login
curl -X POST https://orchestrator.example.com/api/v1/auth/login \
  -d '{"email":"user@example.com","password":"newpassword"}'
```

---
**Last Updated**: 2025-11-14
**Version**: 1.0
