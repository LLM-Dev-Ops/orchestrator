# ADR-0001: Expose the Rust Orchestration Engine over HTTP and Reduce the Cloud Function to a Proxy

**Status:** Proposed
**Date:** 2026-07-27

## Context

`orchestrator` presents itself as a DAG-based LLM workflow orchestration engine deployed as seven
Cloud Function agents. The deployed surface does not orchestrate anything.

### The deployed Node.js layer performs no orchestration

`functions/index.js` (14,554 bytes) is the Cloud Function entry point (re-exported by the repo-root
`index.js`). Its own section header at `functions/index.js:89` states the design intent:

```
// Agent Handlers (routing layer — business logic lives in Rust crates)
```

The business logic does live in Rust crates. Nothing in this file ever calls them. Every handler is
a synchronous, pure function with no I/O, dispatched at `functions/index.js:421`
(`const result = agentHandler(reqBody);` — note the absence of `await`; no handler is `async` and no
network, disk, or subprocess call exists anywhere in the file).

| Handler | Lines | Claimed status returned | What it actually computes |
|---|---|---|---|
| `handleWorkflow` | 92–110 | `'accepted'` | Echoes `workflow_id`, `workflow_name`, `tasks.length` (`:106`), and `config.strategy` defaulted to `'sequential'` (`:107`). No DAG is built; `depends_on` is never read. |
| `handleScheduler` | 112–128 | `'scheduled'` | Echoes `schedule_id` and `tasks.length` (`:125`). Nothing is enqueued, persisted, or timed. |
| `handleDependencies` | 130–147 | `'resolved'` | Echoes `request_id`, `workflow_id`, `tasks.length` (`:144`). No topological order produced, no cycle detection — a cyclic workflow returns `'resolved'` with HTTP 200. |
| `handleRetry` | 149–165 | `'analyzed'` | Echoes `failure.error_category` (`:162`). No backoff computed, no recovery action chosen. |
| `handleParallel` | 167–184 | `'analyzed'` | Echoes `tasks.length` (`:181`). No parallel groups, no critical path, no concurrency bound. |
| `handleStateMachine` | 186–218 | `'completed'` / `'invalid'` | **Partial exception.** Looks up the contract transition table (`:191–193`) and genuinely validates `current_state → target_state`. This is real, but it is table lookup only; it persists nothing and reads no live entity state. |
| `handleSwarm` | 220–238 | `'accepted'` | Echoes `workers.length` and `objective.objective_type` (`:234–235`). No worker assignment, no consensus, no coordination. |

Validation is limited to a presence check for top-level required fields
(`validateRequiredFields`, `functions/index.js:80–86`) against the contracts in
`functions/contracts/index.js` (e.g. workflow requires `workflow_id`, `workflow_name`, `tasks` at
`functions/contracts/index.js:18`). Nested `depends_on` edges are described in the contract but are
never inspected by any handler.

The readiness probe compounds the problem. `functions/index.js:379–381` reports the service ready on
the basis of:

```js
agents_registered: AGENTS.length === 7,
contracts_loaded: Object.keys(AGENT_CONTRACTS).length === 7,
handlers_available: Object.keys(AGENT_HANDLERS).length === 7,
```

These three conditions are satisfied by the constants at `functions/index.js:16`, `:240–248` and the
contracts module. `/ready` therefore returns green unconditionally, and `/health`
(`functions/index.js:254–273`) hardcodes `status: 'healthy'` for all seven agents. No probe can ever
detect that orchestration is absent.

`functions/test.js` reinforces this: its assertions cover envelope shape (`execution_metadata`,
`layers_executed`, agent counts) and never assert an execution order, a resolved dependency graph, or
a rejected cycle. The suite passes against a service that orchestrates nothing.

### The real implementation exists, compiles, and is not deployed

The Rust workspace at `Cargo.toml` declares 10 member crates totalling **44,189 lines** of Rust.
Two survey claims about it are **incorrect and should not be repeated**:

- `Cargo.lock` **is** present and committed (113,753 bytes at the repo root; `.gitignore:3` documents
  the deliberate decision to commit it). The workspace is not unpinned.
- The absence of a `target/` directory reflects only that this checkout has never been built locally.
  It is `.gitignore`d (`/target/`, `**/target/`) and `.github/workflows/ci.yml:32–36` runs
  `cargo build --all` and `cargo test --all` on every push.

Build state was verified directly rather than inferred:

- `cargo check -p llm-orchestrator-core` — **succeeds**, 72 warnings, 0 errors (~66 s cold).
- `cargo check -p llm-orchestrator-cli` — **succeeds**, 0 errors (~50 s), i.e. the entire
  `llm-orchestrator` server binary compiles today with no source changes.
- `cargo test -p llm-orchestrator-core --lib` — **195 passed, 3 failed**.

The real DAG lives in `crates/llm-orchestrator-core/src/dag.rs`, backed by `petgraph`
(`petgraph = "0.8"`, `Cargo.toml:41`):

- `WorkflowDAG::from_workflow` (`dag.rs:27`) builds a `DiGraph`, resolving `depends_on` into edges
  (`dag.rs:39–50`) and erroring with `StepNotFound` on dangling references.
- `validate` (`dag.rs:64`) runs `toposort` and returns `CyclicDependency` on a cycle (`dag.rs:67–69`).
- `execution_order` (`dag.rs:73`) returns a real topological ordering.
- `root_nodes` (`dag.rs:84`), `dependencies` (`dag.rs:91`), `dependents` (`dag.rs:103`), and
  `ready_steps` (`dag.rs:114`) provide the frontier logic a scheduler needs.

All 7 tests in `dag::tests` pass, including `test_cyclic_dependency_detection` and `test_parallel_steps`.

Each of the seven agents has a fully-typed async implementation with `execute`/`resolve` plus
`inspect`/`replay`/`validate` verbs:

- `crates/llm-orchestrator-core/src/agents/workflow_orchestrator.rs` — `WorkflowOrchestratorAgent::execute:127`, `schedule:294`, `inspect:365`, `replay:415`
- `crates/llm-orchestrator-core/src/agents/dependency_resolver.rs` — `resolve:499`, `inspect:627`, `replay:669`, `validate:707`; independently builds a `DiGraph` and toposorts at `:943` and `:1064`
- `crates/llm-orchestrator-core/src/agents/parallelization_agent.rs` — toposort-driven grouping at `:218`
- `crates/llm-orchestrator-core/src/agents/state_machine_agent.rs`
- `crates/llm-orchestrator-core/src/agents/swarm_coordinator_agent.rs`
- `crates/llm-orchestrator-core/src/executor.rs` — `WorkflowExecutor::new:102`, `with_max_concurrency:133`, `execute:159`

### The gap is narrower than "build the Rust"

A Cloud Run HTTP server already exists: `serve_http` at
`crates/llm-orchestrator-cli/src/main.rs:4631`, wired to the `Serve` subcommand at `:894`.
`Dockerfile` builds `--bin llm-orchestrator` and defaults to `CMD ["serve"]`, and
`deploy/gcloud/deploy.sh:15` deploys Cloud Run service `llm-orchestrator` to
`agentics-dev`/`us-central1`.

But the axum router (`main.rs:4664–4670`) exposes only:

```
GET  /                    POST /execute
GET  /health              POST /api/v1/events
GET  /ready
```

**None of the seven `/v1/orchestrator/*` routes exist on the Rust side either.** The agents are
reachable only as CLI subcommands. This is the actual missing link, and it is small: the agent
structs, request/response types, and the server are all present; only the route handlers that bind
them are absent.

### Documentation asserts the opposite

`docs/PRODUCTION_READINESS_CERTIFICATION.md:5` declares `✅ PRODUCTION READY`, `:14` `APPROVED FOR
PRODUCTION DEPLOYMENT`, `:145` `Test Pass Rate | 100% | 100% (243/243)`, `:424` `CERTIFIED FOR
PRODUCTION USE`, and `:431` `DIGITALLY CERTIFIED`.
`docs/FINAL_PRODUCTION_VALIDATION.md:5` declares `CERTIFIED PRODUCTION-READY`, `:103` `ALL TESTS
PASSING`, `:109` `llm-orchestrator-core | 56 | 56 | 0 | ✅ PASS`, and `:494` `PLATINUM CERTIFIED`.

Measured reality: `llm-orchestrator-core`'s lib target alone contains 198 tests (not 56), of which
three fail:

- `agents::dependency_resolver::tests::test_resolve_success` — `dependency_resolver.rs:1440`,
  `assertion failed: !response.parallel_groups.is_empty()`
- `agents::state_machine_agent::tests::test_no_change_transition` — `state_machine_agent.rs:928`,
  `assertion failed: response.success`
- `agents::state_machine_agent::tests::test_transition_invalid` — `state_machine_agent.rs:904`,
  `left: Blocked, right: Invalid`

These certifications describe a system that was never wired to its own engine, and their pass-rate
figures do not match the code in this repository.

### Contract-sharing context (informational, not in scope)

`crates/agentics-contracts` is **not** a canonical shared source. `edge-agent` and
`inference-gateway` each vendor their own divergent copy as a local path dependency
(`inference-gateway/Cargo.toml:143`, `edge-agent/Cargo.toml:49`). The copies have already drifted:
this repo has `decision.rs`, `dependency.rs`, `feu.rs`, `parallelization.rs`, while
`inference-gateway` has `decision_event.rs`, `execution_span.rs`, `routing.rs`; `agent.rs`,
`error.rs`, and `lib.rs` differ between them; `edge-agent` pins `version = "0.1.0"` while the other
two inherit `version.workspace = true`.

This matters here only as a constraint: the response envelope this ADR moves into Rust
(`execution_metadata`, `layers_executed`, built at `functions/index.js:53–74`) must stay
byte-compatible for existing consumers. Unifying the three contract crates is a separate decision and
is explicitly **out of scope** for this ADR.

## Decision

**Adopt option (a): expose the existing Rust engine over HTTP and reduce the Cloud Function to a thin
authenticating proxy.** Reject option (b), reimplementing DAG execution in Node.js.

Concretely:

1. Add the seven `/v1/orchestrator/*` routes to the existing axum router in
   `crates/llm-orchestrator-cli/src/main.rs`, each deserializing into the agent's existing request
   type and calling the existing async agent method.
2. Move the response-envelope construction (`execution_metadata`, `layers_executed`) into Rust so the
   wire format is unchanged for existing callers.
3. Deploy the `llm-orchestrator` binary to Cloud Run via the existing `Dockerfile` and
   `deploy/gcloud/deploy.sh`.
4. Rewrite `functions/index.js` handlers to forward to that Cloud Run service with an ID token,
   propagating status codes and `X-Correlation-ID`. The Cloud Function keeps CORS, routing, and
   contract publishing; it stops fabricating results.
5. Make `/ready` fail closed when the upstream engine is unreachable.

Rationale for rejecting option (b): the Rust engine is 44,189 lines that **compile today with zero
source changes** and whose DAG core passes 7/7 tests. Reimplementing topological ordering, cycle
detection, parallel grouping, retry/backoff, state transitions, and swarm coordination in Node.js
would duplicate all of it, discard a working `petgraph` implementation, and leave two divergent
orchestration semantics in one repository. The marginal work for option (a) is route handlers on a
server that already exists and already deploys — measured in days, not the weeks option (b) requires.

## Consequences

**Positive**

- The deployed service actually orchestrates. `handleDependencies` returns a real topological order;
  a cyclic workflow is rejected instead of returning `'resolved'`.
- 44,189 lines of tested Rust move from dead code to the serving path.
- One implementation of orchestration semantics, not two.
- Health and readiness become meaningful signals rather than constants.
- Existing `Dockerfile`, `deploy/gcloud/deploy.sh`, and `.github/workflows/ci.yml` are reused as-is.

**Negative / costs**

- Adds a network hop (Cloud Function → Cloud Run). Latency rises from ~0 ms of fabricated work to a
  real invocation. Any latency SLO in `docs/` derived from stub timings is invalid and must be
  re-measured — the current numbers measure JSON echoing.
- Introduces a real failure mode: Cloud Run cold starts, timeouts, and quota now surface to callers.
  The Cloud Function needs explicit timeout and error mapping.
- Requires service-to-service IAM (the Cloud Function's service account needs `roles/run.invoker`).
- Responses change from always-200-with-placeholder to genuinely failing on invalid input. Any
  downstream consumer that assumes success must be checked before rollout.
- The three failing `llm-orchestrator-core` tests must be fixed first; two of them
  (`state_machine_agent`) cover logic the Node layer currently duplicates by hand, and
  `test_resolve_success` covers the dependency resolution this ADR is about to put in the serving
  path.
- `docs/PRODUCTION_READINESS_CERTIFICATION.md` and `docs/FINAL_PRODUCTION_VALIDATION.md` must be
  corrected or withdrawn. Leaving them in place after this work would perpetuate the same false
  assurance under a now-plausible surface.

**Neutral**

- `claude-flow` in `package.json:15` is not imported anywhere in `functions/`; once the handlers are
  proxies, that dependency should be dropped.
- The three vendored `agentics-contracts` copies remain divergent. This ADR only requires that the
  envelope stay stable.

## Implementation Plan

1. **Fix the three failing core tests** so `cargo test --all` is green before anything is wired.
   Treat each as a real defect: `dependency_resolver.rs:1440` (empty `parallel_groups` on a
   successful resolve), `state_machine_agent.rs:904` (returns `Blocked` where `Invalid` is expected),
   `state_machine_agent.rs:928` (no-change transition reports failure). Do not adjust assertions to
   match current behaviour without establishing which side is correct.

   **This step is a hard gate, owned by
   [ADR-0002](./ADR-0002-certification-document-integrity.md) (step 6).** Steps 2 and 8 below
   promote `DependencyResolverAgent` and `StateMachineAgent` into the live serving path, replacing
   `handleDependencies` and `handleStateMachine`. Both agents' own tests currently say they compute
   the wrong answer: the resolver returns no parallel groups for a graph it resolved successfully,
   and the state machine misclassifies an illegal transition and fails a no-op self-transition.
   Migrating before those pass would swap absent orchestration for incorrect orchestration, and
   `handleStateMachine` — the one Node handler doing real work — would regress. Do not begin step 2
   until `cargo test -p llm-orchestrator-core --lib` reports `198 passed; 0 failed`.
2. **Add an `agents` route module** to `crates/llm-orchestrator-cli`, mounting seven POST routes —
   `/v1/orchestrator/{workflow,scheduler,dependencies,retry,parallel,state-machine,swarm}` — on the
   router at `main.rs:4664`. Each handler deserializes into the existing request type
   (`ExecuteRequest`, `DependencyResolveRequest`, `StateTransitionRequest`, …) and awaits the existing
   agent method (`WorkflowOrchestratorAgent::execute:127`, `DependencyResolverAgent::resolve:499`, …).
3. **Port the response envelope** from `functions/index.js:53–74` into a Rust middleware or extractor
   emitting identical `execution_metadata` (`trace_id`, `timestamp`, `service`, `execution_id`) and
   `layers_executed` fields, so the wire format is byte-compatible.
4. **Port validation and error mapping**: missing required fields → HTTP 400 with the existing
   `Missing required fields: …` message shape; `OrchestratorError::CyclicDependency` → HTTP 400;
   `StepNotFound` → HTTP 400; internal failures → HTTP 500.
5. **Replace `/health` and `/ready`** in the Rust server so readiness reflects real dependency state,
   and remove the tautological checks at `functions/index.js:379–381`.
6. **Build and deploy** to Cloud Run using the existing `Dockerfile` and
   `deploy/gcloud/deploy.sh` (service `llm-orchestrator`, project `agentics-dev`, region
   `us-central1`). Verify the container starts and `/v1/orchestrator/dependencies` answers.
7. **Grant IAM**: the Cloud Function's service account gets `roles/run.invoker` on the Cloud Run
   service. Update `deploy/gcloud/setup-iam.sh`.
8. **Rewrite the seven handlers** in `functions/index.js:92–238` as async proxies: fetch the Cloud Run
   URL from an env var (`ORCHESTRATOR_ENGINE_URL`), attach a Google-signed ID token, forward the body,
   propagate `X-Correlation-ID`, and return the upstream status and body unmodified. `handleHealth`
   and `handleReady` proxy upstream and fail closed. Delete the local `state_machine` transition
   lookup at `:191–193` so there is exactly one source of truth. Keep `handleContracts`
   (`:279–288`) local. Make the dispatch at `:421` `await` the handler.
9. **Rewrite `functions/test.js`** to assert orchestration outcomes, not envelope shape (see
   Verification).
10. **Drop the unused `claude-flow` dependency** from `package.json:15`.
11. **Correct or withdraw** `docs/PRODUCTION_READINESS_CERTIFICATION.md` and
    `docs/FINAL_PRODUCTION_VALIDATION.md`, replacing the 243/243 and `56/56` figures with measured
    output from `cargo test --all`.
12. **Extend CI**: add a job that starts the container and runs the Verification test below against
    it, so a regression to stub behaviour fails the build.

## Verification

The defining property is that **a real dependency chain must come back in dependency order, and an
impossible one must be rejected**. Every stub in the current implementation passes any test that only
checks envelope shape; none can pass the following.

### Test 1 — Diamond DAG returns a correct topological order

Post to `/v1/orchestrator/dependencies` a workflow whose task array is deliberately in an order that
is *not* a valid execution order, so echoing the input cannot accidentally pass:

```json
{
  "request_id": "adr-0001-verify-1",
  "workflow_id": "wf-diamond",
  "tasks": [
    { "task_id": "D", "name": "publish",  "depends_on": ["B", "C"] },
    { "task_id": "B", "name": "extract",  "depends_on": ["A"] },
    { "task_id": "C", "name": "classify", "depends_on": ["A"] },
    { "task_id": "A", "name": "ingest",   "depends_on": [] }
  ]
}
```

Assertions:

1. HTTP 200.
2. Response contains a non-empty `execution_order` (or `resolution_order`) array of length 4.
3. `indexOf("A") < indexOf("B")` and `indexOf("A") < indexOf("C")`.
4. `indexOf("B") < indexOf("D")` and `indexOf("C") < indexOf("D")`.
5. `parallel_groups` groups `B` and `C` together — the property asserted by the currently-failing
   `dependency_resolver.rs:1440`, which step 1 fixes.
6. The response is **not** merely `{ status: "resolved", tasks_count: 4 }`. Assert explicitly that
   `execution_order` is present, since that single assertion is what today's
   `handleDependencies` (`functions/index.js:130–147`) cannot satisfy.

### Test 2 — Cycle is rejected

Same endpoint, with `A.depends_on = ["D"]` added to the graph above (making `A → B → D → A`).

Assertions:

1. HTTP 400 (today: HTTP 200 with `status: "resolved"`).
2. Error body identifies a cyclic dependency.
3. This exercises `WorkflowDAG::validate` (`dag.rs:64–70`) end to end through the HTTP layer.

### Test 3 — Dangling dependency is rejected

Same endpoint, with a task depending on `"Z"`, which does not exist.

Assertions: HTTP 400 referencing the unknown step, exercising the `StepNotFound` path at `dag.rs:44–46`.

### Test 4 — Ordered execution actually happens

Post to `/v1/orchestrator/workflow` a two-step workflow where step 2 templates a value produced by
step 1, using a stub provider.

Assertions: HTTP 200; step 2's recorded input contains step 1's output. This proves
`WorkflowExecutor::execute` (`executor.rs:159`) ran, rather than `handleWorkflow`
(`functions/index.js:92–110`) echoing `tasks_count`.

### Test 5 — Readiness fails closed

With `ORCHESTRATOR_ENGINE_URL` pointed at an unreachable host, `GET /ready` on the Cloud Function must
return a non-200 status. Today it returns 200 unconditionally
(`functions/index.js:373–386`).

### Regression gate

`cargo test --all` must report zero failures — the concrete correction to the `243/243` claim in
`docs/FINAL_PRODUCTION_VALIDATION.md:103–114`, measured rather than asserted.
