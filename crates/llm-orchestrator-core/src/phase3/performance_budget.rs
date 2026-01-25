// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Performance Budget Enforcement for Phase 3 Layer 1.
//!
//! Performance Budgets:
//! - MAX_TOKENS=1500
//! - MAX_LATENCY_MS=3000
//! - MAX_CALLS_PER_RUN=4

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Default performance budget values.
pub const DEFAULT_MAX_TOKENS: u32 = 1500;
pub const DEFAULT_MAX_LATENCY_MS: u64 = 3000;
pub const DEFAULT_MAX_CALLS_PER_RUN: u32 = 4;

/// Budget violation error.
#[derive(Debug, Error)]
pub enum BudgetViolation {
    #[error("Token budget exceeded: used {used} tokens, max {max}")]
    TokenBudgetExceeded { used: u32, max: u32 },

    #[error("Latency budget exceeded: {elapsed_ms}ms > {max_ms}ms")]
    LatencyBudgetExceeded { elapsed_ms: u64, max_ms: u64 },

    #[error("Call budget exceeded: {calls} calls > {max} max")]
    CallBudgetExceeded { calls: u32, max: u32 },

    #[error("Budget check failed: {reason}")]
    CheckFailed { reason: String },
}

/// Performance budget configuration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PerformanceBudget {
    /// Maximum tokens per request.
    pub max_tokens: u32,
    /// Maximum latency in milliseconds.
    pub max_latency_ms: u64,
    /// Maximum calls per run.
    pub max_calls_per_run: u32,
}

impl Default for PerformanceBudget {
    fn default() -> Self {
        Self {
            max_tokens: DEFAULT_MAX_TOKENS,
            max_latency_ms: DEFAULT_MAX_LATENCY_MS,
            max_calls_per_run: DEFAULT_MAX_CALLS_PER_RUN,
        }
    }
}

impl PerformanceBudget {
    /// Creates a new performance budget with custom values.
    pub fn new(max_tokens: u32, max_latency_ms: u64, max_calls_per_run: u32) -> Self {
        Self {
            max_tokens,
            max_latency_ms,
            max_calls_per_run,
        }
    }

    /// Creates a budget from environment variables.
    pub fn from_env() -> Self {
        Self {
            max_tokens: std::env::var("MAX_TOKENS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_MAX_TOKENS),
            max_latency_ms: std::env::var("MAX_LATENCY_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_MAX_LATENCY_MS),
            max_calls_per_run: std::env::var("MAX_CALLS_PER_RUN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_MAX_CALLS_PER_RUN),
        }
    }
}

/// Budget enforcer with runtime tracking.
#[derive(Debug)]
pub struct BudgetEnforcer {
    budget: PerformanceBudget,
    tokens_used: AtomicU32,
    calls_made: AtomicU32,
    start_time: Instant,
    hard_fail: bool,
}

impl BudgetEnforcer {
    /// Creates a new budget enforcer.
    pub fn new(budget: PerformanceBudget) -> Self {
        Self {
            budget,
            tokens_used: AtomicU32::new(0),
            calls_made: AtomicU32::new(0),
            start_time: Instant::now(),
            hard_fail: true,
        }
    }

    /// Creates a default budget enforcer.
    pub fn default_enforcer() -> Self {
        Self::new(PerformanceBudget::default())
    }

    /// Sets whether to hard fail on violations.
    pub fn with_hard_fail(mut self, hard_fail: bool) -> Self {
        self.hard_fail = hard_fail;
        self
    }

    /// Records token usage and checks budget.
    ///
    /// # Errors
    ///
    /// Returns `BudgetViolation::TokenBudgetExceeded` if the token budget is exceeded.
    pub fn record_tokens(&self, tokens: u32) -> Result<(), BudgetViolation> {
        let new_total = self.tokens_used.fetch_add(tokens, Ordering::SeqCst) + tokens;

        if new_total > self.budget.max_tokens {
            let violation = BudgetViolation::TokenBudgetExceeded {
                used: new_total,
                max: self.budget.max_tokens,
            };
            if self.hard_fail {
                tracing::error!(
                    used = new_total,
                    max = self.budget.max_tokens,
                    "HARD FAIL: Token budget exceeded"
                );
            }
            return Err(violation);
        }

        tracing::debug!(
            tokens_added = tokens,
            tokens_total = new_total,
            tokens_remaining = self.budget.max_tokens - new_total,
            "Token budget updated"
        );

        Ok(())
    }

    /// Records a call and checks budget.
    ///
    /// # Errors
    ///
    /// Returns `BudgetViolation::CallBudgetExceeded` if the call budget is exceeded.
    pub fn record_call(&self) -> Result<(), BudgetViolation> {
        let new_total = self.calls_made.fetch_add(1, Ordering::SeqCst) + 1;

        if new_total > self.budget.max_calls_per_run {
            let violation = BudgetViolation::CallBudgetExceeded {
                calls: new_total,
                max: self.budget.max_calls_per_run,
            };
            if self.hard_fail {
                tracing::error!(
                    calls = new_total,
                    max = self.budget.max_calls_per_run,
                    "HARD FAIL: Call budget exceeded"
                );
            }
            return Err(violation);
        }

        tracing::debug!(
            calls_made = new_total,
            calls_remaining = self.budget.max_calls_per_run - new_total,
            "Call budget updated"
        );

        Ok(())
    }

    /// Checks the latency budget.
    ///
    /// # Errors
    ///
    /// Returns `BudgetViolation::LatencyBudgetExceeded` if the latency budget is exceeded.
    pub fn check_latency(&self) -> Result<(), BudgetViolation> {
        let elapsed = self.start_time.elapsed();
        let elapsed_ms = elapsed.as_millis() as u64;

        if elapsed_ms > self.budget.max_latency_ms {
            let violation = BudgetViolation::LatencyBudgetExceeded {
                elapsed_ms,
                max_ms: self.budget.max_latency_ms,
            };
            if self.hard_fail {
                tracing::error!(
                    elapsed_ms = elapsed_ms,
                    max_ms = self.budget.max_latency_ms,
                    "HARD FAIL: Latency budget exceeded"
                );
            }
            return Err(violation);
        }

        tracing::debug!(
            elapsed_ms = elapsed_ms,
            remaining_ms = self.budget.max_latency_ms - elapsed_ms,
            "Latency budget check passed"
        );

        Ok(())
    }

    /// Validates all budgets.
    ///
    /// # Errors
    ///
    /// Returns the first budget violation found.
    pub fn validate_all(&self) -> Result<(), BudgetViolation> {
        self.check_latency()?;
        // Token and call budgets are validated on record
        Ok(())
    }

    /// Returns current budget usage statistics.
    pub fn usage_stats(&self) -> BudgetUsage {
        let elapsed = self.start_time.elapsed();
        BudgetUsage {
            tokens_used: self.tokens_used.load(Ordering::SeqCst),
            tokens_max: self.budget.max_tokens,
            calls_made: self.calls_made.load(Ordering::SeqCst),
            calls_max: self.budget.max_calls_per_run,
            elapsed_ms: elapsed.as_millis() as u64,
            latency_max_ms: self.budget.max_latency_ms,
        }
    }

    /// Resets the enforcer for a new run.
    pub fn reset(&self) {
        self.tokens_used.store(0, Ordering::SeqCst);
        self.calls_made.store(0, Ordering::SeqCst);
        // Note: start_time cannot be reset due to Instant being non-Copy
        // A new enforcer should be created for a fresh run
    }

    /// Returns the underlying budget configuration.
    pub fn budget(&self) -> &PerformanceBudget {
        &self.budget
    }
}

/// Budget usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetUsage {
    pub tokens_used: u32,
    pub tokens_max: u32,
    pub calls_made: u32,
    pub calls_max: u32,
    pub elapsed_ms: u64,
    pub latency_max_ms: u64,
}

impl BudgetUsage {
    /// Returns the token utilization percentage.
    pub fn token_utilization(&self) -> f64 {
        if self.tokens_max == 0 {
            0.0
        } else {
            (self.tokens_used as f64 / self.tokens_max as f64) * 100.0
        }
    }

    /// Returns the call utilization percentage.
    pub fn call_utilization(&self) -> f64 {
        if self.calls_max == 0 {
            0.0
        } else {
            (self.calls_made as f64 / self.calls_max as f64) * 100.0
        }
    }

    /// Returns the latency utilization percentage.
    pub fn latency_utilization(&self) -> f64 {
        if self.latency_max_ms == 0 {
            0.0
        } else {
            (self.elapsed_ms as f64 / self.latency_max_ms as f64) * 100.0
        }
    }
}

/// Scoped budget guard for automatic validation on drop.
pub struct BudgetGuard<'a> {
    enforcer: &'a BudgetEnforcer,
    operation: String,
}

impl<'a> BudgetGuard<'a> {
    /// Creates a new budget guard.
    pub fn new(enforcer: &'a BudgetEnforcer, operation: impl Into<String>) -> Self {
        Self {
            enforcer,
            operation: operation.into(),
        }
    }

    /// Records tokens used during this operation.
    pub fn record_tokens(&self, tokens: u32) -> Result<(), BudgetViolation> {
        self.enforcer.record_tokens(tokens)
    }

    /// Records a call made during this operation.
    pub fn record_call(&self) -> Result<(), BudgetViolation> {
        self.enforcer.record_call()
    }
}

impl<'a> Drop for BudgetGuard<'a> {
    fn drop(&mut self) {
        if let Err(e) = self.enforcer.check_latency() {
            tracing::warn!(
                operation = %self.operation,
                error = %e,
                "Budget guard detected latency violation on drop"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_budget_values() {
        let budget = PerformanceBudget::default();
        assert_eq!(budget.max_tokens, 1500);
        assert_eq!(budget.max_latency_ms, 3000);
        assert_eq!(budget.max_calls_per_run, 4);
    }

    #[test]
    fn test_token_budget_enforcement() {
        let enforcer = BudgetEnforcer::new(PerformanceBudget::new(100, 3000, 4))
            .with_hard_fail(false);

        assert!(enforcer.record_tokens(50).is_ok());
        assert!(enforcer.record_tokens(40).is_ok());
        assert!(enforcer.record_tokens(20).is_err()); // Exceeds 100
    }

    #[test]
    fn test_call_budget_enforcement() {
        let enforcer = BudgetEnforcer::new(PerformanceBudget::new(1500, 3000, 2))
            .with_hard_fail(false);

        assert!(enforcer.record_call().is_ok());
        assert!(enforcer.record_call().is_ok());
        assert!(enforcer.record_call().is_err()); // Exceeds 2
    }

    #[test]
    fn test_budget_usage_stats() {
        let enforcer = BudgetEnforcer::new(PerformanceBudget::new(100, 3000, 4));

        enforcer.record_tokens(25).unwrap();
        enforcer.record_call().unwrap();

        let stats = enforcer.usage_stats();
        assert_eq!(stats.tokens_used, 25);
        assert_eq!(stats.calls_made, 1);
        assert_eq!(stats.token_utilization(), 25.0);
        assert_eq!(stats.call_utilization(), 25.0);
    }
}
