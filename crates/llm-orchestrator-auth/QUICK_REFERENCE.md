# LLM Orchestrator Auth - Quick Reference Card

## Installation

```toml
[dependencies]
llm-orchestrator-auth = "0.1.0"
```

## Quick Start (30 seconds)

```rust
use llm_orchestrator_auth::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup
    let jwt_auth = Arc::new(JwtAuth::new(b"secret-key-32-bytes".to_vec()));
    let api_key_store = Arc::new(InMemoryApiKeyStore::new());
    let api_key_manager = Arc::new(ApiKeyManager::new(api_key_store));
    let rbac = Arc::new(RbacEngine::new());
    let auth = AuthMiddleware::new(jwt_auth.clone(), api_key_manager, rbac);

    // 2. Generate token
    let token = jwt_auth.generate_token("user", vec!["developer".to_string()])?;

    // 3. Authenticate
    let ctx = auth.authenticate(Some(&format!("Bearer {}", token))).await?;

    // 4. Authorize
    auth.authorize(&ctx, &Permission::WorkflowExecute)?;

    Ok(())
}
```

## JWT Operations

### Generate Token
```rust
let token = jwt_auth.generate_token("user_id", vec!["developer".to_string()])?;
```

### Verify Token
```rust
let claims = jwt_auth.verify_token(&token)?;
println!("User: {}, Roles: {:?}", claims.sub, claims.roles);
```

### Refresh Token
```rust
let refresh_token = jwt_auth.generate_refresh_token("user_id")?;
let new_token = jwt_auth.refresh_access_token(&refresh_token, vec!["executor"])?;
```

## API Key Operations

### Create Key
```rust
let key = api_key_manager.create_key(
    "user_id",
    vec!["workflow:read".to_string(), "workflow:execute".to_string()],
    Some("My Key".to_string()),
    Some(90), // expires in 90 days
).await?;

println!("Save this: {}", key.key); // Only shown once!
```

### Lookup Key
```rust
let info = api_key_manager.lookup_key(&api_key).await?;
println!("User: {}, Scopes: {:?}", info.user_id, info.scopes);
```

### Revoke Key
```rust
api_key_manager.revoke_key(&key_id).await?;
```

### List User Keys
```rust
let keys = api_key_manager.list_keys("user_id").await?;
```

## RBAC Operations

### Check Permission
```rust
let has_perm = rbac.check_permission(&["developer".to_string()], &Permission::WorkflowWrite);
```

### Compute Permissions
```rust
let perms = rbac.compute_permissions(&["viewer".to_string(), "executor".to_string()]);
```

### Add Custom Role
```rust
rbac.add_role(
    "data_scientist",
    vec![Permission::WorkflowRead, Permission::WorkflowExecute],
    Some("Data science role".to_string()),
);
```

## Auth Middleware

### Authenticate Request
```rust
// JWT
let ctx = auth.authenticate(Some("Bearer eyJ0...")).await?;

// API Key
let ctx = auth.authenticate(Some("ApiKey llm_orch_abc123...")).await?;
```

### Authorize Operation
```rust
auth.authorize(&ctx, &Permission::WorkflowExecute)?;
```

## Predefined Roles

| Role | Permissions |
|------|-------------|
| `viewer` | read workflows, read executions |
| `executor` | read workflows, execute workflows, read executions |
| `developer` | read, write, execute workflows; read, cancel executions |
| `admin` | all permissions |

## Available Permissions

- `WorkflowRead`
- `WorkflowWrite`
- `WorkflowExecute`
- `WorkflowDelete`
- `ExecutionRead`
- `ExecutionCancel`
- `AdminAccess`

## API Key Scopes

- `workflow:read`
- `workflow:write`
- `workflow:execute`
- `workflow:delete`
- `execution:read`
- `execution:cancel`
- `admin`

## Error Handling

```rust
match auth.authenticate(auth_header).await {
    Ok(ctx) => { /* authenticated */ },
    Err(AuthError::MissingCredentials) => { /* 401 */ },
    Err(AuthError::InvalidCredentials) => { /* 401 */ },
    Err(AuthError::TokenExpired) => { /* 401 */ },
    Err(AuthError::ApiKeyNotFound) => { /* 401 */ },
    Err(AuthError::InsufficientPermissions { .. }) => { /* 403 */ },
    Err(e) => { /* 500 */ },
}
```

## Configuration

### Custom JWT Settings
```rust
let jwt_auth = JwtAuth::builder(secret)
    .issuer("my-app".to_string())
    .expiry_seconds(3600)          // 1 hour
    .refresh_expiry_seconds(2592000) // 30 days
    .build();
```

## Common Patterns

### Protect Endpoint
```rust
async fn execute_workflow(
    auth_header: Option<String>,
    workflow_id: String,
) -> Result<Response> {
    let ctx = auth.authenticate(auth_header.as_deref()).await?;
    auth.authorize(&ctx, &Permission::WorkflowExecute)?;

    // Execute workflow
    Ok(response)
}
```

### Check Multiple Permissions
```rust
if rbac.check_all_permissions(&ctx.roles, &[
    Permission::WorkflowRead,
    Permission::WorkflowWrite,
]) {
    // User has both permissions
}
```

### User Has Any Permission
```rust
if rbac.check_any_permission(&ctx.roles, &[
    Permission::WorkflowWrite,
    Permission::WorkflowDelete,
]) {
    // User has at least one permission
}
```

## Security Best Practices

### ✅ DO
- Store JWT secret in environment variables
- Use HTTPS for token transmission
- Set appropriate token expiration
- Hash API keys before storage
- Check permissions before operations
- Rotate API keys regularly

### ❌ DON'T
- Hardcode secrets in source code
- Store raw API keys
- Skip permission checks
- Use tokens without expiration
- Log raw tokens or keys

## Performance Tips

- JWT verification: < 1ms (stateless)
- API key lookup: < 5ms with in-memory store
- Permission checks: < 0.1ms
- Cache auth contexts if needed

## Testing

```rust
#[tokio::test]
async fn test_auth() {
    let jwt_auth = JwtAuth::new(b"test-secret-32-bytes".to_vec());
    let token = jwt_auth.generate_token("user", vec!["admin".to_string()]).unwrap();
    let claims = jwt_auth.verify_token(&token).unwrap();
    assert_eq!(claims.sub, "user");
}
```

## Examples

Run examples:
```bash
cargo run --example jwt_auth_example
cargo run --example api_key_example
cargo run --example rbac_example
cargo run --example full_auth_flow
```

## More Information

- Full docs: `README.md`
- Implementation details: `IMPLEMENTATION_SUMMARY.md`
- API docs: `cargo doc --open`
