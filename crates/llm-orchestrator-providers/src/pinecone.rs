// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Pinecone vector database client implementation.

use crate::traits::*;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Pinecone vector database client.
pub struct PineconeClient {
    client: Client,
    api_key: String,
    environment: String,
}

impl PineconeClient {
    /// Create a new Pinecone client.
    ///
    /// # Arguments
    /// * `api_key` - Pinecone API key
    /// * `environment` - Pinecone environment (e.g., "us-west1-gcp")
    pub fn new(api_key: String, environment: String) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ProviderError::HttpError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            api_key,
            environment,
        })
    }

    /// Get the base URL for an index.
    fn get_index_url(&self, index: &str) -> String {
        format!("https://{}-{}.svc.{}.pinecone.io", index, "default", self.environment)
    }
}

#[async_trait]
impl VectorSearchProvider for PineconeClient {
    async fn search(&self, request: VectorSearchRequest) -> Result<VectorSearchResponse, ProviderError> {
        // Build Pinecone query request
        let api_request = PineconeQueryRequest {
            vector: request.query,
            top_k: request.top_k,
            namespace: request.namespace.clone(),
            filter: request.filter.clone(),
            include_metadata: request.include_metadata,
            include_values: request.include_vectors,
        };

        let url = format!("{}/query", self.get_index_url(&request.index));

        let response = self
            .client
            .post(&url)
            .header("Api-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&api_request)
            .send()
            .await
            .map_err(|e| ProviderError::HttpError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(match status.as_u16() {
                401 => ProviderError::AuthError(error_text),
                429 => ProviderError::RateLimitExceeded,
                400..=499 => ProviderError::InvalidRequest(error_text),
                _ => ProviderError::ProviderSpecific(error_text),
            });
        }

        let api_response: PineconeQueryResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::SerializationError(e.to_string()))?;

        // Convert to standard format
        let results = api_response
            .matches
            .into_iter()
            .map(|m| SearchResult {
                id: m.id,
                score: m.score,
                metadata: if request.include_metadata {
                    m.metadata
                } else {
                    None
                },
                vector: if request.include_vectors {
                    m.values
                } else {
                    None
                },
            })
            .collect();

        Ok(VectorSearchResponse {
            results,
            metadata: HashMap::new(),
        })
    }

    async fn upsert(&self, request: UpsertRequest) -> Result<UpsertResponse, ProviderError> {
        // Build Pinecone upsert request
        let vectors: Vec<PineconeVector> = request
            .vectors
            .into_iter()
            .map(|v| PineconeVector {
                id: v.id,
                values: v.vector,
                metadata: v.metadata,
            })
            .collect();

        let api_request = PineconeUpsertRequest {
            vectors,
            namespace: request.namespace,
        };

        let url = format!("{}/vectors/upsert", self.get_index_url(&request.index));

        let response = self
            .client
            .post(&url)
            .header("Api-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&api_request)
            .send()
            .await
            .map_err(|e| ProviderError::HttpError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(match status.as_u16() {
                401 => ProviderError::AuthError(error_text),
                429 => ProviderError::RateLimitExceeded,
                400..=499 => ProviderError::InvalidRequest(error_text),
                _ => ProviderError::ProviderSpecific(error_text),
            });
        }

        let api_response: PineconeUpsertResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::SerializationError(e.to_string()))?;

        Ok(UpsertResponse {
            upserted_count: api_response.upserted_count,
            metadata: HashMap::new(),
        })
    }

    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResponse, ProviderError> {
        // Save the count before moving request.ids
        let ids_count = request.ids.len();

        let api_request = if request.delete_all {
            PineconeDeleteRequest {
                delete_all: Some(true),
                ids: None,
                namespace: request.namespace,
            }
        } else {
            PineconeDeleteRequest {
                delete_all: None,
                ids: Some(request.ids),
                namespace: request.namespace,
            }
        };

        let url = format!("{}/vectors/delete", self.get_index_url(&request.index));

        let response = self
            .client
            .post(&url)
            .header("Api-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&api_request)
            .send()
            .await
            .map_err(|e| ProviderError::HttpError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(match status.as_u16() {
                401 => ProviderError::AuthError(error_text),
                429 => ProviderError::RateLimitExceeded,
                400..=499 => ProviderError::InvalidRequest(error_text),
                _ => ProviderError::ProviderSpecific(error_text),
            });
        }

        // Pinecone delete returns an empty object on success
        Ok(DeleteResponse {
            deleted_count: ids_count,
            metadata: HashMap::new(),
        })
    }

    fn name(&self) -> &str {
        "pinecone"
    }
}

// Pinecone-specific request/response types

#[derive(Debug, Serialize)]
struct PineconeQueryRequest {
    vector: Vec<f32>,
    #[serde(rename = "topK")]
    top_k: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<serde_json::Value>,
    #[serde(rename = "includeMetadata")]
    include_metadata: bool,
    #[serde(rename = "includeValues")]
    include_values: bool,
}

#[derive(Debug, Deserialize)]
struct PineconeQueryResponse {
    matches: Vec<PineconeMatch>,
}

#[derive(Debug, Deserialize)]
struct PineconeMatch {
    id: String,
    score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    values: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct PineconeUpsertRequest {
    vectors: Vec<PineconeVector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<String>,
}

#[derive(Debug, Serialize)]
struct PineconeVector {
    id: String,
    values: Vec<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct PineconeUpsertResponse {
    #[serde(rename = "upsertedCount")]
    upserted_count: usize,
}

#[derive(Debug, Serialize)]
struct PineconeDeleteRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "deleteAll")]
    delete_all: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_client_creation() {
        let client = PineconeClient::new(
            "test-key".to_string(),
            "us-west1-gcp".to_string(),
        )
        .unwrap();
        assert_eq!(client.name(), "pinecone");
    }

    #[test]
    fn test_index_url_generation() {
        let client = PineconeClient::new(
            "test-key".to_string(),
            "us-west1-gcp".to_string(),
        )
        .unwrap();
        let url = client.get_index_url("test-index");
        assert!(url.contains("test-index"));
        assert!(url.contains("us-west1-gcp"));
    }

    #[tokio::test]
    async fn test_search_request_building() {
        let client = PineconeClient::new(
            "test-key".to_string(),
            "us-west1-gcp".to_string(),
        )
        .unwrap();

        let request = VectorSearchRequest {
            index: "test-index".to_string(),
            query: vec![0.1, 0.2, 0.3],
            top_k: 10,
            namespace: Some("test-ns".to_string()),
            filter: Some(json!({"genre": "action"})),
            include_metadata: true,
            include_vectors: false,
        };

        // URL should be correctly formatted
        let url = client.get_index_url(&request.index);
        assert!(url.contains("test-index"));
    }

    #[test]
    fn test_upsert_request_serialization() {
        let upsert_req = PineconeUpsertRequest {
            vectors: vec![
                PineconeVector {
                    id: "vec1".to_string(),
                    values: vec![0.1, 0.2, 0.3],
                    metadata: Some(json!({"key": "value"})),
                },
            ],
            namespace: Some("test-ns".to_string()),
        };

        let json_str = serde_json::to_string(&upsert_req).unwrap();
        assert!(json_str.contains("vec1"));
        assert!(json_str.contains("namespace"));
    }

    #[test]
    fn test_delete_request_serialization() {
        let delete_req = PineconeDeleteRequest {
            ids: Some(vec!["id1".to_string(), "id2".to_string()]),
            delete_all: None,
            namespace: Some("test-ns".to_string()),
        };

        let json_str = serde_json::to_string(&delete_req).unwrap();
        assert!(json_str.contains("id1"));
        assert!(json_str.contains("id2"));
    }

    #[test]
    fn test_delete_all_request_serialization() {
        let delete_req = PineconeDeleteRequest {
            ids: None,
            delete_all: Some(true),
            namespace: Some("test-ns".to_string()),
        };

        let json_str = serde_json::to_string(&delete_req).unwrap();
        assert!(json_str.contains("deleteAll"));
    }
}
