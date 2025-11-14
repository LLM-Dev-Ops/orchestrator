# Disaster Recovery Procedures

**Version:** 1.0
**Last Updated:** 2025-11-14
**Owner:** DevOps/SRE Team

## Executive Summary

This document outlines disaster recovery procedures for the LLM Orchestrator system. It covers failure scenarios, detection methods, recovery procedures, and validation steps to ensure business continuity.

### Recovery Objectives

- **RTO (Recovery Time Objective):** < 5 minutes for most scenarios
- **RPO (Recovery Point Objective):** < 1 minute (checkpoint interval)
- **Data Loss Target:** 0% for database failures
- **Availability Target:** 99.9% (8.76 hours downtime/year)

## Table of Contents

1. [Database Failure Scenarios](#database-failure-scenarios)
2. [Application Crash Scenarios](#application-crash-scenarios)
3. [Network Partition Scenarios](#network-partition-scenarios)
4. [Data Corruption Scenarios](#data-corruption-scenarios)
5. [Backup and Restore Procedures](#backup-and-restore-procedures)
6. [Multi-Region Failover](#multi-region-failover)
7. [Contact Information](#contact-information)

---

## Database Failure Scenarios

### Scenario 1: Database Connection Loss

**Description:** Application cannot connect to PostgreSQL database due to network issues, connection pool exhaustion, or firewall changes.

**Detection:**
- Health check endpoint `/health` returns 503
- Connection pool timeout errors in logs
- Prometheus alert: `database_connection_failures > 10`
- Circuit breaker opens

**Recovery Procedure:**

1. **Diagnose the issue** (1-2 minutes)
   ```bash
   # Check database is running
   docker-compose ps postgres

   # Check connection from app container
   docker-compose exec orchestrator pg_isready -h postgres -p 5432

   # Check connection pool status
   curl http://localhost:8080/metrics | grep db_pool
   ```

2. **Verify network connectivity** (1 minute)
   ```bash
   # Test network path
   telnet postgres 5432

   # Check firewall rules
   iptables -L -n | grep 5432
   ```

3. **Restart connection pool** (30 seconds)
   ```bash
   # Restart application to reset connection pool
   docker-compose restart orchestrator

   # Or send SIGHUP to reload config
   kill -HUP $(pgrep orchestrator)
   ```

4. **Verify recovery** (1 minute)
   ```bash
   # Check health endpoint
   curl http://localhost:8080/health

   # Verify workflows can be queried
   curl http://localhost:8080/api/v1/workflows
   ```

**Expected RTO:** 2 minutes
**Expected RPO:** 0 (no data loss)

**Rollback:** If restart causes issues, rollback to previous version:
```bash
docker-compose down
git checkout <previous-tag>
docker-compose up -d
```

---

### Scenario 2: Database Crash During Execution

**Description:** PostgreSQL process crashes while workflows are actively executing.

**Detection:**
- PostgreSQL container exits (Docker restart policy may auto-restart)
- All database queries fail with connection refused
- Workflow executions pause with database errors
- Alert: `postgres_up == 0`

**Recovery Procedure:**

1. **Verify database is down** (30 seconds)
   ```bash
   docker-compose ps postgres
   # Should show "Exit" status

   systemctl status postgresql  # If running natively
   ```

2. **Check crash reason** (1 minute)
   ```bash
   # Check PostgreSQL logs
   docker-compose logs postgres --tail=100

   # Common issues: OOM, assertion failure, corruption
   ```

3. **Restart database** (1-2 minutes)
   ```bash
   # Docker environment
   docker-compose up -d postgres

   # Native installation
   systemctl start postgresql

   # Wait for ready
   until pg_isready -h localhost -p 5432; do sleep 1; done
   ```

4. **Verify data integrity** (1 minute)
   ```bash
   # Check for corruption
   docker-compose exec postgres psql -U postgres -d workflows \
     -c "SELECT pg_stat_database.datname, pg_stat_database.conflicts FROM pg_stat_database;"

   # Verify workflow count
   docker-compose exec postgres psql -U postgres -d workflows \
     -c "SELECT COUNT(*) FROM workflow_states;"
   ```

5. **Resume workflows** (1 minute)
   ```bash
   # Application automatically discovers paused workflows
   # and resumes from last checkpoint

   # Monitor resumption
   tail -f /var/log/llm-orchestrator/app.log | grep "Resuming workflow"
   ```

**Expected RTO:** 2 minutes
**Expected RPO:** < 1 minute (last checkpoint)

**Rollback:** If database won't start due to corruption:
```bash
# Restore from last backup
./scripts/backup/restore.sh --verify /backups/llm-orchestrator-backup-latest.tar.gz
```

---

### Scenario 3: Database Primary Failover

**Description:** Primary database server fails, need to promote replica to primary.

**Detection:**
- Primary database unreachable
- Replication lag alerts stop (no replication)
- Write queries fail
- Alert: `postgres_primary_up == 0`

**Recovery Procedure:**

1. **Verify primary is down** (1 minute)
   ```bash
   # Check primary health
   pg_isready -h primary-db.example.com -p 5432

   # Check replication status on replica
   psql -h replica-db.example.com -U replication -c "SELECT * FROM pg_stat_replication;"
   ```

2. **Promote replica to primary** (1-2 minutes)
   ```bash
   # On replica server
   pg_ctl promote -D /var/lib/postgresql/data

   # Or using pg_promote
   docker-compose exec postgres-replica pg_ctl promote
   ```

3. **Update connection strings** (1 minute)
   ```bash
   # Update application configuration
   # Option 1: Environment variable
   export DB_HOST=replica-db.example.com
   docker-compose restart orchestrator

   # Option 2: Update DNS CNAME
   # postgres.example.com → replica-db.example.com

   # Option 3: Update connection pool
   # (Most systems auto-detect via health checks)
   ```

4. **Verify new primary** (1 minute)
   ```bash
   # Check writes work
   psql -h replica-db.example.com -U postgres -d workflows \
     -c "CREATE TABLE test_write (id int); DROP TABLE test_write;"

   # Verify workflows accessible
   curl http://localhost:8080/api/v1/workflows
   ```

5. **Monitor replication lag** (ongoing)
   ```bash
   # If you have another replica, check it's replicating from new primary
   psql -h new-primary -U postgres \
     -c "SELECT client_addr, state, sync_state FROM pg_stat_replication;"
   ```

**Expected RTO:** 2 minutes
**Expected RPO:** Replication lag (typically < 30 seconds)

**Rollback:** If promotion fails:
```bash
# Restore from backup
./scripts/backup/restore.sh /backups/llm-orchestrator-backup-latest.tar.gz
```

---

## Application Crash Scenarios

### Scenario 4: Application Process Crash (SIGKILL)

**Description:** Orchestrator process killed unexpectedly (OOM, kill -9, etc.)

**Detection:**
- Process not running (check with `ps` or `docker ps`)
- Health check fails (no response)
- API returns connection refused
- Alert: `orchestrator_up == 0`

**Recovery Procedure:**

1. **Verify process is down** (10 seconds)
   ```bash
   # Check process
   ps aux | grep orchestrator

   # Docker environment
   docker-compose ps orchestrator
   ```

2. **Check crash reason** (30 seconds)
   ```bash
   # Check logs
   docker-compose logs orchestrator --tail=100

   # Check for OOM
   dmesg | grep -i "out of memory"

   # Check exit code
   echo $?  # Non-zero indicates abnormal exit
   ```

3. **Restart application** (30 seconds - 1 minute)
   ```bash
   # Docker environment
   docker-compose up -d orchestrator

   # Kubernetes
   kubectl rollout restart deployment/orchestrator

   # Native
   systemctl restart llm-orchestrator
   ```

4. **Wait for initialization** (30 seconds)
   ```bash
   # Wait for health check
   until curl -f http://localhost:8080/health; do
     echo "Waiting for health check..."
     sleep 2
   done
   ```

5. **Verify workflow recovery** (1 minute)
   ```bash
   # Check active workflows resumed
   curl http://localhost:8080/api/v1/workflows?status=running

   # Monitor logs for recovery
   docker-compose logs -f orchestrator | grep "Recovered workflow"
   ```

**Expected RTO:** 30 seconds (pod restart)
**Expected RPO:** Last checkpoint (< 1 minute)

**Rollback:** N/A (restart is rollback)

---

### Scenario 5: Out of Memory (OOM) Kill

**Description:** Process killed by OOM killer due to memory exhaustion.

**Detection:**
- Process suddenly terminates
- `dmesg` shows OOM killer messages
- Kubernetes event: `OOMKilled`
- Memory metrics show 100% utilization

**Recovery Procedure:**

1. **Confirm OOM kill** (1 minute)
   ```bash
   # Check kernel logs
   dmesg | grep -i "out of memory" | tail -20

   # Kubernetes
   kubectl describe pod orchestrator-xxx | grep -A 5 "OOMKilled"
   ```

2. **Identify memory leak** (2 minutes)
   ```bash
   # Check recent memory usage
   kubectl top pod orchestrator-xxx

   # Review metrics
   curl http://localhost:9090/api/v1/query?query=container_memory_usage_bytes
   ```

3. **Increase memory limits** (1 minute)
   ```bash
   # Edit deployment
   kubectl edit deployment orchestrator

   # Update resources:
   #   limits:
   #     memory: 4Gi  # Increased from 2Gi

   # Or update docker-compose.yml:
   #   mem_limit: 4g
   ```

4. **Restart with new limits** (1 minute)
   ```bash
   kubectl rollout restart deployment/orchestrator

   # Or
   docker-compose up -d orchestrator
   ```

5. **Monitor memory usage** (ongoing)
   ```bash
   # Watch memory
   watch -n 5 'kubectl top pod | grep orchestrator'

   # Set alert for 80% memory usage
   ```

**Expected RTO:** 1 minute
**Expected RPO:** Last checkpoint (< 1 minute)

**Rollback:** Revert memory limits if application doesn't start:
```bash
kubectl rollout undo deployment/orchestrator
```

---

## Network Partition Scenarios

### Scenario 6: Network Partition (App to Database)

**Description:** Network connectivity lost between application and database.

**Detection:**
- Connection timeout errors
- Circuit breaker opens
- All database queries fail
- Alert: `circuit_breaker_open == 1`

**Recovery Procedure:**

1. **Diagnose network issue** (1-2 minutes)
   ```bash
   # Test connectivity
   ping postgres.example.com
   telnet postgres.example.com 5432

   # Check routes
   traceroute postgres.example.com

   # Check firewall
   iptables -L -n -v | grep 5432
   ```

2. **Check for partition** (1 minute)
   ```bash
   # From app side
   curl http://localhost:8080/health
   # Should show database unhealthy

   # From database side (if accessible)
   docker-compose exec postgres pg_isready
   # Should show database is up
   ```

3. **Fix network issue** (varies)
   ```bash
   # Restart network interface (if local issue)
   ifdown eth0 && ifup eth0

   # Fix firewall rule (if blocked)
   iptables -I INPUT -p tcp --dport 5432 -j ACCEPT

   # Wait for network recovery (if external)
   # Circuit breaker will auto-close when connection restored
   ```

4. **Verify recovery** (1 minute)
   ```bash
   # Check health endpoint
   curl http://localhost:8080/health

   # Check circuit breaker status
   curl http://localhost:8080/metrics | grep circuit_breaker
   ```

**Expected RTO:** 1 minute (after network restored)
**Expected RPO:** 0 (no writes during partition)

**Rollback:** N/A

---

## Data Corruption Scenarios

### Scenario 7: Corrupted Workflow State

**Description:** Workflow state data is corrupted in database (invalid JSON, checksum mismatch).

**Detection:**
- Deserialization errors in logs
- Workflow load fails
- Checksum verification fails
- Alert: `state_corruption_detected == 1`

**Recovery Procedure:**

1. **Identify corrupted workflow** (1 minute)
   ```bash
   # Check logs for corruption errors
   docker-compose logs orchestrator | grep -i corruption

   # Find affected workflow
   # Should show workflow ID in error message
   ```

2. **Verify corruption** (30 seconds)
   ```bash
   # Query workflow state
   docker-compose exec postgres psql -U postgres -d workflows \
     -c "SELECT id, workflow_id, status FROM workflow_states WHERE id='<uuid>';"

   # Try to load context data
   docker-compose exec postgres psql -U postgres -d workflows \
     -c "SELECT context_data::text FROM workflow_states WHERE id='<uuid>';"
   # If this fails, data is corrupted
   ```

3. **Find last valid checkpoint** (1 minute)
   ```bash
   # Query checkpoints
   docker-compose exec postgres psql -U postgres -d workflows \
     -c "SELECT id, created_at, checkpoint_data FROM checkpoints
         WHERE workflow_state_id='<uuid>'
         ORDER BY created_at DESC LIMIT 5;"
   ```

4. **Restore from checkpoint** (30 seconds)
   ```bash
   # Using application API
   curl -X POST http://localhost:8080/api/v1/workflows/<id>/restore \
     -H "Content-Type: application/json" \
     -d '{"checkpoint_id": "<checkpoint-uuid>"}'

   # Or manually update database
   docker-compose exec postgres psql -U postgres -d workflows \
     -c "UPDATE workflow_states
         SET context_data = (SELECT checkpoint_data FROM checkpoints WHERE id='<checkpoint-uuid>')
         WHERE id='<workflow-state-uuid>';"
   ```

5. **Verify recovery** (30 seconds)
   ```bash
   # Load workflow
   curl http://localhost:8080/api/v1/workflows/<id>

   # Check status
   # Should show workflow in valid state
   ```

**Expected RTO:** 2 minutes
**Expected RPO:** Last checkpoint (< 1 minute)

**Rollback:** If restore fails, mark workflow as failed:
```bash
curl -X PUT http://localhost:8080/api/v1/workflows/<id> \
  -d '{"status": "failed", "error": "Corruption detected, manual review needed"}'
```

---

## Backup and Restore Procedures

### Scenario 8: Full System Restore from Backup

**Description:** Complete database loss requiring full restore from backup.

**Detection:**
- Database completely unavailable
- All data appears lost
- Database files corrupted beyond repair

**Recovery Procedure:**

1. **Locate latest backup** (1 minute)
   ```bash
   # List backups
   ls -lh /backups/llm-orchestrator/

   # Or from S3
   aws s3 ls s3://my-bucket/backups/llm-orchestrator/ --recursive
   ```

2. **Verify backup integrity** (2 minutes)
   ```bash
   # Download if needed
   aws s3 cp s3://my-bucket/backups/llm-orchestrator-backup-latest.tar.gz /tmp/

   # Verify checksums
   ./scripts/backup/verify_backup.sh /tmp/llm-orchestrator-backup-latest.tar.gz
   ```

3. **Stop application** (30 seconds)
   ```bash
   docker-compose stop orchestrator
   ```

4. **Restore database** (5-10 minutes depending on size)
   ```bash
   # Use restore script
   ./scripts/backup/restore.sh --verify /tmp/llm-orchestrator-backup-latest.tar.gz

   # Follow prompts
   # Enter 'yes' when prompted to confirm
   ```

5. **Verify restoration** (2 minutes)
   ```bash
   # Check workflow count
   docker-compose exec postgres psql -U postgres -d workflows \
     -c "SELECT COUNT(*) FROM workflow_states;"

   # Check data integrity
   docker-compose exec postgres psql -U postgres -d workflows \
     -c "SELECT id, workflow_id, status FROM workflow_states LIMIT 10;"
   ```

6. **Restart application** (1 minute)
   ```bash
   docker-compose up -d orchestrator

   # Wait for health check
   until curl -f http://localhost:8080/health; do sleep 2; done
   ```

7. **Verify application functionality** (2 minutes)
   ```bash
   # Query workflows
   curl http://localhost:8080/api/v1/workflows

   # Try to create a test workflow
   curl -X POST http://localhost:8080/api/v1/workflows \
     -H "Content-Type: application/json" \
     -d @test-workflow.json
   ```

**Expected RTO:** 10 minutes
**Expected RPO:** Time since last backup (hourly backups = up to 1 hour)

**Rollback:** N/A (this IS the rollback)

---

## Multi-Region Failover

### Scenario 9: Primary Region Failure

**Description:** Entire primary region (datacenter/cloud region) becomes unavailable.

**Detection:**
- All services in primary region unreachable
- Health checks fail across the board
- Multi-region monitoring shows region down
- Alert: `region_health{region="primary"} == 0`

**Recovery Procedure:**

1. **Confirm region failure** (2 minutes)
   ```bash
   # Check primary region health
   curl https://primary-region.example.com/health
   # Should timeout or connection refused

   # Check from multiple locations
   # Rule: If 3+ monitoring locations report down, it's a region failure
   ```

2. **Verify secondary region status** (1 minute)
   ```bash
   # Check secondary region is operational
   curl https://secondary-region.example.com/health

   # Check data replication lag
   psql -h secondary-db.example.com -U postgres \
     -c "SELECT now() - pg_last_xact_replay_timestamp() AS replication_lag;"
   ```

3. **Activate secondary region** (2 minutes)
   ```bash
   # Promote secondary database to primary
   psql -h secondary-db.example.com -U postgres \
     -c "SELECT pg_promote();"

   # Scale up secondary application instances
   kubectl scale deployment/orchestrator --replicas=10 -n secondary
   ```

4. **Update DNS** (1-2 minutes + TTL)
   ```bash
   # Update Route53 or DNS provider
   aws route53 change-resource-record-sets \
     --hosted-zone-id Z123456 \
     --change-batch file://failover-dns.json

   # Wait for TTL expiration (typically 60 seconds)
   # Monitor DNS propagation
   dig api.example.com @8.8.8.8
   ```

5. **Verify failover** (2 minutes)
   ```bash
   # Check traffic routing to secondary
   curl https://api.example.com/health
   # Should return secondary region health

   # Verify workflows accessible
   curl https://api.example.com/api/v1/workflows

   # Check metrics
   # Should show traffic in secondary region
   ```

6. **Monitor stability** (ongoing)
   ```bash
   # Watch error rates
   # Watch latency
   # Watch resource utilization

   # Secondary region should handle 100% of traffic
   ```

**Expected RTO:** 5 minutes
**Expected RPO:** Replication lag (< 1 minute typically)

**Rollback (Failback to Primary):** When primary region recovered:
```bash
# 1. Sync data from secondary to primary
# 2. Gradually shift traffic back to primary
# 3. Update DNS to primary
# See Scenario 10 for detailed failback procedure
```

---

## Contact Information

### On-Call Escalation

**Level 1: On-Call Engineer**
- Slack: #oncall-sre
- PagerDuty: Primary on-call

**Level 2: SRE Lead**
- Name: [SRE Lead Name]
- Phone: [Phone Number]
- Slack: @sre-lead

**Level 3: Engineering Manager**
- Name: [Engineering Manager]
- Phone: [Phone Number]
- Slack: @engineering-manager

**Level 4: VP Engineering**
- Name: [VP Engineering]
- Phone: [Phone Number]
- Slack: @vp-engineering

### When to Escalate

- **Immediate Escalation (L2+):**
  - Data loss detected
  - Security incident
  - Multi-region failure
  - RTO/RPO targets cannot be met

- **15-Minute Escalation (L2):**
  - Recovery procedure not working
  - Multiple simultaneous failures
  - Unclear root cause

- **30-Minute Escalation (L2):**
  - Recovery exceeding expected RTO
  - Repeated failures

### Communication Channels

- **Incident Channel:** #incident-response
- **Status Page:** https://status.example.com
- **Monitoring:** https://grafana.example.com
- **Runbook Wiki:** https://wiki.example.com/runbooks

---

## Appendix

### Pre-Recovery Checklist

Before executing any recovery procedure:

- [ ] Verify you have the correct environment (staging vs production)
- [ ] Notify team in #incident-response channel
- [ ] Take screenshots of error states
- [ ] Check if others are already working on the issue
- [ ] Review recent changes (deployments, config updates)
- [ ] Ensure you have necessary credentials and access

### Post-Recovery Checklist

After successful recovery:

- [ ] Verify all services healthy
- [ ] Check recent workflows completed successfully
- [ ] Monitor error rates for 30 minutes
- [ ] Document what happened
- [ ] Update runbooks if procedures changed
- [ ] Schedule post-mortem (for major incidents)
- [ ] Update status page (incident resolved)

### Testing Schedule

Recovery procedures should be tested regularly:

- **Monthly:** Application crash recovery
- **Monthly:** Database connection loss
- **Quarterly:** Backup restore
- **Quarterly:** Database failover
- **Semi-Annual:** Multi-region failover

---

**Document Control:**
- Version: 1.0
- Last Review: 2025-11-14
- Next Review: 2026-02-14
- Approved By: SRE Team Lead
