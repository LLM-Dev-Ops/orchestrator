#!/bin/bash
# LLM Orchestrator API - Workflow Execution Examples

BASE_URL="https://api.llm-orchestrator.io/api/v1"

# Load tokens and workflow IDs
source .api_tokens 2>/dev/null || { echo "Error: Run 01-authentication.sh first"; exit 1; }
source .workflow_ids 2>/dev/null || { echo "Error: Run 02-workflows.sh first"; exit 1; }

echo "=== Workflow Execution Examples ==="

# 1. Execute sentiment analysis workflow (async)
echo -e "\n1. Execute Workflow (Async)"
EXEC_RESPONSE=$(curl -s -X POST "$BASE_URL/workflows/$WORKFLOW_ID/execute" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "input": "This product is absolutely amazing! Best purchase I have ever made. The quality is outstanding and the customer service was exceptional."
    },
    "async": true
  }')

echo "$EXEC_RESPONSE" | jq '.'

EXECUTION_ID=$(echo "$EXEC_RESPONSE" | jq -r '.execution_id')
echo "Execution ID: $EXECUTION_ID"

# 2. Check execution status
echo -e "\n2. Check Execution Status"
sleep 2  # Wait for execution to progress
curl -s "$BASE_URL/workflows/$WORKFLOW_ID/status?executionId=$EXECUTION_ID" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  | jq '.'

# 3. Poll until completion
echo -e "\n3. Poll Until Completion"
MAX_ATTEMPTS=30
ATTEMPT=0
while [ $ATTEMPT -lt $MAX_ATTEMPTS ]; do
  STATUS_RESPONSE=$(curl -s "$BASE_URL/workflows/$WORKFLOW_ID/status?executionId=$EXECUTION_ID" \
    -H "Authorization: Bearer $ACCESS_TOKEN")

  STATUS=$(echo "$STATUS_RESPONSE" | jq -r '.status')
  echo "Attempt $((ATTEMPT+1)): Status = $STATUS"

  if [ "$STATUS" = "completed" ] || [ "$STATUS" = "failed" ]; then
    echo "Final result:"
    echo "$STATUS_RESPONSE" | jq '.'
    break
  fi

  sleep 2
  ATTEMPT=$((ATTEMPT+1))
done

# 4. Execute RAG workflow
echo -e "\n4. Execute RAG Workflow"
RAG_EXEC_RESPONSE=$(curl -s -X POST "$BASE_URL/workflows/$RAG_WORKFLOW_ID/execute" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "query": "What are the main features of LLM Orchestrator?"
    },
    "async": true
  }')

echo "$RAG_EXEC_RESPONSE" | jq '.'

RAG_EXECUTION_ID=$(echo "$RAG_EXEC_RESPONSE" | jq -r '.execution_id')

# 5. Execute with custom timeout
echo -e "\n5. Execute with Custom Timeout"
TIMEOUT_EXEC=$(curl -s -X POST "$BASE_URL/workflows/$WORKFLOW_ID/execute" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "input": "Test message"
    },
    "async": true,
    "timeout_override": 120
  }')

echo "$TIMEOUT_EXEC" | jq '.'

TIMEOUT_EXECUTION_ID=$(echo "$TIMEOUT_EXEC" | jq -r '.execution_id')

# 6. Pause workflow (if supported)
echo -e "\n6. Pause Workflow"
sleep 1
curl -s -X POST "$BASE_URL/workflows/$RAG_WORKFLOW_ID/pause?executionId=$RAG_EXECUTION_ID" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  | jq '.'

# 7. Resume workflow
echo -e "\n7. Resume Workflow"
sleep 2
curl -s -X POST "$BASE_URL/workflows/$RAG_WORKFLOW_ID/resume?executionId=$RAG_EXECUTION_ID" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  | jq '.'

# 8. Cancel workflow
echo -e "\n8. Cancel Workflow"
sleep 1
curl -s -X POST "$BASE_URL/workflows/$TIMEOUT_EXECUTION_ID/cancel?executionId=$TIMEOUT_EXECUTION_ID" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  | jq '.'

# 9. Synchronous execution (wait for result)
echo -e "\n9. Synchronous Execution"
SYNC_EXEC=$(curl -s -X POST "$BASE_URL/workflows/$WORKFLOW_ID/execute" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "input": "This is terrible!"
    },
    "async": false
  }')

echo "$SYNC_EXEC" | jq '.'

# Save execution IDs for state management scripts
cat > .execution_ids << EOF
EXECUTION_ID=$EXECUTION_ID
RAG_EXECUTION_ID=$RAG_EXECUTION_ID
EOF

echo -e "\nExecution IDs saved to .execution_ids"
