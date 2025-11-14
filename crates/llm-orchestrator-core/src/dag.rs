// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! DAG (Directed Acyclic Graph) builder and validator for workflows.

use crate::error::{OrchestratorError, Result};
use crate::workflow::Workflow;
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// A DAG representation of a workflow.
#[derive(Debug, Clone)]
pub struct WorkflowDAG {
    /// The underlying directed graph.
    graph: DiGraph<String, ()>,

    /// Mapping from step ID to node index.
    step_to_node: HashMap<String, NodeIndex>,

    /// Mapping from node index to step ID.
    node_to_step: HashMap<NodeIndex, String>,
}

impl WorkflowDAG {
    /// Build a DAG from a workflow.
    pub fn from_workflow(workflow: &Workflow) -> Result<Self> {
        let mut graph = DiGraph::new();
        let mut step_to_node = HashMap::new();
        let mut node_to_step = HashMap::new();

        // First pass: create nodes for all steps
        for step in &workflow.steps {
            let node_idx = graph.add_node(step.id.clone());
            step_to_node.insert(step.id.clone(), node_idx);
            node_to_step.insert(node_idx, step.id.clone());
        }

        // Second pass: add edges for dependencies
        for step in &workflow.steps {
            let target_idx = step_to_node[&step.id];

            for dep_id in &step.depends_on {
                let source_idx = step_to_node
                    .get(dep_id)
                    .ok_or_else(|| OrchestratorError::StepNotFound(dep_id.clone()))?;

                graph.add_edge(*source_idx, target_idx, ());
            }
        }

        let dag = Self {
            graph,
            step_to_node,
            node_to_step,
        };

        // Validate that the graph is acyclic
        dag.validate()?;

        Ok(dag)
    }

    /// Validate that the DAG is acyclic.
    pub fn validate(&self) -> Result<()> {
        // Attempt topological sort - will fail if there's a cycle
        toposort(&self.graph, None)
            .map_err(|_| OrchestratorError::CyclicDependency)?;

        Ok(())
    }

    /// Get execution order (topological sort).
    pub fn execution_order(&self) -> Result<Vec<String>> {
        let sorted_indices = toposort(&self.graph, None)
            .map_err(|_| OrchestratorError::CyclicDependency)?;

        Ok(sorted_indices
            .into_iter()
            .map(|idx| self.node_to_step[&idx].clone())
            .collect())
    }

    /// Get root nodes (nodes with no dependencies).
    pub fn root_nodes(&self) -> Vec<String> {
        self.graph
            .externals(petgraph::Direction::Incoming)
            .map(|idx| self.node_to_step[&idx].clone())
            .collect()
    }

    /// Get dependencies for a step.
    pub fn dependencies(&self, step_id: &str) -> Option<Vec<String>> {
        let node_idx = self.step_to_node.get(step_id)?;

        Some(
            self.graph
                .neighbors_directed(*node_idx, petgraph::Direction::Incoming)
                .map(|idx| self.node_to_step[&idx].clone())
                .collect(),
        )
    }

    /// Get dependents (steps that depend on this step).
    pub fn dependents(&self, step_id: &str) -> Option<Vec<String>> {
        let node_idx = self.step_to_node.get(step_id)?;

        Some(
            self.graph
                .neighbors_directed(*node_idx, petgraph::Direction::Outgoing)
                .map(|idx| self.node_to_step[&idx].clone())
                .collect(),
        )
    }

    /// Get steps that are ready to execute (all dependencies completed).
    pub fn ready_steps(&self, completed: &std::collections::HashSet<String>) -> Vec<String> {
        self.step_to_node
            .iter()
            .filter_map(|(step_id, &node_idx)| {
                // Skip if already completed
                if completed.contains(step_id) {
                    return None;
                }

                // Check if all dependencies are completed
                let all_deps_completed = self
                    .graph
                    .neighbors_directed(node_idx, petgraph::Direction::Incoming)
                    .all(|dep_idx| {
                        let dep_id = &self.node_to_step[&dep_idx];
                        completed.contains(dep_id)
                    });

                if all_deps_completed {
                    Some(step_id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get total number of steps.
    pub fn step_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Check if a step exists in the DAG.
    pub fn contains_step(&self, step_id: &str) -> bool {
        self.step_to_node.contains_key(step_id)
    }

    /// Get all step IDs.
    pub fn step_ids(&self) -> Vec<String> {
        self.step_to_node.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{LlmStepConfig, Step, StepConfig, StepType};

    fn create_test_step(id: &str, depends_on: Vec<&str>) -> Step {
        Step {
            id: id.to_string(),
            step_type: StepType::Llm,
            depends_on: depends_on.into_iter().map(String::from).collect(),
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
        }
    }

    #[test]
    fn test_simple_dag() {
        let mut workflow = Workflow::new("test");
        workflow.steps.push(create_test_step("step1", vec![]));
        workflow.steps.push(create_test_step("step2", vec!["step1"]));
        workflow.steps.push(create_test_step("step3", vec!["step2"]));

        let dag = WorkflowDAG::from_workflow(&workflow).unwrap();
        assert_eq!(dag.step_count(), 3);

        let order = dag.execution_order().unwrap();
        assert_eq!(order, vec!["step1", "step2", "step3"]);
    }

    #[test]
    fn test_parallel_steps() {
        let mut workflow = Workflow::new("test");
        workflow.steps.push(create_test_step("step1", vec![]));
        workflow.steps.push(create_test_step("step2", vec!["step1"]));
        workflow.steps.push(create_test_step("step3", vec!["step1"]));
        workflow.steps.push(create_test_step("step4", vec!["step2", "step3"]));

        let dag = WorkflowDAG::from_workflow(&workflow).unwrap();

        let order = dag.execution_order().unwrap();
        // step1 must be first, step4 must be last
        assert_eq!(order[0], "step1");
        assert_eq!(order[3], "step4");
        // step2 and step3 can be in any order
        assert!(order.contains(&"step2".to_string()));
        assert!(order.contains(&"step3".to_string()));
    }

    #[test]
    fn test_cyclic_dependency_detection() {
        let mut workflow = Workflow::new("test");
        workflow.steps.push(create_test_step("step1", vec!["step2"]));
        workflow.steps.push(create_test_step("step2", vec!["step1"]));

        let result = WorkflowDAG::from_workflow(&workflow);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OrchestratorError::CyclicDependency));
    }

    #[test]
    fn test_root_nodes() {
        let mut workflow = Workflow::new("test");
        workflow.steps.push(create_test_step("step1", vec![]));
        workflow.steps.push(create_test_step("step2", vec![]));
        workflow.steps.push(create_test_step("step3", vec!["step1", "step2"]));

        let dag = WorkflowDAG::from_workflow(&workflow).unwrap();
        let roots = dag.root_nodes();

        assert_eq!(roots.len(), 2);
        assert!(roots.contains(&"step1".to_string()));
        assert!(roots.contains(&"step2".to_string()));
    }

    #[test]
    fn test_ready_steps() {
        let mut workflow = Workflow::new("test");
        workflow.steps.push(create_test_step("step1", vec![]));
        workflow.steps.push(create_test_step("step2", vec!["step1"]));
        workflow.steps.push(create_test_step("step3", vec!["step1"]));
        workflow.steps.push(create_test_step("step4", vec!["step2", "step3"]));

        let dag = WorkflowDAG::from_workflow(&workflow).unwrap();

        let mut completed = std::collections::HashSet::new();
        let ready = dag.ready_steps(&completed);
        assert_eq!(ready, vec!["step1"]);

        completed.insert("step1".to_string());
        let ready = dag.ready_steps(&completed);
        assert_eq!(ready.len(), 2);
        assert!(ready.contains(&"step2".to_string()));
        assert!(ready.contains(&"step3".to_string()));

        completed.insert("step2".to_string());
        completed.insert("step3".to_string());
        let ready = dag.ready_steps(&completed);
        assert_eq!(ready, vec!["step4"]);
    }

    #[test]
    fn test_dependencies() {
        let mut workflow = Workflow::new("test");
        workflow.steps.push(create_test_step("step1", vec![]));
        workflow.steps.push(create_test_step("step2", vec!["step1"]));
        workflow.steps.push(create_test_step("step3", vec!["step1", "step2"]));

        let dag = WorkflowDAG::from_workflow(&workflow).unwrap();

        let deps = dag.dependencies("step3").unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"step1".to_string()));
        assert!(deps.contains(&"step2".to_string()));
    }

    #[test]
    fn test_dependents() {
        let mut workflow = Workflow::new("test");
        workflow.steps.push(create_test_step("step1", vec![]));
        workflow.steps.push(create_test_step("step2", vec!["step1"]));
        workflow.steps.push(create_test_step("step3", vec!["step1"]));

        let dag = WorkflowDAG::from_workflow(&workflow).unwrap();

        let dependents = dag.dependents("step1").unwrap();
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&"step2".to_string()));
        assert!(dependents.contains(&"step3".to_string()));
    }
}
