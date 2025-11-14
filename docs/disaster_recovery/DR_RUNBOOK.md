# Disaster Recovery Runbook

**Quick Reference for On-Call Engineers**

## Incident Severity Classification

### P0 - Critical (15 min response)
- Complete system outage
- Data loss detected
- Security breach
- Multi-region failure

**Action:** Page L2 immediately

### P1 - High (30 min response)
- Service degradation affecting >50% users
- Database primary down
- Workflow execution failures
- Authentication outage

**Action:** Engage on-call, notify L2

### P2 - Medium (1 hour response)
- Service degradation affecting <50% users
- Performance issues
- Non-critical service down

**Action:** Engage on-call

### P3 - Low (4 hour response)
- Minor issues
- Individual workflow failures
- Logging issues

**Action:** Create ticket, handle during business hours

---

## Quick Diagnostics

### Is the System Down?

```bash
# Check health endpoint
curl http://localhost:8080/health

# Check application status
docker-compose ps
# or
kubectl get pods -n llm-orchestrator

# Check database
pg_isready -h localhost -p 5432
```

### What's Failing?

```bash
# Check recent logs
docker-compose logs --tail=100 orchestrator

# Check error rates
curl http://localhost:8080/metrics | grep error_total

# Check active alerts
curl http://prometheus:9090/api/v1/alerts
```

---

## Top 5 Most Common Issues

### 1. Database Connection Pool Exhausted

**Symptoms:**
- "Connection pool timeout" in logs
- Health check fails
- Workflows stuck

**Quick Fix:**
```bash
# Restart application
docker-compose restart orchestrator

# Monitor connection pool
curl http://localhost:8080/metrics | grep db_pool
```

**Root Cause:** Too many concurrent workflows or connection leak

**Prevention:** Tune connection pool size, add connection timeouts

---

### 2. Application Pod Crashed (OOM)

**Symptoms:**
- Pod in CrashLoopBackoff
- OOMKilled events
- Memory metrics at 100%

**Quick Fix:**
```bash
# Check OOM
kubectl describe pod orchestrator-xxx | grep OOMKilled

# Increase memory limit
kubectl edit deployment orchestrator
# Update memory: 4Gi

# Restart
kubectl rollout restart deployment/orchestrator
```

**Root Cause:** Memory leak or insufficient memory allocation

**Prevention:** Set proper limits, monitor memory usage

---

### 3. Workflow Stuck in Running State

**Symptoms:**
- Workflow not progressing
- No recent checkpoints
- Timeout errors

**Quick Fix:**
```bash
# Check workflow status
curl http://localhost:8080/api/v1/workflows/<id>

# Cancel and retry
curl -X POST http://localhost:8080/api/v1/workflows/<id>/cancel
curl -X POST http://localhost:8080/api/v1/workflows/<id>/retry
```

**Root Cause:** External API timeout, deadlock, or bug

**Prevention:** Set workflow timeouts, implement retry logic

---

### 4. Database Replication Lag High

**Symptoms:**
- Replication lag >30 seconds
- Stale data on replica
- Failover concerns

**Quick Fix:**
```bash
# Check replication lag
psql -h replica -U postgres \
  -c "SELECT now() - pg_last_xact_replay_timestamp() AS lag;"

# Check replication status
psql -h primary -U postgres \
  -c "SELECT * FROM pg_stat_replication;"

# If lag excessive, may need to rebuild replica
```

**Root Cause:** Heavy write load, network latency, replica under-resourced

**Prevention:** Monitor lag, scale replica resources

---

### 5. Circuit Breaker Open

**Symptoms:**
- "Circuit breaker open" errors
- All requests to external service failing
- Automatic retry not working

**Quick Fix:**
```bash
# Check circuit breaker status
curl http://localhost:8080/metrics | grep circuit_breaker

# Wait for cool-down period (typically 30-60 seconds)
# Circuit breaker will auto-close if service recovers

# Force reset (use with caution)
curl -X POST http://localhost:8080/admin/circuit-breaker/reset
```

**Root Cause:** External service outage or network issue

**Prevention:** Tune circuit breaker thresholds, implement fallbacks

---

## Emergency Commands

### Restart Application
```bash
# Docker
docker-compose restart orchestrator

# Kubernetes
kubectl rollout restart deployment/orchestrator
```

### Check Logs (Last 5 minutes)
```bash
# Docker
docker-compose logs --since=5m orchestrator

# Kubernetes
kubectl logs -f deployment/orchestrator --since=5m
```

### Database Health Check
```bash
# Connection test
pg_isready -h localhost -p 5432

# Quick query
psql -U postgres -d workflows -c "SELECT COUNT(*) FROM workflow_states;"
```

### Force Workflow Cancel
```bash
curl -X POST http://localhost:8080/api/v1/workflows/<id>/cancel \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

### Emergency Maintenance Mode
```bash
# Enable maintenance mode (reject new requests)
curl -X POST http://localhost:8080/admin/maintenance/enable

# Disable when done
curl -X POST http://localhost:8080/admin/maintenance/disable
```

---

## Escalation Procedures

### When to Escalate

**Immediate (Page L2):**
- P0 incident
- Data loss
- Security incident
- RTO at risk (>80% of target time elapsed)

**15 Minutes:**
- No progress on recovery
- Unclear root cause
- Multiple failures

**30 Minutes:**
- Recovery not working
- Need additional help

### How to Escalate

1. **Update incident channel:**
   ```
   #incident-response:
   ESCALATING to L2
   Incident: <description>
   Duration: <time>
   Impact: <user impact>
   Attempted: <what you tried>
   ```

2. **Page on-call L2:**
   - PagerDuty: Select "Escalate"
   - Phone: Call L2 directly (in roster)

3. **Prepare handoff:**
   - Timeline of events
   - What was tried
   - Current system state
   - Next steps needed

---

## Communication Templates

### Incident Start
```
#incident-response
🚨 INCIDENT DETECTED 🚨
Severity: P1
Impact: Workflows failing, 50% error rate
Started: 14:32 UTC
Owner: @oncall-engineer
Status Page: UPDATED
```

### Status Update (Every 15 min for P0/P1)
```
#incident-response
📊 UPDATE (15 min)
Current status: Investigating database connection issues
Progress: Identified connection pool exhaustion
Next step: Restarting application to reset pool
ETA: 5 minutes
```

### Incident Resolution
```
#incident-response
✅ INCIDENT RESOLVED
Duration: 23 minutes
Root cause: Database connection pool exhausted
Fix: Restarted application, increased pool size
Impact: ~500 workflows delayed, all recovered
Follow-up:
- Post-mortem scheduled for tomorrow
- Ticket #1234 to tune connection pool
- Monitoring added for pool utilization
```

---

## Post-Incident Actions

### Immediate (Within 1 hour)
- [ ] Update status page (resolved)
- [ ] Verify system stable for 30 minutes
- [ ] Document in incident log
- [ ] Thank responders

### Within 24 hours
- [ ] Write incident summary
- [ ] Update affected customers (if external)
- [ ] Schedule post-mortem
- [ ] Create follow-up tickets

### Within 1 week
- [ ] Conduct post-mortem (blameless)
- [ ] Update runbooks
- [ ] Implement preventive measures
- [ ] Update monitoring/alerts

---

## Important URLs

- **Monitoring:** https://grafana.example.com
- **Logs:** https://kibana.example.com
- **Alerts:** https://prometheus.example.com/alerts
- **Status:** https://status.example.com
- **Runbooks:** https://wiki.example.com/runbooks
- **PagerDuty:** https://company.pagerduty.com

## Important Files

- **Logs:** `/var/log/llm-orchestrator/`
- **Config:** `/etc/llm-orchestrator/config.yaml`
- **Backups:** `/backups/llm-orchestrator/`
- **Scripts:** `/opt/llm-orchestrator/scripts/`

## Database Connection

```bash
# Production
psql -h prod-db.example.com -U postgres -d workflows

# Staging
psql -h staging-db.example.com -U postgres -d workflows

# Local
psql -h localhost -U postgres -d workflows
```

---

## Recovery Time Guidelines

| Scenario | Target RTO | Target RPO |
|----------|-----------|-----------|
| Application crash | 30 seconds | <1 minute |
| Database connection loss | 2 minutes | 0 |
| Database crash | 2 minutes | <1 minute |
| Database failover | 2 minutes | 30 seconds |
| Network partition | 1 minute | 0 |
| Data corruption | 2 minutes | <1 minute |
| Backup restore | 10 minutes | 1 hour |
| Multi-region failover | 5 minutes | <1 minute |

---

**Remember:**
1. Safety first - don't make it worse
2. Communicate early and often
3. Document everything
4. Escalate when stuck
5. It's okay to ask for help

**You've got this! 💪**

---

*Last Updated: 2025-11-14*
*Version: 1.0*
