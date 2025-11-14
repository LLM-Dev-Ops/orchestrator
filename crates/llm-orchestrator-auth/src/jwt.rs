use crate::models::{AuthError, AuthResult, Claims};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT authentication manager
#[derive(Clone)]
pub struct JwtAuth {
    /// Secret key for signing tokens
    secret: Vec<u8>,

    /// Token issuer identifier
    issuer: String,

    /// Access token expiry duration in seconds (default: 900 = 15 minutes)
    expiry_seconds: i64,

    /// Refresh token expiry duration in seconds (default: 604800 = 7 days)
    refresh_expiry_seconds: i64,

    /// Algorithm to use for JWT signing
    algorithm: Algorithm,
}

impl JwtAuth {
    /// Create a new JWT authentication manager with default settings
    ///
    /// # Arguments
    /// * `secret` - Secret key for signing tokens
    ///
    /// # Returns
    /// A new JwtAuth instance with:
    /// - Issuer: "llm-orchestrator"
    /// - Access token expiry: 15 minutes
    /// - Refresh token expiry: 7 days
    /// - Algorithm: HS256
    pub fn new(secret: Vec<u8>) -> Self {
        Self {
            secret,
            issuer: "llm-orchestrator".to_string(),
            expiry_seconds: 900, // 15 minutes
            refresh_expiry_seconds: 604800, // 7 days
            algorithm: Algorithm::HS256,
        }
    }

    /// Create a JWT auth manager with custom settings
    pub fn builder(secret: Vec<u8>) -> JwtAuthBuilder {
        JwtAuthBuilder {
            secret,
            issuer: "llm-orchestrator".to_string(),
            expiry_seconds: 900,
            refresh_expiry_seconds: 604800,
            algorithm: Algorithm::HS256,
        }
    }

    /// Generate an access token for a user
    ///
    /// # Arguments
    /// * `user_id` - Unique user identifier
    /// * `roles` - List of roles assigned to the user
    ///
    /// # Returns
    /// A signed JWT token string
    pub fn generate_token(&self, user_id: &str, roles: Vec<String>) -> AuthResult<String> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.expiry_seconds);

        let claims = Claims {
            sub: user_id.to_string(),
            roles,
            exp: exp.timestamp() as u64,
            iat: now.timestamp() as u64,
            iss: self.issuer.clone(),
            jti: Some(Uuid::new_v4().to_string()),
        };

        let header = Header::new(self.algorithm);
        let token = encode(&header, &claims, &EncodingKey::from_secret(&self.secret))
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        Ok(token)
    }

    /// Generate a refresh token for a user
    ///
    /// Refresh tokens have a longer expiry and minimal claims
    pub fn generate_refresh_token(&self, user_id: &str) -> AuthResult<String> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.refresh_expiry_seconds);

        let claims = RefreshClaims {
            sub: user_id.to_string(),
            exp: exp.timestamp() as u64,
            iat: now.timestamp() as u64,
            iss: self.issuer.clone(),
            jti: Uuid::new_v4().to_string(),
            token_type: "refresh".to_string(),
        };

        let header = Header::new(self.algorithm);
        let token = encode(&header, &claims, &EncodingKey::from_secret(&self.secret))
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        Ok(token)
    }

    /// Verify and decode a JWT token
    ///
    /// # Arguments
    /// * `token` - The JWT token to verify
    ///
    /// # Returns
    /// The decoded claims if valid, or an error
    pub fn verify_token(&self, token: &str) -> AuthResult<Claims> {
        let mut validation = Validation::new(self.algorithm);
        validation.set_issuer(&[&self.issuer]);

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(&self.secret),
            &validation,
        )?;

        // Check if token is expired
        let now = Utc::now().timestamp() as u64;
        if token_data.claims.exp < now {
            return Err(AuthError::TokenExpired);
        }

        Ok(token_data.claims)
    }

    /// Verify a refresh token
    pub fn verify_refresh_token(&self, token: &str) -> AuthResult<String> {
        let mut validation = Validation::new(self.algorithm);
        validation.set_issuer(&[&self.issuer]);

        let token_data = decode::<RefreshClaims>(
            token,
            &DecodingKey::from_secret(&self.secret),
            &validation,
        )?;

        // Check if token is expired
        let now = Utc::now().timestamp() as u64;
        if token_data.claims.exp < now {
            return Err(AuthError::TokenExpired);
        }

        // Verify it's a refresh token
        if token_data.claims.token_type != "refresh" {
            return Err(AuthError::InvalidToken(
                "Not a refresh token".to_string(),
            ));
        }

        Ok(token_data.claims.sub)
    }

    /// Refresh an access token using a refresh token
    ///
    /// # Arguments
    /// * `refresh_token` - The refresh token
    /// * `roles` - The user's current roles (may have changed since refresh token was issued)
    ///
    /// # Returns
    /// A new access token
    pub fn refresh_access_token(
        &self,
        refresh_token: &str,
        roles: Vec<String>,
    ) -> AuthResult<String> {
        let user_id = self.verify_refresh_token(refresh_token)?;
        self.generate_token(&user_id, roles)
    }
}

/// Builder for JwtAuth
pub struct JwtAuthBuilder {
    secret: Vec<u8>,
    issuer: String,
    expiry_seconds: i64,
    refresh_expiry_seconds: i64,
    algorithm: Algorithm,
}

impl JwtAuthBuilder {
    /// Set the token issuer
    pub fn issuer(mut self, issuer: String) -> Self {
        self.issuer = issuer;
        self
    }

    /// Set the access token expiry in seconds
    pub fn expiry_seconds(mut self, seconds: i64) -> Self {
        self.expiry_seconds = seconds;
        self
    }

    /// Set the refresh token expiry in seconds
    pub fn refresh_expiry_seconds(mut self, seconds: i64) -> Self {
        self.refresh_expiry_seconds = seconds;
        self
    }

    /// Set the signing algorithm
    pub fn algorithm(mut self, algorithm: Algorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// Build the JwtAuth instance
    pub fn build(self) -> JwtAuth {
        JwtAuth {
            secret: self.secret,
            issuer: self.issuer,
            expiry_seconds: self.expiry_seconds,
            refresh_expiry_seconds: self.refresh_expiry_seconds,
            algorithm: self.algorithm,
        }
    }
}

/// Refresh token claims
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RefreshClaims {
    /// Subject (user ID)
    sub: String,

    /// Expiry timestamp
    exp: u64,

    /// Issued at timestamp
    iat: u64,

    /// Issuer
    iss: String,

    /// Token ID
    jti: String,

    /// Token type (always "refresh")
    token_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_jwt_auth() -> JwtAuth {
        JwtAuth::new(b"test-secret-key-at-least-32-bytes-long".to_vec())
    }

    #[test]
    fn test_generate_and_verify_token() {
        let jwt_auth = create_test_jwt_auth();
        let token = jwt_auth
            .generate_token("user123", vec!["admin".to_string()])
            .unwrap();

        let claims = jwt_auth.verify_token(&token).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.roles, vec!["admin"]);
        assert_eq!(claims.iss, "llm-orchestrator");
    }

    #[test]
    fn test_generate_and_verify_refresh_token() {
        let jwt_auth = create_test_jwt_auth();
        let token = jwt_auth.generate_refresh_token("user123").unwrap();

        let user_id = jwt_auth.verify_refresh_token(&token).unwrap();
        assert_eq!(user_id, "user123");
    }

    #[test]
    fn test_invalid_token() {
        let jwt_auth = create_test_jwt_auth();
        let result = jwt_auth.verify_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_token_with_different_secret() {
        let jwt_auth1 = JwtAuth::new(b"secret1-at-least-32-bytes-long-abc".to_vec());
        let jwt_auth2 = JwtAuth::new(b"secret2-at-least-32-bytes-long-xyz".to_vec());

        let token = jwt_auth1
            .generate_token("user123", vec!["admin".to_string()])
            .unwrap();

        let result = jwt_auth2.verify_token(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_refresh_access_token() {
        let jwt_auth = create_test_jwt_auth();
        let refresh_token = jwt_auth.generate_refresh_token("user123").unwrap();

        let access_token = jwt_auth
            .refresh_access_token(&refresh_token, vec!["developer".to_string()])
            .unwrap();

        let claims = jwt_auth.verify_token(&access_token).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.roles, vec!["developer"]);
    }

    #[test]
    fn test_builder_custom_settings() {
        let jwt_auth = JwtAuth::builder(b"test-secret-key-at-least-32-bytes-long".to_vec())
            .issuer("custom-issuer".to_string())
            .expiry_seconds(3600)
            .build();

        let token = jwt_auth
            .generate_token("user123", vec!["admin".to_string()])
            .unwrap();

        let claims = jwt_auth.verify_token(&token).unwrap();
        assert_eq!(claims.iss, "custom-issuer");
    }

    #[test]
    fn test_token_has_jti() {
        let jwt_auth = create_test_jwt_auth();
        let token = jwt_auth
            .generate_token("user123", vec!["admin".to_string()])
            .unwrap();

        let claims = jwt_auth.verify_token(&token).unwrap();
        assert!(claims.jti.is_some());
    }
}
