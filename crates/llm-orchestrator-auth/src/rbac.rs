use crate::models::{AuthContext, AuthError, AuthResult, Permission, RolePolicy};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Role-Based Access Control engine
pub struct RbacEngine {
    /// Role policies mapping role names to permissions
    policies: Arc<RwLock<HashMap<String, RolePolicy>>>,
}

impl RbacEngine {
    /// Create a new RBAC engine with default roles
    pub fn new() -> Self {
        let mut policies = HashMap::new();

        // Add predefined roles
        policies.insert(
            "viewer".to_string(),
            RolePolicy {
                role: "viewer".to_string(),
                permissions: vec![Permission::WorkflowRead, Permission::ExecutionRead],
                description: Some("Read-only access to workflows and executions".to_string()),
            },
        );

        policies.insert(
            "executor".to_string(),
            RolePolicy {
                role: "executor".to_string(),
                permissions: vec![
                    Permission::WorkflowRead,
                    Permission::WorkflowExecute,
                    Permission::ExecutionRead,
                ],
                description: Some(
                    "Can read and execute workflows, view execution history".to_string(),
                ),
            },
        );

        policies.insert(
            "developer".to_string(),
            RolePolicy {
                role: "developer".to_string(),
                permissions: vec![
                    Permission::WorkflowRead,
                    Permission::WorkflowWrite,
                    Permission::WorkflowExecute,
                    Permission::ExecutionRead,
                    Permission::ExecutionCancel,
                ],
                description: Some(
                    "Full access to workflows and executions, can cancel running workflows"
                        .to_string(),
                ),
            },
        );

        policies.insert(
            "admin".to_string(),
            RolePolicy {
                role: "admin".to_string(),
                permissions: Permission::all(),
                description: Some("Full administrative access to all resources".to_string()),
            },
        );

        Self {
            policies: Arc::new(RwLock::new(policies)),
        }
    }

    /// Create an empty RBAC engine without default roles
    pub fn new_empty() -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add or update a role policy
    ///
    /// # Arguments
    /// * `role` - Role name
    /// * `permissions` - List of permissions for this role
    /// * `description` - Optional description
    pub fn add_role(
        &self,
        role: &str,
        permissions: Vec<Permission>,
        description: Option<String>,
    ) {
        let policy = RolePolicy {
            role: role.to_string(),
            permissions,
            description,
        };

        self.policies.write().insert(role.to_string(), policy);
    }

    /// Remove a role
    pub fn remove_role(&self, role: &str) -> AuthResult<()> {
        self.policies
            .write()
            .remove(role)
            .ok_or_else(|| AuthError::RoleNotFound(role.to_string()))?;
        Ok(())
    }

    /// Get a role policy
    pub fn get_role(&self, role: &str) -> Option<RolePolicy> {
        self.policies.read().get(role).cloned()
    }

    /// List all roles
    pub fn list_roles(&self) -> Vec<String> {
        self.policies.read().keys().cloned().collect()
    }

    /// Compute permissions for a list of roles
    ///
    /// # Arguments
    /// * `roles` - List of role names
    ///
    /// # Returns
    /// Union of all permissions from the roles
    pub fn compute_permissions(&self, roles: &[String]) -> Vec<Permission> {
        let policies = self.policies.read();
        let mut permissions: HashSet<Permission> = HashSet::new();

        for role in roles {
            if let Some(policy) = policies.get(role) {
                permissions.extend(policy.permissions.iter().cloned());
            }
        }

        // If user has AdminAccess, grant all permissions
        if permissions.contains(&Permission::AdminAccess) {
            return Permission::all();
        }

        permissions.into_iter().collect()
    }

    /// Check if a list of roles has a specific permission
    ///
    /// # Arguments
    /// * `roles` - List of role names
    /// * `permission` - Permission to check
    ///
    /// # Returns
    /// true if any of the roles grants the permission
    pub fn check_permission(&self, roles: &[String], permission: &Permission) -> bool {
        let policies = self.policies.read();

        for role in roles {
            if let Some(policy) = policies.get(role) {
                // Admin role has all permissions
                if policy.permissions.contains(&Permission::AdminAccess) {
                    return true;
                }

                if policy.permissions.contains(permission) {
                    return true;
                }
            }
        }

        false
    }

    /// Require a specific permission from an auth context
    ///
    /// # Arguments
    /// * `ctx` - Authentication context
    /// * `permission` - Required permission
    ///
    /// # Returns
    /// Ok if permission is granted, error otherwise
    pub fn require_permission(
        &self,
        ctx: &AuthContext,
        permission: &Permission,
    ) -> AuthResult<()> {
        if self.check_permission(&ctx.roles, permission) {
            Ok(())
        } else {
            Err(AuthError::InsufficientPermissions {
                required: permission.clone(),
                available: ctx.permissions.clone(),
            })
        }
    }

    /// Check if a list of roles has all of the specified permissions
    pub fn check_all_permissions(&self, roles: &[String], permissions: &[Permission]) -> bool {
        permissions
            .iter()
            .all(|perm| self.check_permission(roles, perm))
    }

    /// Check if a list of roles has any of the specified permissions
    pub fn check_any_permission(&self, roles: &[String], permissions: &[Permission]) -> bool {
        permissions
            .iter()
            .any(|perm| self.check_permission(roles, perm))
    }

    /// Validate that all roles exist
    pub fn validate_roles(&self, roles: &[String]) -> AuthResult<()> {
        let policies = self.policies.read();

        for role in roles {
            if !policies.contains_key(role) {
                return Err(AuthError::RoleNotFound(role.clone()));
            }
        }

        Ok(())
    }
}

impl Default for RbacEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_default_roles_exist() {
        let rbac = RbacEngine::new();

        assert!(rbac.get_role("viewer").is_some());
        assert!(rbac.get_role("executor").is_some());
        assert!(rbac.get_role("developer").is_some());
        assert!(rbac.get_role("admin").is_some());
    }

    #[test]
    fn test_viewer_permissions() {
        let rbac = RbacEngine::new();

        let permissions = rbac.compute_permissions(&["viewer".to_string()]);

        assert!(permissions.contains(&Permission::WorkflowRead));
        assert!(permissions.contains(&Permission::ExecutionRead));
        assert!(!permissions.contains(&Permission::WorkflowWrite));
        assert!(!permissions.contains(&Permission::WorkflowExecute));
    }

    #[test]
    fn test_executor_permissions() {
        let rbac = RbacEngine::new();

        let permissions = rbac.compute_permissions(&["executor".to_string()]);

        assert!(permissions.contains(&Permission::WorkflowRead));
        assert!(permissions.contains(&Permission::WorkflowExecute));
        assert!(permissions.contains(&Permission::ExecutionRead));
        assert!(!permissions.contains(&Permission::WorkflowWrite));
    }

    #[test]
    fn test_developer_permissions() {
        let rbac = RbacEngine::new();

        let permissions = rbac.compute_permissions(&["developer".to_string()]);

        assert!(permissions.contains(&Permission::WorkflowRead));
        assert!(permissions.contains(&Permission::WorkflowWrite));
        assert!(permissions.contains(&Permission::WorkflowExecute));
        assert!(permissions.contains(&Permission::ExecutionRead));
        assert!(permissions.contains(&Permission::ExecutionCancel));
    }

    #[test]
    fn test_admin_has_all_permissions() {
        let rbac = RbacEngine::new();

        let permissions = rbac.compute_permissions(&["admin".to_string()]);

        // Admin should have all permissions
        for permission in Permission::all() {
            assert!(permissions.contains(&permission));
        }
    }

    #[test]
    fn test_multiple_roles_union_permissions() {
        let rbac = RbacEngine::new();

        let permissions = rbac.compute_permissions(&["viewer".to_string(), "executor".to_string()]);

        // Should have union of both roles
        assert!(permissions.contains(&Permission::WorkflowRead));
        assert!(permissions.contains(&Permission::WorkflowExecute));
        assert!(permissions.contains(&Permission::ExecutionRead));
    }

    #[test]
    fn test_check_permission() {
        let rbac = RbacEngine::new();

        assert!(rbac.check_permission(&["viewer".to_string()], &Permission::WorkflowRead));
        assert!(!rbac.check_permission(&["viewer".to_string()], &Permission::WorkflowWrite));
        assert!(rbac.check_permission(&["developer".to_string()], &Permission::WorkflowWrite));
    }

    #[test]
    fn test_admin_has_all_permissions_check() {
        let rbac = RbacEngine::new();

        // Admin should pass all permission checks
        for permission in Permission::all() {
            assert!(rbac.check_permission(&["admin".to_string()], &permission));
        }
    }

    #[test]
    fn test_add_custom_role() {
        let rbac = RbacEngine::new();

        rbac.add_role(
            "custom_role",
            vec![Permission::WorkflowRead, Permission::WorkflowExecute],
            Some("Custom role for testing".to_string()),
        );

        let role = rbac.get_role("custom_role").unwrap();
        assert_eq!(role.permissions.len(), 2);
        assert_eq!(
            role.description,
            Some("Custom role for testing".to_string())
        );
    }

    #[test]
    fn test_remove_role() {
        let rbac = RbacEngine::new();

        rbac.add_role("temp_role", vec![Permission::WorkflowRead], None);
        assert!(rbac.get_role("temp_role").is_some());

        rbac.remove_role("temp_role").unwrap();
        assert!(rbac.get_role("temp_role").is_none());
    }

    #[test]
    fn test_remove_nonexistent_role() {
        let rbac = RbacEngine::new();

        let result = rbac.remove_role("nonexistent");
        assert!(matches!(result, Err(AuthError::RoleNotFound(_))));
    }

    #[test]
    fn test_list_roles() {
        let rbac = RbacEngine::new();

        let roles = rbac.list_roles();
        assert!(roles.contains(&"viewer".to_string()));
        assert!(roles.contains(&"executor".to_string()));
        assert!(roles.contains(&"developer".to_string()));
        assert!(roles.contains(&"admin".to_string()));
    }

    #[test]
    fn test_require_permission_success() {
        let rbac = RbacEngine::new();

        let ctx = AuthContext {
            user_id: "user123".to_string(),
            roles: vec!["developer".to_string()],
            permissions: rbac.compute_permissions(&["developer".to_string()]),
            auth_type: crate::models::AuthType::Jwt("token".to_string()),
            expires_at: Utc::now() + Duration::hours(1),
        };

        assert!(rbac
            .require_permission(&ctx, &Permission::WorkflowWrite)
            .is_ok());
    }

    #[test]
    fn test_require_permission_failure() {
        let rbac = RbacEngine::new();

        let ctx = AuthContext {
            user_id: "user123".to_string(),
            roles: vec!["viewer".to_string()],
            permissions: rbac.compute_permissions(&["viewer".to_string()]),
            auth_type: crate::models::AuthType::Jwt("token".to_string()),
            expires_at: Utc::now() + Duration::hours(1),
        };

        let result = rbac.require_permission(&ctx, &Permission::WorkflowWrite);
        assert!(matches!(
            result,
            Err(AuthError::InsufficientPermissions { .. })
        ));
    }

    #[test]
    fn test_check_all_permissions() {
        let rbac = RbacEngine::new();

        let permissions = vec![Permission::WorkflowRead, Permission::ExecutionRead];

        assert!(rbac.check_all_permissions(&["viewer".to_string()], &permissions));

        let permissions_with_write = vec![
            Permission::WorkflowRead,
            Permission::WorkflowWrite,
            Permission::ExecutionRead,
        ];

        assert!(!rbac.check_all_permissions(&["viewer".to_string()], &permissions_with_write));
        assert!(rbac.check_all_permissions(&["developer".to_string()], &permissions_with_write));
    }

    #[test]
    fn test_check_any_permission() {
        let rbac = RbacEngine::new();

        let permissions = vec![Permission::WorkflowWrite, Permission::WorkflowDelete];

        assert!(!rbac.check_any_permission(&["viewer".to_string()], &permissions));
        assert!(rbac.check_any_permission(&["developer".to_string()], &permissions));
    }

    #[test]
    fn test_validate_roles_success() {
        let rbac = RbacEngine::new();

        let result = rbac.validate_roles(&["viewer".to_string(), "executor".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_roles_failure() {
        let rbac = RbacEngine::new();

        let result = rbac.validate_roles(&["viewer".to_string(), "invalid_role".to_string()]);
        assert!(matches!(result, Err(AuthError::RoleNotFound(_))));
    }

    #[test]
    fn test_empty_roles_no_permissions() {
        let rbac = RbacEngine::new();

        let permissions = rbac.compute_permissions(&[]);
        assert!(permissions.is_empty());
    }
}
