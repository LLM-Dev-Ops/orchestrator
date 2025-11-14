// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Provider trait definitions.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LLM provider trait.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Generate a completion.
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError>;

    /// Get provider name.
    fn name(&self) -> &str;

    /// Check if provider is healthy.
    async fn health_check(&self) -> Result<(), ProviderError> {
        Ok(())
    }
}

/// Completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Model name.
    pub model: String,

    /// Prompt or messages.
    pub prompt: String,

    /// System prompt (optional).
    pub system: Option<String>,

    /// Temperature (0.0 - 2.0).
    pub temperature: Option<f32>,

    /// Maximum tokens to generate.
    pub max_tokens: Option<u32>,

    /// Additional parameters.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Generated text.
    pub text: String,

    /// Model used.
    pub model: String,

    /// Tokens used.
    pub tokens_used: Option<u32>,

    /// Additional metadata.
    #[serde(flatten)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Provider error.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// HTTP request error.
    #[error("HTTP request failed: {0}")]
    HttpError(String),

    /// Authentication error.
    #[error("Authentication failed: {0}")]
    AuthError(String),

    /// Rate limit exceeded.
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// Invalid request.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Provider-specific error.
    #[error("Provider error: {0}")]
    ProviderSpecific(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Timeout error.
    #[error("Request timed out")]
    Timeout,

    /// Unknown error.
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<serde_json::Error> for ProviderError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}
