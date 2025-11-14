use crate::api_keys::ApiKeyManager;
use crate::jwt::JwtAuth;
use crate::models::{AuthContext, AuthError, AuthResult, AuthType};
use crate::rbac::RbacEngine;
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// Authentication middleware for validating requests
pub struct AuthMiddleware {
    /// JWT authentication handler
    jwt_auth: Arc<JwtAuth>,

    /// API key manager
    api_key_manager: Arc<ApiKeyManager>,

    /// RBAC engine for permission checks
    rbac: Arc<RbacEngine>,
}

impl AuthMiddleware {
    /// Create a new authentication middleware
    pub fn new(
        jwt_auth: Arc<JwtAuth>,
        api_key_manager: Arc<ApiKeyManager>,
        rbac: Arc<RbacEngine>,
    ) -> Self {
        Self {
            jwt_auth,
            api_key_manager,
            rbac,
        }
    }

    /// Authenticate a request using either JWT or API key
    ///
    /// # Arguments
    /// * `authorization_header` - The Authorization header value (e.g., "Bearer token" or "ApiKey key")
    ///
    /// # Returns
    /// An authenticated context if successful
    pub async fn authenticate(&self, authorization_header: Option<&str>) -> AuthResult<AuthContext> {
        let auth_header = authorization_header.ok_or(AuthError::MissingCredentials)?;

        // Parse the authorization header
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            self.authenticate_jwt(token).await
        } else if let Some(api_key) = auth_header.strip_prefix("ApiKey ") {
            self.authenticate_api_key(api_key).await
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }

    /// Authenticate using JWT token
    async fn authenticate_jwt(&self, token: &str) -> AuthResult<AuthContext> {
        // Verify and decode the token
        let claims = self.jwt_auth.verify_token(token)?;

        // Compute permissions from roles
        let permissions = self.rbac.compute_permissions(&claims.roles);

        Ok(AuthContext {
            user_id: claims.sub,
            roles: claims.roles,
            permissions,
            auth_type: AuthType::Jwt(token.to_string()),
            expires_at: DateTime::from_timestamp(claims.exp as i64, 0)
                .unwrap_or_else(Utc::now),
        })
    }

    /// Authenticate using API key
    async fn authenticate_api_key(&self, api_key: &str) -> AuthResult<AuthContext> {
        // Lookup and validate the API key
        let key_info = self.api_key_manager.lookup_key(api_key).await?;

        // Convert scopes to permissions
        let permissions = self.scopes_to_permissions(&key_info.scopes);

        // Determine roles from scopes (for backward compatibility)
        let roles = self.scopes_to_roles(&key_info.scopes);

        Ok(AuthContext {
            user_id: key_info.user_id,
            roles,
            permissions,
            auth_type: AuthType::ApiKey(key_info.id),
            expires_at: key_info.expires_at.unwrap_or_else(|| {
                // If no expiration, set to far future
                Utc::now() + chrono::Duration::days(365 * 10)
            }),
        })
    }

    /// Convert API key scopes to permissions
    fn scopes_to_permissions(&self, scopes: &[String]) -> Vec<crate::models::Permission> {
        use crate::models::Permission;

        scopes
            .iter()
            .filter_map(|scope| match scope.as_str() {
                "workflow:read" => Some(Permission::WorkflowRead),
                "workflow:write" => Some(Permission::WorkflowWrite),
                "workflow:execute" => Some(Permission::WorkflowExecute),
                "workflow:delete" => Some(Permission::WorkflowDelete),
                "execution:read" => Some(Permission::ExecutionRead),
                "execution:cancel" => Some(Permission::ExecutionCancel),
                "admin" => Some(Permission::AdminAccess),
                _ => None,
            })
            .collect()
    }

    /// Convert API key scopes to roles (for backward compatibility)
    fn scopes_to_roles(&self, scopes: &[String]) -> Vec<String> {
        // Check if scopes match predefined role patterns
        let has_read = scopes.contains(&"workflow:read".to_string());
        let has_write = scopes.contains(&"workflow:write".to_string());
        let has_execute = scopes.contains(&"workflow:execute".to_string());
        let has_admin = scopes.contains(&"admin".to_string());

        if has_admin {
            vec!["admin".to_string()]
        } else if has_write && has_execute {
            vec!["developer".to_string()]
        } else if has_execute {
            vec!["executor".to_string()]
        } else if has_read {
            vec!["viewer".to_string()]
        } else {
            vec![]
        }
    }

    /// Extract bearer token from authorization header
    pub fn extract_bearer_token(authorization_header: Option<&str>) -> Option<String> {
        authorization_header
            .and_then(|h| h.strip_prefix("Bearer "))
            .map(|t| t.to_string())
    }

    /// Extract API key from authorization header
    pub fn extract_api_key(authorization_header: Option<&str>) -> Option<String> {
        authorization_header
            .and_then(|h| h.strip_prefix("ApiKey "))
            .map(|t| t.to_string())
    }

    /// Check if the auth context has the required permission
    pub fn authorize(
        &self,
        ctx: &AuthContext,
        permission: &crate::models::Permission,
    ) -> AuthResult<()> {
        self.rbac.require_permission(ctx, permission)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_keys::InMemoryApiKeyStore;
    

    async fn create_test_middleware() -> AuthMiddleware {
        let jwt_auth = Arc::new(JwtAuth::new(
            b"test-secret-key-at-least-32-bytes-long".to_vec(),
        ));

        let api_key_store = Arc::new(InMemoryApiKeyStore::new());
        let api_key_manager = Arc::new(ApiKeyManager::new(api_key_store));

        let rbac = Arc::new(RbacEngine::new());

        AuthMiddleware::new(jwt_auth, api_key_manager, rbac)
    }

    #[tokio::test]
    async fn test_authenticate_with_jwt() {
        let middleware = create_test_middleware().await;

        // Generate a token
        let token = middleware
            .jwt_auth
            .generate_token("user123", vec!["developer".to_string()])
            .unwrap();

        // Authenticate with the token
        let auth_header = format!("Bearer {}", token);
        let ctx = middleware.authenticate(Some(&auth_header)).await.unwrap();

        assert_eq!(ctx.user_id, "user123");
        assert_eq!(ctx.roles, vec!["developer"]);
        assert!(matches!(ctx.auth_type, AuthType::Jwt(_)));
    }

    #[tokio::test]
    async fn test_authenticate_with_api_key() {
        let middleware = create_test_middleware().await;

        // Create an API key
        let api_key = middleware
            .api_key_manager
            .create_key(
                "user456",
                vec![
                    "workflow:read".to_string(),
                    "workflow:execute".to_string(),
                ],
                None,
                None,
            )
            .await
            .unwrap();

        // Authenticate with the API key
        let auth_header = format!("ApiKey {}", api_key.key);
        let ctx = middleware.authenticate(Some(&auth_header)).await.unwrap();

        assert_eq!(ctx.user_id, "user456");
        assert!(matches!(ctx.auth_type, AuthType::ApiKey(_)));
        assert!(ctx
            .permissions
            .contains(&crate::models::Permission::WorkflowRead));
        assert!(ctx
            .permissions
            .contains(&crate::models::Permission::WorkflowExecute));
    }

    #[tokio::test]
    async fn test_authenticate_missing_credentials() {
        let middleware = create_test_middleware().await;

        let result = middleware.authenticate(None).await;
        assert!(matches!(result, Err(AuthError::MissingCredentials)));
    }

    #[tokio::test]
    async fn test_authenticate_invalid_format() {
        let middleware = create_test_middleware().await;

        let result = middleware.authenticate(Some("InvalidFormat token")).await;
        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }

    #[tokio::test]
    async fn test_authenticate_invalid_jwt() {
        let middleware = create_test_middleware().await;

        let result = middleware
            .authenticate(Some("Bearer invalid.jwt.token"))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_authenticate_invalid_api_key() {
        let middleware = create_test_middleware().await;

        let result = middleware
            .authenticate(Some("ApiKey invalid_key"))
            .await;
        assert!(matches!(result, Err(AuthError::ApiKeyNotFound)));
    }

    #[tokio::test]
    async fn test_authorize_success() {
        let middleware = create_test_middleware().await;

        let token = middleware
            .jwt_auth
            .generate_token("user123", vec!["developer".to_string()])
            .unwrap();

        let auth_header = format!("Bearer {}", token);
        let ctx = middleware.authenticate(Some(&auth_header)).await.unwrap();

        let result = middleware.authorize(&ctx, &crate::models::Permission::WorkflowWrite);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_authorize_failure() {
        let middleware = create_test_middleware().await;

        let token = middleware
            .jwt_auth
            .generate_token("user123", vec!["viewer".to_string()])
            .unwrap();

        let auth_header = format!("Bearer {}", token);
        let ctx = middleware.authenticate(Some(&auth_header)).await.unwrap();

        let result = middleware.authorize(&ctx, &crate::models::Permission::WorkflowWrite);
        assert!(matches!(
            result,
            Err(AuthError::InsufficientPermissions { .. })
        ));
    }

    #[test]
    fn test_extract_bearer_token() {
        let token = AuthMiddleware::extract_bearer_token(Some("Bearer abc123"));
        assert_eq!(token, Some("abc123".to_string()));

        let none = AuthMiddleware::extract_bearer_token(Some("ApiKey abc123"));
        assert_eq!(none, None);

        let none = AuthMiddleware::extract_bearer_token(None);
        assert_eq!(none, None);
    }

    #[test]
    fn test_extract_api_key() {
        let key = AuthMiddleware::extract_api_key(Some("ApiKey abc123"));
        assert_eq!(key, Some("abc123".to_string()));

        let none = AuthMiddleware::extract_api_key(Some("Bearer abc123"));
        assert_eq!(none, None);

        let none = AuthMiddleware::extract_api_key(None);
        assert_eq!(none, None);
    }

    #[tokio::test]
    async fn test_scopes_to_permissions() {
        let middleware = create_test_middleware().await;

        let scopes = vec![
            "workflow:read".to_string(),
            "workflow:write".to_string(),
            "workflow:execute".to_string(),
        ];

        let permissions = middleware.scopes_to_permissions(&scopes);

        assert_eq!(permissions.len(), 3);
        assert!(permissions.contains(&crate::models::Permission::WorkflowRead));
        assert!(permissions.contains(&crate::models::Permission::WorkflowWrite));
        assert!(permissions.contains(&crate::models::Permission::WorkflowExecute));
    }

    #[tokio::test]
    async fn test_scopes_to_roles() {
        let middleware = create_test_middleware().await;

        // Developer role
        let scopes = vec![
            "workflow:read".to_string(),
            "workflow:write".to_string(),
            "workflow:execute".to_string(),
        ];
        let roles = middleware.scopes_to_roles(&scopes);
        assert_eq!(roles, vec!["developer"]);

        // Executor role
        let scopes = vec!["workflow:read".to_string(), "workflow:execute".to_string()];
        let roles = middleware.scopes_to_roles(&scopes);
        assert_eq!(roles, vec!["executor"]);

        // Viewer role
        let scopes = vec!["workflow:read".to_string()];
        let roles = middleware.scopes_to_roles(&scopes);
        assert_eq!(roles, vec!["viewer"]);

        // Admin role
        let scopes = vec!["admin".to_string()];
        let roles = middleware.scopes_to_roles(&scopes);
        assert_eq!(roles, vec!["admin"]);
    }
}
