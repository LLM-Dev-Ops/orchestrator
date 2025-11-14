// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! OpenAI provider implementation.

use crate::traits::{CompletionRequest, CompletionResponse, LLMProvider, ProviderError};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// OpenAI API provider.
pub struct OpenAIProvider {
    /// HTTP client.
    client: Client,
    /// API key.
    api_key: String,
    /// API base URL.
    base_url: String,
}

/// OpenAI chat completion request.
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(default)]
    stream: bool,
}

/// Chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// OpenAI chat completion response.
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    #[allow(dead_code)]
    id: String,
    choices: Vec<Choice>,
    usage: Usage,
}

/// Completion choice.
#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
    finish_reason: Option<String>,
}

/// Token usage information.
#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// OpenAI error response.
#[derive(Debug, Deserialize)]
struct OpenAIErrorResponse {
    error: OpenAIError,
}

#[derive(Debug, Deserialize)]
struct OpenAIError {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
    #[allow(dead_code)]
    code: Option<String>,
}

impl OpenAIProvider {
    /// Converts a reqwest error to a ProviderError.
    fn convert_reqwest_error(err: reqwest::Error) -> ProviderError {
        if err.is_timeout() {
            ProviderError::Timeout
        } else if err.is_status() {
            if let Some(status) = err.status() {
                if status == 401 || status == 403 {
                    ProviderError::AuthError(err.to_string())
                } else if status == 429 {
                    ProviderError::RateLimitExceeded
                } else {
                    ProviderError::HttpError(err.to_string())
                }
            } else {
                ProviderError::HttpError(err.to_string())
            }
        } else {
            ProviderError::HttpError(err.to_string())
        }
    }

    /// Creates a new OpenAI provider.
    ///
    /// # Arguments
    ///
    /// * `api_key` - OpenAI API key
    ///
    /// # Example
    ///
    /// ```no_run
    /// use llm_orchestrator_providers::OpenAIProvider;
    ///
    /// let provider = OpenAIProvider::new("sk-...".to_string()).unwrap();
    /// ```
    pub fn new(api_key: String) -> Result<Self, ProviderError> {
        Self::with_base_url(api_key, "https://api.openai.com/v1".to_string())
    }

    /// Creates a new OpenAI provider with a custom base URL.
    ///
    /// Useful for testing or using OpenAI-compatible APIs.
    pub fn with_base_url(api_key: String, base_url: String) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| ProviderError::HttpError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            api_key,
            base_url,
        })
    }

    /// Creates a new OpenAI provider from environment variable.
    ///
    /// Reads the API key from `OPENAI_API_KEY` environment variable.
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
            ProviderError::InvalidRequest(
                "OPENAI_API_KEY environment variable not set".to_string(),
            )
        })?;

        Self::new(api_key)
    }

    /// Creates a new OpenAI provider using a secret store.
    ///
    /// # Arguments
    ///
    /// * `secret_store` - The secret store to retrieve the API key from
    /// * `secret_key` - The key to use when retrieving the secret (e.g., "openai/api_key")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "secrets")]
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use llm_orchestrator_providers::OpenAIProvider;
    /// use llm_orchestrator_secrets::{SecretStore, EnvSecretStore};
    /// use std::sync::Arc;
    ///
    /// let secret_store: Arc<dyn SecretStore> = Arc::new(EnvSecretStore::new());
    /// let provider = OpenAIProvider::from_secret_store(
    ///     secret_store,
    ///     "openai/api_key"
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "secrets")]
    pub async fn from_secret_store(
        secret_store: std::sync::Arc<dyn llm_orchestrator_secrets::SecretStore>,
        secret_key: &str,
    ) -> Result<Self, ProviderError> {
        let secret = secret_store
            .get_secret(secret_key)
            .await
            .map_err(|e| ProviderError::InvalidRequest(format!("Failed to retrieve secret: {}", e)))?;

        Self::new(secret.value)
    }

    /// Converts a provider completion request to OpenAI format.
    fn to_openai_request(&self, request: &CompletionRequest) -> ChatCompletionRequest {
        // Build messages array
        let mut messages = Vec::new();

        // Add system message if present
        if let Some(system) = &request.system {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }

        // Add user message
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: request.prompt.clone(),
        });

        // Extract optional parameters from extra
        let top_p = request
            .extra
            .get("top_p")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32);

        let frequency_penalty = request
            .extra
            .get("frequency_penalty")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32);

        let presence_penalty = request
            .extra
            .get("presence_penalty")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32);

        let stop = request
            .extra
            .get("stop")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

        ChatCompletionRequest {
            model: request.model.clone(),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p,
            frequency_penalty,
            presence_penalty,
            stop,
            stream: false,
        }
    }

    /// Parses an error response from OpenAI.
    fn parse_error(&self, status: StatusCode, body: &str) -> ProviderError {
        // Try to parse as OpenAI error format
        if let Ok(error_response) = serde_json::from_str::<OpenAIErrorResponse>(body) {
            let error = error_response.error;

            // Detect rate limiting
            if status == StatusCode::TOO_MANY_REQUESTS || error.error_type == "rate_limit_exceeded"
            {
                return ProviderError::RateLimitExceeded;
            }

            // Detect authentication errors
            if status == StatusCode::UNAUTHORIZED || error.error_type == "invalid_api_key" {
                return ProviderError::AuthError(error.message);
            }

            // Generic API error
            return ProviderError::ProviderSpecific(format!(
                "[{}] {}: {}",
                status.as_u16(),
                error.error_type,
                error.message
            ));
        }

        // Fallback to generic error
        ProviderError::HttpError(format!("[{}] {}", status.as_u16(), body))
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        let openai_request = self.to_openai_request(&request);

        // Make API request
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&openai_request)
            .send()
            .await
            .map_err(Self::convert_reqwest_error)?;

        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read response body"));

        // Handle errors
        if !status.is_success() {
            return Err(self.parse_error(status, &body));
        }

        // Parse success response
        let completion: ChatCompletionResponse = serde_json::from_str(&body)?;

        // Extract response
        let choice = completion
            .choices
            .first()
            .ok_or_else(|| ProviderError::SerializationError("No choices in response".to_string()))?;

        // Build metadata with usage and finish reason
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "usage".to_string(),
            serde_json::json!({
                "prompt_tokens": completion.usage.prompt_tokens,
                "completion_tokens": completion.usage.completion_tokens,
                "total_tokens": completion.usage.total_tokens,
            }),
        );

        if let Some(finish_reason) = &choice.finish_reason {
            metadata.insert("finish_reason".to_string(), serde_json::json!(finish_reason));
        }

        Ok(CompletionResponse {
            text: choice.message.content.clone(),
            model: request.model.clone(),
            tokens_used: Some(completion.usage.total_tokens),
            metadata,
        })
    }

    fn name(&self) -> &str {
        "openai"
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        // Simple health check: list models endpoint
        let response = self
            .client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(Self::convert_reqwest_error)?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(ProviderError::HttpError(format!(
                "Health check failed with status {}",
                response.status().as_u16()
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = OpenAIProvider::new("test-key".to_string()).unwrap();
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.base_url, "https://api.openai.com/v1");
    }

    #[test]
    fn test_provider_with_custom_base_url() {
        let provider =
            OpenAIProvider::with_base_url("test-key".to_string(), "http://localhost:8080".to_string()).unwrap();
        assert_eq!(provider.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_to_openai_request() {
        let provider = OpenAIProvider::new("test-key".to_string()).unwrap();

        let request = CompletionRequest {
            model: "gpt-4".to_string(),
            prompt: "Hello, world!".to_string(),
            system: Some("You are a helpful assistant".to_string()),
            temperature: Some(0.7),
            max_tokens: Some(100),
            extra: std::collections::HashMap::new(),
        };

        let openai_req = provider.to_openai_request(&request);

        assert_eq!(openai_req.model, "gpt-4");
        assert_eq!(openai_req.messages.len(), 2);
        assert_eq!(openai_req.messages[0].role, "system");
        assert_eq!(openai_req.messages[1].role, "user");
        assert_eq!(openai_req.messages[1].content, "Hello, world!");
        assert_eq!(openai_req.temperature, Some(0.7));
        assert_eq!(openai_req.max_tokens, Some(100));
    }

    #[test]
    fn test_parse_rate_limit_error() {
        let provider = OpenAIProvider::new("test-key".to_string()).unwrap();

        let error_json = r#"{
            "error": {
                "message": "Rate limit exceeded",
                "type": "rate_limit_exceeded",
                "code": "rate_limit_exceeded"
            }
        }"#;

        let error = provider.parse_error(StatusCode::TOO_MANY_REQUESTS, error_json);

        match error {
            ProviderError::RateLimitExceeded => {}, // Success
            _ => panic!("Expected RateLimitExceeded error"),
        }
    }

    #[test]
    fn test_parse_auth_error() {
        let provider = OpenAIProvider::new("test-key".to_string()).unwrap();

        let error_json = r#"{
            "error": {
                "message": "Invalid API key",
                "type": "invalid_api_key",
                "code": "invalid_api_key"
            }
        }"#;

        let error = provider.parse_error(StatusCode::UNAUTHORIZED, error_json);

        match error {
            ProviderError::AuthError(msg) => assert_eq!(msg, "Invalid API key"),
            _ => panic!("Expected AuthError"),
        }
    }
}
