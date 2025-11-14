# LLM Orchestrator API Documentation

Complete API documentation for the LLM Orchestrator platform.

## 📚 Documentation Resources

- **[API Reference](./API_REFERENCE.md)** - Complete endpoint reference with examples
- **[OpenAPI Specification](./openapi.yaml)** - Machine-readable API definition (OpenAPI 3.1)
- **[Postman Collection](./postman_collection.json)** - Pre-configured API requests
- **[Code Examples](./examples/)** - Client libraries and code samples

## 🚀 Quick Start

### 1. View Interactive Documentation

#### Option A: Swagger UI (Recommended)

Run locally with Docker:

```bash
docker run -p 8080:8080 \
  -e SWAGGER_JSON=/api/openapi.yaml \
  -v $(pwd)/openapi.yaml:/api/openapi.yaml \
  swaggerapi/swagger-ui
```

Then open: http://localhost:8080

#### Option B: ReDoc

Run locally with Docker:

```bash
docker run -p 8080:80 \
  -e SPEC_URL=openapi.yaml \
  -v $(pwd)/openapi.yaml:/usr/share/nginx/html/openapi.yaml \
  redocly/redoc
```

Then open: http://localhost:8080

#### Option C: VS Code Extension

Install the **OpenAPI (Swagger) Editor** extension and open `openapi.yaml`.

### 2. Import Postman Collection

1. Open Postman
2. Click **Import**
3. Select `postman_collection.json`
4. Configure environment variables:
   - `base_url`: Your API base URL (default: https://api.llm-orchestrator.io/api/v1)
   - `access_token`: Will be auto-set after login
   - `api_key`: Your API key (if using API key auth)

### 3. Try Code Examples

#### cURL Examples

```bash
cd examples/curl

# Make scripts executable
chmod +x *.sh

# Run examples in order
./01-authentication.sh
./02-workflows.sh
./03-execution.sh
./04-state-management.sh
./05-monitoring.sh
```

#### Python Client

```bash
cd examples/python

# Install dependencies
pip install requests

# Run examples
python client.py
```

#### JavaScript Client

```bash
cd examples/javascript

# Install dependencies
npm install axios

# Run examples
node client.js
```

## 📖 API Overview

### Base URL

- **Production**: `https://api.llm-orchestrator.io/api/v1`
- **Staging**: `https://staging-api.llm-orchestrator.io/api/v1`
- **Local**: `http://localhost:8080/api/v1`

### Authentication

Two methods supported:

1. **JWT Bearer Token** (for user sessions):
   ```bash
   curl -H "Authorization: Bearer YOUR_JWT_TOKEN" \
     https://api.llm-orchestrator.io/api/v1/workflows
   ```

2. **API Key** (for service-to-service):
   ```bash
   curl -H "X-API-Key: YOUR_API_KEY" \
     https://api.llm-orchestrator.io/api/v1/workflows
   ```

### Rate Limits

| Tier | Requests/Minute | Requests/Hour |
|------|-----------------|---------------|
| Free | 60 | 1,000 |
| Pro | 600 | 10,000 |
| Enterprise | Custom | Custom |

### Endpoint Categories

- **Authentication** (5 endpoints) - JWT login, token refresh, API key management
- **Workflows** (8 endpoints) - CRUD operations for workflow definitions
- **Execution** (6 endpoints) - Execute, pause, resume, cancel workflows
- **State Management** (4 endpoints) - State persistence and checkpoints
- **Monitoring** (4 endpoints) - Health checks and Prometheus metrics
- **Audit** (2 endpoints) - Query audit logs
- **Admin** (6 endpoints) - User management and system configuration

## 🔧 Generating Client SDKs

### Using OpenAPI Generator

Generate client SDKs in any language:

```bash
# Install OpenAPI Generator
npm install @openapitools/openapi-generator-cli -g

# Generate Python client
openapi-generator-cli generate \
  -i openapi.yaml \
  -g python \
  -o ./sdk/python \
  --additional-properties=packageName=llm_orchestrator_client

# Generate TypeScript client
openapi-generator-cli generate \
  -i openapi.yaml \
  -g typescript-axios \
  -o ./sdk/typescript

# Generate Go client
openapi-generator-cli generate \
  -i openapi.yaml \
  -g go \
  -o ./sdk/go

# Generate Rust client
openapi-generator-cli generate \
  -i openapi.yaml \
  -g rust \
  -o ./sdk/rust

# Generate Java client
openapi-generator-cli generate \
  -i openapi.yaml \
  -g java \
  -o ./sdk/java \
  --additional-properties=library=okhttp-gson
```

### Supported Languages

OpenAPI Generator supports 50+ languages including:
- Python
- JavaScript/TypeScript
- Go
- Rust
- Java
- C#
- PHP
- Ruby
- Swift
- Kotlin
- And many more...

Full list: https://openapi-generator.tech/docs/generators

## 📊 API Validation

### Validate OpenAPI Specification

```bash
# Using swagger-cli
npm install -g @apidevtools/swagger-cli
swagger-cli validate openapi.yaml

# Using openapi-generator
openapi-generator-cli validate -i openapi.yaml

# Using spectral (advanced linting)
npm install -g @stoplight/spectral-cli
spectral lint openapi.yaml
```

Expected output:
```
✓ OpenAPI specification is valid
```

## 🧪 Testing

### Run Integration Tests

```bash
# Test authentication flow
cd examples/curl
./01-authentication.sh

# Test complete workflow lifecycle
./02-workflows.sh
./03-execution.sh

# Test monitoring endpoints
./05-monitoring.sh
```

### Postman Tests

The Postman collection includes automated tests:

1. Import collection
2. Run with Collection Runner
3. View test results

All tests should pass with a properly configured environment.

## 📈 Monitoring API Usage

### Prometheus Metrics

The API exposes metrics at `/metrics`:

```bash
curl http://localhost:8080/api/v1/metrics
```

Key metrics:
- `orchestrator_workflows_total` - Total workflows created
- `orchestrator_executions_total` - Total executions by status
- `orchestrator_execution_duration_seconds` - Execution time histogram
- `orchestrator_api_requests_total` - API request count by endpoint
- `orchestrator_api_request_duration_seconds` - Request latency

### Grafana Dashboard

Import the pre-built dashboard from `../../monitoring/grafana/dashboards/api-metrics.json`.

## 🔐 Security

### API Security Features

- ✅ JWT token authentication with expiry
- ✅ API key rotation support
- ✅ Role-based access control (RBAC)
- ✅ Rate limiting per user/key
- ✅ Comprehensive audit logging
- ✅ TLS 1.2+ required
- ✅ CORS configuration
- ✅ Input validation
- ✅ SQL injection protection
- ✅ Secret management integration

### Best Practices

1. **Never commit secrets** - Use environment variables
2. **Rotate credentials regularly** - API keys should expire
3. **Use least privilege** - Request minimal scopes needed
4. **Monitor audit logs** - Review access patterns
5. **Enable MFA** - For admin accounts
6. **Use HTTPS** - Always in production

## 🐛 Troubleshooting

### Common Issues

#### 401 Unauthorized

**Problem**: Missing or invalid authentication

**Solution**:
```bash
# Check token is set
echo $ACCESS_TOKEN

# Re-login if expired
curl -X POST https://api.llm-orchestrator.io/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"your_password"}'
```

#### 429 Rate Limited

**Problem**: Exceeded rate limit

**Solution**:
```bash
# Check rate limit headers
curl -I https://api.llm-orchestrator.io/api/v1/workflows

# Implement exponential backoff
# Wait for X-RateLimit-Reset timestamp
```

#### 503 Service Unavailable

**Problem**: Service is down or unhealthy

**Solution**:
```bash
# Check health status
curl https://api.llm-orchestrator.io/api/v1/health

# Check system status page
# https://status.llm-orchestrator.io
```

### Debug Mode

Enable verbose output:

```bash
# cURL
curl -v https://api.llm-orchestrator.io/api/v1/workflows

# HTTPie
http --verbose https://api.llm-orchestrator.io/api/v1/workflows
```

## 📞 Support

- **Documentation**: https://docs.llm-orchestrator.io
- **GitHub Issues**: https://github.com/llm-devops/llm-orchestrator/issues
- **Discord**: https://discord.gg/llm-orchestrator
- **Email**: support@llm-devops.io
- **Status Page**: https://status.llm-orchestrator.io

## 📝 Contributing

To contribute to API documentation:

1. Edit `openapi.yaml`
2. Validate changes: `swagger-cli validate openapi.yaml`
3. Update examples if adding new endpoints
4. Test with Postman collection
5. Submit pull request

## 📜 License

MIT OR Apache-2.0

---

**Version**: 0.1.0
**Last Updated**: 2025-11-14
**API Status**: ✅ Production Ready
