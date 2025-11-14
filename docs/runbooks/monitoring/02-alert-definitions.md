# Alert Definitions

## Overview
Prometheus alert rules for the LLM Orchestrator with severity levels and response procedures.

## Alert Rules

### Critical (P0) Alerts

#### ServiceDown
```yaml
- alert: ServiceDown
  expr: up{job="orchestrator"} == 0
  for: 1m
  labels:
    severity: critical
    team: sre
  annotations:
    summary: "LLM Orchestrator service is down"
    description: "Service {{ $labels.instance }} has been down for more than 1 minute"
    runbook: "docs/runbooks/incidents/02-service-unavailable.md"
```

#### HighErrorRate
```yaml
- alert: HighErrorRate
  expr: |
    (
      rate(http_requests_total{status=~"5.."}[5m]) /
      rate(http_requests_total[5m])
    ) * 100 > 5
  for: 5m
  labels:
    severity: critical
    team: sre
  annotations:
    summary: "High error rate detected"
    description: "Error rate is {{ $value }}% (threshold: 5%)"
    runbook: "docs/runbooks/incidents/01-high-latency.md"
```

#### DatabaseDown
```yaml
- alert: DatabaseDown
  expr: pg_up{job="postgres"} == 0
  for: 1m
  labels:
    severity: critical
    team: database
  annotations:
    summary: "PostgreSQL is down"
    description: "Database has been down for more than 1 minute"
    runbook: "docs/runbooks/incidents/03-database-issues.md"
```

#### DiskSpaceCritical
```yaml
- alert: DiskSpaceCritical
  expr: |
    (
      node_filesystem_avail_bytes{mountpoint="/var/lib/postgresql/data"} /
      node_filesystem_size_bytes{mountpoint="/var/lib/postgresql/data"}
    ) * 100 < 10
  for: 5m
  labels:
    severity: critical
    team: sre
  annotations:
    summary: "Disk space critically low"
    description: "Only {{ $value }}% disk space remaining"
    runbook: "docs/runbooks/incidents/07-disk-full.md"
```

### High (P1) Alerts

#### HighLatency
```yaml
- alert: HighLatency
  expr: |
    histogram_quantile(0.99,
      rate(http_request_duration_seconds_bucket[5m])
    ) > 5
  for: 10m
  labels:
    severity: high
    team: sre
  annotations:
    summary: "High request latency"
    description: "P99 latency is {{ $value }}s (threshold: 5s)"
    runbook: "docs/runbooks/incidents/01-high-latency.md"
```

#### LowSuccessRate
```yaml
- alert: LowSuccessRate
  expr: |
    (
      rate(http_requests_total{status="200"}[5m]) /
      rate(http_requests_total[5m])
    ) * 100 < 95
  for: 10m
  labels:
    severity: high
    team: sre
  annotations:
    summary: "Low success rate"
    description: "Success rate is {{ $value }}% (threshold: 95%)"
```

#### HighMemoryUsage
```yaml
- alert: HighMemoryUsage
  expr: |
    (
      container_memory_usage_bytes{pod=~"orchestrator-.*"} /
      container_spec_memory_limit_bytes{pod=~"orchestrator-.*"}
    ) * 100 > 80
  for: 15m
  labels:
    severity: high
    team: sre
  annotations:
    summary: "High memory usage"
    description: "Pod {{ $labels.pod }} memory usage is {{ $value }}%"
    runbook: "docs/runbooks/incidents/06-memory-leak.md"
```

#### DatabaseConnectionPoolExhausted
```yaml
- alert: DatabaseConnectionPoolExhausted
  expr: |
    (
      database_connections_active /
      database_connections_max
    ) * 100 > 90
  for: 5m
  labels:
    severity: high
    team: database
  annotations:
    summary: "Database connection pool nearly exhausted"
    description: "{{ $value }}% of connections in use"
    runbook: "docs/runbooks/incidents/03-database-issues.md"
```

### Medium (P2) Alerts

#### ModerateErrorRate
```yaml
- alert: ModerateErrorRate
  expr: |
    (
      rate(http_requests_total{status=~"5.."}[5m]) /
      rate(http_requests_total[5m])
    ) * 100 > 1
  for: 15m
  labels:
    severity: medium
    team: sre
  annotations:
    summary: "Elevated error rate"
    description: "Error rate is {{ $value }}% (threshold: 1%)"
```

#### WorkflowsStuck
```yaml
- alert: WorkflowsStuck
  expr: |
    count(
      time() - workflow_last_updated_timestamp > 1800
      and workflow_status == "running"
    ) > 5
  for: 10m
  labels:
    severity: medium
    team: platform
  annotations:
    summary: "Multiple workflows stuck"
    description: "{{ $value }} workflows have been running for > 30 minutes"
    runbook: "docs/runbooks/incidents/05-workflow-stuck.md"
```

#### CertificateExpiringSoon
```yaml
- alert: CertificateExpiringSoon
  expr: |
    (
      ssl_certificate_expiry_seconds -
      time()
    ) / 86400 < 30
  for: 1h
  labels:
    severity: medium
    team: sre
  annotations:
    summary: "TLS certificate expiring soon"
    description: "Certificate expires in {{ $value }} days"
    runbook: "docs/runbooks/maintenance/04-certificate-renewal.md"
```

### Low (P3) Alerts

#### DiskSpaceWarning
```yaml
- alert: DiskSpaceWarning
  expr: |
    (
      node_filesystem_avail_bytes{mountpoint="/var/lib/postgresql/data"} /
      node_filesystem_size_bytes{mountpoint="/var/lib/postgresql/data"}
    ) * 100 < 30
  for: 30m
  labels:
    severity: low
    team: sre
  annotations:
    summary: "Disk space low"
    description: "{{ $value }}% disk space remaining"
```

#### PodRestarts
```yaml
- alert: PodRestarts
  expr: |
    increase(
      kube_pod_container_status_restarts_total{pod=~"orchestrator-.*"}[1h]
    ) > 3
  labels:
    severity: low
    team: sre
  annotations:
    summary: "Pod restarting frequently"
    description: "Pod {{ $labels.pod }} has restarted {{ $value }} times in the last hour"
```

## Alert Response

### Critical (P0)
- **Response time**: Immediate (< 5 minutes)
- **Notification**: PagerDuty, SMS, Phone call
- **Escalation**: After 10 minutes to senior engineer
- **Status page**: Update immediately

### High (P1)
- **Response time**: < 30 minutes
- **Notification**: PagerDuty, Slack
- **Escalation**: After 1 hour to team lead
- **Status page**: Update if user-facing

### Medium (P2)
- **Response time**: < 2 hours
- **Notification**: Slack, Email
- **Escalation**: After 4 hours to team lead
- **Status page**: Not required

### Low (P3)
- **Response time**: Next business day
- **Notification**: Email, Ticket
- **Escalation**: Not required
- **Status page**: Not required

## Alert Configuration

Apply alerts to Prometheus:

```bash
# Create PrometheusRule
kubectl apply -f - <<EOF
apiVersion: monitoring.coreos.com/v1
kind: PrometheusRule
metadata:
  name: orchestrator-alerts
  namespace: llm-orchestrator
spec:
  groups:
  - name: orchestrator
    interval: 30s
    rules:
    # Include all alert rules above
EOF
```

---
**Last Updated**: 2025-11-14
**Version**: 1.0
