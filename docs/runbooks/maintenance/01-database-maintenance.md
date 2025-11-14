# Database Maintenance

## Overview
Regular PostgreSQL maintenance tasks including backups, VACUUM, reindexing, and statistics updates.

## Prerequisites
- Database admin access
- Backup storage access (S3/GCS)
- Maintenance window scheduled
- Users notified (if downtime required)

## Impact Assessment
- Severity: Low (routine maintenance)
- User impact: None (if done during low-traffic period)
- Business impact: Ensures database health and performance

## Step-by-Step Procedure

### Daily Tasks

#### Automated Backup

```bash
# Create backup
BACKUP_DATE=$(date +%Y%m%d-%H%M%S)
kubectl exec -n llm-orchestrator postgres-0 -- \
  pg_dump -U orchestrator -Fc orchestrator | \
  gzip > backup-${BACKUP_DATE}.dump.gz

# Upload to S3
aws s3 cp backup-${BACKUP_DATE}.dump.gz \
  s3://llm-orchestrator-backups/daily/backup-${BACKUP_DATE}.dump.gz

# Verify backup integrity
gunzip < backup-${BACKUP_DATE}.dump.gz | head -20

# Clean up old backups (keep 30 days)
aws s3 ls s3://llm-orchestrator-backups/daily/ | \
  awk '{print $4}' | \
  head -n -30 | \
  xargs -I {} aws s3 rm s3://llm-orchestrator-backups/daily/{}
```

#### Verify Backup Restorable

```bash
# Test restore to temporary database (monthly)
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "CREATE DATABASE orchestrator_test;"

gunzip < backup-${BACKUP_DATE}.dump.gz | \
  kubectl exec -i -n llm-orchestrator postgres-0 -- \
  pg_restore -U orchestrator -d orchestrator_test

# Verify data
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator_test -c "SELECT COUNT(*) FROM workflows;"

# Cleanup
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "DROP DATABASE orchestrator_test;"
```

### Weekly Tasks

#### VACUUM and ANALYZE

```bash
# Check database bloat
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      schemaname,
      tablename,
      pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size,
      pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename) -
        pg_relation_size(schemaname||'.'||tablename)) AS external_size
    FROM pg_tables
    WHERE schemaname NOT IN ('pg_catalog', 'information_schema')
    ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC
    LIMIT 10;
  "

# Run VACUUM ANALYZE (online, doesn't lock)
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "VACUUM ANALYZE;"

# For heavily bloated tables, run VACUUM FULL (requires lock)
# Schedule during maintenance window
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "VACUUM FULL VERBOSE workflows;"
```

#### Update Statistics

```bash
# Update table statistics for query planner
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "ANALYZE;"

# Check statistics freshness
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      schemaname,
      tablename,
      last_vacuum,
      last_autovacuum,
      last_analyze,
      last_autoanalyze
    FROM pg_stat_user_tables
    ORDER BY last_analyze NULLS FIRST;
  "
```

### Monthly Tasks

#### Reindex

```bash
# Identify bloated indexes
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      schemaname,
      tablename,
      indexname,
      pg_size_pretty(pg_relation_size(indexrelid)) AS index_size
    FROM pg_stat_user_indexes
    ORDER BY pg_relation_size(indexrelid) DESC
    LIMIT 10;
  "

# Reindex concurrently (doesn't lock)
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "REINDEX INDEX CONCURRENTLY idx_workflows_status;"

# Or reindex entire database (during maintenance window)
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "REINDEX DATABASE orchestrator;"
```

#### Check for Corruption

```bash
# Verify data integrity
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT datname, pg_database_size(datname)
    FROM pg_database
    WHERE datname = 'orchestrator';
  "

# Check for corrupted pages
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT * FROM pg_stat_database WHERE datname = 'orchestrator';
  "
```

#### Cleanup Old Data

```bash
# Archive old audit logs (older than 90 days)
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    COPY (
      SELECT * FROM audit_events WHERE timestamp < now() - interval '90 days'
    ) TO STDOUT WITH CSV HEADER
  " > audit-archive-$(date +%Y%m).csv

# Upload archive
aws s3 cp audit-archive-$(date +%Y%m).csv s3://llm-orchestrator-backups/archives/

# Delete archived records
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    DELETE FROM audit_events WHERE timestamp < now() - interval '90 days';
  "

# Cleanup completed workflows (older than 30 days)
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    DELETE FROM workflows
    WHERE status IN ('completed', 'failed')
      AND updated_at < now() - interval '30 days';
  "

# VACUUM after large deletes
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "VACUUM ANALYZE audit_events, workflows;"
```

### Quarterly Tasks

#### Performance Tuning

```bash
# Review slow queries
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      query,
      calls,
      total_time,
      mean_time,
      max_time
    FROM pg_stat_statements
    ORDER BY mean_time DESC
    LIMIT 20;
  "

# Analyze query plans for slow queries
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    EXPLAIN (ANALYZE, BUFFERS)
    SELECT * FROM workflows WHERE status = 'running';
  "

# Add missing indexes based on analysis
# CREATE INDEX CONCURRENTLY idx_name ON table(column);
```

#### Connection Pool Review

```bash
# Check connection usage patterns
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT
      datname,
      usename,
      application_name,
      state,
      COUNT(*)
    FROM pg_stat_activity
    GROUP BY datname, usename, application_name, state
    ORDER BY count DESC;
  "

# Adjust max_connections if needed
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "ALTER SYSTEM SET max_connections = 200;"

# Restart PostgreSQL to apply
kubectl delete pod postgres-0 -n llm-orchestrator
```

## Validation

```bash
# Database healthy
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "SELECT pg_is_in_recovery();"
# Expected: f (false) for primary

# Backup exists and recent
aws s3 ls s3://llm-orchestrator-backups/daily/ | tail -1

# No bloat
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT pg_size_pretty(pg_database_size('orchestrator'));
  "

# Statistics up to date
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "
    SELECT MAX(last_analyze) FROM pg_stat_user_tables;
  "
# Expected: Recent timestamp
```

## Rollback Procedure

```bash
# If maintenance caused issues, restore from backup
gunzip < backup-${BACKUP_DATE}.dump.gz | \
  kubectl exec -i -n llm-orchestrator postgres-0 -- \
  pg_restore -U orchestrator -d orchestrator --clean

# Or restore from automated backup
aws s3 cp s3://llm-orchestrator-backups/daily/backup-<timestamp>.dump.gz - | \
  gunzip | \
  kubectl exec -i -n llm-orchestrator postgres-0 -- \
  pg_restore -U orchestrator -d orchestrator --clean
```

## Common Pitfalls

1. **VACUUM FULL Locks Table**: Use regular VACUUM, not FULL in production
2. **Backup Not Tested**: Always verify backups are restorable
3. **Insufficient Disk Space**: Ensure 2x database size available
4. **Connection Limit During Reindex**: May block application connections
5. **Deleting Too Much Data**: Archive before delete

## Related Runbooks
- [../incidents/03-database-issues.md](../incidents/03-database-issues.md)
- [../incidents/07-disk-full.md](../incidents/07-disk-full.md)
- [06-backup-verification.md](./06-backup-verification.md)

## Escalation

**Escalate if**:
- Backup fails repeatedly
- Database corruption detected
- Performance degradation after maintenance
- Unable to restore from backup

**Escalation Path**:
1. Database Admin - dba@example.com
2. Infrastructure Lead - infra-lead@example.com
3. VP Engineering

---
**Last Updated**: 2025-11-14
**Version**: 1.0
**Owner**: Database Team
**Review Schedule**: Monthly
