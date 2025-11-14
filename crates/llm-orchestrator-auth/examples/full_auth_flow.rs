use llm_orchestrator_auth::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Complete Authentication Flow Example ===\n");

    // Setup: Create all authentication components
    println!("Setting up authentication system...");

    let jwt_auth = Arc::new(JwtAuth::builder(
        b"production-secret-key-should-be-from-env".to_vec()
    )
    .issuer("llm-orchestrator-example".to_string())
    .expiry_seconds(900) // 15 minutes
    .build());

    let api_key_store = Arc::new(InMemoryApiKeyStore::new());
    let api_key_manager = Arc::new(ApiKeyManager::new(api_key_store));

    let rbac = Arc::new(RbacEngine::new());

    let auth_middleware = AuthMiddleware::new(
        jwt_auth.clone(),
        api_key_manager.clone(),
        rbac.clone(),
    );

    println!("✓ Authentication system initialized\n");

    // Scenario 1: User authentication with JWT
    println!("=== Scenario 1: JWT Authentication ===");

    println!("1. User 'alice' logs in with developer role");
    let alice_token = jwt_auth.generate_token(
        "alice",
        vec!["developer".to_string()],
    )?;
    println!("   Token generated: {}...", &alice_token[..30]);

    println!("\n2. Authenticating request with JWT");
    let auth_header = format!("Bearer {}", alice_token);
    let alice_ctx = auth_middleware.authenticate(Some(&auth_header)).await?;
    println!("   ✓ Authenticated as: {}", alice_ctx.user_id);
    println!("   Roles: {:?}", alice_ctx.roles);
    println!("   Permissions: {} total", alice_ctx.permissions.len());

    println!("\n3. Checking permissions for workflow operations");

    // Try to read workflow
    match auth_middleware.authorize(&alice_ctx, &Permission::WorkflowRead) {
        Ok(_) => println!("   ✓ Can read workflows"),
        Err(e) => println!("   ✗ Cannot read workflows: {}", e),
    }

    // Try to write workflow
    match auth_middleware.authorize(&alice_ctx, &Permission::WorkflowWrite) {
        Ok(_) => println!("   ✓ Can write workflows"),
        Err(e) => println!("   ✗ Cannot write workflows: {}", e),
    }

    // Try to delete workflow (developer doesn't have this permission)
    match auth_middleware.authorize(&alice_ctx, &Permission::WorkflowDelete) {
        Ok(_) => println!("   ✓ Can delete workflows"),
        Err(e) => println!("   ✗ Cannot delete workflows: {}", e),
    }

    // Scenario 2: API Key authentication
    println!("\n=== Scenario 2: API Key Authentication ===");

    println!("1. Creating API key for 'bob' with limited scopes");
    let bob_api_key = api_key_manager.create_key(
        "bob",
        vec![
            "workflow:read".to_string(),
            "workflow:execute".to_string(),
        ],
        Some("Bob's CI/CD Key".to_string()),
        Some(90), // 90 days
    ).await?;
    println!("   ✓ API Key created: {}", bob_api_key.key);

    println!("\n2. Authenticating request with API key");
    let api_auth_header = format!("ApiKey {}", bob_api_key.key);
    let bob_ctx = auth_middleware.authenticate(Some(&api_auth_header)).await?;
    println!("   ✓ Authenticated as: {}", bob_ctx.user_id);
    println!("   Permissions: {:?}", bob_ctx.permissions);

    println!("\n3. Checking API key permissions");

    // Try to execute workflow (has permission)
    match auth_middleware.authorize(&bob_ctx, &Permission::WorkflowExecute) {
        Ok(_) => println!("   ✓ Can execute workflows"),
        Err(e) => println!("   ✗ Cannot execute workflows: {}", e),
    }

    // Try to write workflow (doesn't have permission)
    match auth_middleware.authorize(&bob_ctx, &Permission::WorkflowWrite) {
        Ok(_) => println!("   ✓ Can write workflows"),
        Err(e) => println!("   ✗ Cannot write workflows: {}", e),
    }

    // Scenario 3: Admin access
    println!("\n=== Scenario 3: Admin Access ===");

    println!("1. Creating admin token for 'charlie'");
    let charlie_token = jwt_auth.generate_token(
        "charlie",
        vec!["admin".to_string()],
    )?;

    println!("\n2. Authenticating admin request");
    let admin_auth_header = format!("Bearer {}", charlie_token);
    let charlie_ctx = auth_middleware.authenticate(Some(&admin_auth_header)).await?;
    println!("   ✓ Authenticated as admin: {}", charlie_ctx.user_id);

    println!("\n3. Admin can do everything:");
    for permission in [Permission::WorkflowRead,
        Permission::WorkflowWrite,
        Permission::WorkflowExecute,
        Permission::WorkflowDelete,
        Permission::ExecutionCancel] {
        match auth_middleware.authorize(&charlie_ctx, &permission) {
            Ok(_) => println!("   ✓ {:?}", permission),
            Err(_) => println!("   ✗ {:?}", permission),
        }
    }

    // Scenario 4: Token refresh
    println!("\n=== Scenario 4: Token Refresh ===");

    println!("1. Generating refresh token for 'alice'");
    let refresh_token = jwt_auth.generate_refresh_token("alice")?;
    println!("   ✓ Refresh token generated");

    println!("\n2. Using refresh token to get new access token");
    let new_access_token = jwt_auth.refresh_access_token(
        &refresh_token,
        vec!["developer".to_string(), "admin".to_string()], // Promoted!
    )?;
    println!("   ✓ New access token generated");

    println!("\n3. Verifying new token has updated roles");
    let new_claims = jwt_auth.verify_token(&new_access_token)?;
    println!("   User: {}", new_claims.sub);
    println!("   Updated roles: {:?}", new_claims.roles);

    // Scenario 5: Error handling
    println!("\n=== Scenario 5: Error Handling ===");

    println!("1. Testing invalid JWT token");
    match auth_middleware.authenticate(Some("Bearer invalid.token")).await {
        Ok(_) => println!("   ERROR: Should have failed!"),
        Err(e) => println!("   ✓ Expected error: {}", e),
    }

    println!("\n2. Testing invalid API key");
    match auth_middleware.authenticate(Some("ApiKey invalid_key")).await {
        Ok(_) => println!("   ERROR: Should have failed!"),
        Err(e) => println!("   ✓ Expected error: {}", e),
    }

    println!("\n3. Testing missing credentials");
    match auth_middleware.authenticate(None).await {
        Ok(_) => println!("   ERROR: Should have failed!"),
        Err(e) => println!("   ✓ Expected error: {}", e),
    }

    println!("\n4. Revoking Bob's API key");
    api_key_manager.revoke_key(&bob_api_key.id).await?;
    println!("   ✓ Key revoked");

    println!("\n5. Attempting to use revoked key");
    match auth_middleware.authenticate(Some(&api_auth_header)).await {
        Ok(_) => println!("   ERROR: Should have failed!"),
        Err(e) => println!("   ✓ Expected error: {}", e),
    }

    // Summary
    println!("\n=== Summary ===");
    println!("✓ JWT authentication working");
    println!("✓ API key authentication working");
    println!("✓ RBAC permission checks working");
    println!("✓ Token refresh working");
    println!("✓ Error handling working");
    println!("\n=== Example Complete ===");

    Ok(())
}
