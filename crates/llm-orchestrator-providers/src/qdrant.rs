// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Qdrant vector database client implementation.

use crate::traits::*;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Qdrant vector database client.
pub struct QdrantClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl QdrantClient {
    /// Create a new Qdrant client.
    ///
    /// # Arguments
    /// * `base_url` - Qdrant instance URL (e.g., "http://localhost:6333")
    /// * `api_key` - Optional API key for authentication
    pub fn new(base_url: String, api_key: Option<String>) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ProviderError::HttpError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            base_url,
            api_key,
        })
    }
}

#[async_trait]
impl VectorSearchProvider for QdrantClient {
    async fn search(&self, request: VectorSearchRequest) -> Result<VectorSearchResponse, ProviderError> {
        // Build Qdrant search request
        let api_request = QdrantSearchRequest {
            vector: request.query,
            limit: request.top_k,
            filter: request.filter.clone(),
            with_payload: request.include_metadata,
            with_vector: request.include_vectors,
        };

        let url = format!("{}/collections/{}/points/search", self.base_url, request.index);

        let mut req_builder = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&api_request);

        if let Some(api_key) = &self.api_key {
            req_builder = req_builder.header("api-key", api_key);
        }

        let response = req_builder
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

        let api_response: QdrantSearchResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::SerializationError(e.to_string()))?;

        // Check for errors in response
        if let Some(status) = api_response.status {
            if status != "ok" {
                return Err(ProviderError::ProviderSpecific(format!("Qdrant error: {}", status)));
            }
        }

        // Convert to standard format
        let results = api_response
            .result
            .into_iter()
            .map(|r| SearchResult {
                id: r.id.to_string(),
                score: r.score,
                metadata: if request.include_metadata {
                    r.payload
                } else {
                    None
                },
                vector: if request.include_vectors {
                    r.vector
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
        // Save the count before moving request.vectors
        let vectors_count = request.vectors.len();

        // Build Qdrant upsert request
        let points: Vec<QdrantUpsertPoint> = request
            .vectors
            .into_iter()
            .map(|v| QdrantUpsertPoint {
                id: v.id.clone(),
                vector: v.vector,
                payload: v.metadata,
            })
            .collect();

        let api_request = QdrantUpsertRequest { points };

        let url = format!("{}/collections/{}/points", self.base_url, request.index);

        let mut req_builder = self.client
            .put(&url)
            .header("Content-Type", "application/json")
            .json(&api_request);

        if let Some(api_key) = &self.api_key {
            req_builder = req_builder.header("api-key", api_key);
        }

        let response = req_builder
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

        let api_response: QdrantUpsertResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::SerializationError(e.to_string()))?;

        // Check for errors in response
        if let Some(status_text) = api_response.status {
            if status_text != "ok" {
                return Err(ProviderError::ProviderSpecific(format!("Qdrant error: {}", status_text)));
            }
        }

        Ok(UpsertResponse {
            upserted_count: vectors_count,
            metadata: HashMap::new(),
        })
    }

    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResponse, ProviderError> {
        let api_request = QdrantDeleteRequest {
            points: request.ids.clone(),
        };

        let url = format!("{}/collections/{}/points/delete", self.base_url, request.index);

        let mut req_builder = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&api_request);

        if let Some(api_key) = &self.api_key {
            req_builder = req_builder.header("api-key", api_key);
        }

        let response = req_builder
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

        let api_response: QdrantDeleteResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::SerializationError(e.to_string()))?;

        // Check for errors in response
        if let Some(status_text) = api_response.status {
            if status_text != "ok" {
                return Err(ProviderError::ProviderSpecific(format!("Qdrant error: {}", status_text)));
            }
        }

        Ok(DeleteResponse {
            deleted_count: request.ids.len(),
            metadata: HashMap::new(),
        })
    }

    fn name(&self) -> &str {
        "qdrant"
    }
}

// Qdrant-specific request/response types

#[derive(Debug, Serialize)]
struct QdrantSearchRequest {
    vector: Vec<f32>,
    limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<serde_json::Value>,
    with_payload: bool,
    with_vector: bool,
}

#[derive(Debug, Deserialize)]
struct QdrantSearchResponse {
    result: Vec<QdrantPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QdrantPoint {
    id: QdrantPointId,
    score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vector: Option<Vec<f32>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum QdrantPointId {
    Uuid(uuid::Uuid),
    Integer(u64),
}

impl std::fmt::Display for QdrantPointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QdrantPointId::Uuid(id) => write!(f, "{}", id),
            QdrantPointId::Integer(id) => write!(f, "{}", id),
        }
    }
}

#[derive(Debug, Serialize)]
struct QdrantUpsertRequest {
    points: Vec<QdrantUpsertPoint>,
}

#[derive(Debug, Serialize)]
struct QdrantUpsertPoint {
    id: String,
    vector: Vec<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct QdrantUpsertResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

#[derive(Debug, Serialize)]
struct QdrantDeleteRequest {
    points: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct QdrantDeleteResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = QdrantClient::new(
            "http://localhost:6333".to_string(),
            None,
        )
        .unwrap();
        assert_eq!(client.name(), "qdrant");
    }

    #[test]
    fn test_client_with_api_key() {
        let client = QdrantClient::new(
            "http://localhost:6333".to_string(),
            Some("test-key".to_string()),
        )
        .unwrap();
        assert_eq!(client.name(), "qdrant");
    }

    #[test]
    fn test_upsert_request_serialization() {
        use serde_json::json;

        let upsert_req = QdrantUpsertRequest {
            points: vec![
                QdrantUpsertPoint {
                    id: "point1".to_string(),
                    vector: vec![0.1, 0.2, 0.3],
                    payload: Some(json!({"category": "test"})),
                },
            ],
        };

        let json_str = serde_json::to_string(&upsert_req).unwrap();
        assert!(json_str.contains("point1"));
        assert!(json_str.contains("points"));
    }

    #[test]
    fn test_delete_request_serialization() {
        let delete_req = QdrantDeleteRequest {
            points: vec!["id1".to_string(), "id2".to_string()],
        };

        let json_str = serde_json::to_string(&delete_req).unwrap();
        assert!(json_str.contains("id1"));
        assert!(json_str.contains("id2"));
        assert!(json_str.contains("points"));
    }

    #[test]
    fn test_qdrant_point_id_uuid() {
        let uuid_val = uuid::Uuid::new_v4();
        let point_id = QdrantPointId::Uuid(uuid_val);
        let id_str = point_id.to_string();
        assert!(!id_str.is_empty());
    }

    #[test]
    fn test_qdrant_point_id_integer() {
        let point_id = QdrantPointId::Integer(12345);
        let id_str = point_id.to_string();
        assert_eq!(id_str, "12345");
    }

    #[tokio::test]
    async fn test_search_request_formatting() {
        let client = QdrantClient::new(
            "http://localhost:6333".to_string(),
            None,
        )
        .unwrap();

        let request = VectorSearchRequest {
            index: "test-collection".to_string(),
            query: vec![0.1, 0.2, 0.3],
            top_k: 10,
            namespace: None,
            filter: None,
            include_metadata: true,
            include_vectors: false,
        };

        // Verify client and request are correctly structured
        assert_eq!(client.name(), "qdrant");
        assert_eq!(request.top_k, 10);
    }
}
