// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Provider trait definitions (re-exported from core).

// Re-export provider traits from core to maintain compatibility
pub use llm_orchestrator_core::providers::{
    CompletionRequest, CompletionResponse, LLMProvider, ProviderError,
    EmbeddingProvider, EmbeddingRequest, EmbeddingResponse, EmbeddingInput,
    VectorSearchProvider, VectorSearchRequest, VectorSearchResponse, SearchResult,
    UpsertRequest, UpsertResponse, VectorRecord,
    DeleteRequest, DeleteResponse,
};
