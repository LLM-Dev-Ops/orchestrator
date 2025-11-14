// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! LLM provider integrations for LLM Orchestrator.

pub mod anthropic;
pub mod openai;
pub mod traits;

// Re-exports
pub use anthropic::AnthropicProvider;
pub use openai::OpenAIProvider;
pub use traits::{CompletionRequest, CompletionResponse, LLMProvider, ProviderError};

/// Library version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
