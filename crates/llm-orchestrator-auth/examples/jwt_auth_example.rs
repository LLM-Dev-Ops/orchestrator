use llm_orchestrator_auth::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== JWT Authentication Example ===\n");

    // Create JWT auth with a secret key
    let jwt_auth = JwtAuth::new(b"example-secret-key-at-least-32-bytes-long".to_vec());

    // Example 1: Generate and verify access token
    println!("1. Generating access token...");
    let token = jwt_auth.generate_token("alice", vec!["developer".to_string()])?;
    println!("   Token: {}...{}", &token[..20], &token[token.len()-20..]);

    println!("\n2. Verifying token...");
    let claims = jwt_auth.verify_token(&token)?;
    println!("   User ID: {}", claims.sub);
    println!("   Roles: {:?}", claims.roles);
    println!("   Issuer: {}", claims.iss);
    println!("   Expires at: {}", claims.exp);

    // Example 2: Refresh token flow
    println!("\n3. Generating refresh token...");
    let refresh_token = jwt_auth.generate_refresh_token("alice")?;
    println!("   Refresh Token: {}...{}", &refresh_token[..20], &refresh_token[refresh_token.len()-20..]);

    println!("\n4. Using refresh token to get new access token...");
    let new_access_token = jwt_auth.refresh_access_token(
        &refresh_token,
        vec!["developer".to_string(), "admin".to_string()],
    )?;
    println!("   New Access Token: {}...{}", &new_access_token[..20], &new_access_token[new_access_token.len()-20..]);

    let new_claims = jwt_auth.verify_token(&new_access_token)?;
    println!("   Updated roles: {:?}", new_claims.roles);

    // Example 3: Custom JWT configuration
    println!("\n5. Creating JWT auth with custom settings...");
    let custom_jwt_auth = JwtAuth::builder(b"custom-secret-key-at-least-32-bytes-long".to_vec())
        .issuer("my-application".to_string())
        .expiry_seconds(3600) // 1 hour
        .refresh_expiry_seconds(2592000) // 30 days
        .build();

    let custom_token = custom_jwt_auth.generate_token("bob", vec!["executor".to_string()])?;
    let custom_claims = custom_jwt_auth.verify_token(&custom_token)?;
    println!("   Custom issuer: {}", custom_claims.iss);
    println!("   Token duration: {} seconds", custom_claims.exp - custom_claims.iat);

    // Example 4: Handling invalid tokens
    println!("\n6. Testing error handling...");
    match jwt_auth.verify_token("invalid.token.here") {
        Ok(_) => println!("   Unexpected success!"),
        Err(e) => println!("   Expected error: {}", e),
    }

    println!("\n=== Example Complete ===");
    Ok(())
}
