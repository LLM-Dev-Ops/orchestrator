# LLM Orchestrator API Reference

**Version:** 0.1.0
**Base URL:** `https://api.llm-orchestrator.io/api/v1`

## Table of Contents

- [Overview](#overview)
- [Authentication](#authentication)
- [Rate Limiting](#rate-limiting)
- [Pagination](#pagination)
- [Error Handling](#error-handling)
- [Versioning](#versioning)
- [Common Workflows](#common-workflows)
- [Endpoint Reference](#endpoint-reference)

---

## Overview

The LLM Orchestrator API provides a production-grade platform for orchestrating multi-step LLM workflows with:

- **Multi-provider support**: OpenAI, Anthropic, Cohere
- **Vector database integration**: Pinecone, Weaviate, Qdrant
- **State persistence**: PostgreSQL-backed execution state with checkpoints
- **Authentication**: JWT tokens and API keys with RBAC
- **Audit logging**: Comprehensive tamper-evident audit trail
- **Monitoring**: Prometheus metrics and health checks

### API Characteristics

- **REST-based**: Standard HTTP methods (GET, POST, PUT, DELETE)
- **JSON payloads**: All request/response bodies use JSON (except metrics endpoint)
- **HTTPS only**: Production endpoints require TLS 1.2+
- **Idempotent operations**: Safe to retry most requests
- **Async execution**: Long-running workflows execute asynchronously

---

## Authentication

The API supports two authentication methods. Include credentials in every request (except public endpoints).

### JWT Bearer Tokens

**Best for**: User sessions, web applications

1. **Obtain token** via `/auth/login`:

```bash
curl -X POST https://api.llm-orchestrator.io/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "your-username",
    "password": "your-password"
  }'
```

Response:
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "user_id": "user-123",
  "roles": ["developer"]
}
```

2. **Use token** in subsequent requests:

```bash
curl https://api.llm-orchestrator.io/api/v1/workflows \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

3. **Refresh token** before expiry:

```bash
curl -X POST https://api.llm-orchestrator.io/api/v1/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  }'
```

**Token Expiry**: Access tokens expire after 1 hour. Refresh tokens are valid for 30 days.

### API Keys

**Best for**: Service-to-service communication, CI/CD, long-running processes

1. **Create API key** (requires authenticated session):

```bash
curl -X POST https://api.llm-orchestrator.io/api/v1/auth/keys \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Production Service",
    "scopes": ["workflow:read", "workflow:execute"],
    "expires_in_days": 90
  }'
```

Response:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "key": "llo_sk_1234567890abcdefghijklmnopqrstuvwxyz",
  "user_id": "user-123",
  "scopes": ["workflow:read", "workflow:execute"],
  "created_at": "2025-11-14T10:00:00Z",
  "expires_at": "2026-02-12T10:00:00Z",
  "name": "Production Service"
}
```

**⚠️ Important**: The raw API key is only shown once. Store it securely.

2. **Use API key** in requests:

```bash
curl https://api.llm-orchestrator.io/api/v1/workflows \
  -H "X-API-Key: llo_sk_1234567890abcdefghijklmnopqrstuvwxyz"
```

### Public Endpoints

These endpoints don't require authentication:
- `GET /health`
- `GET /health/ready`
- `GET /health/live`
- `GET /metrics`

---

## Rate Limiting

API requests are rate-limited per user/API key to ensure fair usage and system stability.

### Limits

| Tier | Requests/Minute | Requests/Hour | Requests/Day |
|------|----------------|---------------|--------------|
| Free | 60 | 1,000 | 10,000 |
| Pro | 600 | 10,000 | 100,000 |
| Enterprise | Custom | Custom | Custom |

### Rate Limit Headers

Every response includes rate limit information:

```
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 45
X-RateLimit-Reset: 1699876543
```

### Handling Rate Limits

When rate limited, you'll receive a `429 Too Many Requests` response:

```json
{
  "code": "rate_limited",
  "message": "Rate limit exceeded. Please retry after 60 seconds.",
  "details": {
    "retry_after": 60,
    "limit": 60,
    "window": "1 minute"
  }
}
```

**Best Practices**:
- Implement exponential backoff
- Respect `X-RateLimit-Reset` header
- Cache responses when appropriate
- Use webhooks instead of polling

---

## Pagination

List endpoints support cursor-based pagination using `limit` and `offset` parameters.

### Request Parameters

- `limit`: Number of results per page (default: 20, max: 100)
- `offset`: Number of results to skip (default: 0)

### Example Request

```bash
curl "https://api.llm-orchestrator.io/api/v1/workflows?limit=50&offset=100" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

### Response Format

```json
{
  "workflows": [
    { "id": "wf-1", "name": "Workflow 1" },
    { "id": "wf-2", "name": "Workflow 2" }
  ],
  "total": 500,
  "limit": 50,
  "offset": 100
}
```

### Navigation

Calculate page information:
- **Current page**: `Math.floor(offset / limit) + 1`
- **Total pages**: `Math.ceil(total / limit)`
- **Next page**: `offset + limit` (if `offset + limit < total`)
- **Previous page**: `Math.max(0, offset - limit)`

---

## Error Handling

All errors follow a consistent format for easy parsing and debugging.

### Error Response Structure

```json
{
  "code": "validation_error",
  "message": "Invalid workflow definition",
  "details": {
    "errors": [
      {
        "field": "steps",
        "message": "Steps array cannot be empty"
      }
    ]
  },
  "request_id": "req-550e8400-e29b-41d4-a716-446655440000"
}
```

### HTTP Status Codes

| Code | Meaning | Description |
|------|---------|-------------|
| 200 | OK | Request succeeded |
| 201 | Created | Resource created successfully |
| 202 | Accepted | Request accepted (async processing) |
| 204 | No Content | Request succeeded, no response body |
| 400 | Bad Request | Invalid request parameters or body |
| 401 | Unauthorized | Missing or invalid authentication |
| 403 | Forbidden | Insufficient permissions |
| 404 | Not Found | Resource doesn't exist |
| 409 | Conflict | Resource conflict (duplicate, in use) |
| 429 | Too Many Requests | Rate limit exceeded |
| 500 | Internal Server Error | Server-side error |
| 503 | Service Unavailable | Service temporarily unavailable |

### Error Codes

| Code | Description |
|------|-------------|
| `validation_error` | Request validation failed |
| `unauthorized` | Authentication required or invalid |
| `forbidden` | Insufficient permissions |
| `not_found` | Resource not found |
| `conflict` | Resource conflict |
| `rate_limited` | Rate limit exceeded |
| `internal_error` | Internal server error |

### Error Handling Best Practices

1. **Check HTTP status code** first
2. **Parse error code** for programmatic handling
3. **Log request_id** for support/debugging
4. **Display message** to users
5. **Inspect details** for field-level errors

Example error handler (JavaScript):

```javascript
async function handleApiError(response) {
  if (!response.ok) {
    const error = await response.json();

    switch (error.code) {
      case 'unauthorized':
        // Refresh token or redirect to login
        await refreshToken();
        break;

      case 'rate_limited':
        // Implement exponential backoff
        const retryAfter = error.details.retry_after || 60;
        await sleep(retryAfter * 1000);
        break;

      case 'validation_error':
        // Show field-level errors to user
        error.details.errors.forEach(err => {
          showFieldError(err.field, err.message);
        });
        break;

      default:
        // Log error and show generic message
        console.error(`API Error [${error.request_id}]:`, error);
        showError('An unexpected error occurred');
    }
  }
}
```

---

## Versioning

The API uses URL-based versioning: `/api/v1/`

### Version Policy

- **Current version**: v1
- **Deprecation notice**: 6 months before removal
- **Breaking changes**: Only in new major versions
- **Backward compatibility**: Maintained within major versions

### API Changelog

Changes are documented in the [CHANGELOG](./CHANGELOG.md).

### Deprecation Warnings

Deprecated endpoints return a warning header:

```
Deprecation: version="v1", date="2026-05-14"
Sunset: Wed, 14 May 2026 00:00:00 GMT
```

---

## Common Workflows

### Workflow 1: Create and Execute a Simple Workflow

```bash
# 1. Login and get JWT token
TOKEN=$(curl -X POST https://api.llm-orchestrator.io/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"secure_password"}' \
  | jq -r '.access_token')

# 2. Create workflow
WORKFLOW_ID=$(curl -X POST https://api.llm-orchestrator.io/api/v1/workflows \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "sentiment-analyzer",
    "version": "1.0",
    "description": "Analyzes sentiment of input text",
    "steps": [
      {
        "id": "analyze",
        "type": "llm",
        "provider": "openai",
        "model": "gpt-4",
        "prompt": "Analyze sentiment (positive/negative/neutral): {{input}}",
        "temperature": 0.3,
        "max_tokens": 50,
        "output": ["sentiment"]
      }
    ]
  }' | jq -r '.id')

# 3. Execute workflow
EXECUTION_ID=$(curl -X POST "https://api.llm-orchestrator.io/api/v1/workflows/$WORKFLOW_ID/execute" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "input": "This product is amazing! Best purchase ever."
    },
    "async": true
  }' | jq -r '.execution_id')

# 4. Check execution status
curl "https://api.llm-orchestrator.io/api/v1/workflows/$WORKFLOW_ID/status?executionId=$EXECUTION_ID" \
  -H "Authorization: Bearer $TOKEN"
```

### Workflow 2: RAG (Retrieval-Augmented Generation)

```bash
# Create RAG workflow
curl -X POST https://api.llm-orchestrator.io/api/v1/workflows \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "rag-qa",
    "version": "1.0",
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
        "depends_on": ["embed_query"],
        "output": ["relevant_docs"]
      },
      {
        "id": "generate_answer",
        "type": "llm",
        "provider": "anthropic",
        "model": "claude-3-opus-20240229",
        "prompt": "Context:\\n{{relevant_docs}}\\n\\nQuestion: {{query}}\\n\\nAnswer:",
        "max_tokens": 500,
        "depends_on": ["search_docs"],
        "output": ["answer"]
      }
    ]
  }'
```

### Workflow 3: Checkpoint and Recovery

```bash
# 1. Execute long-running workflow
EXECUTION_ID=$(curl -X POST "https://api.llm-orchestrator.io/api/v1/workflows/$WORKFLOW_ID/execute" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"inputs": {"data": "..."}}' | jq -r '.execution_id')

# 2. List checkpoints
curl "https://api.llm-orchestrator.io/api/v1/state/$WORKFLOW_ID/checkpoints?executionId=$EXECUTION_ID" \
  -H "Authorization: Bearer $TOKEN"

# 3. Restore from checkpoint (if execution failed)
CHECKPOINT_ID="ckpt-550e8400-e29b-41d4-a716-446655440000"
curl -X POST "https://api.llm-orchestrator.io/api/v1/state/$WORKFLOW_ID/restore" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"checkpointId\": \"$CHECKPOINT_ID\"}"
```

### Workflow 4: Query Audit Logs

```bash
# Query audit events for a specific user
curl "https://api.llm-orchestrator.io/api/v1/audit/events?user_id=user-123&event_type=workflow_execution&limit=50" \
  -H "Authorization: Bearer $TOKEN"

# Query failed operations in the last 24 hours
START_TIME=$(date -u -d '24 hours ago' +%Y-%m-%dT%H:%M:%SZ)
END_TIME=$(date -u +%Y-%m-%dT%H:%M:%SZ)

curl "https://api.llm-orchestrator.io/api/v1/audit/events?result=failure&start_time=$START_TIME&end_time=$END_TIME" \
  -H "Authorization: Bearer $TOKEN"
```

---

## Endpoint Reference

### Authentication

#### POST /auth/login
Authenticate with username/password and receive JWT tokens.

**Request:**
```json
{
  "username": "admin",
  "password": "secure_password"
}
```

**Response:** `200 OK`
```json
{
  "access_token": "eyJhbG...",
  "refresh_token": "eyJhbG...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "user_id": "user-123",
  "roles": ["developer"]
}
```

#### POST /auth/refresh
Refresh JWT access token.

**Request:**
```json
{
  "refresh_token": "eyJhbG..."
}
```

**Response:** `200 OK` (same as login)

#### POST /auth/keys
Create a new API key.

**Permissions:** `AdminAccess`

**Request:**
```json
{
  "name": "Production Service",
  "scopes": ["workflow:read", "workflow:execute"],
  "expires_in_days": 90
}
```

**Response:** `201 Created`
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "key": "llo_sk_abc123...",
  "user_id": "user-123",
  "scopes": ["workflow:read", "workflow:execute"],
  "created_at": "2025-11-14T10:00:00Z",
  "expires_at": "2026-02-12T10:00:00Z",
  "name": "Production Service"
}
```

#### GET /auth/keys
List all API keys for the authenticated user.

**Query Parameters:**
- `limit` (optional): Results per page (default: 20)
- `offset` (optional): Results to skip (default: 0)

**Response:** `200 OK`
```json
{
  "keys": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "key_hash": "sha256:abc123...",
      "user_id": "user-123",
      "scopes": ["workflow:read"],
      "created_at": "2025-11-14T10:00:00Z",
      "expires_at": "2026-02-12T10:00:00Z",
      "name": "Test Key",
      "last_used_at": "2025-11-14T15:30:00Z"
    }
  ],
  "total": 3,
  "limit": 20,
  "offset": 0
}
```

#### DELETE /auth/keys/{keyId}
Revoke an API key permanently.

**Response:** `204 No Content`

---

### Workflows

#### POST /workflows
Create a new workflow definition.

**Permissions:** `WorkflowWrite`

**Request:**
```json
{
  "name": "sentiment-analyzer",
  "version": "1.0",
  "description": "Analyzes sentiment",
  "steps": [
    {
      "id": "analyze",
      "type": "llm",
      "provider": "openai",
      "model": "gpt-4",
      "prompt": "Analyze sentiment: {{input}}",
      "output": ["sentiment"]
    }
  ],
  "timeout_seconds": 300
}
```

**Response:** `201 Created`
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "sentiment-analyzer",
  "version": "1.0",
  "description": "Analyzes sentiment",
  "steps": [...],
  "timeout_seconds": 300,
  "created_at": "2025-11-14T10:00:00Z",
  "updated_at": "2025-11-14T10:00:00Z",
  "created_by": "user-123"
}
```

#### GET /workflows
List all workflows.

**Permissions:** `WorkflowRead`

**Query Parameters:**
- `limit` (optional): Results per page (default: 20)
- `offset` (optional): Results to skip (default: 0)
- `name` (optional): Filter by name (partial match)
- `version` (optional): Filter by version

**Response:** `200 OK`
```json
{
  "workflows": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "sentiment-analyzer",
      "version": "1.0",
      "created_at": "2025-11-14T10:00:00Z"
    }
  ],
  "total": 15,
  "limit": 20,
  "offset": 0
}
```

#### GET /workflows/{workflowId}
Get workflow details by ID.

**Permissions:** `WorkflowRead`

**Response:** `200 OK`
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "sentiment-analyzer",
  "version": "1.0",
  "steps": [...],
  "created_at": "2025-11-14T10:00:00Z"
}
```

#### PUT /workflows/{workflowId}
Update workflow definition.

**Permissions:** `WorkflowWrite`

**Request:** Same as POST /workflows

**Response:** `200 OK` (same as GET /workflows/{workflowId})

#### DELETE /workflows/{workflowId}
Delete a workflow.

**Permissions:** `WorkflowDelete`

**Response:** `204 No Content`

**Error (409 Conflict):** If workflow has active executions

---

### Execution

#### POST /workflows/{workflowId}/execute
Execute a workflow.

**Permissions:** `WorkflowExecute`

**Request:**
```json
{
  "inputs": {
    "input": "This is great!"
  },
  "async": true,
  "timeout_override": 600
}
```

**Response:** `202 Accepted`
```json
{
  "execution_id": "exec-550e8400-e29b-41d4-a716-446655440000",
  "workflow_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "pending",
  "started_at": "2025-11-14T10:00:00Z"
}
```

#### GET /workflows/{workflowId}/status
Get execution status.

**Permissions:** `ExecutionRead`

**Query Parameters:**
- `executionId` (required): Execution ID

**Response:** `200 OK`
```json
{
  "id": "exec-550e8400-e29b-41d4-a716-446655440000",
  "workflow_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "started_at": "2025-11-14T10:00:00Z",
  "completed_at": "2025-11-14T10:05:23Z",
  "context": {
    "inputs": {"input": "This is great!"},
    "outputs": {"sentiment": "positive"}
  },
  "steps": {
    "analyze": {
      "status": "completed",
      "outputs": {"sentiment": "positive"}
    }
  }
}
```

#### POST /workflows/{workflowId}/pause
Pause running workflow.

**Permissions:** `ExecutionCancel`

**Query Parameters:**
- `executionId` (required)

**Response:** `200 OK` (execution response)

#### POST /workflows/{workflowId}/resume
Resume paused workflow.

**Permissions:** `WorkflowExecute`

**Query Parameters:**
- `executionId` (required)

**Response:** `200 OK` (execution response)

#### POST /workflows/{workflowId}/cancel
Cancel workflow execution.

**Permissions:** `ExecutionCancel`

**Query Parameters:**
- `executionId` (required)

**Response:** `200 OK` (execution response)

---

### State Management

#### GET /state/{workflowId}
Get workflow state.

**Permissions:** `ExecutionRead`

**Query Parameters:**
- `executionId` (required)

**Response:** `200 OK` (same as GET /workflows/{workflowId}/status)

#### GET /state/{workflowId}/checkpoints
List checkpoints.

**Permissions:** `ExecutionRead`

**Query Parameters:**
- `executionId` (required)
- `limit` (optional)
- `offset` (optional)

**Response:** `200 OK`
```json
{
  "checkpoints": [
    {
      "id": "ckpt-550e8400-e29b-41d4-a716-446655440000",
      "workflow_state_id": "exec-...",
      "step_id": "step-2",
      "timestamp": "2025-11-14T10:02:00Z",
      "snapshot": {...}
    }
  ],
  "total": 5,
  "limit": 20,
  "offset": 0
}
```

#### POST /state/{workflowId}/checkpoints
Create checkpoint.

**Permissions:** `WorkflowExecute`

**Request:**
```json
{
  "executionId": "exec-550e8400-e29b-41d4-a716-446655440000",
  "stepId": "step-2"
}
```

**Response:** `201 Created` (checkpoint object)

#### POST /state/{workflowId}/restore
Restore from checkpoint.

**Permissions:** `WorkflowExecute`

**Request:**
```json
{
  "checkpointId": "ckpt-550e8400-e29b-41d4-a716-446655440000"
}
```

**Response:** `200 OK` (execution response)

---

### Monitoring

#### GET /health
Comprehensive health check.

**Public endpoint** (no auth required)

**Response:** `200 OK` (healthy) or `503 Service Unavailable` (unhealthy)
```json
{
  "status": "healthy",
  "timestamp": "2025-11-14T10:00:00Z",
  "checks": {
    "database": {
      "status": "healthy",
      "response_time_ms": 5
    },
    "redis": {
      "status": "healthy",
      "response_time_ms": 2
    }
  }
}
```

#### GET /health/ready
Kubernetes readiness probe.

**Public endpoint**

**Response:** `200 OK` (ready) or `503 Service Unavailable` (not ready)

#### GET /health/live
Kubernetes liveness probe.

**Public endpoint**

**Response:** `200 OK`
```json
{
  "status": "healthy",
  "timestamp": "2025-11-14T10:00:00Z"
}
```

#### GET /metrics
Prometheus metrics.

**Public endpoint**

**Response:** `200 OK` (text/plain)
```
# HELP orchestrator_workflows_total Total number of workflows
# TYPE orchestrator_workflows_total counter
orchestrator_workflows_total 42

# HELP orchestrator_executions_total Total workflow executions
# TYPE orchestrator_executions_total counter
orchestrator_executions_total{status="completed"} 38
orchestrator_executions_total{status="failed"} 4
```

---

### Audit

#### GET /audit/events
Query audit events.

**Permissions:** `AdminAccess`

**Query Parameters:**
- `user_id` (optional)
- `event_type` (optional)
- `resource_type` (optional)
- `resource_id` (optional)
- `start_time` (optional): ISO 8601 datetime
- `end_time` (optional): ISO 8601 datetime
- `result` (optional): success, failure, partial_success
- `limit` (optional)
- `offset` (optional)

**Response:** `200 OK`
```json
{
  "events": [
    {
      "id": "evt-550e8400-e29b-41d4-a716-446655440000",
      "timestamp": "2025-11-14T10:00:00Z",
      "event_type": "workflow_execution",
      "user_id": "user-123",
      "action": "Execute workflow",
      "resource_type": "workflow",
      "resource_id": "wf-123",
      "result": "success",
      "ip_address": "192.168.1.1",
      "event_hash": "sha256:abc123..."
    }
  ],
  "total": 1523,
  "limit": 20,
  "offset": 0
}
```

#### GET /audit/events/{eventId}
Get audit event details.

**Permissions:** `AdminAccess`

**Response:** `200 OK` (single audit event object)

---

### Admin

#### GET /admin/users
List all users.

**Permissions:** `AdminAccess`

**Query Parameters:**
- `limit` (optional)
- `offset` (optional)

**Response:** `200 OK`
```json
{
  "users": [
    {
      "id": "user-123",
      "username": "admin",
      "email": "admin@example.com",
      "roles": ["admin"],
      "created_at": "2025-01-01T00:00:00Z",
      "last_login": "2025-11-14T09:00:00Z"
    }
  ],
  "total": 42,
  "limit": 20,
  "offset": 0
}
```

#### POST /admin/users
Create a new user.

**Permissions:** `AdminAccess`

**Request:**
```json
{
  "username": "newuser",
  "email": "newuser@example.com",
  "password": "SecureP@ssw0rd",
  "roles": ["developer"]
}
```

**Response:** `201 Created` (user object)

#### GET /admin/stats
Get system statistics.

**Permissions:** `AdminAccess`

**Response:** `200 OK`
```json
{
  "total_workflows": 150,
  "total_executions": 5423,
  "executions_by_status": {
    "pending": 5,
    "running": 12,
    "completed": 5324,
    "failed": 82
  },
  "active_users": 42,
  "api_keys_count": 15,
  "avg_execution_time_ms": 2345,
  "uptime_seconds": 2592000
}
```

#### POST /admin/secrets
Store or update secrets.

**Permissions:** `AdminAccess`

**Request:**
```json
{
  "key": "OPENAI_API_KEY",
  "value": "sk-...",
  "ttl_seconds": 86400
}
```

**Response:** `200 OK`
```json
{
  "success": true,
  "key": "OPENAI_API_KEY"
}
```

#### GET /admin/config
Get system configuration.

**Permissions:** `AdminAccess`

**Response:** `200 OK`
```json
{
  "version": "0.1.0",
  "environment": "production",
  "features": {
    "state_persistence": true,
    "audit_logging": true
  }
}
```

---

## SDK Support

Official SDKs are available for:

- **Python**: `pip install llm-orchestrator`
- **JavaScript/TypeScript**: `npm install @llm-orchestrator/client`
- **Go**: `go get github.com/llm-devops/llm-orchestrator-go`
- **Rust**: `cargo add llm-orchestrator-sdk`

See [SDK Documentation](./SDK.md) for usage examples.

---

## Support

- **Documentation**: https://docs.llm-orchestrator.io
- **GitHub Issues**: https://github.com/llm-devops/llm-orchestrator/issues
- **Email**: support@llm-devops.io
- **Slack**: https://llm-orchestrator.slack.com

---

## License

MIT OR Apache-2.0
