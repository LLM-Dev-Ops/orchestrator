#!/bin/bash
# LLM Orchestrator API - Authentication Examples

BASE_URL="https://api.llm-orchestrator.io/api/v1"

echo "=== Authentication Examples ==="

# 1. Login with username/password
echo -e "\n1. Login"
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "secure_password"
  }')

echo "$LOGIN_RESPONSE" | jq '.'

# Extract access token
ACCESS_TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.access_token')
REFRESH_TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.refresh_token')

echo "Access Token: $ACCESS_TOKEN"
echo "Refresh Token: $REFRESH_TOKEN"

# 2. Refresh token
echo -e "\n2. Refresh Token"
REFRESH_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/refresh" \
  -H "Content-Type: application/json" \
  -d "{
    \"refresh_token\": \"$REFRESH_TOKEN\"
  }")

echo "$REFRESH_RESPONSE" | jq '.'

# Update access token
ACCESS_TOKEN=$(echo "$REFRESH_RESPONSE" | jq -r '.access_token')

# 3. Create API key
echo -e "\n3. Create API Key"
API_KEY_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/keys" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Production Service",
    "scopes": ["workflow:read", "workflow:execute"],
    "expires_in_days": 90
  }')

echo "$API_KEY_RESPONSE" | jq '.'

# Extract API key
API_KEY=$(echo "$API_KEY_RESPONSE" | jq -r '.key')
echo "API Key: $API_KEY"

# 4. List API keys
echo -e "\n4. List API Keys"
curl -s "$BASE_URL/auth/keys?limit=10&offset=0" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  | jq '.'

# 5. Test API key authentication
echo -e "\n5. Test API Key Authentication"
curl -s "$BASE_URL/workflows?limit=5" \
  -H "X-API-Key: $API_KEY" \
  | jq '.'

# 6. Revoke API key
KEY_ID=$(echo "$API_KEY_RESPONSE" | jq -r '.id')
echo -e "\n6. Revoke API Key"
curl -s -X DELETE "$BASE_URL/auth/keys/$KEY_ID" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -w "\nHTTP Status: %{http_code}\n"

# Save tokens to file for use in other scripts
cat > .api_tokens << EOF
ACCESS_TOKEN=$ACCESS_TOKEN
REFRESH_TOKEN=$REFRESH_TOKEN
API_KEY=$API_KEY
EOF

echo -e "\nTokens saved to .api_tokens"
