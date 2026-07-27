#!/usr/bin/env bash
# =============================================================================
# LLM-Orchestrator IAM Setup Script
# =============================================================================
# Creates service account with least-privilege permissions
# Run once per environment: ./setup-iam.sh <project-id> <env> [region]
# =============================================================================

set -euo pipefail

PROJECT_ID="${1:-agentics-dev}"
PLATFORM_ENV="${2:-dev}"
SERVICE_NAME="llm-orchestrator"
SA_NAME="${SERVICE_NAME}-sa"
SA_EMAIL="${SA_NAME}@${PROJECT_ID}.iam.gserviceaccount.com"
REGION="${3:-us-central1}"

# The Cloud Function that fronts the engine. Since ADR-0001 it holds no orchestration logic
# and must call the Cloud Run service for every agent request, so it needs run.invoker on
# that service specifically -- not project-wide.
FUNCTION_SA_EMAIL="${FUNCTION_SA_EMAIL:-${PROJECT_ID}@appspot.gserviceaccount.com}"

echo "=============================================="
echo "LLM-Orchestrator IAM Setup"
echo "=============================================="
echo "Project:         ${PROJECT_ID}"
echo "Environment:     ${PLATFORM_ENV}"
echo "Service Account: ${SA_EMAIL}"
echo "=============================================="

# Create service account
echo "[1/7] Creating service account..."
gcloud iam service-accounts create "${SA_NAME}" \
  --project="${PROJECT_ID}" \
  --display-name="LLM-Orchestrator Service Account" \
  --description="Service account for LLM-Orchestrator unified service" \
  2>/dev/null || echo "Service account already exists"

# Grant Cloud Run Invoker role (for internal service calls)
echo "[2/7] Granting Cloud Run Invoker role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member="serviceAccount:${SA_EMAIL}" \
  --role="roles/run.invoker" \
  --condition=None \
  --quiet

# Grant Secret Manager Secret Accessor role
echo "[3/7] Granting Secret Manager accessor role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member="serviceAccount:${SA_EMAIL}" \
  --role="roles/secretmanager.secretAccessor" \
  --condition=None \
  --quiet

# Grant Cloud Logging Writer role
echo "[4/7] Granting Cloud Logging writer role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member="serviceAccount:${SA_EMAIL}" \
  --role="roles/logging.logWriter" \
  --condition=None \
  --quiet

# Grant Cloud Trace Agent role
echo "[5/7] Granting Cloud Trace agent role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member="serviceAccount:${SA_EMAIL}" \
  --role="roles/cloudtrace.agent" \
  --condition=None \
  --quiet

# Grant Cloud Monitoring Metric Writer role
echo "[6/7] Granting Cloud Monitoring metric writer role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member="serviceAccount:${SA_EMAIL}" \
  --role="roles/monitoring.metricWriter" \
  --condition=None \
  --quiet

# Grant the Cloud Function permission to invoke the engine (ADR-0001).
# Scoped to the one service rather than the whole project: the function needs to call the
# orchestration engine and nothing else.
echo "[7/7] Granting the Cloud Function invoker access to ${SERVICE_NAME}..."
gcloud run services add-iam-policy-binding "${SERVICE_NAME}" \
  --project="${PROJECT_ID}" \
  --region="${REGION}" \
  --member="serviceAccount:${FUNCTION_SA_EMAIL}" \
  --role="roles/run.invoker" \
  --quiet \
  || echo "  Skipped: the ${SERVICE_NAME} service does not exist yet in ${REGION}. Re-run after deploy.sh."

echo ""
echo "=============================================="
echo "IAM Setup Complete"
echo "=============================================="
echo ""
echo "Service Account: ${SA_EMAIL}"
echo ""
echo "Granted Roles:"
echo "  - roles/run.invoker (internal service calls)"
echo "  - roles/secretmanager.secretAccessor (secrets access)"
echo "  - roles/logging.logWriter (Cloud Logging)"
echo "  - roles/cloudtrace.agent (Cloud Trace)"
echo "  - roles/monitoring.metricWriter (Cloud Monitoring)"
echo ""
echo "Cloud Function (${FUNCTION_SA_EMAIL}):"
echo "  - roles/run.invoker on the ${SERVICE_NAME} service only (ADR-0001)"
echo ""
echo "NOTE: This service account has NO direct database access."
echo "      All persistence occurs via ruvector-service."
echo "=============================================="
