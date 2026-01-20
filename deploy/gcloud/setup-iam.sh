#!/usr/bin/env bash
# =============================================================================
# LLM-Orchestrator IAM Setup Script
# =============================================================================
# Creates service account with least-privilege permissions
# Run once per environment: ./setup-iam.sh <project-id> <env>
# =============================================================================

set -euo pipefail

PROJECT_ID="${1:-agentics-dev}"
PLATFORM_ENV="${2:-dev}"
SERVICE_NAME="llm-orchestrator"
SA_NAME="${SERVICE_NAME}-sa"
SA_EMAIL="${SA_NAME}@${PROJECT_ID}.iam.gserviceaccount.com"

echo "=============================================="
echo "LLM-Orchestrator IAM Setup"
echo "=============================================="
echo "Project:         ${PROJECT_ID}"
echo "Environment:     ${PLATFORM_ENV}"
echo "Service Account: ${SA_EMAIL}"
echo "=============================================="

# Create service account
echo "[1/6] Creating service account..."
gcloud iam service-accounts create "${SA_NAME}" \
  --project="${PROJECT_ID}" \
  --display-name="LLM-Orchestrator Service Account" \
  --description="Service account for LLM-Orchestrator unified service" \
  2>/dev/null || echo "Service account already exists"

# Grant Cloud Run Invoker role (for internal service calls)
echo "[2/6] Granting Cloud Run Invoker role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member="serviceAccount:${SA_EMAIL}" \
  --role="roles/run.invoker" \
  --condition=None \
  --quiet

# Grant Secret Manager Secret Accessor role
echo "[3/6] Granting Secret Manager accessor role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member="serviceAccount:${SA_EMAIL}" \
  --role="roles/secretmanager.secretAccessor" \
  --condition=None \
  --quiet

# Grant Cloud Logging Writer role
echo "[4/6] Granting Cloud Logging writer role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member="serviceAccount:${SA_EMAIL}" \
  --role="roles/logging.logWriter" \
  --condition=None \
  --quiet

# Grant Cloud Trace Agent role
echo "[5/6] Granting Cloud Trace agent role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member="serviceAccount:${SA_EMAIL}" \
  --role="roles/cloudtrace.agent" \
  --condition=None \
  --quiet

# Grant Cloud Monitoring Metric Writer role
echo "[6/6] Granting Cloud Monitoring metric writer role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member="serviceAccount:${SA_EMAIL}" \
  --role="roles/monitoring.metricWriter" \
  --condition=None \
  --quiet

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
echo "NOTE: This service account has NO direct database access."
echo "      All persistence occurs via ruvector-service."
echo "=============================================="
