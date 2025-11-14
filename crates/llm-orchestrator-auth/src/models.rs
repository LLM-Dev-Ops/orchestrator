use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Authentication context for an authenticated user/API key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    /// Unique user identifier
    pub user_id: String,

    /// User roles
    pub roles: Vec<String>,

    /// Computed permissions from roles
    pub permissions: Vec<Permission>,

    /// Type of authentication used
    pub auth_type: AuthType,

    /// When this authentication expires
    pub expires_at: DateTime<Utc>,
}

impl AuthContext {
    /// Check if the context has a specific permission
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    /// Check if the context is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Require a specific permission, returning an error if not present
    pub fn require_permission(&self, permission: &Permission) -> Result<(), AuthError> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            Err(AuthError::InsufficientPermissions {
                required: permission.clone(),
                available: self.permissions.clone(),
            })
        }
    }
}

/// Authentication type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthType {
    /// JWT token authentication
    Jwt(String),

    /// API key authentication
    ApiKey(String),

    /// No authentication (for public endpoints)
    None,
}

/// Available permissions in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Permission {
    /// Read workflow definitions
    WorkflowRead,

    /// Write/update workflow definitions
    WorkflowWrite,

    /// Execute workflows
    WorkflowExecute,

    /// Delete workflows
    WorkflowDelete,

    /// Administrative access (all permissions)
    AdminAccess,

    /// Read execution history
    ExecutionRead,

    /// Cancel running executions
    ExecutionCancel,
}

impl Permission {
    /// Get all permissions
    pub fn all() -> Vec<Permission> {
        vec![
            Permission::WorkflowRead,
            Permission::WorkflowWrite,
            Permission::WorkflowExecute,
            Permission::WorkflowDelete,
            Permission::AdminAccess,
            Permission::ExecutionRead,
            Permission::ExecutionCancel,
        ]
    }

    /// Get permissions for a predefined role
    pub fn for_role(role: &str) -> Vec<Permission> {
        match role {
            "viewer" => vec![Permission::WorkflowRead, Permission::ExecutionRead],
            "executor" => vec![
                Permission::WorkflowRead,
                Permission::WorkflowExecute,
                Permission::ExecutionRead,
            ],
            "developer" => vec![
                Permission::WorkflowRead,
                Permission::WorkflowWrite,
                Permission::WorkflowExecute,
                Permission::ExecutionRead,
                Permission::ExecutionCancel,
            ],
            "admin" => Permission::all(),
            _ => vec![],
        }
    }
}

/// API key information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Unique key identifier
    pub id: String,

    /// Raw API key (only shown once at creation)
    pub key: String,

    /// SHA-256 hash of the key for storage
    pub key_hash: String,

    /// User who owns this key
    pub user_id: String,

    /// Scopes/permissions for this key
    pub scopes: Vec<String>,

    /// When the key was created
    pub created_at: DateTime<Utc>,

    /// Optional expiration time
    pub expires_at: Option<DateTime<Utc>>,

    /// Optional key name/description
    pub name: Option<String>,
}

/// API key information (without the raw key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    /// Unique key identifier
    pub id: String,

    /// SHA-256 hash of the key
    pub key_hash: String,

    /// User who owns this key
    pub user_id: String,

    /// Scopes/permissions for this key
    pub scopes: Vec<String>,

    /// When the key was created
    pub created_at: DateTime<Utc>,

    /// Optional expiration time
    pub expires_at: Option<DateTime<Utc>>,

    /// Optional key name/description
    pub name: Option<String>,

    /// Last time this key was used
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Role policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolePolicy {
    /// Role name
    pub role: String,

    /// Permissions granted by this role
    pub permissions: Vec<Permission>,

    /// Optional description
    pub description: Option<String>,
}

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,

    /// User roles
    pub roles: Vec<String>,

    /// Expiry timestamp (seconds since epoch)
    pub exp: u64,

    /// Issued at timestamp (seconds since epoch)
    pub iat: u64,

    /// Issuer
    pub iss: String,

    /// Optional token ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,
}

/// Authentication errors
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Missing credentials")]
    MissingCredentials,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("API key not found")]
    ApiKeyNotFound,

    #[error("API key expired")]
    ApiKeyExpired,

    #[error("Insufficient permissions: required {required:?}, available {available:?}")]
    InsufficientPermissions {
        required: Permission,
        available: Vec<Permission>,
    },

    #[error("Role not found: {0}")]
    RoleNotFound(String),

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("JWT error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type AuthResult<T> = Result<T, AuthError>;
