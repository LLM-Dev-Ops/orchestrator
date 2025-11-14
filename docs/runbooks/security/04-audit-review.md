# Regular Audit Log Review

## Overview
Periodic review of audit logs to detect anomalies and ensure compliance.

## Review Schedule
- **Daily**: Failed authentication attempts
- **Weekly**: Permission violations, suspicious activity
- **Monthly**: Comprehensive compliance review
- **Quarterly**: Full audit for compliance reporting

## Step-by-Step Procedure

### Daily Review

```bash
# Failed logins
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      timestamp,
      user_id,
      ip_address,
      event_type
    FROM audit_events
    WHERE timestamp > now() - interval '24 hours'
      AND event_type IN ('login_failed', 'authentication_failed')
    ORDER BY timestamp DESC;
  "

# Alert if > 10 failed attempts from same IP
```

### Weekly Review

```bash
# Permission violations
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      DATE(timestamp) as date,
      user_id,
      COUNT(*) as violation_count
    FROM audit_events
    WHERE timestamp > now() - interval '7 days'
      AND event_type = 'permission_denied'
    GROUP BY DATE(timestamp), user_id
    HAVING COUNT(*) > 5
    ORDER BY violation_count DESC;
  "

# Unusual access patterns
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      user_id,
      COUNT(DISTINCT ip_address) as unique_ips,
      array_agg(DISTINCT ip_address) as ip_addresses
    FROM audit_events
    WHERE timestamp > now() - interval '7 days'
    GROUP BY user_id
    HAVING COUNT(DISTINCT ip_address) > 5
    ORDER BY unique_ips DESC;
  "
```

### Monthly Compliance Review

```bash
# Export audit logs for compliance
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    COPY (
      SELECT *
      FROM audit_events
      WHERE timestamp >= date_trunc('month', now() - interval '1 month')
        AND timestamp < date_trunc('month', now())
    ) TO STDOUT WITH CSV HEADER
  " > audit-compliance-$(date +%Y%m).csv

# Generate compliance report
cat > compliance-report-$(date +%Y%m).md <<EOF
# Audit Compliance Report - $(date +%B\ %Y)

## Summary
- Total events: [count]
- Failed authentications: [count]
- Permission violations: [count]
- Data access events: [count]

## Findings
[List any anomalies or concerns]

## Recommendations
[Security improvements]

## Sign-off
Reviewed by: [Name]
Date: $(date)
EOF
```

## Validation

```bash
# Audit log integrity
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      MIN(timestamp) as oldest_event,
      MAX(timestamp) as newest_event,
      COUNT(*) as total_events
    FROM audit_events;
  "
```

---
**Last Updated**: 2025-11-14
**Version**: 1.0
