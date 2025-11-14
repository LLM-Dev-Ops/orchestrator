// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! LLM provider integrations for LLM Orchestrator.

// LLM providers
pub mod anthropic;
pub mod openai;

// Embedding providers
pub mod openai_embeddings;
pub mod cohere_embeddings;

// Vector database clients
pub mod pinecone;
pub mod weaviate;
pub mod qdrant;

// Traits
pub mod traits;

// Re-exports
pub use anthropic::AnthropicProvider;
pub use openai::OpenAIProvider;
pub use openai_embeddings::OpenAIEmbeddingProvider;
pub use cohere_embeddings::CohereEmbeddingProvider;
pub use pinecone::PineconeClient;
pub use weaviate::WeaviateClient;
pub use qdrant::QdrantClient;
pub use traits::{
    CompletionRequest, CompletionResponse, LLMProvider, ProviderError,
    EmbeddingProvider, EmbeddingRequest, EmbeddingResponse, EmbeddingInput,
    VectorSearchProvider, VectorSearchRequest, VectorSearchResponse, SearchResult,
    UpsertRequest, UpsertResponse, VectorRecord,
    DeleteRequest, DeleteResponse,
};

/// Library version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
