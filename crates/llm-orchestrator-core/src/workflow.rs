// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Workflow definition types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A complete workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Unique workflow identifier.
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,

    /// Workflow name.
    pub name: String,

    /// Workflow version (semantic versioning).
    #[serde(default = "default_version")]
    pub version: String,

    /// Description of what this workflow does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// List of workflow steps.
    pub steps: Vec<Step>,

    /// Default timeout for the entire workflow (in seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,

    /// Workflow metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_version() -> String {
    "1.0".to_string()
}

/// A single step in a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// Unique step identifier within the workflow.
    pub id: String,

    /// Step type (llm, embed, vector_search, transform, etc.).
    #[serde(rename = "type")]
    pub step_type: StepType,

    /// Dependencies on other steps (step IDs).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,

    /// Conditional execution (template expression).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,

    /// Step configuration (provider, model, prompt, etc.).
    #[serde(flatten)]
    pub config: StepConfig,

    /// Output variable name(s).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output: Vec<String>,

    /// Step-specific timeout (in seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,

    /// Retry configuration for this step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryConfig>,
}

/// Step type enumeration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    /// LLM completion step.
    Llm,

    /// Embedding generation step.
    Embed,

    /// Vector database search.
    VectorSearch,

    /// Data transformation step.
    Transform,

    /// Action/side-effect step (log, notify, etc.).
    Action,

    /// Parallel execution group.
    Parallel,

    /// Conditional branch.
    Branch,
}

/// Step configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StepConfig {
    /// LLM step configuration.
    Llm(LlmStepConfig),

    /// Embed step configuration.
    Embed(EmbedStepConfig),

    /// Vector search configuration.
    VectorSearch(VectorSearchConfig),

    /// Transform configuration.
    Transform(TransformConfig),

    /// Action configuration.
    Action(ActionConfig),

    /// Parallel group configuration.
    Parallel(ParallelConfig),

    /// Branch configuration.
    Branch(BranchConfig),
}

/// LLM step configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmStepConfig {
    /// LLM provider (openai, anthropic, cohere, etc.).
    pub provider: String,

    /// Model name.
    pub model: String,

    /// Prompt template (supports Handlebars syntax).
    pub prompt: String,

    /// Temperature parameter (0.0 - 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// System prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// Whether to stream the response.
    #[serde(default)]
    pub stream: bool,

    /// Additional provider-specific parameters.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Embedding step configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedStepConfig {
    /// Embedding provider.
    pub provider: String,

    /// Embedding model.
    pub model: String,

    /// Input text or template.
    pub input: String,

    /// Optional dimension reduction (for providers that support it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,

    /// Batch size for processing multiple texts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<usize>,
}

/// Vector database search configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchConfig {
    /// Vector database provider (pinecone, weaviate, etc.).
    pub database: String,

    /// Index/collection name.
    pub index: String,

    /// Query embedding (from previous step).
    pub query: String,

    /// Number of results to return.
    #[serde(default = "default_top_k")]
    pub top_k: usize,

    /// Optional metadata filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<serde_json::Value>,

    /// Namespace/partition for the search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    /// Include metadata in results.
    #[serde(default = "default_true")]
    pub include_metadata: bool,

    /// Include vector embeddings in results.
    #[serde(default)]
    pub include_vectors: bool,
}

fn default_top_k() -> usize {
    5
}

fn default_true() -> bool {
    true
}

/// Transform configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformConfig {
    /// Transform function name (concat, merge, filter, etc.).
    pub function: String,

    /// Input variables.
    pub inputs: Vec<String>,

    /// Transform-specific parameters.
    #[serde(flatten)]
    pub params: HashMap<String, serde_json::Value>,
}

/// Action configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionConfig {
    /// Action type (log, notify, publish, etc.).
    pub action: String,

    /// Action-specific parameters.
    #[serde(flatten)]
    pub params: HashMap<String, serde_json::Value>,
}

/// Parallel execution configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelConfig {
    /// Parallel tasks.
    pub tasks: Vec<Step>,

    /// Maximum concurrency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_concurrency: Option<usize>,
}

/// Branch configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchConfig {
    /// Condition to evaluate.
    pub condition: String,

    /// Branch mappings (condition value -> steps).
    pub branches: HashMap<String, Vec<Step>>,
}

/// Retry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts.
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,

    /// Backoff strategy.
    #[serde(default)]
    pub backoff: BackoffStrategy,

    /// Initial delay in milliseconds.
    #[serde(default = "default_initial_delay_ms")]
    pub initial_delay_ms: u64,

    /// Maximum delay in milliseconds.
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,
}

fn default_max_attempts() -> u32 {
    3
}

fn default_initial_delay_ms() -> u64 {
    100
}

fn default_max_delay_ms() -> u64 {
    30000
}

/// Backoff strategy for retries.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackoffStrategy {
    /// Exponential backoff (2^n * initial_delay).
    #[default]
    Exponential,

    /// Linear backoff (n * initial_delay).
    Linear,

    /// Constant delay.
    Constant,
}

impl Workflow {
    /// Create a new workflow.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            version: default_version(),
            description: None,
            steps: Vec::new(),
            timeout_seconds: None,
            metadata: HashMap::new(),
        }
    }

    /// Load workflow from YAML string.
    pub fn from_yaml(yaml: &str) -> crate::error::Result<Self> {
        serde_yaml::from_str(yaml).map_err(|e| crate::error::OrchestratorError::parse(e.to_string()))
    }

    /// Load workflow from JSON string.
    pub fn from_json(json: &str) -> crate::error::Result<Self> {
        serde_json::from_str(json).map_err(|e| crate::error::OrchestratorError::parse(e.to_string()))
    }

    /// Convert workflow to YAML string.
    pub fn to_yaml(&self) -> crate::error::Result<String> {
        serde_yaml::to_string(self).map_err(|e| crate::error::OrchestratorError::serialization(e.to_string()))
    }

    /// Convert workflow to JSON string.
    pub fn to_json(&self) -> crate::error::Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| crate::error::OrchestratorError::serialization(e.to_string()))
    }

    /// Get a step by ID.
    pub fn get_step(&self, id: &str) -> Option<&Step> {
        self.steps.iter().find(|s| s.id == id)
    }

    /// Get all step IDs.
    pub fn step_ids(&self) -> Vec<String> {
        self.steps.iter().map(|s| s.id.clone()).collect()
    }

    /// Validate workflow structure.
    pub fn validate(&self) -> crate::error::Result<()> {
        // Check for empty steps
        if self.steps.is_empty() {
            return Err(crate::error::OrchestratorError::validation("Workflow has no steps"));
        }

        // Check for duplicate step IDs
        let mut seen = std::collections::HashSet::new();
        for step in &self.steps {
            if !seen.insert(&step.id) {
                return Err(crate::error::OrchestratorError::validation(format!("Duplicate step ID: {}", step.id)));
            }
        }

        // Check that dependencies reference valid steps
        for step in &self.steps {
            for dep in &step.depends_on {
                if !seen.contains(dep) {
                    return Err(crate::error::OrchestratorError::validation(format!("Step '{}' depends on non-existent step '{}'", step.id, dep)));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_creation() {
        let workflow = Workflow::new("test-workflow");
        assert_eq!(workflow.name, "test-workflow");
        assert_eq!(workflow.version, "1.0");
        assert!(workflow.steps.is_empty());
    }

    #[test]
    fn test_workflow_yaml_parsing() {
        let yaml = r#"
name: "test-workflow"
version: "1.0"
steps:
  - id: "step1"
    type: "llm"
    provider: "openai"
    model: "gpt-4"
    prompt: "Hello {{ name }}"
    output: ["greeting"]
"#;

        let workflow = Workflow::from_yaml(yaml).unwrap();
        assert_eq!(workflow.name, "test-workflow");
        assert_eq!(workflow.steps.len(), 1);
        assert_eq!(workflow.steps[0].id, "step1");
    }

    #[test]
    fn test_workflow_validation() {
        let mut workflow = Workflow::new("test");
        let result = workflow.validate();
        assert!(result.is_err()); // Empty workflow should fail

        workflow.steps.push(Step {
            id: "step1".to_string(),
            step_type: StepType::Llm,
            depends_on: vec![],
            condition: None,
            config: StepConfig::Llm(LlmStepConfig {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                prompt: "test".to_string(),
                temperature: None,
                max_tokens: None,
                system: None,
                stream: false,
                extra: HashMap::new(),
            }),
            output: vec!["result".to_string()],
            timeout_seconds: None,
            retry: None,
        });

        let result = workflow.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_duplicate_step_id_validation() {
        let mut workflow = Workflow::new("test");
        let step = Step {
            id: "step1".to_string(),
            step_type: StepType::Llm,
            depends_on: vec![],
            condition: None,
            config: StepConfig::Llm(LlmStepConfig {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                prompt: "test".to_string(),
                temperature: None,
                max_tokens: None,
                system: None,
                stream: false,
                extra: HashMap::new(),
            }),
            output: vec![],
            timeout_seconds: None,
            retry: None,
        };

        workflow.steps.push(step.clone());
        workflow.steps.push(step.clone());

        let result = workflow.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_dependency_validation() {
        let mut workflow = Workflow::new("test");
        workflow.steps.push(Step {
            id: "step1".to_string(),
            step_type: StepType::Llm,
            depends_on: vec!["nonexistent".to_string()],
            condition: None,
            config: StepConfig::Llm(LlmStepConfig {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                prompt: "test".to_string(),
                temperature: None,
                max_tokens: None,
                system: None,
                stream: false,
                extra: HashMap::new(),
            }),
            output: vec![],
            timeout_seconds: None,
            retry: None,
        });

        let result = workflow.validate();
        assert!(result.is_err());
    }
}
