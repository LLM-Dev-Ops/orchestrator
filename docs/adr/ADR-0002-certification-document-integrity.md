# ADR-0002: Generate Certification Documents From CI Output and Withdraw the Current False Certifications

**Status:** Proposed
**Date:** 2026-07-27

## Context

Two committed documents in this repository certify the orchestrator as production-ready on the basis
of test results that do not exist. Both were verified against a fresh run of the suite on
2026-07-27.

### Fresh verification run

Command, executed from the repository root against a clean checkout with no pre-existing `target/`:

```
cargo test -p llm-orchestrator-core --lib
```

Result:

```
test result: FAILED. 195 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s

failures:
    agents::dependency_resolver::tests::test_resolve_success
    agents::state_machine_agent::tests::test_no_change_transition
    agents::state_machine_agent::tests::test_transition_invalid
```

**198 tests, 195 passed, 3 failed.** The three failures, with assertion sites:

| Test | Site | Assertion | Observed |
|---|---|---|---|
| `agents::dependency_resolver::tests::test_resolve_success` | `crates/llm-orchestrator-core/src/agents/dependency_resolver.rs:1440` | `assert!(!response.parallel_groups.is_empty())` | `parallel_groups` came back empty — no parallel group was computed for a 4-task graph that resolved successfully |
| `agents::state_machine_agent::tests::test_transition_invalid` | `crates/llm-orchestrator-core/src/agents/state_machine_agent.rs:904` | `assert_eq!(response.status, TransitionStatus::Invalid)` | `left: Blocked, right: Invalid` — an illegal `completed → running` transition is reported as `Blocked` rather than `Invalid` |
| `agents::state_machine_agent::tests::test_no_change_transition` | `crates/llm-orchestrator-core/src/agents/state_machine_agent.rs:928` | `assert!(response.success)` | A self-transition `running → running` is not reported as a successful `NoChange` |

These are behavioural defects in the resolver and state-machine agents, not flaky or environmental
failures. The run is deterministic and completes in 0.22s with no network or filesystem dependency.

### What the committed documents claim

`docs/FINAL_PRODUCTION_VALIDATION.md` (dated 2025-11-14, header at `:3`) declares
`**Status:** ✅ **CERTIFIED PRODUCTION-READY**` at `:5`, `**Final Score:** **100/100 (Perfect)**` at
`:13`, and `**Certification Level:** **PLATINUM**` at `:14`. Its test table asserts:

- `:103` — `**Result:** **ALL TESTS PASSING**`
- `:109` — `| **llm-orchestrator-core** | 56 | 56 | 0 | ✅ PASS |`
- `:115` — `| **TOTAL** | **204+** | **204+** | **0** | ✅ **100%** |`
- `:162` — `| **Test Pass Rate** | 100% | 100% (204/204) | ✅ A+ |`
- `:396` — `| Test Failures | 1 | 0 | **100%** |`
- `:475` — `✅ **Zero test failures (204+ tests passing)**`
- `:492` — `**Confidence Level:** **100%**`

`docs/PRODUCTION_READINESS_CERTIFICATION.md` (also dated 2025-11-14, `:3`) declares
`**Certification Level:** ✅ **PRODUCTION READY**` at `:5` and `**Overall Grade:** **A+ (98/100)**`
at `:13`. Its claims:

- `:92` — `- [x] **Unit Tests** - 243+ comprehensive tests`
- `:94` — `- [x] **Test Pass Rate** - 100% (all passing)`
- `:145` — `| **Test Pass Rate** | 100% | 100% (243/243) | ✅ MET |`
- `:223` — `- **Total Tests:** 243+`
- `:414` — `- 243+ comprehensive tests (100% pass rate)`

Neither figure is reachable from any real run. The core crate has 198 lib tests, not 56. Three fail,
so the pass rate is 98.5%, not 100%.

### The two documents contradict each other

`FINAL_PRODUCTION_VALIDATION.md:115` totals **204+** tests; `PRODUCTION_READINESS_CERTIFICATION.md:145`
totals **243/243**. Both are dated 2025-11-14 and both claim a 100% pass rate on the same commit.
Two documents certifying the same tree on the same day cannot both be correct about a number they
disagree on by 39. This is dispositive on its own: neither document was transcribed from a shared
real test run, because a real run produces exactly one total.

### The documents were never revalidated after the code moved

```
$ git log --oneline --date=short --follow -- docs/FINAL_PRODUCTION_VALIDATION.md
9118a94 2025-11-15 docs: Reorganize documentation files into docs/ directory
c1c7a2e 2025-11-14 feat: Complete Phase 4 Optional Enhancements - Kubernetes, Security, DR, API Docs, Runbooks
```

Both files were authored in `c1c7a2e` (2025-11-14) and have been touched exactly once since, by
`9118a94`, a pure file move. Their content has never been updated. Meanwhile six commits have
changed the crate they certify:

```
$ git log --oneline --date=short 9118a94..HEAD -- crates/llm-orchestrator-core
4a73711 2026-02-10 feat: Instrument orchestrator as Foundational Execution Unit (FEU)
1a6a2a0 2026-01-25 feat: Add Phase 3 Automation & Resilience (Layer 1)
63ae636 2026-01-20 feat: Add Cloud Run deployment with HTTP server and agentics contracts
37fa6d5 2025-12-06 feat: Add Phase 2B LLM-Infra integration for Orchestrator
51f018e 2025-12-05 feat: Add Phase 2A/2B upstream dependency integrations
3cd70a8 2025-11-17 chore: Update license from dual MIT/Apache-2.0 to Apache-2.0 only
```

The certifications are roughly eight months stale and assert a pass rate over code written after
they were signed.

### CI already runs the suite — the gap is that nothing connects CI to the certification

`.github/workflows/ci.yml:36` runs `cargo test --all --verbose`, and `:39` runs `cargo test --doc`.
The failing tests are in the default lib target, so CI executes them. The repository therefore
already possesses ground truth on every push. The defect is not missing test infrastructure; it is
that the certification documents are hand-authored prose with no data path from the CI job that
could have contradicted them. A human wrote numbers into a table, and nothing ever checked those
numbers against the run.

### Why this blocks ADR-0001

[ADR-0001](./ADR-0001-implement-real-orchestration-logic.md) establishes that the deployed Node.js
Cloud Function performs no orchestration and proposes moving the Rust engine into the live serving
path. Two of the three failures sit directly in that path:

- `dependency_resolver` is the crate logic intended to replace `handleDependencies`, which ADR-0001
  documents as echoing `tasks.length` with no topological order and no cycle detection. The failing
  assertion is that `parallel_groups` is non-empty — the exact capability the Node handler fakes and
  that ADR-0001 relies on the Rust implementation to supply for real.
- `state_machine_agent` is the intended replacement for `handleStateMachine`, which ADR-0001 singles
  out as the one handler doing genuine work (contract transition-table lookup). Promoting the Rust
  agent in its place would currently be a regression: the Rust version misclassifies an invalid
  transition as `Blocked` and fails to handle a no-op self-transition.

Migrating to a backend whose own tests say it computes the wrong answer would replace fake
orchestration with incorrect orchestration.

## Decision

**1. Both certification documents are withdrawn, effective immediately, and must be corrected or
explicitly marked stale.** Neither `docs/FINAL_PRODUCTION_VALIDATION.md` nor
`docs/PRODUCTION_READINESS_CERTIFICATION.md` may continue to assert production readiness, a 100%
pass rate, a PLATINUM/A+ grade, or a `100/100` score while the measured result is 195/198 with three
substantive failures. Until regenerated from a real run they carry a withdrawal banner naming the
measured numbers.

**2. Certification and status documents are generated artifacts, never hand-authored.** Any document
asserting a test count, pass rate, or readiness grade must be produced by a script that parses
machine-readable test output, and must be stamped with the provenance of the run it came from
(commit SHA, CI run ID, workflow URL, UTC timestamp, exact command). A certification document with
no provenance stamp is invalid by construction and may be deleted on sight. Hand-editing a generated
section is a CI failure, enforced by regenerating in CI and diffing.

**3. A green suite is a precondition for any production-readiness claim.** The three failing tests
must be fixed — not deleted, not `#[ignore]`d, not asserted-around — before any document in this
repository may describe the orchestrator as production-ready. Because `dependency_resolver` and
`state_machine_agent` are load-bearing for ADR-0001's migration, fixing them is also a prerequisite
for that migration, not merely for the paperwork.

**4. Grades and scores that are not derived from a measurement are removed rather than recomputed.**
Figures like `98/100`, `PLATINUM`, and `Confidence Level: 100%` have no defined computation and
cannot be regenerated from CI output. They are removed rather than given a new hand-picked value.

## Consequences

**Positive.** The certification claim becomes falsifiable and self-maintaining: the document cannot
drift from the suite because it is a function of the suite. The eight-month staleness window closes,
since a stale document is visibly stamped with an old commit SHA. Readers gain the ability to
distinguish measurement from assertion. The three real defects, which have been masked by the
paperwork for months, become visible and get fixed.

**Negative / costs.** The repository will, correctly, stop describing itself as production-ready
until the failures are fixed — an accurate but less flattering posture that may surprise downstream
consumers who took the existing certification at face value. Generated documents are less
expressive than hand-written prose: narrative context must move to a separate, clearly non-normative
document. Someone must build and maintain the generator and the CI wiring.

**Risks.** The generator becomes a trusted component; a bug in it produces confidently wrong output
with an authentic-looking provenance stamp, which is worse than obvious prose. It must therefore be
tested (including a fixture with known failures) and kept small. There is also a real risk that the
three failures are "fixed" by weakening the assertions rather than correcting the logic — the
implementation plan addresses this explicitly.

**Scope note.** This ADR governs certification and status documents. It does not restrict design
documents, ADRs, or runbooks, which are legitimately hand-authored because they assert intent rather
than measurement.

## Implementation Plan

1. **Add withdrawal banners (immediate, no code change).** Insert at the top of both
   `docs/FINAL_PRODUCTION_VALIDATION.md` and `docs/PRODUCTION_READINESS_CERTIFICATION.md`, directly
   under the `#` heading:

   > **⚠️ WITHDRAWN — 2026-07-27.** The results below were never produced by a real test run and are
   > superseded. Measured on 2026-07-27 at commit `9766e44`: `cargo test -p llm-orchestrator-core
   > --lib` yields **195 passed, 3 failed, 198 total (98.5%)**. Failing:
   > `agents::dependency_resolver::tests::test_resolve_success`,
   > `agents::state_machine_agent::tests::test_no_change_transition`,
   > `agents::state_machine_agent::tests::test_transition_invalid`. This document does not certify
   > production readiness. See ADR-0002.

   Correct the specific false lines in place: `FINAL_PRODUCTION_VALIDATION.md` `:5`, `:13`, `:14`,
   `:103`, `:109`, `:115`, `:162`, `:396`, `:475`, `:492`; and
   `PRODUCTION_READINESS_CERTIFICATION.md` `:5`, `:13`, `:92`, `:94`, `:145`, `:223`, `:414`.

2. **Adopt machine-readable test output.** Add `cargo-nextest` to CI and standardise on
   `cargo nextest run --all --message-format libtest-json` (or, if nextest is undesirable,
   `cargo test --all -- -Z unstable-options --format json` on the pinned toolchain). Nextest is
   preferred: stable JSON, per-test timing, and a non-zero exit that cannot be swallowed by a pipe.

3. **Write the generator at `scripts/generate-test-certification.py`.** Contract:
   - Reads the JSON event stream on stdin plus `--commit`, `--run-id`, `--run-url`, `--command`.
   - Aggregates per-crate and total `passed` / `failed` / `ignored`, and collects failing test names
     with their assertion file:line.
   - Emits `docs/TEST_CERTIFICATION.md` containing a provenance header (commit SHA, CI run ID and
     URL, UTC timestamp, exact command, toolchain version), the per-crate table, and an explicit
     failure list.
   - Emits a single verdict line: `PASS` only when `failed == 0`; otherwise `FAIL (N failing)`.
   - Never writes the words "production ready", "certified", or a letter grade. Readiness is a human
     decision that may *cite* this artifact; the artifact does not confer it.
   - Ships with unit tests over recorded fixtures, including one all-pass fixture and one carrying
     these three failures, asserting the generator reports `FAIL (3 failing)`.

4. **Wire it into `.github/workflows/ci.yml`.** In the `test` job, after the run step, pipe the JSON
   through the generator, upload `TEST_CERTIFICATION.md` as a build artifact, then run
   `git diff --exit-code docs/TEST_CERTIFICATION.md`. A non-empty diff means someone hand-edited a
   generated file, and fails the build. Use `if: always()` so the certification is generated on
   failing runs too — a certification that only appears when tests pass reintroduces exactly the
   blind spot this ADR closes.

5. **Add a guard against reintroduction.** A CI step greps `docs/` for readiness vocabulary
   ("CERTIFIED PRODUCTION-READY", "Test Pass Rate", "PLATINUM", "/100") in files lacking a provenance
   header, and fails with a pointer to this ADR. Seed its allowlist with the two withdrawn documents
   only until step 7 retires them.

6. **Fix the three failures — in the logic, not the assertions.** Each fix is a separate commit whose
   message states the defect, and none may modify the assertion to match observed behaviour unless a
   written rationale explains why the *test* encoded the wrong contract:
   - `dependency_resolver.rs` — parallel-group computation returns empty for a successfully resolved
     4-task graph; produce the correct groups (`:1440`).
   - `state_machine_agent.rs` — an illegal `completed → running` transition must yield
     `TransitionStatus::Invalid`, not `Blocked` (`:904`).
   - `state_machine_agent.rs` — a `running → running` self-transition must succeed as
     `TransitionStatus::NoChange` (`:928`).

7. **Retire the withdrawn documents.** Once step 6 lands and CI is green, delete both files and
   replace them with a short pointer to the generated `docs/TEST_CERTIFICATION.md`. Do not
   resurrect them with new hand-written numbers.

8. **Gate ADR-0001 on step 6.** Record in ADR-0001's implementation sequencing that promoting
   `dependency_resolver` and `state_machine_agent` into the live serving path is blocked until their
   tests pass, so the migration does not swap absent orchestration for wrong orchestration.

## Verification

This ADR is satisfied when all of the following hold:

1. `cargo test -p llm-orchestrator-core --lib` reports `test result: ok. 198 passed; 0 failed` and
   `cargo test --all` is green.
2. `docs/TEST_CERTIFICATION.md` exists, and its stated totals equal the output of a fresh local run
   on the same commit. Verify by re-running the generator locally and diffing.
3. Deliberately breaking one test in a scratch branch causes CI to fail *and* still produces a
   `TEST_CERTIFICATION.md` whose verdict line reads `FAIL (1 failing)` and which names the broken
   test. This confirms the artifact reports bad news, which is the property the current documents
   lack.
4. Hand-editing a number in `docs/TEST_CERTIFICATION.md` and pushing causes the
   `git diff --exit-code` step to fail.
5. `grep -rniE "certified production-ready|243/243|204/204|100% \(all passing\)|PLATINUM" docs/`
   returns no hits outside this ADR.
6. No file under `docs/` asserts a test count or pass rate without a provenance header naming a
   commit SHA and CI run ID.
7. ADR-0001 records the dependency declared in step 8.
