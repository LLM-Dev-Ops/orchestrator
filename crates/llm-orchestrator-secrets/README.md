# llm-orchestrator-secrets

Comprehensive secret management for the LLM Orchestrator, providing secure storage and retrieval of sensitive configuration data.

## Features

- **Multiple Backends**: HashiCorp Vault, AWS Secrets Manager, or environment variables
- **Caching**: Optional TTL-based in-memory caching to reduce backend calls
- **Secret Rotation**: Support for rotating secrets without downtime
- **Version Management**: Access historical versions of secrets (where supported)
- **Security**: Zero secrets in logs, secure token handling
- **Async/Await**: Full async support with tokio
- **Type Safe**: Strongly typed error handling with comprehensive error types

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
llm-orchestrator-secrets = "0.1"
```

## Quick Start

### Environment Variables (Development)

```rust
use llm_orchestrator_secrets::{EnvSecretStore, SecretStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = EnvSecretStore::new();

    // Reads from OPENAI_API_KEY environment variable
    let secret = store.get_secret("openai/api_key").await?;

    println!("Retrieved API key: {}", secret.key);
    Ok(())
}
```

### HashiCorp Vault

```rust
use llm_orchestrator_secrets::{VaultSecretStore, SecretStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = VaultSecretStore::new(
        "https://vault.example.com:8200".to_string(),
        "hvs.YOUR_TOKEN".to_string(),
    )?;

    let secret = store.get_secret("database/password").await?;
    Ok(())
}
```

### AWS Secrets Manager

```rust
use llm_orchestrator_secrets::{AwsSecretStore, SecretStore};
use aws_sdk_secretsmanager::config::Region;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = AwsSecretStore::new(Region::new("us-east-1")).await?;

    let secret = store.get_secret("prod/api/key").await?;
    Ok(())
}
```

### With Caching

```rust
use llm_orchestrator_secrets::{SecretManagerBuilder, SecretStoreType};
use chrono::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = SecretManagerBuilder::new(SecretStoreType::Environment)
        .with_cache(Duration::minutes(10))
        .build()
        .await?;

    let secret = store.get_secret("api_key").await?;
    Ok(())
}
```

## Supported Backends

| Backend | Production Ready | Versioning | Rotation | Caching |
|---------|-----------------|------------|----------|---------|
| Environment Variables | Development only | ❌ | ❌ | ✅ |
| HashiCorp Vault | ✅ | ✅ | ✅ | ✅ |
| AWS Secrets Manager | ✅ | ✅ | ✅ | ✅ |

## API Overview

### SecretStore Trait

All backends implement the `SecretStore` trait:

```rust
#[async_trait]
pub trait SecretStore: Send + Sync {
    async fn get_secret(&self, key: &str) -> Result<Secret>;
    async fn put_secret(&self, key: &str, value: &str, metadata: Option<SecretMetadata>) -> Result<()>;
    async fn delete_secret(&self, key: &str) -> Result<()>;
    async fn list_secrets(&self, prefix: &str) -> Result<Vec<String>>;
    async fn rotate_secret(&self, key: &str) -> Result<Secret>;
    async fn health_check(&self) -> Result<()>;
    async fn get_secret_versions(&self, key: &str) -> Result<Vec<SecretVersion>>;
    async fn get_secret_version(&self, key: &str, version: &str) -> Result<Secret>;
}
```

### Secret Model

```rust
pub struct Secret {
    pub key: String,
    pub value: String,
    pub version: Option<String>,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}
```

## Performance

### Cache Performance

- **Cache hit**: < 1ms
- **Cache miss (env)**: ~1ms
- **Cache miss (Vault)**: 50-100ms
- **Cache miss (AWS)**: 50-150ms

### Recommended Cache TTL

- **Development**: 5-10 minutes
- **Production**: 2-5 minutes for API keys, 1 minute for credentials

## Security Best Practices

1. **Never log secret values**
2. **Use Vault or AWS in production** (not environment variables)
3. **Enable caching cautiously** (balance performance vs. freshness)
4. **Rotate secrets regularly**
5. **Use least-privilege access** (IAM roles, Vault policies)
6. **Monitor secret access** (audit logs)

## Testing

The crate includes comprehensive unit and integration tests:

```bash
cargo test -p llm-orchestrator-secrets
```

Tests cover:
- All secret store implementations
- Cache functionality with TTL
- Secret versioning
- Error handling
- Concurrent access

## Examples

See the main documentation for complete examples:

- Environment variable usage
- HashiCorp Vault integration with namespaces
- AWS Secrets Manager with rotation
- Caching strategies
- Integration with LLM providers

## Documentation

- [Full Secret Management Guide](../../docs/SECRET_MANAGEMENT.md)
- [API Documentation](https://docs.rs/llm-orchestrator-secrets)

## License

MIT OR Apache-2.0
