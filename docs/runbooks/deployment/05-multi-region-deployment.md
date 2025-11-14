# Multi-Region Deployment

## Overview
Deploy and manage the LLM Orchestrator across multiple regions for high availability and disaster recovery.

## Prerequisites
- Multiple Kubernetes clusters in different regions
- Cross-region database replication setup
- Global load balancer configured
- Secrets synchronized across regions

## Impact Assessment
- Severity: Medium
- User impact: Improved availability and latency
- Business impact: Enables global reach and HA

## Step-by-Step Procedure

### Step 1: Deploy to Primary Region

```bash
# Deploy to us-east-1 (primary)
kubectl --context=us-east-1 apply -f deployment/

# Verify deployment
kubectl --context=us-east-1 get pods -n llm-orchestrator
```

### Step 2: Setup Database Replication

```bash
# Configure PostgreSQL streaming replication
# Primary: us-east-1, Replica: us-west-2

# On primary, enable WAL archiving
kubectl --context=us-east-1 exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "ALTER SYSTEM SET wal_level = replica;"

# Create replication user
kubectl --context=us-east-1 exec -n llm-orchestrator postgres-0 -- \
  psql -U orchestrator -c "CREATE USER replicator WITH REPLICATION ENCRYPTED PASSWORD 'rep_password';"

# On replica region, setup standby
kubectl --context=us-west-2 apply -f deployment/postgres-standby.yaml
```

### Step 3: Deploy to Secondary Regions

```bash
# Deploy to us-west-2 (secondary)
kubectl --context=us-west-2 apply -f deployment/

# Configure to use replica database
kubectl --context=us-west-2 patch deployment orchestrator -n llm-orchestrator -p '
{
  "spec": {
    "template": {
      "spec": {
        "containers": [{
          "name": "orchestrator",
          "env": [{
            "name": "DATABASE_URL",
            "value": "postgresql://orchestrator:password@postgres-replica.llm-orchestrator.svc.cluster.local:5432/orchestrator?target_session_attrs=prefer-standby"
          }]
        }]
      }
    }
  }
}'
```

### Step 4: Configure Global Load Balancer

```bash
# Using AWS Route53 with health checks
aws route53 change-resource-record-sets --hosted-zone-id Z1234 --change-batch '
{
  "Changes": [{
    "Action": "UPSERT",
    "ResourceRecordSet": {
      "Name": "orchestrator.example.com",
      "Type": "A",
      "SetIdentifier": "us-east-1",
      "Weight": 70,
      "AliasTarget": {
        "HostedZoneId": "Z1234",
        "DNSName": "us-east-1-lb.example.com",
        "EvaluateTargetHealth": true
      }
    }
  },
  {
    "Action": "UPSERT",
    "ResourceRecordSet": {
      "Name": "orchestrator.example.com",
      "Type": "A",
      "SetIdentifier": "us-west-2",
      "Weight": 30,
      "AliasTarget": {
        "HostedZoneId": "Z5678",
        "DNSName": "us-west-2-lb.example.com",
        "EvaluateTargetHealth": true
      }
    }
  }]
}'
```

### Step 5: Verify Multi-Region Setup

```bash
# Test primary region
curl https://us-east-1.orchestrator.example.com/health

# Test secondary region
curl https://us-west-2.orchestrator.example.com/health

# Test global endpoint (should route to closest/healthiest)
curl https://orchestrator.example.com/health

# Verify database replication lag
kubectl --context=us-west-2 exec -n llm-orchestrator postgres-replica-0 -- \
  psql -U orchestrator -c "SELECT now() - pg_last_xact_replay_timestamp() AS replication_lag;"

# Expected: Lag < 1 second
```

## Validation

```bash
# All regions healthy
for region in us-east-1 us-west-2; do
  echo "Checking $region..."
  kubectl --context=$region get pods -n llm-orchestrator
done

# Database replication working
kubectl --context=us-west-2 exec postgres-replica-0 -n llm-orchestrator -- \
  psql -U orchestrator -c "SELECT pg_is_in_recovery();"
# Expected: t (true)

# Global routing working
dig orchestrator.example.com
```

## Related Runbooks
- [01-initial-deployment.md](./01-initial-deployment.md)
- [../incidents/08-network-issues.md](../incidents/08-network-issues.md)
- [../maintenance/01-database-maintenance.md](../maintenance/01-database-maintenance.md)

---
**Last Updated**: 2025-11-14
**Version**: 1.0
