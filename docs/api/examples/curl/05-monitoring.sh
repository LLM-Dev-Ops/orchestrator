#!/bin/bash
# LLM Orchestrator API - Monitoring Examples

BASE_URL="https://api.llm-orchestrator.io/api/v1"

echo "=== Monitoring Examples ==="

# 1. Basic health check
echo -e "\n1. Basic Health Check"
curl -s "$BASE_URL/health" | jq '.'

# 2. Readiness probe (Kubernetes)
echo -e "\n2. Readiness Probe"
READY_RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" "$BASE_URL/health/ready")
echo "$READY_RESPONSE" | grep -v "HTTP_CODE" | jq '.'
HTTP_CODE=$(echo "$READY_RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)
echo "HTTP Status: $HTTP_CODE"

if [ "$HTTP_CODE" = "200" ]; then
  echo "Service is READY to accept traffic"
else
  echo "Service is NOT READY"
fi

# 3. Liveness probe (Kubernetes)
echo -e "\n3. Liveness Probe"
LIVE_RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" "$BASE_URL/health/live")
echo "$LIVE_RESPONSE" | grep -v "HTTP_CODE" | jq '.'
HTTP_CODE=$(echo "$LIVE_RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)
echo "HTTP Status: $HTTP_CODE"

if [ "$HTTP_CODE" = "200" ]; then
  echo "Service is ALIVE"
else
  echo "Service is NOT ALIVE"
fi

# 4. Prometheus metrics
echo -e "\n4. Prometheus Metrics"
curl -s "$BASE_URL/metrics" | head -n 50

# 5. Parse specific metrics
echo -e "\n5. Parse Specific Metrics"

echo "Total Workflows:"
curl -s "$BASE_URL/metrics" | grep "^orchestrator_workflows_total" | grep -v "^#"

echo -e "\nTotal Executions by Status:"
curl -s "$BASE_URL/metrics" | grep "^orchestrator_executions_total" | grep -v "^#"

echo -e "\nExecution Duration (seconds):"
curl -s "$BASE_URL/metrics" | grep "^orchestrator_execution_duration_seconds" | grep -v "^#"

# 6. Health check with component details
echo -e "\n6. Detailed Component Health"
curl -s "$BASE_URL/health" | jq '.checks'

# 7. Monitor health over time
echo -e "\n7. Monitor Health Over Time"
echo "Checking health every 5 seconds for 20 seconds..."

for i in {1..4}; do
  echo -e "\n--- Check $i at $(date +%H:%M:%S) ---"
  HEALTH=$(curl -s "$BASE_URL/health")

  STATUS=$(echo "$HEALTH" | jq -r '.status')
  TIMESTAMP=$(echo "$HEALTH" | jq -r '.timestamp')

  echo "Status: $STATUS"
  echo "Timestamp: $TIMESTAMP"
  echo "Components:"
  echo "$HEALTH" | jq -r '.checks | to_entries[] | "\(.key): \(.value.status)"'

  sleep 5
done

# 8. Check response times
echo -e "\n8. Component Response Times"
curl -s "$BASE_URL/health" | jq '.checks | to_entries[] | {component: .key, response_time_ms: .value.response_time_ms}'

# 9. Export metrics for monitoring
echo -e "\n9. Export Metrics to File"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
METRICS_FILE="metrics_${TIMESTAMP}.txt"

curl -s "$BASE_URL/metrics" > "$METRICS_FILE"
echo "Metrics exported to $METRICS_FILE"

# 10. Health check script for automation
echo -e "\n10. Automated Health Check"

check_health() {
  HEALTH=$(curl -s "$BASE_URL/health")
  STATUS=$(echo "$HEALTH" | jq -r '.status')

  if [ "$STATUS" = "healthy" ]; then
    echo "✓ Service is healthy"
    return 0
  elif [ "$STATUS" = "degraded" ]; then
    echo "⚠ Service is degraded"
    echo "$HEALTH" | jq '.checks | to_entries[] | select(.value.status != "healthy")'
    return 1
  else
    echo "✗ Service is unhealthy"
    echo "$HEALTH" | jq '.checks'
    return 2
  fi
}

check_health
EXIT_CODE=$?

echo -e "\nHealth check exit code: $EXIT_CODE"
echo "(0 = healthy, 1 = degraded, 2 = unhealthy)"

echo -e "\nMonitoring examples completed!"
