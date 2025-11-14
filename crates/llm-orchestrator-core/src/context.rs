// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Execution context management for workflows.

use crate::error::{OrchestratorError, Result};
use handlebars::Handlebars;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Execution context for a workflow run.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Input variables provided to the workflow.
    inputs: Arc<RwLock<HashMap<String, Value>>>,

    /// Output values from completed steps.
    outputs: Arc<RwLock<HashMap<String, Value>>>,

    /// Template renderer.
    renderer: Arc<Handlebars<'static>>,

    /// Workflow metadata.
    metadata: Arc<RwLock<HashMap<String, Value>>>,
}

impl ExecutionContext {
    /// Create a new execution context.
    pub fn new(inputs: HashMap<String, Value>) -> Self {
        let mut renderer = Handlebars::new();
        // Disable HTML escaping for LLM prompts
        renderer.register_escape_fn(handlebars::no_escape);

        Self {
            inputs: Arc::new(RwLock::new(inputs)),
            outputs: Arc::new(RwLock::new(HashMap::new())),
            renderer: Arc::new(renderer),
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set an output value for a step.
    pub fn set_output(&self, step_id: impl Into<String>, value: Value) {
        let mut outputs = self.outputs.write();
        outputs.insert(step_id.into(), value);
    }

    /// Get an output value from a step.
    pub fn get_output(&self, step_id: &str) -> Option<Value> {
        let outputs = self.outputs.read();
        outputs.get(step_id).cloned()
    }

    /// Get an input value.
    pub fn get_input(&self, key: &str) -> Option<Value> {
        let inputs = self.inputs.read();
        inputs.get(key).cloned()
    }

    /// Render a template string with the current context.
    pub fn render_template(&self, template: &str) -> Result<String> {
        // Build context data for rendering
        let mut context_data = serde_json::Map::new();

        // Add inputs (flat at root level for backward compatibility)
        let inputs = self.inputs.read();
        for (key, value) in inputs.iter() {
            context_data.insert(key.clone(), value.clone());
        }

        // Add inputs under "inputs" key for explicit access
        if !inputs.is_empty() {
            let inputs_map: serde_json::Map<String, Value> = inputs.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            context_data.insert("inputs".to_string(), Value::Object(inputs_map));
        }

        // Add outputs under both "outputs" and "steps" keys
        let outputs = self.outputs.read();
        if !outputs.is_empty() {
            let mut outputs_map = serde_json::Map::new();
            for (step_id, value) in outputs.iter() {
                outputs_map.insert(step_id.clone(), value.clone());
            }

            // Support both {{outputs.step_id}} (backward compat) and {{steps.step_id.field}} (new)
            context_data.insert("outputs".to_string(), Value::Object(outputs_map.clone()));
            context_data.insert("steps".to_string(), Value::Object(outputs_map));
        }

        // Render template
        self.renderer
            .render_template(template, &Value::Object(context_data))
            .map_err(|e| OrchestratorError::template(e.to_string()))
    }

    /// Evaluate a condition expression.
    pub fn evaluate_condition(&self, condition: &str) -> Result<bool> {
        // For MVP, support simple equality checks
        // e.g., "{{ sentiment }} == 'positive'"
        // Full expression evaluation can be added later

        let rendered = self.render_template(condition)?;
        let trimmed = rendered.trim();

        // Simple boolean evaluation
        match trimmed.to_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(true),
            "false" | "0" | "no" | "" => Ok(false),
            _ => {
                // Try to evaluate as equality expression
                if let Some((left, right)) = trimmed.split_once("==") {
                    Ok(left.trim() == right.trim().trim_matches('\'').trim_matches('\"'))
                } else if let Some((left, right)) = trimmed.split_once("!=") {
                    Ok(left.trim() != right.trim().trim_matches('\'').trim_matches('\"'))
                } else {
                    // Treat non-empty string as true
                    Ok(!trimmed.is_empty())
                }
            }
        }
    }

    /// Set metadata value.
    pub fn set_metadata(&self, key: impl Into<String>, value: Value) {
        let mut metadata = self.metadata.write();
        metadata.insert(key.into(), value);
    }

    /// Get metadata value.
    pub fn get_metadata(&self, key: &str) -> Option<Value> {
        let metadata = self.metadata.read();
        metadata.get(key).cloned()
    }

    /// Get all outputs.
    pub fn all_outputs(&self) -> HashMap<String, Value> {
        self.outputs.read().clone()
    }

    /// Get all inputs.
    pub fn all_inputs(&self) -> HashMap<String, Value> {
        self.inputs.read().clone()
    }

    /// Clear all outputs (useful for testing).
    #[cfg(test)]
    pub fn clear_outputs(&self) {
        let mut outputs = self.outputs.write();
        outputs.clear();
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new(HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_context_creation() {
        let mut inputs = HashMap::new();
        inputs.insert("name".to_string(), json!("Alice"));

        let ctx = ExecutionContext::new(inputs);
        assert_eq!(ctx.get_input("name"), Some(json!("Alice")));
    }

    #[test]
    fn test_output_management() {
        let ctx = ExecutionContext::default();

        ctx.set_output("step1", json!("result1"));
        ctx.set_output("step2", json!("result2"));

        assert_eq!(ctx.get_output("step1"), Some(json!("result1")));
        assert_eq!(ctx.get_output("step2"), Some(json!("result2")));
        assert_eq!(ctx.get_output("step3"), None);
    }

    #[test]
    fn test_template_rendering_with_inputs() {
        let mut inputs = HashMap::new();
        inputs.insert("name".to_string(), json!("World"));

        let ctx = ExecutionContext::new(inputs);
        let result = ctx.render_template("Hello {{ name }}!").unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_template_rendering_with_outputs() {
        let ctx = ExecutionContext::default();
        ctx.set_output("step1", json!("positive"));

        let result = ctx.render_template("Sentiment: {{ outputs.step1 }}").unwrap();
        assert_eq!(result, "Sentiment: positive");
    }

    #[test]
    fn test_template_rendering_complex() {
        let mut inputs = HashMap::new();
        inputs.insert("query".to_string(), json!("What is Rust?"));

        let ctx = ExecutionContext::new(inputs);
        ctx.set_output("search", json!(["doc1", "doc2", "doc3"]));

        let template = "Query: {{ query }}\nResults: {{ outputs.search }}";
        let result = ctx.render_template(template).unwrap();
        assert!(result.contains("Query: What is Rust?"));
        assert!(result.contains("Results:"));
    }

    #[test]
    fn test_condition_evaluation_boolean() {
        let ctx = ExecutionContext::default();

        assert!(ctx.evaluate_condition("true").unwrap());
        assert!(ctx.evaluate_condition("TRUE").unwrap());
        assert!(ctx.evaluate_condition("1").unwrap());
        assert!(ctx.evaluate_condition("yes").unwrap());

        assert!(!ctx.evaluate_condition("false").unwrap());
        assert!(!ctx.evaluate_condition("FALSE").unwrap());
        assert!(!ctx.evaluate_condition("0").unwrap());
        assert!(!ctx.evaluate_condition("no").unwrap());
        assert!(!ctx.evaluate_condition("").unwrap());
    }

    #[test]
    fn test_condition_evaluation_equality() {
        let ctx = ExecutionContext::default();
        ctx.set_output("sentiment", json!("positive"));

        let template = "{{ outputs.sentiment }} == 'positive'";
        assert!(ctx.evaluate_condition(template).unwrap());

        let template = "{{ outputs.sentiment }} == 'negative'";
        assert!(!ctx.evaluate_condition(template).unwrap());

        let template = "{{ outputs.sentiment }} != 'negative'";
        assert!(ctx.evaluate_condition(template).unwrap());
    }

    #[test]
    fn test_metadata() {
        let ctx = ExecutionContext::default();

        ctx.set_metadata("start_time", json!("2025-11-14T10:00:00Z"));
        ctx.set_metadata("user_id", json!("user123"));

        assert_eq!(ctx.get_metadata("start_time"), Some(json!("2025-11-14T10:00:00Z")));
        assert_eq!(ctx.get_metadata("user_id"), Some(json!("user123")));
        assert_eq!(ctx.get_metadata("nonexistent"), None);
    }

    #[test]
    fn test_all_outputs() {
        let ctx = ExecutionContext::default();
        ctx.set_output("step1", json!("result1"));
        ctx.set_output("step2", json!("result2"));

        let all_outputs = ctx.all_outputs();
        assert_eq!(all_outputs.len(), 2);
        assert_eq!(all_outputs.get("step1"), Some(&json!("result1")));
        assert_eq!(all_outputs.get("step2"), Some(&json!("result2")));
    }

    #[test]
    fn test_template_nested_field_access() {
        let ctx = ExecutionContext::default();

        // Set output with nested object
        ctx.set_output("step1", json!({
            "greeting": "Hello",
            "sentiment": "positive"
        }));

        // Test both old and new syntax
        // Note: Handlebars renders objects as "[object]" by default
        let result_old = ctx.render_template("{{ outputs.step1 }}").unwrap();
        assert_eq!(result_old, "[object]");

        let result_new = ctx.render_template("{{ steps.step1.greeting }}").unwrap();
        assert_eq!(result_new, "Hello");

        let result_sentiment = ctx.render_template("{{ steps.step1.sentiment }}").unwrap();
        assert_eq!(result_sentiment, "positive");
    }

    #[test]
    fn test_template_inputs_namespace() {
        let mut inputs = HashMap::new();
        inputs.insert("name".to_string(), json!("Alice"));
        inputs.insert("age".to_string(), json!(30));

        let ctx = ExecutionContext::new(inputs);

        // Test both root-level and namespaced access
        let result1 = ctx.render_template("{{ name }}").unwrap();
        assert_eq!(result1, "Alice");

        let result2 = ctx.render_template("{{ inputs.name }}").unwrap();
        assert_eq!(result2, "Alice");

        let result3 = ctx.render_template("{{ inputs.age }}").unwrap();
        assert_eq!(result3, "30");
    }
}
