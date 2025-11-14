#!/bin/bash
# LLM Orchestrator API - State Management Examples

BASE_URL="https://api.llm-orchestrator.io/api/v1"

# Load tokens and IDs
source .api_tokens 2>/dev/null || { echo "Error: Run 01-authentication.sh first"; exit 1; }
source .workflow_ids 2>/dev/null || { echo "Error: Run 02-workflows.sh first"; exit 1; }
source .execution_ids 2>/dev/null || { echo "Error: Run 03-execution.sh first"; exit 1; }

echo "=== State Management Examples ==="

# 1. Get workflow state
echo -e "\n1. Get Workflow State"
curl -s "$BASE_URL/state/$WORKFLOW_ID?executionId=$EXECUTION_ID" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  | jq '.'

# 2. List checkpoints
echo -e "\n2. List Checkpoints"
curl -s "$BASE_URL/state/$WORKFLOW_ID/checkpoints?executionId=$EXECUTION_ID&limit=10" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  | jq '.'

# 3. Create manual checkpoint
echo -e "\n3. Create Manual Checkpoint"
CHECKPOINT_RESPONSE=$(curl -s -X POST "$BASE_URL/state/$RAG_WORKFLOW_ID/checkpoints" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"executionId\": \"$RAG_EXECUTION_ID\",
    \"stepId\": \"search_docs\"
  }")

echo "$CHECKPOINT_RESPONSE" | jq '.'

CHECKPOINT_ID=$(echo "$CHECKPOINT_RESPONSE" | jq -r '.id')
echo "Checkpoint ID: $CHECKPOINT_ID"

# 4. Get RAG workflow state with full context
echo -e "\n4. Get RAG Workflow State"
curl -s "$BASE_URL/state/$RAG_WORKFLOW_ID?executionId=$RAG_EXECUTION_ID" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  | jq '.'

# 5. List all checkpoints with pagination
echo -e "\n5. List All Checkpoints (Paginated)"
curl -s "$BASE_URL/state/$RAG_WORKFLOW_ID/checkpoints?executionId=$RAG_EXECUTION_ID&limit=5&offset=0" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  | jq '.'

# 6. Restore from checkpoint
echo -e "\n6. Restore from Checkpoint"
if [ ! -z "$CHECKPOINT_ID" ]; then
  curl -s -X POST "$BASE_URL/state/$RAG_WORKFLOW_ID/restore" \
    -H "Authorization: Bearer $ACCESS_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
      \"checkpointId\": \"$CHECKPOINT_ID\"
    }" \
    | jq '.'
else
  echo "No checkpoint available to restore"
fi

# 7. Get detailed step state
echo -e "\n7. Get Detailed Step State"
STATE_RESPONSE=$(curl -s "$BASE_URL/state/$WORKFLOW_ID?executionId=$EXECUTION_ID" \
  -H "Authorization: Bearer $ACCESS_TOKEN")

echo "$STATE_RESPONSE" | jq '.steps'

# 8. Monitor state changes over time
echo -e "\n8. Monitor State Changes"
echo "Monitoring state for 10 seconds..."
for i in {1..5}; do
  echo -e "\n--- Iteration $i ---"
  curl -s "$BASE_URL/state/$RAG_WORKFLOW_ID?executionId=$RAG_EXECUTION_ID" \
    -H "Authorization: Bearer $ACCESS_TOKEN" \
    | jq '{status: .status, updated_at: .updated_at, steps: (.steps | keys)}'
  sleep 2
done

echo -e "\nState management examples completed!"
