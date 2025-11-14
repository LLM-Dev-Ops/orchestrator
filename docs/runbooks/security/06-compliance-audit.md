# Compliance Audit Preparation

## Overview
Prepare for external compliance audits (SOC 2, ISO 27001, GDPR, etc.).

## Audit Preparation Checklist

### Documentation Review

```bash
# Gather required documents:
- [ ] System architecture diagrams
- [ ] Data flow diagrams
- [ ] Access control policies
- [ ] Incident response plan
- [ ] Disaster recovery plan
- [ ] Change management procedures
- [ ] Security policies
- [ ] Privacy policy
```

### Evidence Collection

```bash
# Export audit logs (last 12 months)
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    COPY (
      SELECT * FROM audit_events
      WHERE timestamp > now() - interval '12 months'
    ) TO STDOUT WITH CSV HEADER
  " > audit-evidence-12months.csv

# Backup logs
aws s3 cp audit-evidence-12months.csv \
  s3://llm-orchestrator-compliance/audits/2025/

# User access review
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      u.email,
      r.name as role,
      ur.granted_at,
      ur.granted_by
    FROM users u
    JOIN user_roles ur ON u.id = ur.user_id
    JOIN roles r ON ur.role_id = r.id
    ORDER BY u.email, r.name;
  " > user-access-report.csv
```

### Security Controls Verification

```bash
# Verify encryption at rest
kubectl get pvc postgres-pvc -n llm-orchestrator -o yaml | grep storageClassName

# Verify encryption in transit
curl -vI https://orchestrator.example.com 2>&1 | grep -E "TLS|SSL"

# Verify audit logging enabled
kubectl logs -n llm-orchestrator -l app=orchestrator | grep "AUDIT:" | head -5

# Verify backups exist
aws s3 ls s3://llm-orchestrator-backups/daily/ | tail -10

# Verify monitoring active
curl https://orchestrator.example.com/metrics | head -20
```

### Compliance Reports

Generate compliance-specific reports:

**GDPR Compliance**:
```bash
# Data Processing Activities
- Personal data collected: [List]
- Legal basis: [Contract/Consent/Legitimate Interest]
- Data retention: [Period]
- Third parties: [LLM providers]
- International transfers: [Details]
```

**SOC 2 Controls**:
- CC6.1: Logical access controls
- CC6.6: Encryption
- CC6.7: System monitoring
- CC7.2: Change management
- CC9.1: Availability

## Validation

Audit readiness checklist:
- [ ] All documentation up-to-date
- [ ] Audit logs complete and accessible
- [ ] Access controls documented
- [ ] Security controls verified
- [ ] Backup/DR tested
- [ ] Incident response plan current
- [ ] Team trained on audit process

---
**Last Updated**: 2025-11-14
**Version**: 1.0
