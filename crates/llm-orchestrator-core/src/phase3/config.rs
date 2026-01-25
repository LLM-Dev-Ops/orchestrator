// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Phase 3 Configuration and Startup Hardening.
//!
//! Implements crashloop behavior on misconfiguration as per constitution.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Phase 3 Layer identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase3Layer {
    /// Layer 1 - Automation & Resilience
    Layer1,
    /// Layer 2 - Advanced Coordination (future)
    Layer2,
}

impl std::fmt::Display for Phase3Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Layer1 => write!(f, "layer1"),
            Self::Layer2 => write!(f, "layer2"),
        }
    }
}

impl std::str::FromStr for Phase3Layer {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "layer1" | "1" => Ok(Self::Layer1),
            "layer2" | "2" => Ok(Self::Layer2),
            _ => Err(ConfigError::InvalidLayer(s.to_string())),
        }
    }
}

/// Phase 3 configuration errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("RUVECTOR_SERVICE_ENDPOINT not set - Ruvector is REQUIRED")]
    RuvectorEndpointMissing,

    #[error("RUVECTOR_API_KEY not set - Ruvector authentication REQUIRED")]
    RuvectorApiKeyMissing,

    #[error("Invalid AGENT_PHASE: expected 'phase3', got '{0}'")]
    InvalidPhase(String),

    #[error("Invalid AGENT_LAYER: {0}")]
    InvalidLayer(String),

    #[error("Ruvector service unavailable: {0}")]
    RuvectorUnavailable(String),

    #[error("Configuration validation failed: {0}")]
    ValidationFailed(String),
}

/// Phase 3 configuration for Layer 1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase3Config {
    /// Agent phase identifier.
    pub agent_phase: String,
    /// Agent layer identifier.
    pub agent_layer: Phase3Layer,
    /// Ruvector service endpoint (REQUIRED).
    pub ruvector_endpoint: String,
    /// Ruvector API key (REQUIRED).
    pub ruvector_api_key: String,
    /// Maximum tokens per request.
    pub max_tokens: u32,
    /// Maximum latency in milliseconds.
    pub max_latency_ms: u64,
    /// Maximum calls per run.
    pub max_calls_per_run: u32,
    /// Whether to enforce hard fail on violations.
    pub hard_fail_on_violation: bool,
    /// Startup validation timeout.
    pub startup_timeout: Duration,
}

impl Default for Phase3Config {
    fn default() -> Self {
        Self {
            agent_phase: "phase3".to_string(),
            agent_layer: Phase3Layer::Layer1,
            ruvector_endpoint: String::new(),
            ruvector_api_key: String::new(),
            max_tokens: 1500,
            max_latency_ms: 3000,
            max_calls_per_run: 4,
            hard_fail_on_violation: true,
            startup_timeout: Duration::from_secs(30),
        }
    }
}

impl Phase3Config {
    /// Creates a new Phase 3 configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if required environment variables are missing
    /// or invalid. This will cause crashloop behavior on misconfiguration.
    pub fn from_env() -> Result<Self, ConfigError> {
        let agent_phase = std::env::var("AGENT_PHASE").unwrap_or_else(|_| "phase3".to_string());
        if agent_phase != "phase3" {
            return Err(ConfigError::InvalidPhase(agent_phase));
        }

        let agent_layer = std::env::var("AGENT_LAYER")
            .unwrap_or_else(|_| "layer1".to_string())
            .parse::<Phase3Layer>()?;

        let ruvector_endpoint = std::env::var("RUVECTOR_SERVICE_ENDPOINT")
            .map_err(|_| ConfigError::RuvectorEndpointMissing)?;

        let ruvector_api_key = std::env::var("RUVECTOR_API_KEY")
            .map_err(|_| ConfigError::RuvectorApiKeyMissing)?;

        let max_tokens = std::env::var("MAX_TOKENS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1500);

        let max_latency_ms = std::env::var("MAX_LATENCY_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3000);

        let max_calls_per_run = std::env::var("MAX_CALLS_PER_RUN")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4);

        Ok(Self {
            agent_phase,
            agent_layer,
            ruvector_endpoint,
            ruvector_api_key,
            max_tokens,
            max_latency_ms,
            max_calls_per_run,
            hard_fail_on_violation: true,
            startup_timeout: Duration::from_secs(30),
        })
    }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.agent_phase != "phase3" {
            return Err(ConfigError::InvalidPhase(self.agent_phase.clone()));
        }

        if self.ruvector_endpoint.is_empty() {
            return Err(ConfigError::RuvectorEndpointMissing);
        }

        if self.ruvector_api_key.is_empty() {
            return Err(ConfigError::RuvectorApiKeyMissing);
        }

        Ok(())
    }
}

/// Startup validator for Phase 3 crashloop behavior.
#[derive(Debug)]
pub struct StartupValidator {
    config: Phase3Config,
}

impl StartupValidator {
    /// Creates a new startup validator.
    pub fn new(config: Phase3Config) -> Self {
        Self { config }
    }

    /// Validates startup configuration and Ruvector availability.
    ///
    /// # Crashloop Behavior
    ///
    /// If validation fails, this function will log the error and return
    /// an error. The calling code MUST exit with non-zero status to
    /// trigger Cloud Run crashloop behavior.
    pub async fn validate_startup(&self) -> Result<(), ConfigError> {
        tracing::info!(
            phase = %self.config.agent_phase,
            layer = %self.config.agent_layer,
            "Phase 3 startup validation beginning"
        );

        // Step 1: Validate configuration
        self.config.validate()?;
        tracing::info!("Configuration validation passed");

        // Step 2: Validate Ruvector availability
        self.validate_ruvector_availability().await?;
        tracing::info!("Ruvector availability validation passed");

        tracing::info!(
            phase = %self.config.agent_phase,
            layer = %self.config.agent_layer,
            max_tokens = self.config.max_tokens,
            max_latency_ms = self.config.max_latency_ms,
            max_calls_per_run = self.config.max_calls_per_run,
            "Phase 3 startup validation complete"
        );

        Ok(())
    }

    /// Validates Ruvector service availability.
    async fn validate_ruvector_availability(&self) -> Result<(), ConfigError> {
        use crate::adapters::RuVectorServiceClient;

        let client = RuVectorServiceClient::new(&self.config.ruvector_endpoint)
            .with_api_key(&self.config.ruvector_api_key)
            .with_timeout(self.config.startup_timeout);

        let health = client.health_check().await.map_err(|e| {
            ConfigError::RuvectorUnavailable(format!("Health check failed: {}", e))
        })?;

        if !health.healthy {
            return Err(ConfigError::RuvectorUnavailable(health.message));
        }

        tracing::info!(
            ruvector_latency_ms = ?health.latency_ms,
            "Ruvector service is healthy"
        );

        Ok(())
    }

    /// Returns the validated configuration.
    pub fn config(&self) -> &Phase3Config {
        &self.config
    }
}

/// Macro for crashloop-safe startup in main().
#[macro_export]
macro_rules! phase3_startup {
    () => {{
        use $crate::phase3::{Phase3Config, StartupValidator, ConfigError};

        let config = Phase3Config::from_env().unwrap_or_else(|e| {
            tracing::error!(error = %e, "Phase 3 configuration failed - triggering crashloop");
            std::process::exit(1);
        });

        let validator = StartupValidator::new(config);

        // Run async validation in blocking context for startup
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(async {
            validator.validate_startup().await.unwrap_or_else(|e| {
                tracing::error!(error = %e, "Phase 3 startup validation failed - triggering crashloop");
                std::process::exit(1);
            });
        });

        validator
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase3_layer_parsing() {
        assert_eq!("layer1".parse::<Phase3Layer>().unwrap(), Phase3Layer::Layer1);
        assert_eq!("1".parse::<Phase3Layer>().unwrap(), Phase3Layer::Layer1);
        assert_eq!("layer2".parse::<Phase3Layer>().unwrap(), Phase3Layer::Layer2);
        assert!("invalid".parse::<Phase3Layer>().is_err());
    }

    #[test]
    fn test_default_config_values() {
        let config = Phase3Config::default();
        assert_eq!(config.agent_phase, "phase3");
        assert_eq!(config.agent_layer, Phase3Layer::Layer1);
        assert_eq!(config.max_tokens, 1500);
        assert_eq!(config.max_latency_ms, 3000);
        assert_eq!(config.max_calls_per_run, 4);
    }

    #[test]
    fn test_config_validation_missing_ruvector() {
        let config = Phase3Config::default();
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_success() {
        let config = Phase3Config {
            ruvector_endpoint: "http://localhost:8080".to_string(),
            ruvector_api_key: "test-key".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }
}
