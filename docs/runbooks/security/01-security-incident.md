# Security Incident Response

## Overview
Response procedure for security incidents including breaches, unauthorized access, or data exposure.

## Prerequisites
- Security team access
- Incident response authority
- Forensics tools
- Communication channels

## Impact Assessment
- Severity: Critical (P0)
- User impact: Potential data breach
- Business impact: Reputation, compliance, legal

## Step-by-Step Procedure

### Step 1: Contain the Incident (IMMEDIATE)

```bash
# STOP - Do not delete evidence yet!

# Isolate affected pods (prevent further damage)
kubectl label pod <affected-pod> -n llm-orchestrator quarantine=true

# Update NetworkPolicy to isolate
kubectl apply -f - <<EOF
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: quarantine-policy
  namespace: llm-orchestrator
spec:
  podSelector:
    matchLabels:
      quarantine: "true"
  policyTypes:
  - Ingress
  - Egress
  egress:
  - to:
    - podSelector:
        matchLabels:
          app: forensics
EOF

# Revoke all active API keys immediately
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    UPDATE api_keys SET revoked_at = now() WHERE revoked_at IS NULL;
  "

# Invalidate all JWT tokens (rotate secret)
NEW_SECRET=$(openssl rand -base64 64)
kubectl create secret generic orchestrator-jwt-secret-emergency \
  --from-literal=secret=$NEW_SECRET -n llm-orchestrator

# Force all users to re-authenticate
```

### Step 2: Assess Scope

```bash
# Check audit logs for suspicious activity
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      timestamp,
      event_type,
      user_id,
      resource_id,
      ip_address,
      user_agent
    FROM audit_events
    WHERE timestamp > now() - interval '24 hours'
      AND (
        event_type IN ('unauthorized_access', 'authentication_failed', 'permission_denied')
        OR ip_address NOT IN ('10.0.0.0/8', '172.16.0.0/12')
      )
    ORDER BY timestamp DESC
    LIMIT 100;
  "

# Identify compromised accounts
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT DISTINCT user_id, ip_address
    FROM audit_events
    WHERE event_type = 'login_success'
      AND ip_address IN (<suspicious-ips>);
  "

# Check for data exfiltration
kubectl logs -n llm-orchestrator -l app=orchestrator --since=24h | \
  grep -E "(download|export|bulk_query)" | \
  awk '{if ($NF > 1000000) print}'  # Large data transfers
```

### Step 3: Preserve Evidence

```bash
# Snapshot pod for forensic analysis
kubectl debug -n llm-orchestrator <affected-pod> \
  --image=nicolaka/netshoot \
  --share-processes \
  --copy-to=forensic-pod

# Export all audit logs
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    COPY (SELECT * FROM audit_events WHERE timestamp > now() - interval '7 days')
    TO STDOUT WITH CSV HEADER
  " > security-incident-audit-$(date +%Y%m%d-%H%M%S).csv

# Preserve application logs
kubectl logs -n llm-orchestrator -l app=orchestrator --since=7d \
  > security-incident-logs-$(date +%Y%m%d-%H%M%S).log

# Create PVC snapshot (if supported)
kubectl apply -f - <<EOF
apiVersion: snapshot.storage.k8s.io/v1
kind: VolumeSnapshot
metadata:
  name: postgres-incident-snapshot
  namespace: llm-orchestrator
spec:
  volumeSnapshotClassName: csi-snapclass
  source:
    persistentVolumeClaimName: postgres-pvc
EOF
```

### Step 4: Eradicate Threat

```bash
# Remove malicious code/backdoors
# Review recent code changes
git log --since="7 days ago" --oneline

# If backdoor found, revert to known-good version
kubectl set image deployment/orchestrator \
  orchestrator=ghcr.io/llm-orchestrator/orchestrator:v0.1.0-verified \
  -n llm-orchestrator

# Rotate ALL secrets
# Database password
NEW_DB_PASSWORD=$(openssl rand -base64 32)
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "ALTER USER orchestrator WITH PASSWORD '$NEW_DB_PASSWORD';"

# LLM API keys
# Obtain new keys from providers and update secrets

# Remove compromised user accounts
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    UPDATE users SET disabled_at = now()
    WHERE id IN (<compromised-user-ids>);
  "
```

### Step 5: Recover

```bash
# Deploy clean version
kubectl rollout restart deployment/orchestrator -n llm-orchestrator

# Re-enable security controls
kubectl apply -f kubernetes/network-policies/
kubectl apply -f kubernetes/pod-security-policies/

# Monitor for continued suspicious activity
kubectl logs -n llm-orchestrator -l app=orchestrator -f | \
  grep -E "(unauthorized|denied|failed)"
```

### Step 6: Notify Stakeholders

```bash
# Internal notification
# Slack: #security-incidents
# Email: security@example.com, legal@example.com, executives@example.com

# External notification (if data breach)
# Customers: Via email and status page
# Regulators: Per GDPR/CCPA requirements (within 72 hours)
# Law enforcement: If criminal activity

# Create incident report
cat > security-incident-report.md <<EOF
# Security Incident Report

**Incident ID**: SEC-$(date +%Y%m%d)-001
**Date**: $(date -u +"%Y-%m-%d %H:%M:%S UTC")
**Severity**: Critical
**Status**: Contained

## Summary
[Brief description of incident]

## Timeline
- [Time]: Incident detected
- [Time]: Containment actions taken
- [Time]: Threat eradicated
- [Time]: Service restored

## Impact
- Affected users: [count]
- Data exposed: [type and volume]
- Downtime: [duration]

## Root Cause
[Analysis of how breach occurred]

## Remediation
[Actions taken]

## Prevention
[Measures to prevent recurrence]

## Compliance Notifications
- GDPR: [Required/Not Required]
- CCPA: [Required/Not Required]
- PCI DSS: [Required/Not Required]
EOF
```

## Validation

```bash
# No unauthorized access in last hour
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT COUNT(*) FROM audit_events
    WHERE timestamp > now() - interval '1 hour'
      AND event_type IN ('unauthorized_access', 'authentication_failed');
  "
# Expected: 0

# All secrets rotated
# Verify secret modification times
kubectl get secrets -n llm-orchestrator -o json | \
  jq '.items[] | {name: .metadata.name, modified: .metadata.creationTimestamp}'

# System healthy
curl https://orchestrator.example.com/health

# Monitoring active
# Check all security alerts firing correctly
```

## Post-Incident Actions

1. **Immediate** (Within 24 hours):
   - Complete incident report
   - Notify affected users
   - File regulatory notifications
   - Update status page

2. **Short Term** (Within 1 week):
   - Conduct forensic analysis
   - Identify all compromised data
   - Enhanced monitoring
   - Security audit

3. **Long Term** (Within 1 month):
   - Blameless post-mortem
   - Implement preventive controls
   - Security training
   - Penetration testing
   - Update security policies

## Common Attack Vectors

1. **SQL Injection**: Sanitize all inputs
2. **API Key Compromise**: Rotate regularly, monitor usage
3. **Container Escape**: Use pod security policies
4. **Insider Threat**: Principle of least privilege
5. **Supply Chain**: Verify dependencies

## Related Runbooks
- [02-user-lockout.md](./02-user-lockout.md)
- [03-api-key-compromise.md](./03-api-key-compromise.md)
- [04-audit-review.md](./04-audit-review.md)
- [../incidents/10-audit-log-gaps.md](../incidents/10-audit-log-gaps.md)

## Escalation

**Escalate IMMEDIATELY**:
- Security Team Lead - security-lead@example.com - Phone: xxx-xxx-xxxx
- CISO - ciso@example.com - Phone: xxx-xxx-xxxx
- Legal Team - legal@example.com
- PR/Communications - pr@example.com
- CEO (for major breaches)

**External Contacts**:
- Forensics Firm: [Contact info]
- Cyber Insurance: [Policy number, contact]
- FBI Cyber Division: https://www.fbi.gov/investigate/cyber
- Data Protection Authority: [Contact for GDPR]

---
**Last Updated**: 2025-11-14
**Version**: 1.0
**Owner**: Security Team
**Classification**: CONFIDENTIAL
