#!/bin/bash
# LLM Orchestrator API - Workflow Management Examples

BASE_URL="https://api.llm-orchestrator.io/api/v1"

# Load tokens from authentication script
source .api_tokens 2>/dev/null || {
  echo "Error: Run 01-authentication.sh first"
  exit 1
}

echo "=== Workflow Management Examples ==="

# 1. Create a simple sentiment analysis workflow
echo -e "\n1. Create Workflow"
WORKFLOW_RESPONSE=$(curl -s -X POST "$BASE_URL/workflows" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "sentiment-analyzer",
    "version": "1.0",
    "description": "Analyzes sentiment of input text using GPT-4",
    "steps": [
      {
        "id": "analyze",
        "type": "llm",
        "provider": "openai",
        "model": "gpt-4",
        "prompt": "Analyze the sentiment of the following text and respond with only one word: positive, negative, or neutral.\\n\\nText: {{input}}\\n\\nSentiment:",
        "temperature": 0.3,
        "max_tokens": 50,
        "output": ["sentiment"]
      }
    ],
    "timeout_seconds": 300,
    "metadata": {
      "author": "api-user",
      "tags": ["nlp", "sentiment"],
      "version": "1.0"
    }
  }')

echo "$WORKFLOW_RESPONSE" | jq '.'

# Extract workflow ID
WORKFLOW_ID=$(echo "$WORKFLOW_RESPONSE" | jq -r '.id')
echo "Workflow ID: $WORKFLOW_ID"

# 2. List all workflows
echo -e "\n2. List Workflows"
curl -s "$BASE_URL/workflows?limit=20&offset=0" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  | jq '.'

# 3. Get workflow details
echo -e "\n3. Get Workflow Details"
curl -s "$BASE_URL/workflows/$WORKFLOW_ID" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  | jq '.'

# 4. Create a complex RAG workflow
echo -e "\n4. Create RAG Workflow"
RAG_WORKFLOW_RESPONSE=$(curl -s -X POST "$BASE_URL/workflows" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "rag-qa-system",
    "version": "1.0",
    "description": "Retrieval-Augmented Generation for Q&A",
    "steps": [
      {
        "id": "embed_query",
        "type": "embed",
        "provider": "openai",
        "model": "text-embedding-ada-002",
        "input": "{{query}}",
        "output": ["query_embedding"]
      },
      {
        "id": "search_docs",
        "type": "vector_search",
        "database": "pinecone",
        "index": "knowledge-base",
        "query": "{{query_embedding}}",
        "top_k": 5,
        "include_metadata": true,
        "depends_on": ["embed_query"],
        "output": ["relevant_docs"]
      },
      {
        "id": "generate_answer",
        "type": "llm",
        "provider": "anthropic",
        "model": "claude-3-opus-20240229",
        "system": "You are a helpful assistant. Answer questions based only on the provided context.",
        "prompt": "Context:\\n{{relevant_docs}}\\n\\nQuestion: {{query}}\\n\\nProvide a detailed answer based on the context above:",
        "max_tokens": 500,
        "temperature": 0.7,
        "depends_on": ["search_docs"],
        "output": ["answer", "sources"]
      }
    ],
    "timeout_seconds": 600
  }')

echo "$RAG_WORKFLOW_RESPONSE" | jq '.'

RAG_WORKFLOW_ID=$(echo "$RAG_WORKFLOW_RESPONSE" | jq -r '.id')
echo "RAG Workflow ID: $RAG_WORKFLOW_ID"

# 5. Update workflow (add retry config)
echo -e "\n5. Update Workflow"
UPDATE_RESPONSE=$(curl -s -X PUT "$BASE_URL/workflows/$WORKFLOW_ID" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "sentiment-analyzer",
    "version": "1.1",
    "description": "Analyzes sentiment of input text using GPT-4 with retry",
    "steps": [
      {
        "id": "analyze",
        "type": "llm",
        "provider": "openai",
        "model": "gpt-4",
        "prompt": "Analyze sentiment: {{input}}",
        "temperature": 0.3,
        "max_tokens": 50,
        "output": ["sentiment"],
        "retry": {
          "max_attempts": 3,
          "backoff": "exponential",
          "initial_delay_ms": 100,
          "max_delay_ms": 5000
        }
      }
    ],
    "timeout_seconds": 300
  }')

echo "$UPDATE_RESPONSE" | jq '.'

# 6. Filter workflows by name
echo -e "\n6. Filter Workflows by Name"
curl -s "$BASE_URL/workflows?name=sentiment&limit=10" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  | jq '.'

# 7. Create YAML workflow
echo -e "\n7. Create Workflow from YAML"
YAML_WORKFLOW='
name: multi-step-analysis
version: "2.0"
description: Multi-step text analysis pipeline
steps:
  - id: extract_entities
    type: llm
    provider: openai
    model: gpt-4
    prompt: "Extract named entities from: {{text}}"
    output: [entities]
  - id: classify_topic
    type: llm
    provider: openai
    model: gpt-4
    prompt: "Classify topic of: {{text}}"
    output: [topic]
    depends_on: []
  - id: summarize
    type: llm
    provider: anthropic
    model: claude-3-sonnet-20240229
    prompt: "Summarize: {{text}}"
    depends_on: [extract_entities, classify_topic]
    output: [summary]
'

curl -s -X POST "$BASE_URL/workflows" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/yaml" \
  --data-binary "$YAML_WORKFLOW" \
  | jq '.'

# Save workflow IDs for execution scripts
cat > .workflow_ids << EOF
WORKFLOW_ID=$WORKFLOW_ID
RAG_WORKFLOW_ID=$RAG_WORKFLOW_ID
EOF

echo -e "\nWorkflow IDs saved to .workflow_ids"
