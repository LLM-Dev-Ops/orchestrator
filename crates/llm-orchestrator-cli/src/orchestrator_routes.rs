// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! The seven `/v1/orchestrator/*` HTTP routes.
//!
//! These bind the existing agents in `llm-orchestrator-core` to the axum server so the
//! deployed service performs real orchestration. Before this module the agents were reachable
//! only as CLI subcommands, and the Cloud Function answered every request by echoing its input
//! back with a `status` field.
//!
//! The response envelope (`execution_metadata`, `layers_executed`) is byte-compatible with the
//! one the Cloud Function used to build, so existing consumers see the same shape.
//!
//! Implements ADR-0001.

use axum::{
    extract::Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use chrono::Utc;
use llm_orchestrator_core::adapters::retry_recovery::{
    RecoveryError, RecoveryRequest, RetryRecoveryAgent,
};
use llm_orchestrator_core::adapters::{ScheduleRequest, SchedulerError, TaskSchedulerAgent};
use llm_orchestrator_core::{
    AgentConfig, DependencyResolveRequest, DependencyResolverAgent, DependencyResolverConfig,
    ExecuteRequest, OrchestratorError, ParallelizationAgent, ParallelizationAgentConfig,
    ParallelizationRequest, StateMachineAgent, StateMachineAgentConfig, StateTransitionRequest,
    SwarmCoordinationRequest, SwarmCoordinatorAgent, SwarmCoordinatorAgentConfig,
    WorkflowOrchestratorAgent,
};
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::time::Instant;
use uuid::Uuid;

/// Service name reported in `execution_metadata.service`.
///
/// Deliberately identical to `SERVICE_NAME` in `functions/index.js` -- consumers correlate on
/// this string, and the engine taking over the work must not change it.
const SERVICE_NAME: &str = "orchestrator-agents";

const RUVECTOR_ENDPOINT_ENV: &str = "RUVECTOR_SERVICE_ENDPOINT";

// ============================================================================
// RESPONSE ENVELOPE
// ============================================================================

#[derive(Debug, Serialize)]
struct ExecutionMetadata {
    trace_id: String,
    timestamp: String,
    service: &'static str,
    execution_id: String,
}

#[derive(Debug, Serialize)]
struct LayerExecuted {
    layer: String,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u64>,
}

/// Describes one agent's identity as published in `functions/contracts/index.js`.
struct AgentDescriptor {
    /// Route segment, e.g. `dependencies`.
    route: &'static str,
    /// Contract `agent_id`.
    id: &'static str,
    /// Contract `agent_version`.
    version: &'static str,
    /// Contract `classification`.
    classification: &'static str,
}

const WORKFLOW: AgentDescriptor = AgentDescriptor {
    route: "workflow",
    id: "workflow-orchestrator",
    version: "0.1.0",
    classification: "WORKFLOW_EXECUTION",
};
const SCHEDULER: AgentDescriptor = AgentDescriptor {
    route: "scheduler",
    id: "task-scheduler",
    version: "0.1.0",
    classification: "TASK_COORDINATION",
};
const DEPENDENCIES: AgentDescriptor = AgentDescriptor {
    route: "dependencies",
    id: "dependency-resolver",
    version: "0.1.0",
    classification: "TASK_COORDINATION",
};
const RETRY: AgentDescriptor = AgentDescriptor {
    route: "retry",
    id: "retry-recovery",
    version: "0.1.0",
    classification: "TASK_COORDINATION",
};
const PARALLEL: AgentDescriptor = AgentDescriptor {
    route: "parallel",
    id: "parallelization-agent",
    version: "0.1.0",
    classification: "TASK_COORDINATION",
};
const STATE_MACHINE: AgentDescriptor = AgentDescriptor {
    route: "state-machine",
    id: "state-machine-agent",
    version: "1.0.0",
    classification: "STATE_MANAGEMENT",
};
const SWARM: AgentDescriptor = AgentDescriptor {
    route: "swarm",
    id: "swarm-coordinator-agent",
    version: "0.1.0",
    classification: "TASK_COORDINATION",
};

/// Builds the envelope the Cloud Function used to build, merged into the agent's own payload.
///
/// Mirrors `buildResponse` at `functions/index.js:62`: agent fields and envelope keys sit at the
/// top level of the same object, not nested under a `data` key.
fn envelope(
    headers: &HeaderMap,
    agent: &AgentDescriptor,
    status: &'static str,
    mut payload: Map<String, Value>,
    started: Instant,
) -> Value {
    let trace_id = headers
        .get("x-correlation-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let duration_ms = started.elapsed().as_millis() as u64;

    payload.insert("agent".into(), json!(agent.id));
    payload.insert("agent_version".into(), json!(agent.version));
    payload.insert("classification".into(), json!(agent.classification));

    payload.insert(
        "execution_metadata".into(),
        json!(ExecutionMetadata {
            trace_id,
            timestamp: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            service: SERVICE_NAME,
            execution_id: Uuid::new_v4().to_string(),
        }),
    );
    payload.insert(
        "layers_executed".into(),
        json!(vec![
            LayerExecuted {
                layer: "AGENT_ROUTING".into(),
                status: "completed",
                duration_ms: None,
            },
            LayerExecuted {
                layer: format!(
                    "ORCHESTRATOR_{}",
                    agent.route.to_uppercase().replace('-', "_")
                ),
                status,
                duration_ms: Some(duration_ms),
            },
        ]),
    );

    Value::Object(payload)
}

/// Serialises an agent response into the envelope.
fn ok_response<T: Serialize>(
    headers: &HeaderMap,
    agent: &AgentDescriptor,
    body: T,
    started: Instant,
) -> Response {
    let payload = match serde_json::to_value(body) {
        Ok(Value::Object(map)) => map,
        // An agent whose response is not a JSON object would be a contract break, not a
        // request error, so it is reported as such rather than silently reshaped.
        Ok(other) => {
            return error_response(
                headers,
                agent,
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Agent returned a non-object response: {other}"),
                started,
            )
        }
        Err(e) => {
            return error_response(
                headers,
                agent,
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize agent response: {e}"),
                started,
            )
        }
    };

    (
        StatusCode::OK,
        Json(envelope(headers, agent, "completed", payload, started)),
    )
        .into_response()
}

fn error_response(
    headers: &HeaderMap,
    agent: &AgentDescriptor,
    status: StatusCode,
    message: impl Into<String>,
    started: Instant,
) -> Response {
    let mut payload = Map::new();
    payload.insert("error".into(), json!(message.into()));
    (
        status,
        Json(envelope(headers, agent, "error", payload, started)),
    )
        .into_response()
}

// ============================================================================
// VALIDATION AND ERROR MAPPING
// ============================================================================

/// Reproduces `validateRequiredFields` at `functions/index.js:80`, including its message.
fn missing_required(body: &Value, required: &[&str]) -> Option<String> {
    let missing: Vec<&str> = required
        .iter()
        .copied()
        .filter(|field| match body.get(*field) {
            None | Some(Value::Null) => true,
            Some(_) => false,
        })
        .collect();

    if missing.is_empty() {
        None
    } else {
        Some(format!("Missing required fields: {}", missing.join(", ")))
    }
}

/// A malformed body is the caller's error, so deserialization failures map to 400.
fn parse_request<T: serde::de::DeserializeOwned>(body: Value) -> Result<T, String> {
    serde_json::from_value(body).map_err(|e| format!("Invalid request body: {e}"))
}

/// Supplies `timestamp` when the caller omits it.
///
/// `StateTransitionRequest`, `ParallelizationRequest` and `SwarmCoordinationRequest` all declare
/// `timestamp` without a serde default, but none of the published contracts list it as required,
/// so a contract-conformant body would fail to deserialize. Reconciling the two is a change to
/// `agentics-contracts`, which two other repositories vendor and which ADR-0001 puts out of
/// scope; filling the value in at the boundary keeps existing callers working meanwhile.
fn default_timestamp(body: &mut Value) {
    if let Some(map) = body.as_object_mut() {
        if !matches!(map.get("timestamp"), Some(v) if !v.is_null()) {
            map.insert(
                "timestamp".into(),
                json!(Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
            );
        }
    }
}

/// Maps an agent error to the status code its cause deserves.
///
/// A graph the caller described wrongly -- a cycle, a dangling dependency, a bad config -- is a
/// 400. Everything else is the engine's fault and is a 500.
fn status_for(error: &OrchestratorError) -> StatusCode {
    match error {
        OrchestratorError::CyclicDependency
        | OrchestratorError::StepNotFound(_)
        | OrchestratorError::ValidationError(_)
        | OrchestratorError::ParseError(_)
        | OrchestratorError::InvalidStepConfig { .. }
        | OrchestratorError::InvalidStateTransition { .. }
        | OrchestratorError::ContextVariableNotFound(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Boilerplate every handler shares: validate, parse, and return early on failure.
macro_rules! prepare {
    ($headers:expr, $agent:expr, $started:expr, $body:expr, $required:expr, $ty:ty) => {{
        if let Some(message) = missing_required(&$body, $required) {
            return error_response($headers, $agent, StatusCode::BAD_REQUEST, message, $started);
        }
        match parse_request::<$ty>($body) {
            Ok(request) => request,
            Err(message) => {
                return error_response(
                    $headers,
                    $agent,
                    StatusCode::BAD_REQUEST,
                    message,
                    $started,
                )
            }
        }
    }};
}

/// Converts an agent `Result` into a response, mapping the error by cause.
macro_rules! respond {
    ($headers:expr, $agent:expr, $started:expr, $result:expr) => {
        match $result {
            Ok(response) => ok_response($headers, $agent, response, $started),
            Err(e) => {
                let status = status_for(&e);
                error_response($headers, $agent, status, e.to_string(), $started)
            }
        }
    };
}

fn ruvector_endpoint() -> Option<String> {
    std::env::var(RUVECTOR_ENDPOINT_ENV).ok().filter(|s| !s.is_empty())
}

// ============================================================================
// HANDLERS
// ============================================================================

/// `POST /v1/orchestrator/workflow` -- executes a workflow through `WorkflowExecutor`.
///
/// Takes the engine-native `{ workflow, inputs }` shape. The published contract's flat
/// `{ workflow_id, workflow_name, tasks }` shape is rejected with a pointer to the right one:
/// its `tasks` carry no step type or step config, so there is no faithful translation into
/// `Workflow.steps`, and guessing one would execute something the caller did not ask for. See
/// ADR-0001 for the contract reconciliation this defers.
async fn workflow_handler(headers: HeaderMap, Json(body): Json<Value>) -> Response {
    let started = Instant::now();

    if body.get("workflow").is_none() && body.get("tasks").is_some() {
        return error_response(
            &headers,
            &WORKFLOW,
            StatusCode::BAD_REQUEST,
            "This endpoint expects the engine-native shape { workflow: { name, steps: [...] }, \
             inputs: {...} }. The contract's flat { workflow_id, workflow_name, tasks } shape \
             carries no step type or configuration and cannot be executed. See ADR-0001.",
            started,
        );
    }

    let request = prepare!(
        &headers,
        &WORKFLOW,
        started,
        body,
        &["workflow"],
        ExecuteRequest
    );

    let agent = WorkflowOrchestratorAgent::new(AgentConfig::default());
    respond!(&headers, &WORKFLOW, started, agent.execute(request).await)
}

/// `POST /v1/orchestrator/scheduler` -- schedules a task via `TaskSchedulerAgent`.
async fn scheduler_handler(headers: HeaderMap, Json(body): Json<Value>) -> Response {
    let started = Instant::now();

    let request = prepare!(
        &headers,
        &SCHEDULER,
        started,
        body,
        &["schedule_id", "tasks"],
        ScheduleRequest
    );

    let Some(endpoint) = ruvector_endpoint() else {
        return error_response(
            &headers,
            &SCHEDULER,
            StatusCode::SERVICE_UNAVAILABLE,
            format!("{RUVECTOR_ENDPOINT_ENV} is not configured; the scheduler cannot persist"),
            started,
        );
    };

    let agent = TaskSchedulerAgent::new(endpoint);
    match agent.schedule(request).await {
        Ok(result) => ok_response(&headers, &SCHEDULER, result, started),
        Err(e) => {
            let status = scheduler_status(&e);
            error_response(&headers, &SCHEDULER, status, e.to_string(), started)
        }
    }
}

/// `POST /v1/orchestrator/dependencies` -- resolves a dependency graph.
///
/// This is the route ADR-0001 is defined by: it returns a real topological order and rejects a
/// cyclic graph, where the handler it replaces returned `status: "resolved"` with HTTP 200 for
/// any input at all.
async fn dependencies_handler(headers: HeaderMap, Json(body): Json<Value>) -> Response {
    let started = Instant::now();

    let request = prepare!(
        &headers,
        &DEPENDENCIES,
        started,
        body,
        &["request_id", "workflow_id", "tasks"],
        DependencyResolveRequest
    );

    let agent = DependencyResolverAgent::new(DependencyResolverConfig::default());
    match agent.resolve(request).await {
        // The resolver reports a cycle or a dangling dependency in its status rather than as an
        // Err, so an unsuccessful resolution has to be mapped to 400 here. Returning 200 would
        // reproduce exactly the defect this ADR exists to remove.
        Ok(response) if !response.success => {
            let mut payload = match serde_json::to_value(&response) {
                Ok(Value::Object(map)) => map,
                _ => Map::new(),
            };
            payload.insert(
                "error".into(),
                json!(response
                    .error
                    .clone()
                    .unwrap_or_else(|| format!("Resolution failed: {:?}", response.status))),
            );
            (
                StatusCode::BAD_REQUEST,
                Json(envelope(&headers, &DEPENDENCIES, "error", payload, started)),
            )
                .into_response()
        }
        Ok(response) => ok_response(&headers, &DEPENDENCIES, response, started),
        Err(e) => {
            let status = status_for(&e);
            error_response(&headers, &DEPENDENCIES, status, e.to_string(), started)
        }
    }
}

/// `POST /v1/orchestrator/retry` -- evaluates a failure and chooses a recovery action.
async fn retry_handler(headers: HeaderMap, Json(mut body): Json<Value>) -> Response {
    let started = Instant::now();

    if let Some(message) = missing_required(&body, &["request_id", "failure"]) {
        return error_response(&headers, &RETRY, StatusCode::BAD_REQUEST, message, started);
    }

    // The published contract names this field `failure`; `RecoveryRequest` names it `error`.
    // Accept the contract's name on the wire rather than break existing callers.
    if let Some(map) = body.as_object_mut() {
        if let Some(failure) = map.remove("failure") {
            map.insert("error".into(), failure);
        }
    }

    let request = match parse_request::<RecoveryRequest>(body) {
        Ok(request) => request,
        Err(message) => {
            return error_response(&headers, &RETRY, StatusCode::BAD_REQUEST, message, started)
        }
    };

    let agent = match ruvector_endpoint() {
        Some(endpoint) => RetryRecoveryAgent::new(endpoint),
        None => RetryRecoveryAgent::disabled(),
    };

    match agent.evaluate(request).await {
        Ok(decision) => ok_response(&headers, &RETRY, decision, started),
        Err(e) => {
            let status = recovery_status(&e);
            error_response(&headers, &RETRY, status, e.to_string(), started)
        }
    }
}

/// `POST /v1/orchestrator/parallel` -- computes parallel execution phases.
async fn parallel_handler(headers: HeaderMap, Json(mut body): Json<Value>) -> Response {
    let started = Instant::now();
    default_timestamp(&mut body);

    let request = prepare!(
        &headers,
        &PARALLEL,
        started,
        body,
        &["request_id", "workflow_id", "tasks"],
        ParallelizationRequest
    );

    let agent = ParallelizationAgent::new(ParallelizationAgentConfig::default());
    respond!(&headers, &PARALLEL, started, agent.analyze(request).await)
}

/// `POST /v1/orchestrator/state-machine` -- validates and applies a state transition.
///
/// Replaces the one Cloud Function handler that did real work. The agent is constructed per
/// request because `transition` takes `&mut self` and keeps its history in memory; sharing one
/// across requests would leak one caller's entity states into another's validation.
async fn state_machine_handler(headers: HeaderMap, Json(mut body): Json<Value>) -> Response {
    let started = Instant::now();
    default_timestamp(&mut body);

    let request = prepare!(
        &headers,
        &STATE_MACHINE,
        started,
        body,
        &[
            "request_id",
            "execution_id",
            "entity_type",
            "current_state",
            "target_state",
            "reason",
            "initiated_by",
        ],
        StateTransitionRequest
    );

    let mut agent = StateMachineAgent::new(StateMachineAgentConfig::default());
    respond!(
        &headers,
        &STATE_MACHINE,
        started,
        agent.transition(request).await
    )
}

/// `POST /v1/orchestrator/swarm` -- coordinates a swarm of workers toward an objective.
async fn swarm_handler(headers: HeaderMap, Json(mut body): Json<Value>) -> Response {
    let started = Instant::now();
    default_timestamp(&mut body);

    let request = prepare!(
        &headers,
        &SWARM,
        started,
        body,
        &["request_id", "workflow_id", "objective", "workers"],
        SwarmCoordinationRequest
    );

    let agent = SwarmCoordinatorAgent::new(SwarmCoordinatorAgentConfig::default());
    respond!(&headers, &SWARM, started, agent.coordinate(request).await)
}

fn scheduler_status(error: &SchedulerError) -> StatusCode {
    match error {
        SchedulerError::ValidationError(_) | SchedulerError::DependencyError(_) => {
            StatusCode::BAD_REQUEST
        }
        SchedulerError::AgentDisabled => StatusCode::SERVICE_UNAVAILABLE,
        SchedulerError::Timeout => StatusCode::GATEWAY_TIMEOUT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn recovery_status(error: &RecoveryError) -> StatusCode {
    match error {
        RecoveryError::ValidationError(_) => StatusCode::BAD_REQUEST,
        RecoveryError::AgentDisabled => StatusCode::SERVICE_UNAVAILABLE,
        RecoveryError::Timeout => StatusCode::GATEWAY_TIMEOUT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// ============================================================================
// ROUTER
// ============================================================================

/// The seven agent routes, mountable on any router regardless of its state type.
pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/v1/orchestrator/workflow", post(workflow_handler))
        .route("/v1/orchestrator/scheduler", post(scheduler_handler))
        .route("/v1/orchestrator/dependencies", post(dependencies_handler))
        .route("/v1/orchestrator/retry", post(retry_handler))
        .route("/v1/orchestrator/parallel", post(parallel_handler))
        .route("/v1/orchestrator/state-machine", post(state_machine_handler))
        .route("/v1/orchestrator/swarm", post(swarm_handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(json: Value) -> Json<Value> {
        Json(json)
    }

    #[test]
    fn missing_required_matches_the_cloud_function_message() {
        let body = json!({ "workflow_id": "wf-1" });
        assert_eq!(
            missing_required(&body, &["request_id", "workflow_id", "tasks"]),
            Some("Missing required fields: request_id, tasks".to_string())
        );
    }

    #[test]
    fn explicit_null_counts_as_missing() {
        let body = json!({ "tasks": null });
        assert_eq!(
            missing_required(&body, &["tasks"]),
            Some("Missing required fields: tasks".to_string())
        );
    }

    #[test]
    fn present_fields_pass() {
        let body = json!({ "request_id": "r", "workflow_id": "w", "tasks": [] });
        assert_eq!(
            missing_required(&body, &["request_id", "workflow_id", "tasks"]),
            None
        );
    }

    #[test]
    fn caller_errors_are_400_and_engine_errors_are_500() {
        assert_eq!(
            status_for(&OrchestratorError::CyclicDependency),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            status_for(&OrchestratorError::StepNotFound("z".into())),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            status_for(&OrchestratorError::Internal("boom".into())),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn envelope_is_byte_compatible_with_the_cloud_function() {
        let headers = HeaderMap::new();
        let mut payload = Map::new();
        payload.insert("status".into(), json!("resolved"));

        let value = envelope(&headers, &DEPENDENCIES, "completed", payload, Instant::now());

        // Agent identity, as published in the contract.
        assert_eq!(value["agent"], json!("dependency-resolver"));
        assert_eq!(value["agent_version"], json!("0.1.0"));
        assert_eq!(value["classification"], json!("TASK_COORDINATION"));

        // execution_metadata keys, verbatim from functions/index.js:53-60.
        let meta = &value["execution_metadata"];
        assert_eq!(meta["service"], json!("orchestrator-agents"));
        for key in ["trace_id", "timestamp", "service", "execution_id"] {
            assert!(meta.get(key).is_some(), "missing execution_metadata.{key}");
        }

        // layers_executed, verbatim from functions/index.js:64-67.
        let layers = value["layers_executed"].as_array().expect("array");
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0]["layer"], json!("AGENT_ROUTING"));
        assert_eq!(layers[0]["status"], json!("completed"));
        assert_eq!(layers[1]["layer"], json!("ORCHESTRATOR_DEPENDENCIES"));
        assert_eq!(layers[1]["status"], json!("completed"));
        assert!(layers[1]["duration_ms"].is_u64());

        // The agent's own payload stays at the top level, not nested.
        assert_eq!(value["status"], json!("resolved"));
    }

    #[test]
    fn hyphenated_routes_become_underscored_layer_names() {
        let value = envelope(
            &HeaderMap::new(),
            &STATE_MACHINE,
            "completed",
            Map::new(),
            Instant::now(),
        );
        assert_eq!(
            value["layers_executed"][1]["layer"],
            json!("ORCHESTRATOR_STATE_MACHINE")
        );
    }

    #[test]
    fn correlation_id_is_propagated_as_the_trace_id() {
        let mut headers = HeaderMap::new();
        headers.insert("x-correlation-id", "trace-abc".parse().unwrap());

        let value = envelope(
            &headers,
            &DEPENDENCIES,
            "completed",
            Map::new(),
            Instant::now(),
        );
        assert_eq!(value["execution_metadata"]["trace_id"], json!("trace-abc"));
    }

    #[tokio::test]
    async fn diamond_dag_comes_back_in_dependency_order() {
        // Deliberately listed out of execution order so echoing the input cannot pass.
        let request = json!({
            "request_id": "adr-0001-verify-1",
            "workflow_id": Uuid::new_v4(),
            "tasks": [
                { "task_id": "D", "name": "publish",  "depends_on": ["B", "C"] },
                { "task_id": "B", "name": "extract",  "depends_on": ["A"] },
                { "task_id": "C", "name": "classify", "depends_on": ["A"] },
                { "task_id": "A", "name": "ingest",   "depends_on": [] }
            ]
        });

        let response = dependencies_handler(HeaderMap::new(), body(request)).await;
        assert_eq!(response.status(), StatusCode::OK);

        let value = read_body(response).await;
        let order: Vec<String> = value["execution_order"]
            .as_array()
            .expect("execution_order")
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();

        assert_eq!(order.len(), 4);
        let position = |id: &str| order.iter().position(|t| t == id).unwrap();
        assert!(position("A") < position("B"));
        assert!(position("A") < position("C"));
        assert!(position("B") < position("D"));
        assert!(position("C") < position("D"));

        // B and C are independent and must share a parallel group.
        let groups = value["parallel_groups"].as_array().expect("parallel_groups");
        assert!(
            groups.iter().any(|group| {
                let ids: Vec<&str> = group["task_ids"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                    .unwrap_or_default();
                ids.contains(&"B") && ids.contains(&"C")
            }),
            "expected B and C in one parallel group, got {groups:?}"
        );
    }

    #[tokio::test]
    async fn cycle_is_rejected_with_400() {
        let request = json!({
            "request_id": "adr-0001-verify-2",
            "workflow_id": Uuid::new_v4(),
            "tasks": [
                { "task_id": "A", "name": "ingest",  "depends_on": ["D"] },
                { "task_id": "B", "name": "extract", "depends_on": ["A"] },
                { "task_id": "D", "name": "publish", "depends_on": ["B"] }
            ]
        });

        let response = dependencies_handler(HeaderMap::new(), body(request)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let value = read_body(response).await;
        let error = value["error"].as_str().unwrap_or_default().to_lowercase();
        assert!(error.contains("cycle"), "error did not name a cycle: {error}");
    }

    #[tokio::test]
    async fn dangling_dependency_is_rejected_with_400() {
        let request = json!({
            "request_id": "adr-0001-verify-3",
            "workflow_id": Uuid::new_v4(),
            "tasks": [
                { "task_id": "A", "name": "ingest", "depends_on": ["Z"] }
            ]
        });

        let response = dependencies_handler(HeaderMap::new(), body(request)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn missing_fields_are_rejected_before_the_agent_runs() {
        let response = dependencies_handler(HeaderMap::new(), body(json!({ "tasks": [] }))).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let value = read_body(response).await;
        assert_eq!(
            value["error"],
            json!("Missing required fields: request_id, workflow_id")
        );
    }

    #[tokio::test]
    async fn an_illegal_state_transition_is_reported_as_invalid() {
        let request = json!({
            "request_id": Uuid::new_v4(),
            "execution_id": Uuid::new_v4(),
            "entity_type": "task",
            "current_state": "completed",
            "target_state": "running",
            "reason": "adr-0001 verification",
            "initiated_by": "test"
        });

        let response = state_machine_handler(HeaderMap::new(), body(request)).await;
        assert_eq!(response.status(), StatusCode::OK);

        let value = read_body(response).await;
        assert_eq!(value["success"], json!(false));
        assert_eq!(value["status"], json!("invalid"));
        assert_eq!(value["new_state"], json!("completed"));
    }

    #[tokio::test]
    async fn the_contract_shaped_workflow_body_is_rejected_rather_than_guessed() {
        let request = json!({
            "workflow_id": Uuid::new_v4(),
            "workflow_name": "demo",
            "tasks": [{ "task_id": "A", "name": "ingest", "task_type": "llm" }]
        });

        let response = workflow_handler(HeaderMap::new(), body(request)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let value = read_body(response).await;
        assert!(value["error"]
            .as_str()
            .unwrap_or_default()
            .contains("engine-native shape"));
    }

    async fn read_body(response: Response) -> Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        serde_json::from_slice(&bytes).expect("json")
    }
}
