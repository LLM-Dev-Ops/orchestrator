#!/usr/bin/env bash
# =============================================================================
# LLM-Orchestrator Deployment Script
# =============================================================================
# Deploys LLM-Orchestrator unified service to Google Cloud Run
# Usage: ./deploy.sh <project-id> <env> [region]
# =============================================================================

set -euo pipefail

# Configuration
PROJECT_ID="${1:-agentics-dev}"
PLATFORM_ENV="${2:-dev}"
REGION="${3:-us-central1}"
SERVICE_NAME="llm-orchestrator"
SERVICE_VERSION="0.1.5"
IMAGE_TAG="${PLATFORM_ENV}-$(date +%Y%m%d%H%M%S)"

echo "=============================================="
echo "LLM-Orchestrator Deployment"
echo "=============================================="
echo "Project:     ${PROJECT_ID}"
echo "Environment: ${PLATFORM_ENV}"
echo "Region:      ${REGION}"
echo "Service:     ${SERVICE_NAME}"
echo "Version:     ${SERVICE_VERSION}"
echo "Image Tag:   ${IMAGE_TAG}"
echo "=============================================="
echo ""

# Confirm deployment
read -p "Proceed with deployment? [y/N] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
  echo "Deployment cancelled."
  exit 0
fi

# Set project
echo ""
echo "[1/7] Setting project..."
gcloud config set project "${PROJECT_ID}"

# Enable required APIs
echo ""
echo "[2/7] Enabling required APIs..."
gcloud services enable \
  run.googleapis.com \
  cloudbuild.googleapis.com \
  secretmanager.googleapis.com \
  containerregistry.googleapis.com \
  --quiet

# Build Docker image
echo ""
echo "[3/7] Building Docker image..."
cd "$(dirname "$0")/../.."
docker build \
  -t "gcr.io/${PROJECT_ID}/${SERVICE_NAME}:${IMAGE_TAG}" \
  -t "gcr.io/${PROJECT_ID}/${SERVICE_NAME}:${PLATFORM_ENV}" \
  -t "gcr.io/${PROJECT_ID}/${SERVICE_NAME}:latest" \
  -f Dockerfile \
  .

# Configure Docker for GCR
echo ""
echo "[4/7] Configuring Docker authentication..."
gcloud auth configure-docker --quiet

# Push image
echo ""
echo "[5/7] Pushing image to Container Registry..."
docker push "gcr.io/${PROJECT_ID}/${SERVICE_NAME}:${IMAGE_TAG}"
docker push "gcr.io/${PROJECT_ID}/${SERVICE_NAME}:${PLATFORM_ENV}"
docker push "gcr.io/${PROJECT_ID}/${SERVICE_NAME}:latest"

# Deploy to Cloud Run
echo ""
echo "[6/7] Deploying to Cloud Run..."
gcloud run deploy "${SERVICE_NAME}" \
  --image "gcr.io/${PROJECT_ID}/${SERVICE_NAME}:${IMAGE_TAG}" \
  --region "${REGION}" \
  --platform managed \
  --allow-unauthenticated \
  --port 8080 \
  --cpu 2 \
  --memory 2Gi \
  --min-instances 0 \
  --max-instances 10 \
  --timeout 300s \
  --concurrency 80 \
  --set-env-vars "PLATFORM_ENV=${PLATFORM_ENV},SERVICE_NAME=${SERVICE_NAME},SERVICE_VERSION=${SERVICE_VERSION},RUST_LOG=info" \
  --set-secrets "RUVECTOR_SERVICE_URL=ruvector-service-url:latest,RUVECTOR_API_KEY=ruvector-api-key:latest,TELEMETRY_ENDPOINT=observatory-endpoint:latest" \
  --service-account "${SERVICE_NAME}-sa@${PROJECT_ID}.iam.gserviceaccount.com" \
  --labels "service=${SERVICE_NAME},env=${PLATFORM_ENV},version=${SERVICE_VERSION//\./-}" \
  --quiet

# Get service URL
echo ""
echo "[7/7] Verifying deployment..."
SERVICE_URL=$(gcloud run services describe "${SERVICE_NAME}" \
  --region="${REGION}" \
  --format='value(status.url)')

echo ""
echo "=============================================="
echo "Deployment Complete"
echo "=============================================="
echo ""
echo "Service URL: ${SERVICE_URL}"
echo ""
echo "Agent Endpoints:"
echo "  Workflow Orchestrator: ${SERVICE_URL}/agent/execute"
echo "  Task Scheduler:        ${SERVICE_URL}/schedule/execute"
echo "  Dependency Resolver:   ${SERVICE_URL}/dependency/resolve"
echo "  Retry & Recovery:      ${SERVICE_URL}/recovery/evaluate"
echo "  Parallelization:       ${SERVICE_URL}/parallel/analyze"
echo "  State Machine:         ${SERVICE_URL}/state-machine/transition"
echo "  Swarm Coordinator:     ${SERVICE_URL}/swarm/coordinate"
echo ""
echo "CLI Usage:"
echo "  export LLM_ORCHESTRATOR_URL=${SERVICE_URL}"
echo "  llm-orchestrator agent execute --file workflow.yaml"
echo ""
echo "=============================================="
