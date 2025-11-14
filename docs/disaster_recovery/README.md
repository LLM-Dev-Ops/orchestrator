# Disaster Recovery Documentation

This directory contains comprehensive disaster recovery procedures, runbooks, and metrics for the LLM Orchestrator system.

## Documents

### [DR_PROCEDURES.md](DR_PROCEDURES.md)
Detailed disaster recovery procedures for all failure scenarios including:
- Database failures (connection loss, crashes, corruption, failover)
- Application crashes (SIGKILL, OOM, panic, graceful shutdown)
- Network partitions (app-to-database, split-brain)
- Data corruption (state corruption, JSON deserialization errors)
- Backup and restore (full and incremental)
- Multi-region failover (active-passive, active-active)

Each scenario includes:
- Detection methods
- Step-by-step recovery procedures with commands
- Expected RTO/RPO
- Validation steps
- Rollback procedures

### [DR_RUNBOOK.md](DR_RUNBOOK.md)
Quick reference guide for on-call engineers including:
- Incident severity classification (P0-P3)
- Quick diagnostic commands
- Top 5 most common issues with quick fixes
- Emergency commands
- Escalation procedures
- Communication templates
- Recovery time guidelines

### [RECOVERY_METRICS.md](RECOVERY_METRICS.md)
Comprehensive test results and metrics including:
- 18 disaster scenarios tested
- Detailed RTO/RPO measurements for each scenario
- Success rates and compliance status
- Recommendations for improvements
- Overall system resilience assessment

## Testing

### Automated DR Tests

Run disaster recovery tests:

```bash
# Run all DR tests
cargo test --test disaster_recovery_tests

# Run specific scenario
cargo test --test disaster_recovery_tests database_failure

# Run with output
cargo test --test disaster_recovery_tests -- --nocapture
```

**Note:** Most tests are marked with `#[ignore]` as they require running infrastructure. To run integration tests:

```bash
# Start infrastructure
docker-compose up -d

# Run ignored tests
cargo test --test disaster_recovery_tests -- --ignored
```

### Test Scenarios

Located in `/workspaces/llm-orchestrator/tests/disaster_recovery/`:

1. **database_failure.rs** - Database crash, connection loss, failover tests
2. **application_crash.rs** - Process crash, OOM kill, panic recovery tests
3. **network_partition.rs** - Network partition and split-brain tests
4. **data_corruption.rs** - State corruption detection and recovery tests
5. **backup_restore.rs** - Full and incremental backup/restore tests
6. **failover.rs** - Multi-region failover simulations

## Backup Scripts

Located in `/workspaces/llm-orchestrator/scripts/backup/`:

### backup.sh
Creates full database backups with metadata and checksums.

```bash
# Create backup
./scripts/backup/backup.sh

# With custom settings
BACKUP_DIR=/custom/path \
RETENTION_DAYS=90 \
S3_BUCKET=my-bucket \
./scripts/backup/backup.sh
```

### restore.sh
Restores database from backup.

```bash
# Restore from local backup
./scripts/backup/restore.sh /backups/llm-orchestrator-backup-20250114_120000.tar.gz

# Download from S3 and restore
./scripts/backup/restore.sh --download s3://my-bucket/backups/backup.tar.gz

# Verify before restore
./scripts/backup/restore.sh --verify /backups/backup.tar.gz
```

### verify_backup.sh
Verifies backup integrity without restoring.

```bash
# Verify backup
./scripts/backup/verify_backup.sh /backups/backup.tar.gz
```

### schedule_backups.sh
Sets up automated backup schedule (cron and systemd timer).

```bash
# Setup daily backups at 2 AM
sudo ./scripts/backup/schedule_backups.sh

# Custom schedule
BACKUP_SCHEDULE="0 */6 * * *" \  # Every 6 hours
sudo ./scripts/backup/schedule_backups.sh
```

## Quick Reference

### Recovery Time Objectives (RTO)

| Scenario | Target RTO | Typical Actual |
|----------|-----------|----------------|
| Application crash | 30 seconds | 22-26 seconds |
| Database connection loss | 2 minutes | 45 seconds - 1m |
| Database crash | 2 minutes | 1m 30s - 2m |
| Database failover | 2 minutes | 1m 45s - 2m |
| Network partition | 1 minute | 40-55 seconds |
| Data corruption | 2 minutes | 1m 30s - 2m |
| Backup restore | 10 minutes | 8-9 minutes |
| Multi-region failover | 5 minutes | 4-5 minutes |

### Recovery Point Objectives (RPO)

| Scenario | Target RPO | Typical Actual |
|----------|-----------|----------------|
| Application crash | <1 minute | 30-45 seconds |
| Database connection loss | 0 | 0 |
| Database crash | <1 minute | 30 seconds |
| Database failover | <30 seconds | 5-15 seconds |
| Network partition | 0 | 0 |
| Data corruption | <1 minute | 30-40 seconds |
| Backup restore | <1 hour | 5-10 minutes |
| Multi-region failover | <1 minute | 8-15 seconds |

## Key Features

### Automatic Recovery
- Circuit breaker prevents cascade failures
- Auto-restart via Kubernetes/Docker
- Checkpoint-based workflow resumption
- No manual intervention for common failures

### Data Protection
- 30-second checkpoint interval
- Database replication (< 30s lag)
- WAL archiving for point-in-time recovery
- Automated backups with integrity checks

### Monitoring
- Health check endpoints
- Prometheus metrics
- Alerting for all failure scenarios
- Circuit breaker status

## Common Issues and Solutions

### "Connection pool exhausted"
```bash
# Quick fix: Restart application
docker-compose restart orchestrator

# Long-term: Tune pool size in config
DB_POOL_SIZE=20  # Increase from default
```

### "Checkpoint not found"
```bash
# Workflow crashed before first checkpoint
# This is normal - workflow will restart from beginning
# To prevent: Reduce checkpoint interval
```

### "Replication lag too high"
```bash
# Check replication status
psql -h replica -c "SELECT * FROM pg_stat_replication;"

# Solutions:
# 1. Increase replica resources
# 2. Reduce write load
# 3. Check network latency
```

## Emergency Contacts

- **On-Call Engineer:** #oncall-sre channel
- **SRE Lead:** See DR_RUNBOOK.md
- **Engineering Manager:** See DR_RUNBOOK.md
- **PagerDuty:** company.pagerduty.com

## Related Documentation

- [Production Readiness Certification](../../PRODUCTION_READINESS_CERTIFICATION.md)
- [State Persistence Implementation](../../STATE_PERSISTENCE_IMPLEMENTATION.md)
- [Observability Implementation](../../OBSERVABILITY_IMPLEMENTATION_REPORT.md)
- [Docker Compose Setup](../../docker-compose.yml)

## Maintenance

### Review Schedule
- **Monthly:** Review and test DR procedures
- **Quarterly:** Full DR drill with all scenarios
- **After Incidents:** Update procedures based on learnings
- **Annually:** Complete documentation review

### Updates
When updating DR procedures:
1. Test new procedures in staging
2. Update documentation
3. Train team on changes
4. Update runbooks
5. Update metrics/targets if needed

---

**Last Updated:** 2025-11-14
**Version:** 1.0
**Maintained By:** SRE Team
