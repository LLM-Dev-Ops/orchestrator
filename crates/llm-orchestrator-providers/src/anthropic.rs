// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Anthropic (Claude) provider implementation.

use crate::traits::{CompletionRequest, CompletionResponse, LLMProvider, ProviderError};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Anthropic API provider.
pub struct AnthropicProvider {
    /// HTTP client.
    client: Client,
    /// API key.
    api_key: String,
    /// API base URL.
    base_url: String,
    /// Default API version.
    api_version: String,
}

/// Anthropic messages request.
#[derive(Debug, Serialize)]
struct MessagesRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

/// Message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

/// Anthropic messages response.
#[derive(Debug, Deserialize)]
struct MessagesResponse {
    id: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    response_type: String,
    #[allow(dead_code)]
    role: String,
    content: Vec<ContentBlock>,
    model: String,
    stop_reason: Option<String>,
    #[allow(dead_code)]
    stop_sequence: Option<String>,
    usage: Usage,
}

/// Content block in response.
#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    content_type: String,
    text: String,
}

/// Token usage information.
#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

/// Anthropic error response.
#[derive(Debug, Deserialize)]
struct AnthropicErrorResponse {
    error: AnthropicError,
}

#[derive(Debug, Deserialize)]
struct AnthropicError {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

impl AnthropicProvider {
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

    /// Creates a new Anthropic provider.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Anthropic API key
    ///
    /// # Example
    ///
    /// ```no_run
    /// use llm_orchestrator_providers::AnthropicProvider;
    ///
    /// let provider = AnthropicProvider::new("sk-ant-...".to_string());
    /// ```
    pub fn new(api_key: String) -> Self {
        Self::with_base_url(
            api_key,
            "https://api.anthropic.com/v1".to_string(),
            "2023-06-01".to_string(),
        )
    }

    /// Creates a new Anthropic provider with custom base URL and API version.
    pub fn with_base_url(api_key: String, base_url: String, api_version: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            base_url,
            api_version,
        }
    }

    /// Creates a new Anthropic provider from environment variable.
    ///
    /// Reads the API key from `ANTHROPIC_API_KEY` environment variable.
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
            ProviderError::InvalidRequest(
                "ANTHROPIC_API_KEY environment variable not set".to_string(),
            )
        })?;

        Ok(Self::new(api_key))
    }

    /// Converts a provider completion request to Anthropic format.
    fn to_anthropic_request(&self, request: &CompletionRequest) -> MessagesRequest {
        // Build messages array
        let messages = vec![Message {
            role: "user".to_string(),
            content: request.prompt.clone(),
        }];

        // Extract optional parameters from extra
        let top_p = request
            .extra
            .get("top_p")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32);

        let top_k = request
            .extra
            .get("top_k")
            .and_then(|v| v.as_u64())
            .map(|u| u as u32);

        let stop_sequences = request
            .extra
            .get("stop_sequences")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

        MessagesRequest {
            model: request.model.clone(),
            messages,
            max_tokens: request.max_tokens.unwrap_or(1024),
            system: request.system.clone(),
            temperature: request.temperature,
            top_p,
            top_k,
            stop_sequences,
        }
    }

    /// Parses an error response from Anthropic.
    fn parse_error(&self, status: StatusCode, body: &str) -> ProviderError {
        // Try to parse as Anthropic error format
        if let Ok(error_response) = serde_json::from_str::<AnthropicErrorResponse>(body) {
            let error = error_response.error;

            // Detect rate limiting
            if status == StatusCode::TOO_MANY_REQUESTS || error.error_type == "rate_limit_error" {
                return ProviderError::RateLimitExceeded;
            }

            // Detect authentication errors
            if status == StatusCode::UNAUTHORIZED
                || status == StatusCode::FORBIDDEN
                || error.error_type == "authentication_error"
                || error.error_type == "permission_error"
            {
                return ProviderError::AuthError(error.message);
            }

            // Detect invalid request errors
            if error.error_type == "invalid_request_error" {
                return ProviderError::InvalidRequest(error.message);
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
impl LLMProvider for AnthropicProvider {
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError> {
        let anthropic_request = self.to_anthropic_request(&request);

        // Make API request
        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.api_version)
            .header("Content-Type", "application/json")
            .json(&anthropic_request)
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
        let messages_response: MessagesResponse = serde_json::from_str(&body)?;

        // Extract text from content blocks
        let text = messages_response
            .content
            .iter()
            .map(|block| block.text.clone())
            .collect::<Vec<_>>()
            .join("");

        // Build metadata with usage and stop reason
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "usage".to_string(),
            serde_json::json!({
                "input_tokens": messages_response.usage.input_tokens,
                "output_tokens": messages_response.usage.output_tokens,
                "total_tokens": messages_response.usage.input_tokens + messages_response.usage.output_tokens,
            }),
        );

        if let Some(stop_reason) = &messages_response.stop_reason {
            metadata.insert("stop_reason".to_string(), serde_json::json!(stop_reason));
        }

        metadata.insert("id".to_string(), serde_json::json!(messages_response.id));

        Ok(CompletionResponse {
            text,
            model: messages_response.model,
            tokens_used: Some(
                messages_response.usage.input_tokens + messages_response.usage.output_tokens,
            ),
            metadata,
        })
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        // Anthropic doesn't have a dedicated health endpoint
        // We'll do a minimal completion request as a health check
        let test_request = CompletionRequest {
            model: "claude-3-haiku-20240307".to_string(),
            prompt: "Hi".to_string(),
            system: None,
            temperature: None,
            max_tokens: Some(5),
            extra: std::collections::HashMap::new(),
        };

        self.complete(test_request).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = AnthropicProvider::new("test-key".to_string());
        assert_eq!(provider.name(), "anthropic");
        assert_eq!(provider.base_url, "https://api.anthropic.com/v1");
    }

    #[test]
    fn test_provider_with_custom_base_url() {
        let provider = AnthropicProvider::with_base_url(
            "test-key".to_string(),
            "http://localhost:8080".to_string(),
            "2023-06-01".to_string(),
        );
        assert_eq!(provider.base_url, "http://localhost:8080");
        assert_eq!(provider.api_version, "2023-06-01");
    }

    #[test]
    fn test_to_anthropic_request() {
        let provider = AnthropicProvider::new("test-key".to_string());

        let request = CompletionRequest {
            model: "claude-3-opus-20240229".to_string(),
            prompt: "Hello, world!".to_string(),
            system: Some("You are a helpful assistant".to_string()),
            temperature: Some(0.7),
            max_tokens: Some(100),
            extra: std::collections::HashMap::new(),
        };

        let anthropic_req = provider.to_anthropic_request(&request);

        assert_eq!(anthropic_req.model, "claude-3-opus-20240229");
        assert_eq!(anthropic_req.messages.len(), 1);
        assert_eq!(anthropic_req.messages[0].role, "user");
        assert_eq!(anthropic_req.messages[0].content, "Hello, world!");
        assert_eq!(
            anthropic_req.system,
            Some("You are a helpful assistant".to_string())
        );
        assert_eq!(anthropic_req.temperature, Some(0.7));
        assert_eq!(anthropic_req.max_tokens, 100);
    }

    #[test]
    fn test_parse_rate_limit_error() {
        let provider = AnthropicProvider::new("test-key".to_string());

        let error_json = r#"{
            "error": {
                "type": "rate_limit_error",
                "message": "Rate limit exceeded"
            }
        }"#;

        let error = provider.parse_error(StatusCode::TOO_MANY_REQUESTS, error_json);

        match error {
            ProviderError::RateLimitExceeded => {} // Success
            _ => panic!("Expected RateLimitExceeded error"),
        }
    }

    #[test]
    fn test_parse_auth_error() {
        let provider = AnthropicProvider::new("test-key".to_string());

        let error_json = r#"{
            "error": {
                "type": "authentication_error",
                "message": "Invalid API key"
            }
        }"#;

        let error = provider.parse_error(StatusCode::UNAUTHORIZED, error_json);

        match error {
            ProviderError::AuthError(msg) => assert_eq!(msg, "Invalid API key"),
            _ => panic!("Expected AuthError"),
        }
    }

    #[test]
    fn test_parse_invalid_request_error() {
        let provider = AnthropicProvider::new("test-key".to_string());

        let error_json = r#"{
            "error": {
                "type": "invalid_request_error",
                "message": "Missing required field"
            }
        }"#;

        let error = provider.parse_error(StatusCode::BAD_REQUEST, error_json);

        match error {
            ProviderError::InvalidRequest(msg) => assert_eq!(msg, "Missing required field"),
            _ => panic!("Expected InvalidRequest error"),
        }
    }
}
