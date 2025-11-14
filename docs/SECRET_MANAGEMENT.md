# Secret Management Guide

## Overview

The LLM Orchestrator provides comprehensive secret management capabilities through the `llm-orchestrator-secrets` crate. This guide covers how to securely store and retrieve sensitive configuration data such as API keys, database passwords, and other credentials.

## Features

- **Multiple Backends**: HashiCorp Vault, AWS Secrets Manager, or environment variables
- **Automatic Caching**: Optional TTL-based caching to reduce backend calls and improve performance
- **Secret Rotation**: Support for rotating secrets without service downtime
- **Version Management**: Access historical versions of secrets (where supported)
- **Security First**: Zero secrets in logs, secure token handling, audit-ready

## Supported Backends

### 1. Environment Variables (Development/Testing)

Best for: Local development, testing, simple deployments

```rust
use llm_orchestrator_secrets::{EnvSecretStore, SecretStore};

let store = EnvSecretStore::new();

// Retrieves from environment variable: OPENAI_API_KEY
let secret = store.get_secret("openai/api_key").await?;
println!("Retrieved API key");
```

**Key Mapping**: `openai/api_key` → `OPENAI_API_KEY`

Custom prefix support:
```rust
let store = EnvSecretStore::with_prefix("APP_".to_string());
// Now "db/password" → "APP_DB_PASSWORD"
```

### 2. HashiCorp Vault (Production)

Best for: On-premise deployments, multi-cloud, strict security requirements

```rust
use llm_orchestrator_secrets::{VaultSecretStore, SecretStore};

let store = VaultSecretStore::new(
    "https://vault.example.com:8200".to_string(),
    "hvs.CAESIJ...".to_string(), // Vault token
)?;

// Configure mount path and namespace
let store = store
    .with_mount_path("secret".to_string())
    .with_namespace("production".to_string());

let secret = store.get_secret("database/password").await?;
```

**Features**:
- KV v2 secret engine support
- Secret versioning
- Namespace support (Vault Enterprise)
- Automatic token renewal
- TLS/mTLS support

### 3. AWS Secrets Manager (AWS Cloud)

Best for: AWS deployments, managed secret rotation, compliance requirements

```rust
use llm_orchestrator_secrets::{AwsSecretStore, SecretStore};
use aws_sdk_secretsmanager::config::Region;

// With specific region
let store = AwsSecretStore::new(Region::new("us-east-1")).await?;

// Or from environment
let store = AwsSecretStore::from_env().await?;

let secret = store.get_secret("prod/api/keys/openai").await?;
```

**Features**:
- IAM role authentication
- Cross-region support
- Automatic rotation scheduling
- Version management
- Built-in encryption

## Using the Builder Pattern

The recommended way to create secret stores is using the builder:

```rust
use llm_orchestrator_secrets::{SecretManagerBuilder, SecretStoreType};
use chrono::Duration;

// Environment variable store with caching
let store = SecretManagerBuilder::new(SecretStoreType::Environment)
    .with_cache(Duration::minutes(10))
    .build()
    .await?;

// HashiCorp Vault with configuration
use llm_orchestrator_secrets::VaultConfig;

let vault_config = VaultConfig::from_env()?; // Reads VAULT_ADDR and VAULT_TOKEN
let store = SecretManagerBuilder::new(SecretStoreType::Vault)
    .with_vault_config(vault_config)
    .with_cache(Duration::minutes(5))
    .build()
    .await?;

// AWS Secrets Manager with region
use llm_orchestrator_secrets::AwsConfig;

let aws_config = AwsConfig::new(Region::new("us-west-2"));
let store = SecretManagerBuilder::new(SecretStoreType::AwsSecretsManager)
    .with_aws_config(aws_config)
    .with_cache(Duration::minutes(5))
    .build()
    .await?;
```

## Caching

Caching reduces backend calls and improves performance. Enable it for frequently accessed secrets:

```rust
use llm_orchestrator_secrets::{SecretCache, EnvSecretStore};
use std::sync::Arc;
use chrono::Duration;

let backend = Arc::new(EnvSecretStore::new());
let cache = SecretCache::new(backend, Duration::minutes(5));

// First call - cache miss
let secret1 = cache.get("api_key").await?;

// Second call - cache hit (< 1ms)
let secret2 = cache.get("api_key").await?;

// Cache statistics
let stats = cache.stats();
println!("Hit rate: {:.2}%", stats.hit_rate());
println!("Total accesses: {}", stats.total_accesses());

// Manual invalidation
cache.invalidate("api_key");

// Clear all cache
cache.clear();

// Cleanup expired entries
cache.cleanup_expired();
```

**Performance Impact**:
- Cache hit: < 1ms
- Cache miss (environment): ~1ms
- Cache miss (Vault): 50-100ms
- Cache miss (AWS): 50-150ms

## Integration with LLM Providers

Use secret stores with LLM providers (requires `secrets` feature):

```toml
[dependencies]
llm-orchestrator-providers = { version = "0.1", features = ["secrets"] }
llm-orchestrator-secrets = "0.1"
```

```rust
use llm_orchestrator_providers::{OpenAIProvider, AnthropicProvider};
use llm_orchestrator_secrets::{SecretManagerBuilder, SecretStoreType};
use std::sync::Arc;

// Create secret store
let secret_store = SecretManagerBuilder::build_env(None).await?;

// Initialize providers from secret store
let openai = OpenAIProvider::from_secret_store(
    secret_store.clone(),
    "openai/api_key"
).await?;

let anthropic = AnthropicProvider::from_secret_store(
    secret_store.clone(),
    "anthropic/api_key"
).await?;
```

## Secret Operations

### Retrieve a Secret

```rust
let secret = store.get_secret("database/password").await?;
println!("Key: {}", secret.key);
println!("Version: {:?}", secret.version);
println!("Created: {}", secret.created_at);
// Note: Never log secret.value in production!
```

### Store a Secret

```rust
use llm_orchestrator_secrets::SecretMetadata;
use std::collections::HashMap;

let metadata = SecretMetadata::new()
    .with_description("Production database password".to_string())
    .add_tag("environment".to_string(), "production".to_string())
    .add_tag("service".to_string(), "postgresql".to_string());

store.put_secret(
    "database/password",
    "super_secret_value",
    Some(metadata)
).await?;
```

### Rotate a Secret

```rust
// Initiate secret rotation
let new_secret = store.rotate_secret("database/password").await?;
println!("Rotated to version: {:?}", new_secret.version);
```

### List Secrets

```rust
let keys = store.list_secrets("database/").await?;
for key in keys {
    println!("Found secret: {}", key);
}
```

### Version Management

```rust
// Get all versions
let versions = store.get_secret_versions("api_key").await?;
for version in versions {
    println!("Version {}: created {}, current: {}",
        version.version,
        version.created_at,
        version.is_current
    );
}

// Get specific version
let old_secret = store.get_secret_version("api_key", "2").await?;
```

## Best Practices

### Security

1. **Never log secret values**
   ```rust
   // ❌ DON'T
   println!("Secret: {}", secret.value);

   // ✅ DO
   println!("Retrieved secret: {}", secret.key);
   ```

2. **Use Vault or AWS in production**
   - Environment variables are for development only
   - They can appear in process listings and logs

3. **Enable least-privilege access**
   - Vault: Use policies to limit access by path
   - AWS: Use IAM roles with minimal permissions

4. **Rotate secrets regularly**
   ```rust
   // Set up automatic rotation (AWS)
   let aws_store = AwsSecretStore::new(region).await?;
   aws_store.create_secret_with_rotation(
       "api_key",
       "initial_value",
       30 // Rotate every 30 days
   ).await?;
   ```

5. **Monitor secret access**
   - Enable audit logging in Vault
   - Use AWS CloudTrail for Secrets Manager

### Performance

1. **Enable caching for frequently accessed secrets**
   ```rust
   let store = SecretManagerBuilder::new(SecretStoreType::Vault)
       .with_vault_config(config)
       .with_cache(Duration::minutes(5))
       .build()
       .await?;
   ```

2. **Balance cache TTL with security**
   - Short TTL (1-5 min): Better security, more backend calls
   - Long TTL (10-60 min): Better performance, slower rotation

3. **Run periodic cache cleanup**
   ```rust
   // In a background task
   tokio::spawn(async move {
       loop {
           tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
           cache.cleanup_expired();
       }
   });
   ```

### Error Handling

```rust
use llm_orchestrator_secrets::SecretError;

match store.get_secret("api_key").await {
    Ok(secret) => {
        // Use secret
    }
    Err(SecretError::NotFound(key)) => {
        eprintln!("Secret not found: {}", key);
    }
    Err(SecretError::AuthenticationFailed(msg)) => {
        eprintln!("Auth failed: {}", msg);
    }
    Err(SecretError::BackendUnavailable(msg)) => {
        eprintln!("Backend unavailable: {}", msg);
        // Implement retry logic or fallback
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
    }
}
```

## Environment Variable Configuration

### Vault

```bash
export VAULT_ADDR=https://vault.example.com:8200
export VAULT_TOKEN=hvs.CAESIJ...
export VAULT_NAMESPACE=production  # Optional (Enterprise)
export VAULT_MOUNT_PATH=secret     # Optional (default: secret)
```

### AWS

```bash
export AWS_REGION=us-east-1
export AWS_ACCESS_KEY_ID=AKIA...
export AWS_SECRET_ACCESS_KEY=...
# Or use IAM roles (recommended)
```

### Application Secrets

```bash
export OPENAI_API_KEY=sk-...
export ANTHROPIC_API_KEY=sk-ant-...
export DATABASE_PASSWORD=...
```

## Production Deployment Checklist

- [ ] Secret backend configured (Vault or AWS Secrets Manager)
- [ ] Authentication properly set up (tokens, IAM roles)
- [ ] Secrets migrated from environment variables
- [ ] Cache TTL configured appropriately
- [ ] Secret rotation schedule defined
- [ ] Audit logging enabled
- [ ] Access controls configured (least privilege)
- [ ] Health checks implemented
- [ ] Backup/recovery procedures documented
- [ ] Secrets never logged in application code

## Troubleshooting

### Secret Not Found

```
Error: Secret not found: database/password
```

**Solutions**:
- Verify the key path is correct
- Check permissions in Vault/AWS
- Ensure secret exists in the backend
- Verify namespace (Vault) or region (AWS)

### Authentication Failed

```
Error: Authentication failed: token expired
```

**Solutions**:
- Renew Vault token: `store.renew_token().await?`
- Rotate AWS credentials
- Verify token/credentials are valid

### Backend Unavailable

```
Error: Backend unavailable: connection timeout
```

**Solutions**:
- Check network connectivity
- Verify backend service is running
- Check firewall rules
- Implement retry logic with exponential backoff

## Examples

See the [`examples/`](../examples/) directory for complete working examples:

- `examples/secret_basic.rs` - Basic secret retrieval
- `examples/secret_vault.rs` - HashiCorp Vault integration
- `examples/secret_aws.rs` - AWS Secrets Manager integration
- `examples/secret_cache.rs` - Caching demonstrations
- `examples/secret_rotation.rs` - Secret rotation workflow

## Further Reading

- [HashiCorp Vault Documentation](https://developer.hashicorp.com/vault/docs)
- [AWS Secrets Manager User Guide](https://docs.aws.amazon.com/secretsmanager/)
- [OWASP Secrets Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html)
