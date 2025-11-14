// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Retry logic with exponential backoff.
//!
//! This module provides configurable retry policies for handling transient failures
//! in LLM API calls and other operations.

use crate::error::Result;
use rand::Rng;
use std::time::Duration;

/// Retry policy configuration.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (0 = no retries).
    pub max_attempts: u32,

    /// Initial delay before the first retry.
    pub initial_delay: Duration,

    /// Multiplier for exponential backoff (typically 2.0).
    pub multiplier: f64,

    /// Maximum delay between retries.
    pub max_delay: Duration,

    /// Whether to add jitter to prevent thundering herd.
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            multiplier: 2.0,
            max_delay: Duration::from_secs(30),
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Creates a new retry policy with custom settings.
    pub fn new(
        max_attempts: u32,
        initial_delay: Duration,
        multiplier: f64,
        max_delay: Duration,
    ) -> Self {
        Self {
            max_attempts,
            initial_delay,
            multiplier,
            max_delay,
            jitter: true,
        }
    }

    /// Creates a retry policy with no retries.
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 0,
            initial_delay: Duration::from_millis(0),
            multiplier: 1.0,
            max_delay: Duration::from_millis(0),
            jitter: false,
        }
    }

    /// Creates a retry policy with fixed delays (no exponential backoff).
    pub fn fixed_delay(max_attempts: u32, delay: Duration) -> Self {
        Self {
            max_attempts,
            initial_delay: delay,
            multiplier: 1.0,
            max_delay: delay,
            jitter: false,
        }
    }

    /// Calculates the delay for a given attempt number (0-indexed).
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt >= self.max_attempts {
            return Duration::from_millis(0);
        }

        // Calculate exponential backoff: initial_delay * multiplier^attempt
        let base_delay_ms = self.initial_delay.as_millis() as f64
            * self.multiplier.powi(attempt as i32);

        let base_delay = Duration::from_millis(base_delay_ms as u64);
        let capped_delay = std::cmp::min(base_delay, self.max_delay);

        if self.jitter {
            self.add_jitter(capped_delay)
        } else {
            capped_delay
        }
    }

    /// Adds random jitter to a delay (±25% of the delay value).
    fn add_jitter(&self, delay: Duration) -> Duration {
        let mut rng = rand::thread_rng();
        let delay_ms = delay.as_millis() as f64;

        // Add jitter: random value between 75% and 125% of original delay
        let jitter_factor = rng.gen_range(0.75..=1.25);
        let jittered_ms = (delay_ms * jitter_factor) as u64;

        Duration::from_millis(jittered_ms)
    }

    /// Returns true if retries are enabled.
    pub fn is_enabled(&self) -> bool {
        self.max_attempts > 0
    }
}

/// Retry executor that handles retry logic with async functions.
pub struct RetryExecutor {
    policy: RetryPolicy,
}

impl RetryExecutor {
    /// Creates a new retry executor with the given policy.
    pub fn new(policy: RetryPolicy) -> Self {
        Self { policy }
    }

    /// Executes an async operation with retries according to the policy.
    ///
    /// The operation will be retried if:
    /// - It returns a retryable error (determined by `OrchestratorError::is_retryable()`)
    /// - The maximum number of attempts has not been reached
    ///
    /// # Examples
    ///
    /// ```
    /// use llm_orchestrator_core::retry::{RetryExecutor, RetryPolicy};
    /// use std::time::Duration;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let policy = RetryPolicy::new(3, Duration::from_millis(100), 2.0, Duration::from_secs(5));
    /// let executor = RetryExecutor::new(policy);
    ///
    /// let result = executor.execute(|| async {
    ///     // Your async operation here
    ///     Ok::<_, llm_orchestrator_core::error::OrchestratorError>(42)
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute<F, Fut, T>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let max_attempts = if self.policy.is_enabled() {
            self.policy.max_attempts + 1 // +1 for initial attempt
        } else {
            1
        };

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    attempt += 1;

                    // Check if we should retry
                    if attempt >= max_attempts || !err.is_retryable() {
                        return Err(err);
                    }

                    // Calculate delay and wait before retrying
                    let delay = self.policy.delay_for_attempt(attempt - 1);
                    if delay > Duration::from_millis(0) {
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
    }

    /// Executes an async operation with retries, providing attempt information to the operation.
    ///
    /// This variant passes the current attempt number to the operation function,
    /// useful for logging or adjusting behavior based on retry count.
    pub async fn execute_with_info<F, Fut, T>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut(u32) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let max_attempts = if self.policy.is_enabled() {
            self.policy.max_attempts + 1
        } else {
            1
        };

        loop {
            match operation(attempt).await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    attempt += 1;

                    if attempt >= max_attempts || !err.is_retryable() {
                        return Err(err);
                    }

                    let delay = self.policy.delay_for_attempt(attempt - 1);
                    if delay > Duration::from_millis(0) {
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::OrchestratorError;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_default_retry_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.initial_delay, Duration::from_millis(100));
        assert_eq!(policy.multiplier, 2.0);
        assert_eq!(policy.max_delay, Duration::from_secs(30));
        assert!(policy.jitter);
        assert!(policy.is_enabled());
    }

    #[test]
    fn test_no_retry_policy() {
        let policy = RetryPolicy::no_retry();
        assert_eq!(policy.max_attempts, 0);
        assert!(!policy.is_enabled());
        assert_eq!(policy.delay_for_attempt(0), Duration::from_millis(0));
    }

    #[test]
    fn test_fixed_delay_policy() {
        let policy = RetryPolicy::fixed_delay(3, Duration::from_millis(500));
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.multiplier, 1.0);
        assert!(!policy.jitter);

        // Fixed delay should not change with attempts (when jitter is disabled)
        assert_eq!(policy.delay_for_attempt(0), Duration::from_millis(500));
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(500));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(500));
    }

    #[test]
    fn test_exponential_backoff_without_jitter() {
        let mut policy = RetryPolicy::new(
            5,
            Duration::from_millis(100),
            2.0,
            Duration::from_secs(10),
        );
        policy.jitter = false; // Disable jitter for deterministic testing

        // Exponential backoff: 100ms * 2^attempt
        assert_eq!(policy.delay_for_attempt(0), Duration::from_millis(100));  // 100 * 2^0
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(200));  // 100 * 2^1
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(400));  // 100 * 2^2
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(800));  // 100 * 2^3
        assert_eq!(policy.delay_for_attempt(4), Duration::from_millis(1600)); // 100 * 2^4
    }

    #[test]
    fn test_max_delay_cap() {
        let mut policy = RetryPolicy::new(
            10,
            Duration::from_millis(100),
            2.0,
            Duration::from_secs(1), // Cap at 1 second
        );
        policy.jitter = false;

        // Should cap at max_delay after a few attempts
        assert_eq!(policy.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(400));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(800));
        assert_eq!(policy.delay_for_attempt(4), Duration::from_secs(1));      // Capped
        assert_eq!(policy.delay_for_attempt(5), Duration::from_secs(1));      // Capped
    }

    #[test]
    fn test_jitter_adds_randomness() {
        let policy = RetryPolicy::new(
            3,
            Duration::from_millis(1000),
            2.0,
            Duration::from_secs(10),
        );

        // Run multiple times to check jitter variability
        let delay1 = policy.delay_for_attempt(0);
        let delay2 = policy.delay_for_attempt(0);
        let delay3 = policy.delay_for_attempt(0);

        // At least one delay should be different (with very high probability)
        // Jitter should produce values in range [750ms, 1250ms] for 1000ms base
        assert!(delay1.as_millis() >= 750 && delay1.as_millis() <= 1250);
        assert!(delay2.as_millis() >= 750 && delay2.as_millis() <= 1250);
        assert!(delay3.as_millis() >= 750 && delay3.as_millis() <= 1250);
    }

    #[tokio::test]
    async fn test_retry_executor_success_on_first_attempt() {
        let policy = RetryPolicy::default();
        let executor = RetryExecutor::new(policy);

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = executor
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok::<i32, OrchestratorError>(42)
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Called only once
    }

    #[tokio::test]
    async fn test_retry_executor_retries_on_retryable_error() {
        let policy = RetryPolicy::new(
            3,
            Duration::from_millis(10),
            2.0,
            Duration::from_millis(100),
        );
        let executor = RetryExecutor::new(policy);

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = executor
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    let count = counter.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        // Fail first 2 attempts with retryable error
                        Err(OrchestratorError::ProviderError {
                            provider: "test".to_string(),
                            message: "retryable error".to_string(),
                        })
                    } else {
                        // Succeed on 3rd attempt
                        Ok::<i32, OrchestratorError>(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3); // Called 3 times
    }

    #[tokio::test]
    async fn test_retry_executor_fails_after_max_attempts() {
        let policy = RetryPolicy::new(
            2,
            Duration::from_millis(10),
            2.0,
            Duration::from_millis(100),
        );
        let executor = RetryExecutor::new(policy);

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = executor
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, OrchestratorError>(OrchestratorError::ProviderError {
                        provider: "test".to_string(),
                        message: "persistent error".to_string(),
                    })
                }
            })
            .await;

        assert!(result.is_err());
        // Should try: initial + 2 retries = 3 attempts total
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_executor_no_retry_on_non_retryable_error() {
        let policy = RetryPolicy::default();
        let executor = RetryExecutor::new(policy);

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = executor
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    // ValidationError is not retryable
                    Err::<i32, OrchestratorError>(OrchestratorError::ValidationError(
                        "bad input".to_string(),
                    ))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Called only once, no retries
    }

    #[tokio::test]
    async fn test_retry_executor_with_info() {
        let policy = RetryPolicy::new(
            3,
            Duration::from_millis(10),
            2.0,
            Duration::from_millis(100),
        );
        let executor = RetryExecutor::new(policy);

        let attempts = Arc::new(std::sync::Mutex::new(Vec::new()));
        let attempts_clone = attempts.clone();

        let result = executor
            .execute_with_info(|attempt_num| {
                let attempts = attempts_clone.clone();
                async move {
                    let mut attempts = attempts.lock().unwrap();
                    attempts.push(attempt_num);

                    if attempt_num < 2 {
                        Err(OrchestratorError::ProviderError {
                            provider: "test".to_string(),
                            message: "retry".to_string(),
                        })
                    } else {
                        Ok::<i32, OrchestratorError>(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        let attempts = attempts.lock().unwrap();
        assert_eq!(*attempts, vec![0, 1, 2]); // Attempt numbers should be sequential
    }

    #[tokio::test]
    async fn test_no_retry_policy_executor() {
        let policy = RetryPolicy::no_retry();
        let executor = RetryExecutor::new(policy);

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = executor
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, OrchestratorError>(OrchestratorError::ProviderError {
                        provider: "test".to_string(),
                        message: "error".to_string(),
                    })
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1); // No retries
    }
}
