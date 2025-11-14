// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Weaviate vector database client implementation.

use crate::traits::*;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

/// Weaviate vector database client.
pub struct WeaviateClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl WeaviateClient {
    /// Create a new Weaviate client.
    ///
    /// # Arguments
    /// * `base_url` - Weaviate instance URL (e.g., "http://localhost:8080")
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
impl VectorSearchProvider for WeaviateClient {
    async fn search(&self, request: VectorSearchRequest) -> Result<VectorSearchResponse, ProviderError> {
        // Build Weaviate GraphQL query
        let fields = if request.include_metadata {
            "_additional { id distance } ... on * { * }"
        } else {
            "_additional { id distance }"
        };

        let vector_str = format!("[{}]",
            request.query.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        let where_clause = if let Some(filter) = &request.filter {
            format!(", where: {}", serde_json::to_string(filter)
                .map_err(|e| ProviderError::SerializationError(e.to_string()))?)
        } else {
            String::new()
        };

        let query = format!(
            r#"{{
                Get {{
                    {} (
                        nearVector: {{ vector: {} }}
                        limit: {}
                        {}
                    ) {{
                        {}
                    }}
                }}
            }}"#,
            request.index,
            vector_str,
            request.top_k,
            where_clause,
            fields
        );

        let graphql_request = json!({
            "query": query
        });

        let url = format!("{}/v1/graphql", self.base_url);

        let mut req_builder = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&graphql_request);

        if let Some(api_key) = &self.api_key {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
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

        let api_response: WeaviateQueryResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::SerializationError(e.to_string()))?;

        // Check for GraphQL errors
        if let Some(errors) = api_response.errors {
            return Err(ProviderError::ProviderSpecific(
                serde_json::to_string(&errors).unwrap_or_else(|_| "GraphQL error".to_string())
            ));
        }

        // Extract results from GraphQL response
        let results = api_response
            .data
            .and_then(|d| d.get("Get").cloned())
            .and_then(|g| g.get(&request.index).cloned())
            .and_then(|r| r.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| {
                let obj = item.as_object()?;
                let additional = obj.get("_additional")?.as_object()?;
                let id = additional.get("id")?.as_str()?.to_string();
                let distance = additional.get("distance")?.as_f64()? as f32;

                // Convert distance to similarity score (Weaviate uses cosine distance)
                let score = 1.0 - distance;

                // Extract metadata (everything except _additional)
                let mut metadata = serde_json::Map::new();
                for (key, value) in obj.iter() {
                    if key != "_additional" {
                        metadata.insert(key.clone(), value.clone());
                    }
                }

                Some(SearchResult {
                    id,
                    score,
                    metadata: if request.include_metadata && !metadata.is_empty() {
                        Some(serde_json::Value::Object(metadata))
                    } else {
                        None
                    },
                    vector: None, // Weaviate doesn't return vectors in this query
                })
            })
            .collect();

        Ok(VectorSearchResponse {
            results,
            metadata: HashMap::new(),
        })
    }

    async fn upsert(&self, request: UpsertRequest) -> Result<UpsertResponse, ProviderError> {
        // Weaviate uses batch import
        let objects: Vec<WeaviateObject> = request
            .vectors
            .into_iter()
            .map(|v| {
                let mut properties = serde_json::Map::new();
                if let Some(serde_json::Value::Object(map)) = v.metadata {
                    properties = map;
                }

                WeaviateObject {
                    id: Some(v.id),
                    class: request.index.clone(),
                    properties,
                    vector: Some(v.vector),
                }
            })
            .collect();

        let api_request = WeaviateBatchRequest { objects };

        let url = format!("{}/v1/batch/objects", self.base_url);

        let mut req_builder = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&api_request);

        if let Some(api_key) = &self.api_key {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
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

        let api_response: Vec<WeaviateBatchResponse> = response
            .json()
            .await
            .map_err(|e| ProviderError::SerializationError(e.to_string()))?;

        let upserted_count = api_response
            .iter()
            .filter(|r| r.result.status == "SUCCESS")
            .count();

        Ok(UpsertResponse {
            upserted_count,
            metadata: HashMap::new(),
        })
    }

    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResponse, ProviderError> {
        let mut deleted_count = 0;

        // Weaviate requires deleting objects one by one
        for id in &request.ids {
            let url = format!("{}/v1/objects/{}/{}", self.base_url, request.index, id);

            let mut req_builder = self.client.delete(&url);

            if let Some(api_key) = &self.api_key {
                req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
            }

            let response = req_builder
                .send()
                .await
                .map_err(|e| ProviderError::HttpError(e.to_string()))?;

            if response.status().is_success() {
                deleted_count += 1;
            }
        }

        Ok(DeleteResponse {
            deleted_count,
            metadata: HashMap::new(),
        })
    }

    fn name(&self) -> &str {
        "weaviate"
    }
}

// Weaviate-specific request/response types

#[derive(Debug, Deserialize)]
struct WeaviateQueryResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize)]
struct WeaviateBatchRequest {
    objects: Vec<WeaviateObject>,
}

#[derive(Debug, Serialize)]
struct WeaviateObject {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    class: String,
    properties: serde_json::Map<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vector: Option<Vec<f32>>,
}

#[derive(Debug, Deserialize)]
struct WeaviateBatchResponse {
    result: WeaviateBatchResult,
}

#[derive(Debug, Deserialize)]
struct WeaviateBatchResult {
    status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = WeaviateClient::new(
            "http://localhost:8080".to_string(),
            None,
        )
        .unwrap();
        assert_eq!(client.name(), "weaviate");
    }

    #[test]
    fn test_client_with_api_key() {
        let client = WeaviateClient::new(
            "http://localhost:8080".to_string(),
            Some("test-key".to_string()),
        )
        .unwrap();
        assert_eq!(client.name(), "weaviate");
    }

    #[test]
    fn test_batch_request_serialization() {
        use serde_json::json;

        let objects = vec![
            WeaviateObject {
                id: Some("id1".to_string()),
                class: "Article".to_string(),
                properties: {
                    let mut map = serde_json::Map::new();
                    map.insert("title".to_string(), json!("Test Article"));
                    map
                },
                vector: Some(vec![0.1, 0.2, 0.3]),
            },
        ];

        let batch_req = WeaviateBatchRequest { objects };
        let json_str = serde_json::to_string(&batch_req).unwrap();
        assert!(json_str.contains("id1"));
        assert!(json_str.contains("Article"));
    }

    #[test]
    fn test_weaviate_object_with_metadata() {
        use serde_json::json;

        let obj = WeaviateObject {
            id: Some("test-id".to_string()),
            class: "TestClass".to_string(),
            properties: {
                let mut map = serde_json::Map::new();
                map.insert("key1".to_string(), json!("value1"));
                map.insert("key2".to_string(), json!(42));
                map
            },
            vector: Some(vec![0.1, 0.2]),
        };

        let json_str = serde_json::to_string(&obj).unwrap();
        assert!(json_str.contains("test-id"));
        assert!(json_str.contains("key1"));
        assert!(json_str.contains("value1"));
    }
}
