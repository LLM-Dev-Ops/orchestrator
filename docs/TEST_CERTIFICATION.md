# Test Certification

<!-- GENERATED FILE -- DO NOT EDIT BY HAND.
     Produced by scripts/generate-test-certification.py from a test run's JSON output.
     CI regenerates the measured sections below and fails if they differ from this file.
     See docs/adr/ADR-0002-certification-document-integrity.md. -->

## Provenance

| Field | Value |
|---|---|
| Commit | `de5363caefb714057df1851505246e63ff8803b5` |
| CI run ID | `local-2026-07-27` |
| CI run URL | n/a — generated from a local run, not CI |
| Command | `cargo nextest run --all --lib --bins --no-fail-fast --message-format libtest-json` |
| Toolchain | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |
| Generated at (UTC) | `2026-07-27T06:34:57Z` |

## Results by crate

**Verdict:** `FAIL (3 failing)`

| Crate | Passed | Failed | Ignored | Total |
|---|---:|---:|---:|---:|
| `agentics-contracts` | 50 | 0 | 0 | 50 |
| `llm-orchestrator-audit` | 16 | 0 | 0 | 16 |
| `llm-orchestrator-auth` | 52 | 0 | 0 | 52 |
| `llm-orchestrator-benchmarks` | 17 | 0 | 0 | 17 |
| `llm-orchestrator-cli` | 13 | 0 | 0 | 13 |
| `llm-orchestrator-core` | 198 | 0 | 0 | 198 |
| `llm-orchestrator-providers` | 48 | 2 | 0 | 50 |
| `llm-orchestrator-secrets` | 21 | 1 | 1 | 23 |
| `llm-orchestrator-state` | 30 | 0 | 1 | 31 |
| **Total** | **445** | **3** | **2** | **450** |

## Failures

| Test | Assertion site |
|---|---|
| `llm-orchestrator-providers` :: `cohere_embeddings::tests::test_multilingual_model` | `crates/llm-orchestrator-providers/src/cohere_embeddings.rs:601` |
| `llm-orchestrator-providers` :: `cohere_embeddings::tests::test_search_query_input_type` | `crates/llm-orchestrator-providers/src/cohere_embeddings.rs:433` |
| `llm-orchestrator-secrets` :: `cache::tests::test_cache_expiration` | `crates/llm-orchestrator-secrets/src/cache.rs:331` |

---

This document reports what the suite measured on the commit named above. It makes no
claim about production readiness and carries no grade.
