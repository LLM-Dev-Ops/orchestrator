#!/usr/bin/env bash
# =============================================================================
# LLM-Orchestrator Secrets Setup Script
# =============================================================================
# Creates required secrets in Google Secret Manager
# Run once per environment: ./setup-secrets.sh <project-id> <env>
# =============================================================================

set -euo pipefail

PROJECT_ID="${1:-agentics-dev}"
PLATFORM_ENV="${2:-dev}"

echo "=============================================="
echo "LLM-Orchestrator Secrets Setup"
echo "=============================================="
echo "Project:     ${PROJECT_ID}"
echo "Environment: ${PLATFORM_ENV}"
echo "=============================================="

# Function to create secret if not exists
create_secret_if_not_exists() {
  local secret_name="$1"
  local description="$2"

  if gcloud secrets describe "${secret_name}" --project="${PROJECT_ID}" &>/dev/null; then
    echo "Secret '${secret_name}' already exists"
  else
    echo "Creating secret '${secret_name}'..."
    gcloud secrets create "${secret_name}" \
      --project="${PROJECT_ID}" \
      --replication-policy="automatic" \
      --labels="service=llm-orchestrator,env=${PLATFORM_ENV}"
    echo "  ✓ Created. Add version with:"
    echo "    echo -n 'YOUR_VALUE' | gcloud secrets versions add ${secret_name} --data-file=-"
  fi
}

echo ""
echo "[1/4] Creating ruvector-service-url secret..."
create_secret_if_not_exists "ruvector-service-url" "RuVector service URL for persistence"

echo ""
echo "[2/4] Creating ruvector-api-key secret..."
create_secret_if_not_exists "ruvector-api-key" "RuVector service API key"

echo ""
echo "[3/4] Creating observatory-endpoint secret..."
create_secret_if_not_exists "observatory-endpoint" "LLM-Observatory telemetry endpoint"

echo ""
echo "[4/4] Creating platform-env secret..."
create_secret_if_not_exists "platform-env" "Platform environment identifier"

echo ""
echo "=============================================="
echo "Secrets Setup Complete"
echo "=============================================="
echo ""
echo "Required secrets created. Add values with:"
echo ""
echo "  # RuVector Service URL"
echo "  echo -n 'https://ruvector-service-${PLATFORM_ENV}.agentics.dev' | \\"
echo "    gcloud secrets versions add ruvector-service-url --data-file=-"
echo ""
echo "  # RuVector API Key"
echo "  echo -n 'YOUR_API_KEY' | \\"
echo "    gcloud secrets versions add ruvector-api-key --data-file=-"
echo ""
echo "  # Observatory Endpoint"
echo "  echo -n 'https://observatory-${PLATFORM_ENV}.agentics.dev' | \\"
echo "    gcloud secrets versions add observatory-endpoint --data-file=-"
echo ""
echo "  # Platform Environment"
echo "  echo -n '${PLATFORM_ENV}' | \\"
echo "    gcloud secrets versions add platform-env --data-file=-"
echo ""
echo "=============================================="
