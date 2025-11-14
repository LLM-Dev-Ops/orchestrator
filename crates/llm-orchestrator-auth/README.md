# LLM Orchestrator Authentication & Authorization

Comprehensive authentication and authorization library for the LLM Orchestrator platform.

## Features

- **JWT Authentication**: Stateless token-based authentication with short-lived access tokens (15 min) and long-lived refresh tokens (7 days)
- **API Key Management**: Secure API key generation with SHA-256 hashing and scope-based permissions
- **Role-Based Access Control (RBAC)**: Fine-grained permission system with predefined roles
- **Auth Middleware**: Ready-to-use middleware for authenticating HTTP requests
- **Security First**: Zero secrets in logs, token expiration validation, cryptographically secure random generation

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
llm-orchestrator-auth = "0.1.0"
```

### Basic JWT Authentication

```rust
use llm_orchestrator_auth::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create JWT auth with a secret key
    let jwt_auth = JwtAuth::new(b"your-secret-key-at-least-32-bytes".to_vec());

    // Generate an access token
    let token = jwt_auth.generate_token(
        "user123",
        vec!["developer".to_string()]
    )?;

    println!("Token: {}", token);

    // Verify the token
    let claims = jwt_auth.verify_token(&token)?;
    println!("User ID: {}", claims.sub);
    println!("Roles: {:?}", claims.roles);

    Ok(())
}
```

### API Key Management

```rust
use llm_orchestrator_auth::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create API key manager with in-memory storage
    let store = Arc::new(InMemoryApiKeyStore::new());
    let manager = ApiKeyManager::new(store);

    // Create an API key for a user
    let api_key = manager.create_key(
        "user123",
        vec!["workflow:read".to_string(), "workflow:execute".to_string()],
        Some("Production API Key".to_string()),
        Some(30), // Expires in 30 days
    ).await?;

    println!("API Key: {}", api_key.key);
    println!("Key ID: {}", api_key.id);

    // Lookup and validate the key
    let key_info = manager.lookup_key(&api_key.key).await?;
    println!("Owner: {}", key_info.user_id);
    println!("Scopes: {:?}", key_info.scopes);

    // List all keys for a user
    let keys = manager.list_keys("user123").await?;
    println!("User has {} API keys", keys.len());

    // Revoke a key
    manager.revoke_key(&api_key.id).await?;

    Ok(())
}
```

### Full Authentication Middleware

```rust
use llm_orchestrator_auth::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup components
    let jwt_auth = Arc::new(JwtAuth::new(
        b"your-secret-key-at-least-32-bytes".to_vec()
    ));

    let api_key_store = Arc::new(InMemoryApiKeyStore::new());
    let api_key_manager = Arc::new(ApiKeyManager::new(api_key_store));

    let rbac = Arc::new(RbacEngine::new());

    // Create middleware
    let auth = AuthMiddleware::new(jwt_auth.clone(), api_key_manager, rbac);

    // Generate a token
    let token = jwt_auth.generate_token("user123", vec!["developer".to_string()])?;

    // Authenticate a request
    let auth_header = format!("Bearer {}", token);
    let ctx = auth.authenticate(Some(&auth_header)).await?;

    println!("Authenticated user: {}", ctx.user_id);
    println!("Roles: {:?}", ctx.roles);

    // Check permissions
    auth.authorize(&ctx, &Permission::WorkflowExecute)?;
    println!("Authorization successful!");

    Ok(())
}
```

### Using Refresh Tokens

```rust
use llm_orchestrator_auth::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let jwt_auth = JwtAuth::new(b"your-secret-key-at-least-32-bytes".to_vec());

    // Generate a refresh token
    let refresh_token = jwt_auth.generate_refresh_token("user123")?;

    // Later, use the refresh token to get a new access token
    let access_token = jwt_auth.refresh_access_token(
        &refresh_token,
        vec!["developer".to_string()]
    )?;

    // Verify the new access token
    let claims = jwt_auth.verify_token(&access_token)?;
    println!("New token for user: {}", claims.sub);

    Ok(())
}
```

### Custom RBAC Roles

```rust
use llm_orchestrator_auth::*;

fn main() {
    let rbac = RbacEngine::new();

    // Add a custom role
    rbac.add_role(
        "data_scientist",
        vec![
            Permission::WorkflowRead,
            Permission::WorkflowExecute,
            Permission::ExecutionRead,
        ],
        Some("Data science team role".to_string()),
    );

    // Check permissions
    let has_permission = rbac.check_permission(
        &["data_scientist".to_string()],
        &Permission::WorkflowExecute
    );

    println!("Has execute permission: {}", has_permission);

    // Compute all permissions for a user's roles
    let permissions = rbac.compute_permissions(&[
        "viewer".to_string(),
        "data_scientist".to_string(),
    ]);

    println!("Total permissions: {}", permissions.len());
}
```

## Predefined Roles

The system comes with four predefined roles:

| Role | Permissions | Description |
|------|-------------|-------------|
| `viewer` | `workflow:read`, `execution:read` | Read-only access to workflows and executions |
| `executor` | `workflow:read`, `workflow:execute`, `execution:read` | Can read and execute workflows |
| `developer` | `workflow:read`, `workflow:write`, `workflow:execute`, `execution:read`, `execution:cancel` | Full access to workflows and executions |
| `admin` | All permissions | Full administrative access |

## Available Permissions

- `WorkflowRead` - Read workflow definitions
- `WorkflowWrite` - Create/update workflow definitions
- `WorkflowExecute` - Execute workflows
- `WorkflowDelete` - Delete workflows
- `ExecutionRead` - Read execution history
- `ExecutionCancel` - Cancel running executions
- `AdminAccess` - Full administrative access

## API Key Scopes

When creating API keys, use these scope strings:

- `workflow:read`
- `workflow:write`
- `workflow:execute`
- `workflow:delete`
- `execution:read`
- `execution:cancel`
- `admin`

## Security Best Practices

### 1. Secret Key Management

```rust
// DO: Use environment variables or secret management systems
let secret = std::env::var("JWT_SECRET")?.into_bytes();
let jwt_auth = JwtAuth::new(secret);

// DON'T: Hardcode secrets
let jwt_auth = JwtAuth::new(b"hardcoded-secret".to_vec()); // ❌
```

### 2. Token Expiration

```rust
// Configure custom expiration times
let jwt_auth = JwtAuth::builder(secret)
    .expiry_seconds(900)          // 15 minutes for access tokens
    .refresh_expiry_seconds(604800) // 7 days for refresh tokens
    .build();
```

### 3. API Key Storage

```rust
// API keys are automatically hashed with SHA-256
// The raw key is only shown once at creation
let api_key = manager.create_key(
    "user123",
    vec!["workflow:read".to_string()],
    None,
    Some(90), // 90-day expiration
).await?;

// Store this securely - it won't be shown again!
println!("Save this key: {}", api_key.key);
```

### 4. Permission Checks

```rust
// Always check permissions before operations
async fn execute_workflow(ctx: &AuthContext, workflow_id: &str) -> Result<()> {
    ctx.require_permission(&Permission::WorkflowExecute)?;

    // Proceed with execution...
    Ok(())
}
```

## Architecture

```
┌─────────────────────────────────────────┐
│         Auth Middleware                 │
│  ┌───────────────────────────────────┐  │
│  │  Extract Authorization Header     │  │
│  └───────────────┬───────────────────┘  │
│                  │                       │
│                  ▼                       │
│         ┌────────┴────────┐             │
│         │  JWT or API Key? │             │
│         └────────┬────────┘             │
│                  │                       │
│        ┌─────────┴─────────┐            │
│        ▼                   ▼            │
│  ┌──────────┐        ┌──────────┐      │
│  │   JWT    │        │ API Key  │      │
│  │  Verify  │        │  Lookup  │      │
│  └─────┬────┘        └─────┬────┘      │
│        │                   │            │
│        └─────────┬─────────┘            │
│                  ▼                       │
│         ┌────────────────┐              │
│         │  Compute       │              │
│         │  Permissions   │              │
│         │  (RBAC Engine) │              │
│         └────────┬───────┘              │
│                  ▼                       │
│         ┌────────────────┐              │
│         │  Auth Context  │              │
│         └────────────────┘              │
└─────────────────────────────────────────┘
```

## Performance

- **JWT Verification**: < 1ms average
- **API Key Lookup**: < 5ms with in-memory store
- **Permission Check**: < 0.1ms
- **Token Generation**: < 1ms

## Testing

Run the test suite:

```bash
cargo test -p llm-orchestrator-auth
```

The crate includes:
- 40+ unit tests
- 5+ integration tests
- 100% coverage of critical security paths

## Examples

See the `examples/` directory for more detailed examples:

- `jwt_auth.rs` - JWT authentication flow
- `api_keys.rs` - API key management
- `rbac.rs` - Custom role creation
- `middleware.rs` - Full middleware integration

## License

MIT OR Apache-2.0
