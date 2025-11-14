# Operations Checklist

## Overview
Regular operational tasks to maintain the health, security, and performance of the LLM Orchestrator.

## Daily Tasks

### Morning Health Check (10 minutes)

```bash
# 1. Check system status
kubectl get pods,svc -n llm-orchestrator

# Expected: All pods Running with 1/1 READY
# ✓ orchestrator-xxx-yyy    1/1     Running   0     Xd
# ✓ orchestrator-xxx-zzz    1/1     Running   0     Xd
# ✓ orchestrator-xxx-www    1/1     Running   0     Xd
# ✓ postgres-0              1/1     Running   0     Xd

# 2. Verify health endpoints
curl https://orchestrator.example.com/health | jq
# Expected: {"status":"healthy","version":"X.X.X","database":"connected"}

# 3. Check for pod restarts
kubectl get pods -n llm-orchestrator -o jsonpath='{range .items[*]}{.metadata.name}{"\t"}{.status.containerStatuses[0].restartCount}{"\n"}{end}'
# Expected: Restart count should be 0 or stable (not increasing)

# 4. Review overnight alerts
# Check PagerDuty/Slack for any triggered alerts

# 5. Check error rate (last 24h)
# Open Grafana dashboard
# Verify error rate < 1%

# 6. Check disk space
kubectl exec -n llm-orchestrator postgres-0 -- df -h | grep "/var/lib/postgresql/data"
# Expected: Usage < 70%

# 7. Verify backups completed
aws s3 ls s3://llm-orchestrator-backups/daily/ | tail -1
# Expected: Backup from last night exists

# 8. Check database connections
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "SELECT count(*), state FROM pg_stat_activity GROUP BY state;"
# Expected: No excessive idle connections
```

### Daily Report Template

```markdown
## Daily Operations Report - [Date]

### System Status
- [✓] All pods healthy
- [✓] Health checks passing
- [✓] No pod restarts
- [✓] Error rate < 1%
- [✓] Disk space < 70%
- [✓] Backup completed

### Alerts
- No alerts triggered / [List any alerts]

### Notes
- [Any observations or concerns]

### Actions Taken
- [List any interventions]

Prepared by: [Your name]
```

## Weekly Tasks

### Monday: Performance Review (30 minutes)

```bash
# 1. Review P99 latency trend (last 7 days)
# Grafana query: histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[7d]))
# Expected: Stable, < 2 seconds

# 2. Check database performance
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      schemaname,
      tablename,
      seq_scan,
      idx_scan,
      seq_tup_read,
      idx_tup_fetch
    FROM pg_stat_user_tables
    ORDER BY seq_scan DESC LIMIT 10;
  "
# Look for tables with high seq_scan but low idx_scan (may need indexes)

# 3. Review slow queries
# See: docs/runbooks/monitoring/01-metrics-guide.md

# 4. Check resource utilization trends
kubectl top pods -n llm-orchestrator
kubectl top nodes
# Note if trending upward, may need scaling

# 5. Review workflow completion rates
# Check Grafana dashboard for workflow success/failure rates
```

### Wednesday: Security Check (20 minutes)

```bash
# 1. Review failed authentication attempts
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      DATE(timestamp) as date,
      COUNT(*) as failed_attempts
    FROM audit_events
    WHERE event_type = 'authentication_failed'
      AND timestamp > now() - interval '7 days'
    GROUP BY DATE(timestamp)
    ORDER BY date DESC;
  "

# 2. Check for permission violations
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT user_id, COUNT(*) as violations
    FROM audit_events
    WHERE event_type = 'permission_denied'
      AND timestamp > now() - interval '7 days'
    GROUP BY user_id
    HAVING COUNT(*) > 10;
  "

# 3. Verify audit logging is working
kubectl logs -n llm-orchestrator -l app=orchestrator --since=1h | grep "AUDIT:" | head -5
# Expected: Recent audit events present

# 4. Check certificate expiry
echo | openssl s_client -servername orchestrator.example.com \
  -connect orchestrator.example.com:443 2>/dev/null | \
  openssl x509 -noout -dates
# Expected: > 30 days until expiry

# 5. Run dependency security scan
cd /workspaces/llm-orchestrator
cargo audit
# Expected: No vulnerabilities
```

### Friday: Capacity Planning Review (15 minutes)

```bash
# 1. Check request volume trend
# Grafana query: rate(http_requests_total[7d])
# Note growth rate

# 2. Check database size
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "SELECT pg_size_pretty(pg_database_size('orchestrator'));"
# Note growth rate

# 3. Review HPA status
kubectl get hpa orchestrator -n llm-orchestrator
# Check if frequently hitting max replicas

# 4. Check PVC usage
kubectl get pvc -n llm-orchestrator
# Note if approaching capacity

# 5. Update capacity planning spreadsheet
# Document current usage and trends
```

## Monthly Tasks

### First Monday: Comprehensive Maintenance (2 hours)

#### Database Maintenance

```bash
# 1. Create monthly backup
BACKUP_DATE=$(date +%Y%m%d)
kubectl exec -n llm-orchestrator postgres-0 -- \
  pg_dump -U orchestrator -Fc orchestrator | \
  gzip > backup-monthly-${BACKUP_DATE}.dump.gz

# Upload to S3
aws s3 cp backup-monthly-${BACKUP_DATE}.dump.gz \
  s3://llm-orchestrator-backups/monthly/

# 2. VACUUM and ANALYZE
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "VACUUM ANALYZE;"

# 3. Reindex
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "REINDEX DATABASE orchestrator;"

# 4. Update statistics
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "ANALYZE;"

# 5. Check for bloat
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      schemaname,
      tablename,
      pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
    FROM pg_tables
    WHERE schemaname NOT IN ('pg_catalog', 'information_schema')
    ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC
    LIMIT 10;
  "

# See: docs/runbooks/maintenance/01-database-maintenance.md
```

#### Backup Verification

```bash
# Test restore from latest monthly backup
# See: docs/runbooks/maintenance/06-backup-verification.md

# Document results in backup-verification-log.txt
```

#### Data Cleanup

```bash
# Archive old audit logs (> 90 days)
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    COPY (
      SELECT * FROM audit_events WHERE timestamp < now() - interval '90 days'
    ) TO STDOUT WITH CSV HEADER
  " > audit-archive-$(date +%Y%m).csv

aws s3 cp audit-archive-$(date +%Y%m).csv s3://llm-orchestrator-backups/archives/

kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    DELETE FROM audit_events WHERE timestamp < now() - interval '90 days';
  "

# Clean up old workflows
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    DELETE FROM workflows
    WHERE status IN ('completed', 'failed')
      AND updated_at < now() - interval '30 days';
  "
```

### Second Monday: Security Review (1.5 hours)

```bash
# 1. Review all user accounts
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT email, created_at, last_login_at
    FROM users
    WHERE disabled_at IS NULL
    ORDER BY last_login_at DESC NULLS LAST;
  "

# 2. Review API keys
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      user_id,
      name,
      created_at,
      last_used_at,
      CASE
        WHEN last_used_at IS NULL THEN 'never_used'
        WHEN last_used_at < now() - interval '90 days' THEN 'stale'
        ELSE 'active'
      END as status
    FROM api_keys
    WHERE revoked_at IS NULL;
  "

# Revoke unused keys (never used in 90+ days)

# 3. Review RBAC permissions
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      u.email,
      array_agg(r.name) as roles
    FROM users u
    JOIN user_roles ur ON u.id = ur.user_id
    JOIN roles r ON ur.role_id = r.id
    GROUP BY u.email
    ORDER BY u.email;
  "

# 4. Audit log review
# See: docs/runbooks/security/04-audit-review.md

# 5. Vulnerability scan
cargo audit
docker scan ghcr.io/llm-orchestrator/orchestrator:latest
```

### Third Monday: Update Dependencies (1 hour)

```bash
# 1. Check for Rust crate updates
cd /workspaces/llm-orchestrator
cargo outdated

# 2. Check for security advisories
cargo audit

# 3. Update dependencies (non-breaking)
cargo update

# 4. Run tests
cargo test --all

# 5. Build and test locally
cargo build --release
./target/release/llm-orchestrator --version

# 6. Schedule deployment if updates needed
# See: docs/runbooks/maintenance/05-dependency-updates.md
```

### Fourth Monday: Monitoring Review (1 hour)

```bash
# 1. Review alert definitions
kubectl get prometheusrule orchestrator-alerts -n llm-orchestrator -o yaml

# 2. Check alert firing history (last 30 days)
# Review Prometheus/Grafana for alert frequency
# Tune thresholds if too many false positives

# 3. Review dashboard effectiveness
# Identify missing metrics
# Add new panels if needed

# 4. Update runbooks
# Document any new issues encountered
# Update troubleshooting steps

# 5. Test alert routing
# Trigger test alert
# Verify PagerDuty/Slack notifications working
```

## Quarterly Tasks

### Q1, Q2, Q3, Q4: Major Reviews

#### Disaster Recovery Drill (4 hours)

```bash
# 1. Schedule drill with team
# 2. Execute one DR scenario
# Scenarios:
#   - Pod failure
#   - Database failure
#   - Regional outage
#   - Backup restoration
# 3. Document results
# 4. Update DR plan based on findings

# See: plans/Phase-4-Optional-Enhancements-SPARC.md (Section 3)
```

#### Capacity Planning (2 hours)

```bash
# 1. Analyze growth trends (90 days)
# 2. Forecast 3, 6, 12 month needs
# 3. Create capacity plan document
# 4. Budget for additional resources
# 5. Schedule infrastructure upgrades

# See: docs/runbooks/maintenance/08-capacity-planning.md
```

#### Security Audit (3 hours)

```bash
# 1. Comprehensive security review
# 2. Update security policies
# 3. Penetration testing (if scheduled)
# 4. Compliance checks
# 5. Security training for team

# See: docs/runbooks/security/06-compliance-audit.md
```

#### Performance Tuning (2 hours)

```bash
# 1. Analyze performance metrics
# 2. Identify bottlenecks
# 3. Optimize database queries
# 4. Tune application configuration
# 5. Load testing

# See: docs/runbooks/maintenance/07-performance-tuning.md
```

## Annual Tasks

### Once Per Year

#### Secret Rotation (All Secrets)

```bash
# Rotate all secrets in sequence:
# 1. JWT secret (see docs/runbooks/maintenance/03-secret-rotation.md)
# 2. Database password
# 3. LLM provider API keys
# 4. TLS certificates (if not auto-renewed)
# 5. SSH keys
# 6. Service account tokens

# Schedule: Q1 for predictability
```

#### Architecture Review

```bash
# 1. Review system architecture
# 2. Identify technical debt
# 3. Plan major upgrades
# 4. Evaluate new technologies
# 5. Create roadmap for next year
```

#### Compliance Certification Renewal

```bash
# If applicable:
# - SOC 2 Type II audit
# - ISO 27001 certification
# - HIPAA compliance review
# - PCI DSS assessment

# See: docs/runbooks/security/06-compliance-audit.md
```

## Task Tracking

### Checklist Template

```markdown
# Operations Tasks - Week of [Date]

## Daily
- [ ] Mon: Health check
- [ ] Tue: Health check
- [ ] Wed: Health check + Security check
- [ ] Thu: Health check
- [ ] Fri: Health check + Capacity review

## Weekly
- [ ] Performance review (Monday)
- [ ] Security check (Wednesday)
- [ ] Capacity planning (Friday)

## Monthly (if applicable)
- [ ] Database maintenance
- [ ] Backup verification
- [ ] Security review
- [ ] Dependency updates
- [ ] Monitoring review

## Notes
[Add any observations or issues]
```

## Automation Opportunities

Consider automating:
- Daily health checks → Slack notifications
- Database backups → Cron jobs
- Security scans → CI/CD pipeline
- Alert testing → Scheduled jobs
- Report generation → Scripts

## Emergency Override

If major incident or emergency maintenance needed:
- **Skip non-critical tasks**
- **Focus on service stability**
- **Catch up on missed tasks after resolution**
- **Document reason for skipped tasks**

---
**Last Updated**: 2025-11-14
**Version**: 1.0
**Owner**: SRE Team
**Review Schedule**: Quarterly
