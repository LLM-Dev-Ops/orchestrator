//! # LLM Orchestrator Authentication & Authorization
//!
//! This crate provides comprehensive authentication and authorization for the LLM Orchestrator.
//!
//! ## Features
//!
//! - **JWT Authentication**: Stateless token-based authentication with short-lived access tokens
//!   and long-lived refresh tokens
//! - **API Key Management**: Secure API key generation, hashing, and validation
//! - **Role-Based Access Control (RBAC)**: Fine-grained permission system with predefined roles
//! - **Auth Middleware**: Ready-to-use middleware for authenticating requests
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use llm_orchestrator_auth::*;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create JWT auth
//!     let jwt_auth = Arc::new(JwtAuth::new(b"your-secret-key-at-least-32-bytes".to_vec()));
//!
//!     // Create API key manager
//!     let api_key_store = Arc::new(InMemoryApiKeyStore::new());
//!     let api_key_manager = Arc::new(ApiKeyManager::new(api_key_store));
//!
//!     // Create RBAC engine
//!     let rbac = Arc::new(RbacEngine::new());
//!
//!     // Create auth middleware
//!     let auth = AuthMiddleware::new(jwt_auth.clone(), api_key_manager.clone(), rbac.clone());
//!
//!     // Generate a JWT token
//!     let token = jwt_auth.generate_token("user123", vec!["developer".to_string()])?;
//!     println!("JWT Token: {}", token);
//!
//!     // Authenticate a request
//!     let auth_header = format!("Bearer {}", token);
//!     let ctx = auth.authenticate(Some(&auth_header)).await?;
//!     println!("Authenticated user: {}", ctx.user_id);
//!
//!     // Check permissions
//!     ctx.require_permission(&Permission::WorkflowExecute)?;
//!     println!("User has permission to execute workflows");
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Predefined Roles
//!
//! - **viewer**: Read-only access to workflows and executions
//! - **executor**: Can read and execute workflows
//! - **developer**: Full access to workflows, can create/update/delete
//! - **admin**: Full administrative access to all resources
//!
//! ## Security Features
//!
//! - JWT tokens expire after 15 minutes by default
//! - Refresh tokens expire after 7 days by default
//! - API keys are hashed using SHA-256 before storage
//! - Cryptographically secure random key generation
//! - Token expiration validation
//! - Permission-based authorization

pub mod api_keys;
pub mod jwt;
pub mod middleware;
pub mod models;
pub mod rbac;

// Re-export main types for convenience
pub use api_keys::{ApiKeyManager, ApiKeyStore, InMemoryApiKeyStore};
pub use jwt::JwtAuth;
pub use middleware::AuthMiddleware;
pub use models::{
    ApiKey, ApiKeyInfo, AuthContext, AuthError, AuthResult, AuthType, Claims, Permission,
    RolePolicy,
};
pub use rbac::RbacEngine;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_full_jwt_flow() {
        // Setup
        let jwt_auth = Arc::new(JwtAuth::new(
            b"test-secret-key-at-least-32-bytes-long".to_vec(),
        ));
        let api_key_store = Arc::new(InMemoryApiKeyStore::new());
        let api_key_manager = Arc::new(ApiKeyManager::new(api_key_store));
        let rbac = Arc::new(RbacEngine::new());
        let auth = AuthMiddleware::new(jwt_auth.clone(), api_key_manager, rbac);

        // Generate token
        let token = jwt_auth
            .generate_token("user123", vec!["developer".to_string()])
            .unwrap();

        // Authenticate
        let auth_header = format!("Bearer {}", token);
        let ctx = auth.authenticate(Some(&auth_header)).await.unwrap();

        // Verify context
        assert_eq!(ctx.user_id, "user123");
        assert_eq!(ctx.roles, vec!["developer"]);
        assert!(ctx.has_permission(&Permission::WorkflowWrite));

        // Check authorization
        assert!(auth
            .authorize(&ctx, &Permission::WorkflowWrite)
            .is_ok());
    }

    #[tokio::test]
    async fn test_full_api_key_flow() {
        // Setup
        let jwt_auth = Arc::new(JwtAuth::new(
            b"test-secret-key-at-least-32-bytes-long".to_vec(),
        ));
        let api_key_store = Arc::new(InMemoryApiKeyStore::new());
        let api_key_manager = Arc::new(ApiKeyManager::new(api_key_store));
        let rbac = Arc::new(RbacEngine::new());
        let auth = AuthMiddleware::new(jwt_auth, api_key_manager.clone(), rbac);

        // Create API key
        let api_key = api_key_manager
            .create_key(
                "user456",
                vec!["workflow:read".to_string(), "workflow:execute".to_string()],
                Some("My Test Key".to_string()),
                Some(30),
            )
            .await
            .unwrap();

        // Authenticate with API key
        let auth_header = format!("ApiKey {}", api_key.key);
        let ctx = auth.authenticate(Some(&auth_header)).await.unwrap();

        // Verify context
        assert_eq!(ctx.user_id, "user456");
        assert!(ctx.has_permission(&Permission::WorkflowRead));
        assert!(ctx.has_permission(&Permission::WorkflowExecute));
        assert!(!ctx.has_permission(&Permission::WorkflowWrite));

        // List user's keys
        let keys = api_key_manager.list_keys("user456").await.unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].name, Some("My Test Key".to_string()));

        // Revoke key
        api_key_manager.revoke_key(&api_key.id).await.unwrap();

        // Verify key is revoked
        let result = auth.authenticate(Some(&auth_header)).await;
        assert!(matches!(result, Err(AuthError::ApiKeyNotFound)));
    }

    #[tokio::test]
    async fn test_refresh_token_flow() {
        // Setup
        let jwt_auth = JwtAuth::new(b"test-secret-key-at-least-32-bytes-long".to_vec());

        // Generate refresh token
        let refresh_token = jwt_auth.generate_refresh_token("user789").unwrap();

        // Verify refresh token
        let user_id = jwt_auth.verify_refresh_token(&refresh_token).unwrap();
        assert_eq!(user_id, "user789");

        // Use refresh token to get new access token
        let access_token = jwt_auth
            .refresh_access_token(&refresh_token, vec!["executor".to_string()])
            .unwrap();

        // Verify new access token
        let claims = jwt_auth.verify_token(&access_token).unwrap();
        assert_eq!(claims.sub, "user789");
        assert_eq!(claims.roles, vec!["executor"]);
    }

    #[tokio::test]
    async fn test_rbac_permission_checks() {
        let rbac = RbacEngine::new();

        // Test viewer permissions
        assert!(rbac.check_permission(&["viewer".to_string()], &Permission::WorkflowRead));
        assert!(!rbac.check_permission(&["viewer".to_string()], &Permission::WorkflowWrite));

        // Test executor permissions
        assert!(rbac.check_permission(
            &["executor".to_string()],
            &Permission::WorkflowExecute
        ));

        // Test developer permissions
        assert!(rbac.check_permission(
            &["developer".to_string()],
            &Permission::WorkflowWrite
        ));

        // Test admin permissions
        for permission in Permission::all() {
            assert!(rbac.check_permission(&["admin".to_string()], &permission));
        }
    }

    #[tokio::test]
    async fn test_multiple_roles() {
        let jwt_auth = Arc::new(JwtAuth::new(
            b"test-secret-key-at-least-32-bytes-long".to_vec(),
        ));
        let rbac = Arc::new(RbacEngine::new());

        // Create token with multiple roles
        let token = jwt_auth
            .generate_token(
                "user123",
                vec!["viewer".to_string(), "executor".to_string()],
            )
            .unwrap();

        let claims = jwt_auth.verify_token(&token).unwrap();

        // Compute combined permissions
        let permissions = rbac.compute_permissions(&claims.roles);

        // Should have union of both roles
        assert!(permissions.contains(&Permission::WorkflowRead));
        assert!(permissions.contains(&Permission::WorkflowExecute));
        assert!(permissions.contains(&Permission::ExecutionRead));
    }
}
