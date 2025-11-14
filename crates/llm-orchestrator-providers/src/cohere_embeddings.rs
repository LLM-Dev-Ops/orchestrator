// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cohere embedding provider implementation.
//!
//! Supports:
//! - Models: embed-english-v3.0, embed-multilingual-v3.0, embed-english-light-v3.0
//! - Batch support: up to 96 inputs per request
//! - Input types: search_document, search_query, classification, clustering
//! - Automatic retries with exponential backoff

use crate::traits::*;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, warn};

/// Maximum batch size for Cohere embeddings API.
pub const COHERE_MAX_BATCH_SIZE: usize = 96;

/// Default retry configuration.
const MAX_RETRIES: u32 = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 1000;

/// Input type for Cohere embeddings.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CohereInputType {
    SearchDocument,
    SearchQuery,
    Classification,
    Clustering,
}

/// Cohere embedding provider.
pub struct CohereEmbeddingProvider {
    client: Client,
    api_key: String,
    base_url: String,
    max_retries: u32,
    input_type: CohereInputType,
}

impl CohereEmbeddingProvider {
    /// Create a new Cohere embedding provider.
    pub fn new(api_key: String) -> Result<Self, ProviderError> {
        Self::with_base_url(api_key, "https://api.cohere.ai/v1".to_string())
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
            input_type: CohereInputType::SearchDocument, // Default
        })
    }

    /// Set maximum number of retries for failed requests.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set input type for embeddings.
    pub fn with_input_type(mut self, input_type: CohereInputType) -> Self {
        self.input_type = input_type;
        self
    }

    /// Create from environment variables.
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = std::env::var("COHERE_API_KEY")
            .map_err(|_| ProviderError::AuthError("COHERE_API_KEY not set".to_string()))?;
        Self::new(api_key)
    }

    /// Perform a single embedding request with retries.
    async fn embed_with_retry(&self, api_request: &CohereEmbeddingRequest) -> Result<CohereEmbeddingResponse, ProviderError> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = Duration::from_millis(INITIAL_RETRY_DELAY_MS * 2_u64.pow(attempt - 1));
                warn!("Retry attempt {} after {}ms", attempt, delay.as_millis());
                tokio::time::sleep(delay).await;
            }

            let url = format!("{}/embed", self.base_url);

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

            match response.json::<CohereEmbeddingResponse>().await {
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
impl EmbeddingProvider for CohereEmbeddingProvider {
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, ProviderError> {
        // Extract input texts
        let texts = match &request.input {
            EmbeddingInput::Single { input } => vec![input.clone()],
            EmbeddingInput::Batch { input } => input.clone(),
        };

        // Check batch size
        if texts.len() > COHERE_MAX_BATCH_SIZE {
            return Err(ProviderError::InvalidRequest(format!(
                "Batch size {} exceeds Cohere maximum of {}",
                texts.len(),
                COHERE_MAX_BATCH_SIZE
            )));
        }

        debug!(
            "Embedding {} texts with model {} (input_type: {:?})",
            texts.len(),
            request.model,
            self.input_type
        );

        // Build Cohere API request
        let api_request = CohereEmbeddingRequest {
            model: request.model.clone(),
            texts,
            input_type: Some(self.input_type.clone()),
            truncate: Some(CohereTruncate::End),
        };

        // Execute request with retries
        let api_response = self.embed_with_retry(&api_request).await?;

        // Extract token count from metadata if available
        let tokens_used = api_response
            .meta
            .as_ref()
            .and_then(|m| m.billed_units.as_ref())
            .and_then(|b| b.input_tokens);

        debug!(
            "Successfully embedded {} texts, used {} tokens",
            api_response.embeddings.len(),
            tokens_used.unwrap_or(0)
        );

        Ok(EmbeddingResponse {
            embeddings: api_response.embeddings,
            model: request.model.clone(), // Cohere doesn't return model in response
            tokens_used,
            metadata: HashMap::new(),
        })
    }

    fn name(&self) -> &str {
        "cohere_embeddings"
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        // Simple health check: try to embed a single short text
        let request = EmbeddingRequest {
            model: "embed-english-v3.0".to_string(),
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

// Cohere-specific request/response types

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
#[serde(rename_all = "UPPERCASE")]
enum CohereTruncate {
    None,
    Start,
    End,
}

#[derive(Debug, Serialize)]
struct CohereEmbeddingRequest {
    model: String,
    texts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_type: Option<CohereInputType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncate: Option<CohereTruncate>,
}

#[derive(Debug, Deserialize)]
struct CohereEmbeddingResponse {
    embeddings: Vec<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<CohereMeta>,
}

#[derive(Debug, Deserialize)]
struct CohereMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    billed_units: Option<CohereBilledUnits>,
}

#[derive(Debug, Deserialize)]
struct CohereBilledUnits {
    #[serde(skip_serializing_if = "Option::is_none")]
    input_tokens: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[test]
    fn test_provider_creation() {
        let provider = CohereEmbeddingProvider::new("test-key".to_string()).unwrap();
        assert_eq!(provider.name(), "cohere_embeddings");
        assert_eq!(provider.max_retries, MAX_RETRIES);
    }

    #[test]
    fn test_provider_with_custom_base_url() {
        let provider = CohereEmbeddingProvider::with_base_url(
            "test-key".to_string(),
            "https://custom.api.com".to_string(),
        )
        .unwrap();
        assert_eq!(provider.name(), "cohere_embeddings");
    }

    #[test]
    fn test_provider_with_max_retries() {
        let provider = CohereEmbeddingProvider::new("test-key".to_string())
            .unwrap()
            .with_max_retries(5);
        assert_eq!(provider.max_retries, 5);
    }

    #[test]
    fn test_provider_with_input_type() {
        let provider = CohereEmbeddingProvider::new("test-key".to_string())
            .unwrap()
            .with_input_type(CohereInputType::SearchQuery);
        // Provider was created successfully
        assert_eq!(provider.name(), "cohere_embeddings");
    }

    #[tokio::test]
    async fn test_single_text_embedding() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/embed")
            .match_header("authorization", "Bearer test-key")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body(
                r#"{
                    "embeddings": [[0.1, 0.2, 0.3]],
                    "meta": {
                        "billed_units": {
                            "input_tokens": 5
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let provider =
            CohereEmbeddingProvider::with_base_url("test-key".to_string(), server.url()).unwrap();

        let request = EmbeddingRequest {
            model: "embed-english-v3.0".to_string(),
            input: EmbeddingInput::Single {
                input: "Hello, world!".to_string(),
            },
            dimensions: None,
            extra: HashMap::new(),
        };

        let response = provider.embed(request).await.unwrap();

        assert_eq!(response.embeddings.len(), 1);
        assert_eq!(response.embeddings[0], vec![0.1, 0.2, 0.3]);
        assert_eq!(response.model, "embed-english-v3.0");
        assert_eq!(response.tokens_used, Some(5));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_batch_embedding() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/embed")
            .with_status(200)
            .with_body(
                r#"{
                    "embeddings": [
                        [0.1, 0.2],
                        [0.3, 0.4]
                    ],
                    "meta": {
                        "billed_units": {
                            "input_tokens": 10
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let provider =
            CohereEmbeddingProvider::with_base_url("test-key".to_string(), server.url()).unwrap();

        let request = EmbeddingRequest {
            model: "embed-english-v3.0".to_string(),
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
    async fn test_search_query_input_type() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/embed")
            .match_body(mockito::Matcher::PartialJsonString(
                r#""input_type":"search_query""#.to_string(),
            ))
            .with_status(200)
            .with_body(
                r#"{
                    "embeddings": [[0.1, 0.2, 0.3]],
                    "meta": {
                        "billed_units": {
                            "input_tokens": 3
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let provider = CohereEmbeddingProvider::with_base_url("test-key".to_string(), server.url())
            .unwrap()
            .with_input_type(CohereInputType::SearchQuery);

        let request = EmbeddingRequest {
            model: "embed-english-v3.0".to_string(),
            input: EmbeddingInput::Single {
                input: "search query".to_string(),
            },
            dimensions: None,
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
            .mock("POST", "/embed")
            .with_status(401)
            .with_body(r#"{"message": "Invalid API key"}"#)
            .create_async()
            .await;

        let provider =
            CohereEmbeddingProvider::with_base_url("bad-key".to_string(), server.url()).unwrap();

        let request = EmbeddingRequest {
            model: "embed-english-v3.0".to_string(),
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
            .mock("POST", "/embed")
            .with_status(429)
            .with_body(r#"{"message": "Rate limit exceeded"}"#)
            .expect(1)
            .create_async()
            .await;

        let mock_success = server
            .mock("POST", "/embed")
            .with_status(200)
            .with_body(
                r#"{
                    "embeddings": [[0.1, 0.2, 0.3]],
                    "meta": {
                        "billed_units": {
                            "input_tokens": 5
                        }
                    }
                }"#,
            )
            .expect(1)
            .create_async()
            .await;

        let provider = CohereEmbeddingProvider::with_base_url("test-key".to_string(), server.url())
            .unwrap()
            .with_max_retries(1);

        let request = EmbeddingRequest {
            model: "embed-english-v3.0".to_string(),
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
        let provider = CohereEmbeddingProvider::new("test-key".to_string()).unwrap();

        // Create a batch that exceeds the limit
        let large_batch: Vec<String> = (0..=COHERE_MAX_BATCH_SIZE)
            .map(|i| format!("Text {}", i))
            .collect();

        let request = EmbeddingRequest {
            model: "embed-english-v3.0".to_string(),
            input: EmbeddingInput::Batch { input: large_batch },
            dimensions: None,
            extra: HashMap::new(),
        };

        let result = provider.embed(request).await;
        assert!(matches!(result, Err(ProviderError::InvalidRequest(_))));
    }

    #[tokio::test]
    async fn test_response_without_metadata() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/embed")
            .with_status(200)
            .with_body(
                r#"{
                    "embeddings": [[0.1, 0.2, 0.3]]
                }"#,
            )
            .create_async()
            .await;

        let provider =
            CohereEmbeddingProvider::with_base_url("test-key".to_string(), server.url()).unwrap();

        let request = EmbeddingRequest {
            model: "embed-english-v3.0".to_string(),
            input: EmbeddingInput::Single {
                input: "test".to_string(),
            },
            dimensions: None,
            extra: HashMap::new(),
        };

        let response = provider.embed(request).await.unwrap();
        assert_eq!(response.embeddings.len(), 1);
        assert_eq!(response.tokens_used, None);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_multilingual_model() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/embed")
            .match_body(mockito::Matcher::PartialJsonString(
                r#""model":"embed-multilingual-v3.0""#.to_string(),
            ))
            .with_status(200)
            .with_body(
                r#"{
                    "embeddings": [[0.1, 0.2, 0.3, 0.4]],
                    "meta": {
                        "billed_units": {
                            "input_tokens": 8
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let provider =
            CohereEmbeddingProvider::with_base_url("test-key".to_string(), server.url()).unwrap();

        let request = EmbeddingRequest {
            model: "embed-multilingual-v3.0".to_string(),
            input: EmbeddingInput::Single {
                input: "Bonjour le monde".to_string(),
            },
            dimensions: None,
            extra: HashMap::new(),
        };

        let response = provider.embed(request).await.unwrap();
        assert_eq!(response.embeddings.len(), 1);
        assert_eq!(response.embeddings[0], vec![0.1, 0.2, 0.3, 0.4]);

        mock.assert_async().await;
    }
}
