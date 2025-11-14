// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! OpenAI embedding provider implementation.
//!
//! Supports:
//! - Models: text-embedding-3-small, text-embedding-3-large, text-embedding-ada-002
//! - Batch support: up to 2048 inputs per request
//! - Dimension reduction: optional parameter for text-embedding-3-* models
//! - Automatic retries with exponential backoff

use crate::traits::*;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, warn};

/// Maximum batch size for OpenAI embeddings API.
pub const OPENAI_MAX_BATCH_SIZE: usize = 2048;

/// Default retry configuration.
const MAX_RETRIES: u32 = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 1000;

/// OpenAI embedding provider.
pub struct OpenAIEmbeddingProvider {
    client: Client,
    api_key: String,
    base_url: String,
    max_retries: u32,
}

impl OpenAIEmbeddingProvider {
    /// Create a new OpenAI embedding provider.
    pub fn new(api_key: String) -> Result<Self, ProviderError> {
        Self::with_base_url(api_key, "https://api.openai.com/v1".to_string())
    }

    /// Create a provider with a custom base URL.
    pub fn with_base_url(api_key: String, base_url: String) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| ProviderError::HttpError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            api_key,
            base_url,
            max_retries: MAX_RETRIES,
        })
    }

    /// Set maximum number of retries for failed requests.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Create from environment variables.
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| ProviderError::AuthError("OPENAI_API_KEY not set".to_string()))?;
        Self::new(api_key)
    }

    /// Perform a single embedding request with retries.
    async fn embed_with_retry(&self, api_request: &OpenAIEmbeddingRequest) -> Result<OpenAIEmbeddingResponse, ProviderError> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = Duration::from_millis(INITIAL_RETRY_DELAY_MS * 2_u64.pow(attempt - 1));
                warn!("Retry attempt {} after {}ms", attempt, delay.as_millis());
                tokio::time::sleep(delay).await;
            }

            let url = format!("{}/embeddings", self.base_url);

            let response = match self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&api_request)
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = Some(ProviderError::HttpError(e.to_string()));
                    continue;
                }
            };

            let status = response.status();
            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());

                let error = match status.as_u16() {
                    401 => ProviderError::AuthError(error_text),
                    429 => {
                        // Rate limit - always retry
                        last_error = Some(ProviderError::RateLimitExceeded);
                        continue;
                    }
                    400..=499 => ProviderError::InvalidRequest(error_text),
                    500..=599 => {
                        // Server error - retry
                        last_error = Some(ProviderError::ProviderSpecific(error_text));
                        continue;
                    }
                    _ => ProviderError::ProviderSpecific(error_text),
                };

                return Err(error);
            }

            match response.json::<OpenAIEmbeddingResponse>().await {
                Ok(api_response) => return Ok(api_response),
                Err(e) => {
                    last_error = Some(ProviderError::SerializationError(e.to_string()));
                    continue;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| ProviderError::Unknown("Max retries exceeded".to_string())))
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddingProvider {
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, ProviderError> {
        // Extract input texts
        let texts = match &request.input {
            EmbeddingInput::Single { input } => vec![input.clone()],
            EmbeddingInput::Batch { input } => input.clone(),
        };

        // Check batch size
        if texts.len() > OPENAI_MAX_BATCH_SIZE {
            return Err(ProviderError::InvalidRequest(format!(
                "Batch size {} exceeds OpenAI maximum of {}",
                texts.len(),
                OPENAI_MAX_BATCH_SIZE
            )));
        }

        debug!(
            "Embedding {} texts with model {}",
            texts.len(),
            request.model
        );

        // Build OpenAI API request
        let api_request = OpenAIEmbeddingRequest {
            model: request.model.clone(),
            input: if texts.len() == 1 {
                OpenAIInput::Single(texts[0].clone())
            } else {
                OpenAIInput::Batch(texts)
            },
            dimensions: request.dimensions,
            encoding_format: None, // Use default "float"
        };

        // Execute request with retries
        let api_response = self.embed_with_retry(&api_request).await?;

        // Convert to standard format
        let mut embeddings_with_index: Vec<(usize, Vec<f32>)> = api_response
            .data
            .into_iter()
            .map(|item| (item.index, item.embedding))
            .collect();

        // Sort by index to ensure correct order
        embeddings_with_index.sort_by_key(|(index, _)| *index);
        let embeddings: Vec<Vec<f32>> = embeddings_with_index
            .into_iter()
            .map(|(_, embedding)| embedding)
            .collect();

        debug!(
            "Successfully embedded {} texts, used {} tokens",
            embeddings.len(),
            api_response.usage.total_tokens
        );

        Ok(EmbeddingResponse {
            embeddings,
            model: api_response.model,
            tokens_used: Some(api_response.usage.total_tokens),
            metadata: HashMap::new(),
        })
    }

    fn name(&self) -> &str {
        "openai_embeddings"
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        // Simple health check: try to embed a single short text
        let request = EmbeddingRequest {
            model: "text-embedding-3-small".to_string(),
            input: EmbeddingInput::Single {
                input: "health check".to_string(),
            },
            dimensions: None,
            extra: HashMap::new(),
        };

        self.embed(request).await?;
        Ok(())
    }
}

// OpenAI-specific request/response types

#[derive(Debug, Serialize)]
struct OpenAIEmbeddingRequest {
    model: String,
    input: OpenAIInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding_format: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum OpenAIInput {
    Single(String),
    Batch(Vec<String>),
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
    model: String,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAIUsage {
    prompt_tokens: u32,
    total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[test]
    fn test_provider_creation() {
        let provider = OpenAIEmbeddingProvider::new("test-key".to_string()).unwrap();
        assert_eq!(provider.name(), "openai_embeddings");
        assert_eq!(provider.max_retries, MAX_RETRIES);
    }

    #[test]
    fn test_provider_with_custom_base_url() {
        let provider = OpenAIEmbeddingProvider::with_base_url(
            "test-key".to_string(),
            "https://custom.api.com".to_string(),
        )
        .unwrap();
        assert_eq!(provider.name(), "openai_embeddings");
    }

    #[test]
    fn test_provider_with_max_retries() {
        let provider = OpenAIEmbeddingProvider::new("test-key".to_string())
            .unwrap()
            .with_max_retries(5);
        assert_eq!(provider.max_retries, 5);
    }

    #[tokio::test]
    async fn test_single_text_embedding() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/embeddings")
            .match_header("authorization", "Bearer test-key")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body(
                r#"{
                    "object": "list",
                    "data": [{
                        "object": "embedding",
                        "index": 0,
                        "embedding": [0.1, 0.2, 0.3]
                    }],
                    "model": "text-embedding-3-small",
                    "usage": {
                        "prompt_tokens": 5,
                        "total_tokens": 5
                    }
                }"#,
            )
            .create_async()
            .await;

        let provider =
            OpenAIEmbeddingProvider::with_base_url("test-key".to_string(), server.url()).unwrap();

        let request = EmbeddingRequest {
            model: "text-embedding-3-small".to_string(),
            input: EmbeddingInput::Single {
                input: "Hello, world!".to_string(),
            },
            dimensions: None,
            extra: HashMap::new(),
        };

        let response = provider.embed(request).await.unwrap();

        assert_eq!(response.embeddings.len(), 1);
        assert_eq!(response.embeddings[0], vec![0.1, 0.2, 0.3]);
        assert_eq!(response.model, "text-embedding-3-small");
        assert_eq!(response.tokens_used, Some(5));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_batch_embedding() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/embeddings")
            .with_status(200)
            .with_body(
                r#"{
                    "object": "list",
                    "data": [
                        {
                            "object": "embedding",
                            "index": 0,
                            "embedding": [0.1, 0.2]
                        },
                        {
                            "object": "embedding",
                            "index": 1,
                            "embedding": [0.3, 0.4]
                        }
                    ],
                    "model": "text-embedding-3-small",
                    "usage": {
                        "prompt_tokens": 10,
                        "total_tokens": 10
                    }
                }"#,
            )
            .create_async()
            .await;

        let provider =
            OpenAIEmbeddingProvider::with_base_url("test-key".to_string(), server.url()).unwrap();

        let request = EmbeddingRequest {
            model: "text-embedding-3-small".to_string(),
            input: EmbeddingInput::Batch {
                input: vec!["First text".to_string(), "Second text".to_string()],
            },
            dimensions: None,
            extra: HashMap::new(),
        };

        let response = provider.embed(request).await.unwrap();

        assert_eq!(response.embeddings.len(), 2);
        assert_eq!(response.embeddings[0], vec![0.1, 0.2]);
        assert_eq!(response.embeddings[1], vec![0.3, 0.4]);
        assert_eq!(response.tokens_used, Some(10));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_dimension_reduction() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/embeddings")
            .match_body(mockito::Matcher::JsonString(
                r#"{"model":"text-embedding-3-small","input":"test","dimensions":256}"#.to_string(),
            ))
            .with_status(200)
            .with_body(
                r#"{
                    "object": "list",
                    "data": [{
                        "object": "embedding",
                        "index": 0,
                        "embedding": [0.1, 0.2]
                    }],
                    "model": "text-embedding-3-small",
                    "usage": {
                        "prompt_tokens": 3,
                        "total_tokens": 3
                    }
                }"#,
            )
            .create_async()
            .await;

        let provider =
            OpenAIEmbeddingProvider::with_base_url("test-key".to_string(), server.url()).unwrap();

        let request = EmbeddingRequest {
            model: "text-embedding-3-small".to_string(),
            input: EmbeddingInput::Single {
                input: "test".to_string(),
            },
            dimensions: Some(256),
            extra: HashMap::new(),
        };

        let response = provider.embed(request).await.unwrap();

        assert_eq!(response.embeddings.len(), 1);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_auth_error() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/embeddings")
            .with_status(401)
            .with_body(r#"{"error": {"message": "Invalid API key"}}"#)
            .create_async()
            .await;

        let provider =
            OpenAIEmbeddingProvider::with_base_url("bad-key".to_string(), server.url()).unwrap();

        let request = EmbeddingRequest {
            model: "text-embedding-3-small".to_string(),
            input: EmbeddingInput::Single {
                input: "test".to_string(),
            },
            dimensions: None,
            extra: HashMap::new(),
        };

        let result = provider.embed(request).await;
        assert!(matches!(result, Err(ProviderError::AuthError(_))));
    }

    #[tokio::test]
    async fn test_rate_limit_retry() {
        let mut server = Server::new_async().await;
        // First request returns 429, second returns success
        let mock_fail = server
            .mock("POST", "/embeddings")
            .with_status(429)
            .with_body(r#"{"error": {"message": "Rate limit exceeded"}}"#)
            .expect(1)
            .create_async()
            .await;

        let mock_success = server
            .mock("POST", "/embeddings")
            .with_status(200)
            .with_body(
                r#"{
                    "object": "list",
                    "data": [{
                        "object": "embedding",
                        "index": 0,
                        "embedding": [0.1, 0.2, 0.3]
                    }],
                    "model": "text-embedding-3-small",
                    "usage": {
                        "prompt_tokens": 5,
                        "total_tokens": 5
                    }
                }"#,
            )
            .expect(1)
            .create_async()
            .await;

        let provider = OpenAIEmbeddingProvider::with_base_url("test-key".to_string(), server.url())
            .unwrap()
            .with_max_retries(1);

        let request = EmbeddingRequest {
            model: "text-embedding-3-small".to_string(),
            input: EmbeddingInput::Single {
                input: "test".to_string(),
            },
            dimensions: None,
            extra: HashMap::new(),
        };

        let response = provider.embed(request).await.unwrap();
        assert_eq!(response.embeddings.len(), 1);

        mock_fail.assert_async().await;
        mock_success.assert_async().await;
    }

    #[tokio::test]
    async fn test_batch_size_validation() {
        let provider = OpenAIEmbeddingProvider::new("test-key".to_string()).unwrap();

        // Create a batch that exceeds the limit
        let large_batch: Vec<String> = (0..=OPENAI_MAX_BATCH_SIZE)
            .map(|i| format!("Text {}", i))
            .collect();

        let request = EmbeddingRequest {
            model: "text-embedding-3-small".to_string(),
            input: EmbeddingInput::Batch { input: large_batch },
            dimensions: None,
            extra: HashMap::new(),
        };

        let result = provider.embed(request).await;
        assert!(matches!(result, Err(ProviderError::InvalidRequest(_))));
    }

    #[tokio::test]
    async fn test_embedding_index_ordering() {
        let mut server = Server::new_async().await;
        // Return embeddings in reversed order to test sorting
        let mock = server
            .mock("POST", "/embeddings")
            .with_status(200)
            .with_body(
                r#"{
                    "object": "list",
                    "data": [
                        {
                            "object": "embedding",
                            "index": 2,
                            "embedding": [0.5, 0.6]
                        },
                        {
                            "object": "embedding",
                            "index": 0,
                            "embedding": [0.1, 0.2]
                        },
                        {
                            "object": "embedding",
                            "index": 1,
                            "embedding": [0.3, 0.4]
                        }
                    ],
                    "model": "text-embedding-3-small",
                    "usage": {
                        "prompt_tokens": 15,
                        "total_tokens": 15
                    }
                }"#,
            )
            .create_async()
            .await;

        let provider =
            OpenAIEmbeddingProvider::with_base_url("test-key".to_string(), server.url()).unwrap();

        let request = EmbeddingRequest {
            model: "text-embedding-3-small".to_string(),
            input: EmbeddingInput::Batch {
                input: vec![
                    "First".to_string(),
                    "Second".to_string(),
                    "Third".to_string(),
                ],
            },
            dimensions: None,
            extra: HashMap::new(),
        };

        let response = provider.embed(request).await.unwrap();

        // Verify embeddings are sorted correctly by index
        assert_eq!(response.embeddings[0], vec![0.1, 0.2]);
        assert_eq!(response.embeddings[1], vec![0.3, 0.4]);
        assert_eq!(response.embeddings[2], vec![0.5, 0.6]);

        mock.assert_async().await;
    }
}
