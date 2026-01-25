// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Execution Role Guards for Phase 3 Layer 1.
//!
//! Agents in this phase MUST:
//! - Coordinate
//! - Route
//! - Optimize
//! - Escalate
//!
//! Agents MUST NOT:
//! - Make final decisions
//! - Emit executive conclusions

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

/// Execution roles permitted in Phase 3 Layer 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionRole {
    /// Coordinate tasks and workflows.
    Coordinate,
    /// Route requests to appropriate handlers.
    Route,
    /// Optimize execution strategies.
    Optimize,
    /// Escalate issues and incidents.
    Escalate,
}

impl std::fmt::Display for ExecutionRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Coordinate => write!(f, "coordinate"),
            Self::Route => write!(f, "route"),
            Self::Optimize => write!(f, "optimize"),
            Self::Escalate => write!(f, "escalate"),
        }
    }
}

/// Prohibited actions in Phase 3 Layer 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProhibitedAction {
    /// Making final decisions.
    FinalDecision,
    /// Emitting executive conclusions.
    ExecutiveConclusion,
    /// Direct execution without coordination.
    DirectExecution,
    /// Bypassing escalation requirements.
    BypassEscalation,
}

impl std::fmt::Display for ProhibitedAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FinalDecision => write!(f, "final_decision"),
            Self::ExecutiveConclusion => write!(f, "executive_conclusion"),
            Self::DirectExecution => write!(f, "direct_execution"),
            Self::BypassEscalation => write!(f, "bypass_escalation"),
        }
    }
}

/// Guard violation error.
#[derive(Debug, Error)]
pub enum GuardViolation {
    #[error("Prohibited action attempted: {action} - Agents MUST NOT make final decisions or emit executive conclusions")]
    ProhibitedAction { action: ProhibitedAction },

    #[error("Invalid execution role: {role} - Only coordinate, route, optimize, escalate permitted")]
    InvalidRole { role: String },

    #[error("Missing required role: {role} - Action requires {role} capability")]
    MissingRole { role: ExecutionRole },

    #[error("Execution guard check failed: {reason}")]
    CheckFailed { reason: String },
}

/// Execution guard for Phase 3 Layer 1 role enforcement.
#[derive(Debug, Clone)]
pub struct ExecutionGuard {
    /// Permitted roles for this agent.
    permitted_roles: HashSet<ExecutionRole>,
    /// Whether to enforce hard fail on violation.
    hard_fail: bool,
}

impl Default for ExecutionGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionGuard {
    /// Creates a new execution guard with all Phase 3 Layer 1 roles permitted.
    pub fn new() -> Self {
        let mut permitted_roles = HashSet::new();
        permitted_roles.insert(ExecutionRole::Coordinate);
        permitted_roles.insert(ExecutionRole::Route);
        permitted_roles.insert(ExecutionRole::Optimize);
        permitted_roles.insert(ExecutionRole::Escalate);

        Self {
            permitted_roles,
            hard_fail: true,
        }
    }

    /// Creates a guard with specific permitted roles.
    pub fn with_roles(roles: impl IntoIterator<Item = ExecutionRole>) -> Self {
        Self {
            permitted_roles: roles.into_iter().collect(),
            hard_fail: true,
        }
    }

    /// Sets whether to hard fail on violations.
    pub fn with_hard_fail(mut self, hard_fail: bool) -> Self {
        self.hard_fail = hard_fail;
        self
    }

    /// Checks if a role is permitted.
    pub fn is_role_permitted(&self, role: ExecutionRole) -> bool {
        self.permitted_roles.contains(&role)
    }

    /// Validates that an action is permitted.
    ///
    /// # Errors
    ///
    /// Returns `GuardViolation` if the action is prohibited.
    pub fn validate_action(&self, action_type: &str) -> Result<(), GuardViolation> {
        // Check for prohibited actions
        let prohibited = self.detect_prohibited_action(action_type);
        if let Some(action) = prohibited {
            let violation = GuardViolation::ProhibitedAction { action };
            if self.hard_fail {
                tracing::error!(
                    action = %action,
                    "HARD FAIL: Execution guard violation - prohibited action attempted"
                );
            }
            return Err(violation);
        }

        Ok(())
    }

    /// Validates that the required role is available.
    ///
    /// # Errors
    ///
    /// Returns `GuardViolation` if the required role is not permitted.
    pub fn require_role(&self, role: ExecutionRole) -> Result<(), GuardViolation> {
        if !self.permitted_roles.contains(&role) {
            let violation = GuardViolation::MissingRole { role };
            if self.hard_fail {
                tracing::error!(
                    role = %role,
                    "HARD FAIL: Execution guard violation - missing required role"
                );
            }
            return Err(violation);
        }
        Ok(())
    }

    /// Detects if an action type maps to a prohibited action.
    fn detect_prohibited_action(&self, action_type: &str) -> Option<ProhibitedAction> {
        let action_lower = action_type.to_lowercase();

        // Detect final decision attempts
        if action_lower.contains("final_decision")
            || action_lower.contains("finaldecision")
            || action_lower.contains("make_decision")
            || action_lower.contains("decide_final")
            || action_lower.contains("approve")
            || action_lower.contains("reject")
            || action_lower.contains("commit")
        {
            return Some(ProhibitedAction::FinalDecision);
        }

        // Detect executive conclusion attempts
        if action_lower.contains("executive_conclusion")
            || action_lower.contains("executiveconclusion")
            || action_lower.contains("emit_conclusion")
            || action_lower.contains("final_output")
            || action_lower.contains("conclude")
        {
            return Some(ProhibitedAction::ExecutiveConclusion);
        }

        // Detect direct execution bypass
        if action_lower.contains("direct_execute")
            || action_lower.contains("bypass_coordination")
            || action_lower.contains("skip_routing")
        {
            return Some(ProhibitedAction::DirectExecution);
        }

        // Detect escalation bypass
        if action_lower.contains("bypass_escalation")
            || action_lower.contains("skip_escalation")
            || action_lower.contains("suppress_incident")
        {
            return Some(ProhibitedAction::BypassEscalation);
        }

        None
    }

    /// Creates an action validator for a specific role.
    pub fn for_role(&self, role: ExecutionRole) -> Result<RoleValidator, GuardViolation> {
        self.require_role(role)?;
        Ok(RoleValidator { role, guard: self })
    }

    /// Returns a summary of permitted roles.
    pub fn permitted_roles_summary(&self) -> Vec<String> {
        self.permitted_roles.iter().map(|r| r.to_string()).collect()
    }
}

/// Role-specific validator for scoped action validation.
#[derive(Debug)]
pub struct RoleValidator<'a> {
    role: ExecutionRole,
    guard: &'a ExecutionGuard,
}

impl<'a> RoleValidator<'a> {
    /// Validates an action within this role's scope.
    pub fn validate(&self, action_type: &str) -> Result<(), GuardViolation> {
        self.guard.validate_action(action_type)
    }

    /// Returns the role being validated.
    pub fn role(&self) -> ExecutionRole {
        self.role
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_guard_has_all_roles() {
        let guard = ExecutionGuard::new();
        assert!(guard.is_role_permitted(ExecutionRole::Coordinate));
        assert!(guard.is_role_permitted(ExecutionRole::Route));
        assert!(guard.is_role_permitted(ExecutionRole::Optimize));
        assert!(guard.is_role_permitted(ExecutionRole::Escalate));
    }

    #[test]
    fn test_prohibited_action_detection() {
        let guard = ExecutionGuard::new();

        // Should fail on final decision
        assert!(guard.validate_action("final_decision").is_err());
        assert!(guard.validate_action("make_decision").is_err());
        assert!(guard.validate_action("approve").is_err());

        // Should fail on executive conclusion
        assert!(guard.validate_action("executive_conclusion").is_err());
        assert!(guard.validate_action("emit_conclusion").is_err());

        // Should pass on permitted actions
        assert!(guard.validate_action("coordinate_task").is_ok());
        assert!(guard.validate_action("route_request").is_ok());
        assert!(guard.validate_action("optimize_strategy").is_ok());
        assert!(guard.validate_action("escalate_incident").is_ok());
    }

    #[test]
    fn test_require_role() {
        let guard = ExecutionGuard::with_roles([ExecutionRole::Coordinate, ExecutionRole::Route]);

        assert!(guard.require_role(ExecutionRole::Coordinate).is_ok());
        assert!(guard.require_role(ExecutionRole::Route).is_ok());
        assert!(guard.require_role(ExecutionRole::Optimize).is_err());
        assert!(guard.require_role(ExecutionRole::Escalate).is_err());
    }

    #[test]
    fn test_role_validator() {
        let guard = ExecutionGuard::new();
        let validator = guard.for_role(ExecutionRole::Coordinate).unwrap();

        assert_eq!(validator.role(), ExecutionRole::Coordinate);
        assert!(validator.validate("coordinate_task").is_ok());
        assert!(validator.validate("final_decision").is_err());
    }
}
