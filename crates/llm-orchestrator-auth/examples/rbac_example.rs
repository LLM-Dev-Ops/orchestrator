use llm_orchestrator_auth::*;

fn main() {
    println!("=== RBAC (Role-Based Access Control) Example ===\n");

    // Create RBAC engine with default roles
    let rbac = RbacEngine::new();

    // Example 1: List default roles
    println!("1. Default roles:");
    for role in rbac.list_roles() {
        if let Some(policy) = rbac.get_role(&role) {
            println!("   - {}: {} permissions", role, policy.permissions.len());
            if let Some(desc) = &policy.description {
                println!("     Description: {}", desc);
            }
        }
    }

    // Example 2: Check permissions for each role
    println!("\n2. Permission checks:");

    // Viewer
    println!("   Viewer:");
    println!("     - Can read workflows? {}",
        rbac.check_permission(&["viewer".to_string()], &Permission::WorkflowRead));
    println!("     - Can write workflows? {}",
        rbac.check_permission(&["viewer".to_string()], &Permission::WorkflowWrite));

    // Executor
    println!("   Executor:");
    println!("     - Can execute workflows? {}",
        rbac.check_permission(&["executor".to_string()], &Permission::WorkflowExecute));
    println!("     - Can delete workflows? {}",
        rbac.check_permission(&["executor".to_string()], &Permission::WorkflowDelete));

    // Developer
    println!("   Developer:");
    println!("     - Can write workflows? {}",
        rbac.check_permission(&["developer".to_string()], &Permission::WorkflowWrite));
    println!("     - Can cancel executions? {}",
        rbac.check_permission(&["developer".to_string()], &Permission::ExecutionCancel));

    // Admin
    println!("   Admin:");
    println!("     - Has admin access? {}",
        rbac.check_permission(&["admin".to_string()], &Permission::AdminAccess));
    println!("     - Can do everything? {}",
        rbac.check_all_permissions(
            &["admin".to_string()],
            &Permission::all()
        ));

    // Example 3: Multiple roles
    println!("\n3. Multiple roles:");
    let roles = vec!["viewer".to_string(), "executor".to_string()];
    let permissions = rbac.compute_permissions(&roles);
    println!("   Roles: {:?}", roles);
    println!("   Total permissions: {}", permissions.len());
    println!("   Can read? {}", permissions.contains(&Permission::WorkflowRead));
    println!("   Can execute? {}", permissions.contains(&Permission::WorkflowExecute));
    println!("   Can write? {}", permissions.contains(&Permission::WorkflowWrite));

    // Example 4: Create custom role
    println!("\n4. Creating custom role 'data_scientist':");
    rbac.add_role(
        "data_scientist",
        vec![
            Permission::WorkflowRead,
            Permission::WorkflowExecute,
            Permission::ExecutionRead,
        ],
        Some("Role for data science team members".to_string()),
    );

    if let Some(policy) = rbac.get_role("data_scientist") {
        println!("   Created role: {}", policy.role);
        println!("   Permissions: {}", policy.permissions.len());
        println!("   Description: {}", policy.description.as_ref().unwrap());
    }

    // Example 5: Permission combinations
    println!("\n5. Permission combinations:");
    let read_and_execute = vec![Permission::WorkflowRead, Permission::WorkflowExecute];

    println!("   Checking if roles have BOTH read AND execute:");
    println!("     - viewer: {}", rbac.check_all_permissions(&["viewer".to_string()], &read_and_execute));
    println!("     - executor: {}", rbac.check_all_permissions(&["executor".to_string()], &read_and_execute));
    println!("     - data_scientist: {}", rbac.check_all_permissions(&["data_scientist".to_string()], &read_and_execute));

    let write_or_delete = vec![Permission::WorkflowWrite, Permission::WorkflowDelete];

    println!("   Checking if roles have EITHER write OR delete:");
    println!("     - viewer: {}", rbac.check_any_permission(&["viewer".to_string()], &write_or_delete));
    println!("     - developer: {}", rbac.check_any_permission(&["developer".to_string()], &write_or_delete));
    println!("     - admin: {}", rbac.check_any_permission(&["admin".to_string()], &write_or_delete));

    // Example 6: Role validation
    println!("\n6. Role validation:");
    let valid_roles = vec!["viewer".to_string(), "executor".to_string()];
    let invalid_roles = vec!["viewer".to_string(), "nonexistent".to_string()];

    match rbac.validate_roles(&valid_roles) {
        Ok(_) => println!("   {:?} - Valid ✓", valid_roles),
        Err(e) => println!("   {:?} - Invalid: {}", valid_roles, e),
    }

    match rbac.validate_roles(&invalid_roles) {
        Ok(_) => println!("   {:?} - Valid ✓", invalid_roles),
        Err(e) => println!("   {:?} - Invalid: {}", invalid_roles, e),
    }

    // Example 7: Remove custom role
    println!("\n7. Removing custom role:");
    match rbac.remove_role("data_scientist") {
        Ok(_) => println!("   Removed 'data_scientist' role successfully"),
        Err(e) => println!("   Error: {}", e),
    }

    println!("   Remaining roles: {:?}", rbac.list_roles());

    println!("\n=== Example Complete ===");
}
