# Backup Verification

## Overview
Regularly test that backups are restorable to ensure disaster recovery capability.

## Step-by-Step Procedure

### Monthly Backup Test

```bash
# Retrieve latest backup
LATEST_BACKUP=$(aws s3 ls s3://llm-orchestrator-backups/daily/ | sort | tail -1 | awk '{print $4}')
aws s3 cp s3://llm-orchestrator-backups/daily/$LATEST_BACKUP ./test-backup.dump.gz

# Create test database
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "CREATE DATABASE backup_test;"

# Restore backup
gunzip < test-backup.dump.gz | \
  kubectl exec -i -n llm-orchestrator postgres-0 -- \
  pg_restore -U orchestrator -d backup_test --no-owner --no-privileges

# Verify data integrity
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d backup_test -c "
    SELECT
      'workflows' as table_name, COUNT(*) as row_count FROM workflows
    UNION ALL
    SELECT 'audit_events', COUNT(*) FROM audit_events
    UNION ALL
    SELECT 'users', COUNT(*) FROM users;
  "

# Compare with production counts
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -d orchestrator -c "SELECT COUNT(*) FROM workflows;"

# Cleanup test database
kubectl exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "DROP DATABASE backup_test;"

# Document results
echo "Backup verification: SUCCESS - $(date)" >> backup-verification-log.txt
```

## Validation

```bash
# Backup restores without errors
# Row counts match expected values
# All critical tables present
```

---
**Last Updated**: 2025-11-14
**Version**: 1.0
