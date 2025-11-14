use llm_orchestrator_auth::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== API Key Management Example ===\n");

    // Create API key manager with in-memory storage
    let store = Arc::new(InMemoryApiKeyStore::new());
    let manager = ApiKeyManager::new(store);

    // Example 1: Create an API key
    println!("1. Creating API key for user 'alice'...");
    let api_key = manager
        .create_key(
            "alice",
            vec![
                "workflow:read".to_string(),
                "workflow:execute".to_string(),
            ],
            Some("Production API Key".to_string()),
            Some(90), // Expires in 90 days
        )
        .await?;

    println!("   API Key: {}", api_key.key);
    println!("   Key ID: {}", api_key.id);
    println!("   Name: {}", api_key.name.as_ref().unwrap());
    println!("   Scopes: {:?}", api_key.scopes);
    println!("   Expires: {:?}", api_key.expires_at);

    // Example 2: Lookup and validate the key
    println!("\n2. Looking up API key...");
    let key_info = manager.lookup_key(&api_key.key).await?;
    println!("   Owner: {}", key_info.user_id);
    println!("   Scopes: {:?}", key_info.scopes);
    println!("   Last used: {:?}", key_info.last_used_at);

    // Example 3: Create multiple keys for the same user
    println!("\n3. Creating additional API keys...");
    let dev_key = manager
        .create_key(
            "alice",
            vec!["workflow:read".to_string(), "workflow:write".to_string()],
            Some("Development Key".to_string()),
            Some(30),
        )
        .await?;
    println!("   Created dev key: {}", dev_key.id);

    let admin_key = manager
        .create_key(
            "alice",
            vec!["admin".to_string()],
            Some("Admin Key".to_string()),
            None, // No expiration
        )
        .await?;
    println!("   Created admin key: {}", admin_key.id);

    // Example 4: List all keys for a user
    println!("\n4. Listing all keys for 'alice'...");
    let keys = manager.list_keys("alice").await?;
    println!("   Found {} keys:", keys.len());
    for (i, key) in keys.iter().enumerate() {
        println!(
            "     {}. {} - {:?} (scopes: {})",
            i + 1,
            key.name.as_ref().unwrap_or(&"Unnamed".to_string()),
            key.id,
            key.scopes.join(", ")
        );
    }

    // Example 5: Revoke a key
    println!("\n5. Revoking development key...");
    manager.revoke_key(&dev_key.id).await?;
    println!("   Key revoked successfully");

    // Verify key is revoked
    match manager.lookup_key(&dev_key.key).await {
        Ok(_) => println!("   ERROR: Key should be revoked!"),
        Err(AuthError::ApiKeyNotFound) => println!("   Confirmed: Key is no longer valid"),
        Err(e) => println!("   Unexpected error: {}", e),
    }

    // Example 6: Check remaining keys
    println!("\n6. Checking remaining keys...");
    let remaining_keys = manager.list_keys("alice").await?;
    println!("   Remaining keys: {}", remaining_keys.len());
    for key in remaining_keys {
        println!("     - {}", key.name.unwrap_or_else(|| "Unnamed".to_string()));
    }

    // Example 7: Create key for another user
    println!("\n7. Creating key for user 'bob'...");
    let bob_key = manager
        .create_key(
            "bob",
            vec!["workflow:read".to_string()],
            Some("Bob's Read-Only Key".to_string()),
            Some(365),
        )
        .await?;
    println!("   Created key: {}", bob_key.id);

    // Verify user isolation
    let alice_keys = manager.list_keys("alice").await?;
    let bob_keys = manager.list_keys("bob").await?;
    println!("   Alice has {} keys, Bob has {} keys", alice_keys.len(), bob_keys.len());

    println!("\n=== Example Complete ===");
    Ok(())
}
