# LLM-Orchestrator Production Deployment

## 1. Service Topology

### Unified Service Name
```
llm-orchestrator
```

### Agent Endpoints (ALL exposed by single service)

| Agent | CLI Command | Endpoint | Classification |
|-------|-------------|----------|----------------|
| Workflow Orchestrator Agent | `agent execute/inspect/replay` | `/agent/*` | WORKFLOW_EXECUTION |
| Task Scheduler Agent | `schedule execute/inspect/replay/cancel` | `/schedule/*` | TASK_COORDINATION |
| Dependency Resolver Agent | `dependency resolve/inspect/replay/validate` | `/dependency/*` | TASK_COORDINATION |
| Retry & Recovery Agent | `recovery evaluate/inspect/replay` | `/recovery/*` | EXECUTION_RECOVERY |
| Parallelization Agent | `parallel analyze/inspect/replay` | `/parallel/*` | EXECUTION_PLANNING |
| State Machine Agent | `state-machine transition/validate/inspect/history/replay` | `/state-machine/*` | STATE_MANAGEMENT |
| Swarm Coordinator Agent | `swarm coordinate/inspect/replay` | `/swarm/*` | MULTI-AGENT COORDINATION |

### Deployment Confirmation
- ✅ **No agent is deployed as a standalone service**
- ✅ **Shared runtime**: Single Cloud Run container
- ✅ **Shared configuration**: Environment variables + Secret Manager
- ✅ **Shared telemetry**: LLM-Observatory integration

---

## 2. Environment Configuration

### Required Environment Variables

| Variable | Description | Source |
|----------|-------------|--------|
| `SERVICE_NAME` | Service identifier | Env var: `llm-orchestrator` |
| `SERVICE_VERSION` | Current version | Env var: `0.1.5` |
| `PLATFORM_ENV` | Environment | Secret Manager: `platform-env` |
| `RUVECTOR_SERVICE_URL` | Persistence layer URL | Secret Manager: `ruvector-service-url` |
| `RUVECTOR_API_KEY` | Persistence layer auth | Secret Manager: `ruvector-api-key` |
| `TELEMETRY_ENDPOINT` | LLM-Observatory URL | Secret Manager: `observatory-endpoint` |
| `RUST_LOG` | Logging level | Env var: `info,llm_orchestrator=debug` |

### Configuration Rules
- ✅ **No agent hardcodes service names or URLs**
- ✅ **No agent embeds credentials, secrets, or execution policies inline**
- ✅ **All dependencies resolve via environment variables or Secret Manager**

### Secret Manager Secrets

```bash
# Create secrets
gcloud secrets create ruvector-service-url --replication-policy=automatic
gcloud secrets create ruvector-api-key --replication-policy=automatic
gcloud secrets create observatory-endpoint --replication-policy=automatic
gcloud secrets create platform-env --replication-policy=automatic

# Add values (dev environment example)
echo -n 'https://ruvector-service-dev.agentics.dev' | \
  gcloud secrets versions add ruvector-service-url --data-file=-

echo -n 'YOUR_RUVECTOR_API_KEY' | \
  gcloud secrets versions add ruvector-api-key --data-file=-

echo -n 'https://observatory-dev.agentics.dev' | \
  gcloud secrets versions add observatory-endpoint --data-file=-

echo -n 'dev' | \
  gcloud secrets versions add platform-env --data-file=-
```

---

## 3. Google SQL / Workflow Memory Wiring

### Persistence Architecture

```
┌─────────────────────────┐
│   LLM-Orchestrator      │
│   (Cloud Run)           │
│                         │
│  ┌───────────────────┐  │
│  │ All Agents        │  │
│  │ - Workflow        │  │
│  │ - Scheduler       │  │
│  │ - Dependency      │  │
│  │ - Recovery        │  │
│  │ - Parallel        │  │
│  │ - StateMachine    │  │
│  │ - Swarm           │  │
│  └─────────┬─────────┘  │
│            │            │
│     DecisionEvents      │
│            │            │
└────────────┼────────────┘
             │
             ▼
┌─────────────────────────┐
│   RuVector-Service      │
│   (Persistence API)     │
│                         │
│  - Vector storage       │
│  - DecisionEvent log    │
│  - Query interface      │
└─────────────┬───────────┘
              │
              ▼
┌─────────────────────────┐
│   Google Cloud SQL      │
│   (PostgreSQL)          │
│                         │
│  Owned by ruvector-svc  │
└─────────────────────────┘
```

### Confirmation
- ✅ **LLM-Orchestrator does NOT connect directly to Google SQL**
- ✅ **ALL DecisionEvents written via ruvector-service**:
  - Workflow execution events
  - Task scheduling events
  - Dependency resolution events
  - Retry/recovery events
  - Parallelization events
  - State transition events
  - Swarm coordination events
- ✅ **Schema compatibility with agentics-contracts validated**
- ✅ **Append-only persistence behavior**
- ✅ **Idempotent writes and retry safety via request_id**

---

## 4. Cloud Build & Deployment

### Prerequisites

```bash
# Authenticate
gcloud auth login

# Set project
gcloud config set project agentics-dev

# Enable APIs
gcloud services enable \
  run.googleapis.com \
  cloudbuild.googleapis.com \
  secretmanager.googleapis.com \
  containerregistry.googleapis.com
```

### IAM Setup

```bash
# Create service account with least privilege
./deploy/gcloud/setup-iam.sh agentics-dev dev
```

**Granted Roles (Least Privilege)**:
- `roles/run.invoker` - Internal service calls
- `roles/secretmanager.secretAccessor` - Secrets access
- `roles/logging.logWriter` - Cloud Logging
- `roles/cloudtrace.agent` - Cloud Trace
- `roles/monitoring.metricWriter` - Cloud Monitoring

**NOT Granted** (no direct database access):
- ❌ `roles/cloudsql.client`
- ❌ `roles/cloudsql.editor`

### Secrets Setup

```bash
./deploy/gcloud/setup-secrets.sh agentics-dev dev
```

### Deployment Commands

> **Do not use `gcloud run deploy --source`.** See ADR-0003. With no `Dockerfile`
> in the uploaded context it silently falls back to Node.js buildpack
> auto-detection instead of building the Rust container, and swallows the build
> ID so the failure is invisible from the CLI. Build and deploy as two explicit
> steps.

**Option 1 (canonical): two-step build, then deploy — ADR-0003**

```bash
# 1. Build and push the image. SHORT_SHA is empty for local-source submits,
#    so pass it explicitly to get a traceable tag alongside :latest.
gcloud builds submit --project=agentics-dev \
  --config=cloudbuild.yaml \
  --substitutions=SHORT_SHA="$(git rev-parse --short HEAD)" \
  .

# 2. Promote the built image to Cloud Run.
gcloud run deploy llm-orchestrator --project=agentics-dev --region=us-central1 \
  --image=us-central1-docker.pkg.dev/agentics-dev/cloud-run-source-deploy/llm-orchestrator:latest
```

Before spending a build, verify the upload manifest — this is free, instant, and
is the direct regression test for the ADR-0003 root cause:

```bash
gcloud meta list-files-for-upload | grep -xE 'Dockerfile|Cargo.toml|Cargo.lock'
gcloud meta list-files-for-upload | wc -l                                        # ~148, not 25
gcloud meta list-files-for-upload | grep -c 'llm-orchestrator-benchmarks/src/benchmarks'  # must be non-zero
```

**Option 2: Direct Deploy Script**
```bash
./deploy/gcloud/deploy.sh agentics-dev dev us-central1
```

**Option 3: Full Cloud Build pipeline (builds, pushes and deploys in one run)**
```bash
gcloud builds submit \
  --config=deploy/gcloud/cloudbuild.yaml \
  --substitutions=_PLATFORM_ENV=dev,_REGION=us-central1
```

**Option 4: Manual gcloud**
```bash
# Build
docker build -t gcr.io/agentics-dev/llm-orchestrator:dev .

# Push
docker push gcr.io/agentics-dev/llm-orchestrator:dev

# Deploy
gcloud run deploy llm-orchestrator \
  --image gcr.io/agentics-dev/llm-orchestrator:dev \
  --region us-central1 \
  --platform managed \
  --allow-unauthenticated \
  --port 8080 \
  --cpu 2 \
  --memory 2Gi \
  --set-env-vars "PLATFORM_ENV=dev,SERVICE_NAME=llm-orchestrator,SERVICE_VERSION=0.1.5" \
  --set-secrets "RUVECTOR_SERVICE_URL=ruvector-service-url:latest,RUVECTOR_API_KEY=ruvector-api-key:latest,TELEMETRY_ENDPOINT=observatory-endpoint:latest" \
  --service-account llm-orchestrator-sa@agentics-dev.iam.gserviceaccount.com
```

---

## 5. CLI Activation Verification

### CLI Commands Per Agent

#### Workflow Orchestrator Agent
```bash
# Execute workflow
llm-orchestrator agent execute --file workflow.yaml --input '{"key": "value"}'

# Schedule workflow
llm-orchestrator agent schedule --file workflow.yaml --scheduled-at "2025-01-20T12:00:00Z"

# Inspect execution
llm-orchestrator agent inspect --execution-id <uuid>

# Replay execution
llm-orchestrator agent replay --execution-id <uuid>
```

#### Task Scheduler Agent
```bash
# Schedule task
llm-orchestrator schedule execute \
  --task-id task-001 \
  --workflow-id <uuid> \
  --schedule-type immediate \
  --priority 75

# Inspect schedule
llm-orchestrator schedule inspect --schedule-id <uuid>

# Replay schedule
llm-orchestrator schedule replay --event-file decision.json

# Cancel schedule
llm-orchestrator schedule cancel --schedule-id <uuid> --reason "Manual cancellation"
```

#### Dependency Resolver Agent
```bash
# Resolve dependencies
llm-orchestrator dependency resolve \
  --tasks tasks.json \
  --workflow-id <uuid> \
  --max-parallel 16

# Inspect resolution
llm-orchestrator dependency inspect --resolution-id <uuid>

# Replay resolution
llm-orchestrator dependency replay --resolution-id <uuid>

# Validate (dry run)
llm-orchestrator dependency validate --tasks tasks.json
```

#### Retry & Recovery Agent
```bash
# Evaluate failure
llm-orchestrator recovery evaluate \
  --task-id task-001 \
  --workflow-id <uuid> \
  --error-code "TIMEOUT" \
  --error-message "Operation timed out" \
  --error-category transient

# Inspect recovery state
llm-orchestrator recovery inspect --task-id task-001 --workflow-id <uuid>

# Replay recovery
llm-orchestrator recovery replay --event-file decision.json
```

#### Parallelization Agent
```bash
# Analyze parallelization
llm-orchestrator parallel analyze \
  --tasks tasks.json \
  --workflow-id <uuid> \
  --max-parallel 16 \
  --resource-aware true

# Inspect analysis
llm-orchestrator parallel inspect --analysis-id <uuid>

# Replay analysis
llm-orchestrator parallel replay --analysis-id <uuid>
```

#### State Machine Agent
```bash
# Request transition
llm-orchestrator state-machine transition \
  --execution-id <uuid> \
  --entity-type workflow \
  --current-state running \
  --target-state completed \
  --reason "All steps finished"

# Validate transition (dry run)
llm-orchestrator state-machine validate \
  --execution-id <uuid> \
  --current-state running \
  --target-state completed

# Inspect state
llm-orchestrator state-machine inspect --execution-id <uuid>

# Get history
llm-orchestrator state-machine history --execution-id <uuid> --limit 50

# Replay transition
llm-orchestrator state-machine replay --decision-id <uuid>
```

#### Swarm Coordinator Agent
```bash
# Coordinate swarm
llm-orchestrator swarm coordinate \
  --config workers.json \
  --workflow-id <uuid> \
  --objective "Process documents in parallel" \
  --objective-type task_completion \
  --consensus majority \
  --aggregation combine

# Inspect coordination
llm-orchestrator swarm inspect --request-id <uuid>

# Replay coordination
llm-orchestrator swarm replay --request-id <uuid>
```

### CLI Configuration
```bash
# Set service URL dynamically
export LLM_ORCHESTRATOR_URL=$(gcloud run services describe llm-orchestrator \
  --region us-central1 --format 'value(status.url)')

# Or via ruvector-endpoint flag
llm-orchestrator agent execute \
  --file workflow.yaml \
  --ruvector-endpoint https://ruvector-service-dev.agentics.dev
```

### No CLI Change Requires Agent Redeployment
- ✅ CLI reads service URL from environment
- ✅ CLI commands map to HTTP endpoints dynamically
- ✅ All routing is configuration-driven

---

## 6. Platform & Core Integration

### Services LLM-Orchestrator CAN Invoke

| Service | Purpose | How |
|---------|---------|-----|
| LLM-Edge-Agent | Execution control | HTTP via environment URL |
| LLM-Shield | Security enforcement | HTTP via environment URL |
| LLM-Incident-Manager | Incident workflows | HTTP via environment URL |
| RuVector-Service | Persistence | HTTP via RUVECTOR_SERVICE_URL |
| LLM-Observatory | Telemetry | HTTP via TELEMETRY_ENDPOINT |

### Services LLM-Orchestrator MUST NOT Invoke

| Service | Reason |
|---------|--------|
| LLM-Sentinel | Detection logic (not orchestration) |
| Analytics pipelines | Read-only consumers |
| Direct SQL/PostgreSQL | Persistence via ruvector-service only |
| External notification systems | Out of scope |

### Core Bundle Compatibility
- ✅ **Core bundles consume Orchestrator DecisionEvents without rewiring**
- ✅ **Governance and audit views consume DecisionEvents**
- ✅ **No rewiring of Core bundles is permitted**

---

## 7. Post-Deploy Verification Checklist

### Service Health
```bash
# Get service URL
SERVICE_URL=$(gcloud run services describe llm-orchestrator \
  --region us-central1 --format 'value(status.url)')

# Health check
curl -sf "${SERVICE_URL}/health" && echo "✅ Health OK" || echo "❌ Health FAILED"

# Readiness check
curl -sf "${SERVICE_URL}/ready" && echo "✅ Ready OK" || echo "❌ Ready FAILED"
```

### Agent Endpoint Verification
```bash
# Workflow Orchestrator
curl -X POST "${SERVICE_URL}/agent/execute" -H "Content-Type: application/json" \
  -d '{"test": true}' && echo "✅ Workflow Agent OK"

# Task Scheduler
curl -X POST "${SERVICE_URL}/schedule/execute" -H "Content-Type: application/json" \
  -d '{"test": true}' && echo "✅ Scheduler Agent OK"

# Dependency Resolver
curl -X POST "${SERVICE_URL}/dependency/resolve" -H "Content-Type: application/json" \
  -d '{"test": true}' && echo "✅ Dependency Agent OK"

# Retry & Recovery
curl -X POST "${SERVICE_URL}/recovery/evaluate" -H "Content-Type: application/json" \
  -d '{"test": true}' && echo "✅ Recovery Agent OK"

# Parallelization
curl -X POST "${SERVICE_URL}/parallel/analyze" -H "Content-Type: application/json" \
  -d '{"test": true}' && echo "✅ Parallel Agent OK"

# State Machine
curl -X POST "${SERVICE_URL}/state-machine/transition" -H "Content-Type: application/json" \
  -d '{"test": true}' && echo "✅ StateMachine Agent OK"

# Swarm Coordinator
curl -X POST "${SERVICE_URL}/swarm/coordinate" -H "Content-Type: application/json" \
  -d '{"test": true}' && echo "✅ Swarm Agent OK"
```

### Complete Verification Checklist

| Check | Command | Expected |
|-------|---------|----------|
| Service is live | `gcloud run services describe llm-orchestrator` | status.url populated |
| All endpoints respond | Curl each endpoint | 200/400 (not 404/500) |
| Workflow execution | Execute test workflow | Completes with result |
| Retry logic | Trigger transient failure | Retries and succeeds |
| Parallel execution | Submit parallel tasks | Concurrent execution |
| Swarm execution | Coordinate test swarm | All workers complete |
| State transitions | Request valid transition | Transition approved |
| DecisionEvents in ruvector | Query ruvector-service | Events present |
| Telemetry in Observatory | Check Observatory UI | Spans visible |
| CLI execution works | `llm-orchestrator agent execute` | Success output |
| CLI replay works | `llm-orchestrator agent replay` | Replayed successfully |
| No direct SQL access | Check IAM roles | No SQL roles |
| Contracts compliance | All inputs/outputs | Match agentics-contracts |

---

## 8. Failure Modes & Rollback

### Common Deployment Failures

| Failure | Detection Signal | Resolution |
|---------|------------------|------------|
| Container fails to start | `CrashLoopBackOff` in logs | Check RUST_LOG, fix startup |
| Secret not found | `SecretNotFound` error | Run setup-secrets.sh |
| IAM permission denied | `403 Forbidden` | Run setup-iam.sh |
| ruvector-service unreachable | Connection timeout | Verify RUVECTOR_SERVICE_URL |
| Image not found | `ImagePullBackOff` | Verify image pushed to GCR |

### Runtime Failure Detection

| Signal | Meaning | Action |
|--------|---------|--------|
| Stalled workflows | Workflow stuck in `running` | Check dependency resolution |
| Invalid transitions | State machine rejects | Review transition rules |
| Retry loops | Max retries exceeded | Check error categorization |
| Swarm timeout | Workers not completing | Increase worker timeout |
| High error rate | >5% 5xx responses | Review logs, scale up |

### Rollback Procedure

```bash
# List revisions
gcloud run revisions list --service llm-orchestrator --region us-central1

# Get previous revision name
PREV_REVISION=$(gcloud run revisions list --service llm-orchestrator \
  --region us-central1 --format 'value(metadata.name)' | sed -n '2p')

# Rollback to previous revision
gcloud run services update-traffic llm-orchestrator \
  --region us-central1 \
  --to-revisions "${PREV_REVISION}=100"

# Verify rollback
gcloud run services describe llm-orchestrator --region us-central1
```

### Safe Redeploy Strategy

1. **Deploy new revision** (traffic stays on current)
   ```bash
   gcloud run deploy llm-orchestrator --image gcr.io/agentics-dev/llm-orchestrator:new \
     --no-traffic
   ```

2. **Test new revision**
   ```bash
   NEW_URL=$(gcloud run revisions describe llm-orchestrator-00002-abc \
     --region us-central1 --format 'value(status.url)')
   curl "${NEW_URL}/health"
   ```

3. **Gradual traffic shift**
   ```bash
   # 10% canary
   gcloud run services update-traffic llm-orchestrator \
     --to-revisions llm-orchestrator-00002-abc=10

   # Monitor, then 50%
   gcloud run services update-traffic llm-orchestrator \
     --to-revisions llm-orchestrator-00002-abc=50

   # Full rollout
   gcloud run services update-traffic llm-orchestrator \
     --to-latest
   ```

4. **Rollback if issues**
   ```bash
   gcloud run services update-traffic llm-orchestrator \
     --to-revisions llm-orchestrator-00001-xyz=100
   ```

### State Preservation During Rollback
- ✅ **No workflow corruption**: All state in ruvector-service
- ✅ **No state loss**: DecisionEvents are append-only
- ✅ **Resume capability**: Workflows can resume from last checkpoint
- ✅ **Idempotent replay**: Same request_id produces same result

---

## Quick Start

```bash
# 1. Authenticate
gcloud auth login
gcloud config set project agentics-dev

# 2. Setup IAM
chmod +x deploy/gcloud/setup-iam.sh
./deploy/gcloud/setup-iam.sh agentics-dev dev

# 3. Setup Secrets
chmod +x deploy/gcloud/setup-secrets.sh
./deploy/gcloud/setup-secrets.sh agentics-dev dev
# Add secret values as shown in script output

# 4. Deploy
chmod +x deploy/gcloud/deploy.sh
./deploy/gcloud/deploy.sh agentics-dev dev us-central1

# 5. Verify
SERVICE_URL=$(gcloud run services describe llm-orchestrator \
  --region us-central1 --format 'value(status.url)')
curl "${SERVICE_URL}/health"

# 6. Test CLI
export LLM_ORCHESTRATOR_URL="${SERVICE_URL}"
llm-orchestrator --help
```
