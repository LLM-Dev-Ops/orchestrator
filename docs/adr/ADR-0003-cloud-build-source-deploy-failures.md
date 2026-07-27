# ADR-0003: Fix `llm-orchestrator` Cloud Build Failures — Repair the `.gcloudignore` That Excludes the Entire Rust Source Tree

**Status:** Implemented
**Date:** 2026-07-27

## Context

`gcloud run deploy llm-orchestrator --source=/workspace/agentics-dev/orchestrator --quiet` failed at the "Building Container" step with no usable diagnostics:

```
ERROR: (gcloud.run.deploy) Build failed; check build logs for details
```

No build ID, no log URL, and `gcloud builds list` showed nothing from the attempt. The working hypothesis was a Cloud Build resource or timeout limit, on the theory that `llm-orchestrator` is a large Rust workspace (~44k LOC, 10 crates) that compiles fine locally and in CI.

**That hypothesis is wrong.** Reproducing the failure with an explicit `gcloud builds submit` produced a real build ID and an unambiguous error in **19 seconds** (`21:27:55Z` → `21:28:14Z`). Nothing about this is resource-related.

### Finding 1 — the `.gcloudignore` excludes the Dockerfile and the entire Rust workspace

Build `18ec06ef-e06e-4624-9239-6cc7ff5296e4` (FAILURE):

```
Creating temporary archive of 25 file(s) totalling 115.0 KiB before compression.
Uploading tarball of [.] to [gs://agentics-dev_cloudbuild/source/1785187673...tgz]

FETCHSOURCE
Fetching storage object: gs://agentics-dev_cloudbuild/source/1785187673...tgz
Operation completed over 1 objects/26.3 KiB.
BUILD
Already have image (with digest): gcr.io/cloud-builders/gcb-internal
unable to prepare context: unable to evaluate symlinks in Dockerfile path: lstat /workspace/Dockerfile: no such file or directory
ERROR: build step 0 "gcr.io/cloud-builders/gcb-internal" failed: step exited with non-zero status: 1
```

**25 files, 115 KiB.** A 10-crate Rust workspace was uploaded as 115 KiB of shell scripts and JSON. The cause is `orchestrator/.gcloudignore`, which excludes exactly the things the container build requires:

```
# Rust build artifacts
target/
crates/          <- the entire source tree
...
# Docker
Dockerfile       <- the build recipe itself
.dockerignore
...
# Rust config
Cargo.toml
Cargo.lock
```

Confirmed directly — the upload manifest contains no Rust and no Dockerfile:

```
$ gcloud meta list-files-for-upload | grep -iE '^(Dockerfile|Cargo.toml|package.json|index.js)$'
package.json
index.js
```

The reason this file exists in this shape is visible in its own comments: it was written for a **Cloud Functions** deploy of `functions/index.js`, not for Cloud Run.

```
# NPM binary wrappers (not needed for Cloud Function)
npm/
```

`gcloud functions deploy` and `gcloud run deploy --source` read the *same* `.gcloudignore`. A file authored to slim down a Node.js function deploy was silently inherited by the Rust container deploy, and it excludes precisely the Rust container's inputs. The `.dockerignore` in this repo is correct and irrelevant — it is not consulted for the source upload, only inside the builder after the tarball has already been assembled.

This also explains the missing `gcloud builds list` entries. With no `Dockerfile` in the uploaded context, `gcloud run deploy --source` does not run a Docker build at all — it falls back to buildpack auto-detection, sees `package.json` and `index.js`, and attempts a **Node.js** build of a Rust service. That path reports through a different surface, which is why the failure was invisible from the CLI all night.

### Finding 2 — with a correct `.gcloudignore`, the build succeeds end-to-end on the default machine

A staged copy of the repo carrying a corrected `.gcloudignore` uploaded **148 files** instead of 25 and **built successfully**.

Build `843006b4-1e45-4313-9740-548f16dd1827`: **`SUCCESS` in 7 minutes 44 seconds** (`21:36:56Z` → `21:44:40Z`), on the **default Cloud Build machine type**, with no `cloudbuild.yaml`, no machine-type override, and no timeout extension. All 16 Dockerfile steps completed:

```
Step 3/16 : COPY Cargo.toml Cargo.lock ./
Step 4/16 : COPY crates/ ./crates/
Step 5/16 : RUN cargo build --release --bin llm-orchestrator
Step 6/16 : RUN ls -la /build/target/release/llm-orchestrator && \
              /build/target/release/llm-orchestrator --version
Step 7/16 : FROM debian:bookworm-slim
...
Step 16/16 : CMD ["serve"]
DONE
```

**This is the decisive result: the `.gcloudignore` fix alone is sufficient.** A ~44k-LOC, 10-crate Rust workspace builds cold, with zero dependency caching, in under 8 minutes on the default Cloud Build machine. The resource-limit hypothesis is not merely unproven — it is directly contradicted by a green build.

Note also that this repo's CI (`.github/workflows/docker.yml`) runs on `ubuntu-latest` — a 2-vCPU standard runner, not a large one. The premise that "CI has a bigger machine than Cloud Build" does not hold either.

### Finding 3 — unanchored ignore patterns silently delete Rust source (reproduced)

The first corrected-`.gcloudignore` attempt (build `b93ecb88-d94f-488f-b6a7-8ad10ebc6ddb`) still failed, but on a *different* and highly instructive error:

```
error[E0583]: file not found for module `benchmarks`
error: could not compile `llm-orchestrator-benchmarks` (lib) due to 1 previous error
The command '/bin/sh -c cargo build --release --bin llm-orchestrator' returned a non-zero code: 101
```

This is not a source defect. `crates/llm-orchestrator-benchmarks/src/lib.rs:30` declares `pub mod benchmarks;` and `crates/llm-orchestrator-benchmarks/src/benchmarks/` exists on disk. The staged `.gcloudignore` contained an **unanchored** `benchmarks/` line intended to drop the top-level `benchmarks/` directory. `.gcloudignore` uses gitignore matching semantics, so `benchmarks/` matches a directory of that name at *any* depth — including `crates/llm-orchestrator-benchmarks/src/benchmarks/`. Confirmed:

```
$ gcloud meta list-files-for-upload | grep -c 'llm-orchestrator-benchmarks/src/benchmarks'
0                     # unanchored "benchmarks/"
$ # after changing the pattern to "/benchmarks/"
7
```

Anchoring the pattern (`/benchmarks/`) restored the files. This failure mode is worth recording because it is *indistinguishable from a real compile error in the logs* — it presents as `E0583` and sends you looking for a source bug that does not exist. The same trap applies to `llm-shield`, which has `crates/llm-shield-benchmarks/src/benchmarks/`, `crates/llm-shield-api/src/models/`, and `crates/llm-shield-dashboard/src/models/`.

### Finding 4 — resource limits are ruled out

No `cloudbuild.yaml` exists in this repo, so the default machine type is in use. But the original failure occurred 19 seconds in, at context-preparation time, before any compiler ran. No OOM, no disk exhaustion, no timeout. Machine sizing was never the problem.

### Finding 5 (incidental) — no dependency layer caching

`Dockerfile:7-8` copies `Cargo.toml`, `Cargo.lock`, and all of `crates/` before the single `cargo build --release` at `Dockerfile:11`, with an explicit comment that this is "no dummy file caching - cleaner approach". Every source change therefore recompiles the entire dependency tree from scratch. This is a build-time cost, not a correctness bug, and is addressed as a follow-up rather than a blocker.

## Decision

1. **Rewrite `orchestrator/.gcloudignore` to serve the Cloud Run container build**, not the Cloud Functions deploy it was originally written for. It must not exclude `Dockerfile`, `Cargo.toml`, `Cargo.lock`, or `crates/`.
2. **Anchor every top-level exclusion with a leading `/`.** Unanchored directory patterns match at any depth and silently strip nested Rust modules, producing compile errors that misdirect debugging (Finding 3).
3. **Add a root `cloudbuild.yaml`** as the declared, reviewable build entrypoint, with `timeout: 1800s` and **no `machineType` override**. Keep the default machine. A measured green build at 7m44s (Finding 2) leaves ample headroom, and paying for `E2_HIGHCPU_8` to speed up an already-fast build is unjustified. Revisit only if measured build time approaches the timeout.
4. **Deprecate `gcloud run deploy --source` for this repo.** Use `gcloud builds submit --config=cloudbuild.yaml` followed by `gcloud run deploy --image=...`. The `--source` path is what silently swapped a Rust container build for a Node buildpack build and swallowed the build ID; that opacity cost a full night of debugging.
5. **Pin the builder base image.** `Dockerfile:2` uses `FROM rustlang/rust:nightly`, an unpinned floating tag. A deployable service should not have its toolchain change underneath it between builds.

The equivalent decision for `llm-shield` is recorded in `shield/docs/adr/ADR-0002`. Both repos are broken by the same root-cause *class* — `.gitignore` and `.dockerignore` do not govern what Cloud Build receives, and `.gcloudignore` does — but the concrete defects differ. `llm-orchestrator` has a wrong `.gcloudignore`; `llm-shield` has *no* `.gcloudignore` (so `gcloud` derives one from `.gitignore`, which drops `Cargo.lock`) plus a private-git-dependency authentication failure. Neither is a resource limit.

## Consequences

**Positive**

- The container build receives its actual inputs, so it can succeed at all.
- Failures produce real build IDs and greppable logs instead of `Build failed; check build logs for details`.
- Anchored patterns eliminate a class of misleading `E0583`-style compile errors.
- An explicit `cloudbuild.yaml` gives build configuration a reviewable home.

**Negative / costs**

- The upload grows from 115 KiB to roughly 11 MB. This is the correct size for the build and is immaterial.
- None on build cost: this ADR keeps the default machine type, so the fix is spend-neutral.
- Deploys become two commands instead of one — a deliberate trade of convenience for diagnosability.
- One repo cannot serve two deploy targets from a single root ignore file. A repo-wide search found no live `gcloud functions deploy` consumer (see Implementation Plan step 1), so this cost is currently theoretical — but if a function deploy is added later it will need its own `functions/.gcloudignore`.

**Risks**

- Rewriting `.gcloudignore` for Cloud Run would break an undocumented `gcloud functions deploy` workflow if one exists. Searched and none found; step 1 re-checks before merge.
- The unpinned `rustlang/rust:nightly` base means a build that passes today can fail tomorrow for reasons unrelated to any code change. Pinning is included in the plan.

## Implementation Plan

1. **Confirm no Cloud Functions deploy still depends on the current `.gcloudignore`.** A repo-wide search performed during this investigation found **no `gcloud functions deploy` or `firebase deploy` invocation anywhere** in `scripts/`, `.github/workflows/`, `deploy/`, or any tracked shell/YAML/JSON/Markdown file — so the file appears to be a leftover with no live consumer. Re-run the check before merging in case one has been added since:

   ```bash
   grep -rn 'functions deploy\|firebase deploy' orchestrator \
     --include='*.sh' --include='*.yml' --include='*.yaml' --include='*.json' \
     | grep -v '/target/'
   ```

   If a consumer turns up, relocate it to deploy from `functions/` with its own `functions/.gcloudignore` rather than repurposing the root file.

2. **Replace `orchestrator/.gcloudignore`** with a Cloud-Run-appropriate version. Note every top-level entry is anchored:

   ```
   # Upload exclusions for `gcloud builds submit` / `gcloud run deploy --source`.
   #
   # NOTE 1: this file previously targeted a Cloud Functions deploy and excluded
   #         Dockerfile, Cargo.toml, Cargo.lock and crates/ — i.e. the entire
   #         container build input. Do not re-add those.
   # NOTE 2: leading "/" anchors each pattern to the repo root. Do NOT drop it:
   #         an unanchored "benchmarks/" also matches
   #         crates/llm-orchestrator-benchmarks/src/benchmarks/ and breaks the
   #         build with a misleading "error[E0583]: file not found for module".
   .git/
   .github/
   /target/
   **/target/
   **/node_modules/
   /docs/
   /benchmarks/
   /examples/
   /plans/
   /helm/
   /deploy/
   /npm/
   /tests/
   *.log
   .env
   .env.*
   ```

3. **Pin the builder base image** in `Dockerfile:2`. A search of `crates/` found **no `#![feature(...)]` attributes**, so nightly appears to be unnecessary. Prefer moving to a pinned stable base — `FROM rust:1.93-slim-bookworm AS builder`, matching what `shield/Dockerfile:11` already uses — over pinning a dated nightly. Note that stable images are slim and will need `pkg-config`/`libssl-dev` installed in the builder stage, as shield's Dockerfile does.

4. **Create `orchestrator/cloudbuild.yaml`**:

   ```yaml
   # Default machine type is deliberate: a measured cold build of this
   # workspace completes in ~7m44s on it (ADR-0003 Finding 2). Do not add
   # options.machineType without a measurement justifying the spend.
   timeout: 1800s

   options:
     logging: CLOUD_LOGGING_ONLY

   steps:
     - id: build
       name: gcr.io/cloud-builders/docker
       args:
         - build
         - -t
         - '$_IMAGE:$SHORT_SHA'
         - -t
         - '$_IMAGE:latest'
         - .

   substitutions:
     _IMAGE: us-central1-docker.pkg.dev/agentics-dev/cloud-run-source-deploy/llm-orchestrator

   images:
     - '$_IMAGE:$SHORT_SHA'
     - '$_IMAGE:latest'
   ```

5. **Verify the upload manifest before spending a build.** This is free and instant, and would have caught the original defect immediately:

   ```bash
   cd orchestrator
   gcloud meta list-files-for-upload | grep -xE 'Dockerfile|Cargo.toml|Cargo.lock'
   gcloud meta list-files-for-upload | wc -l
   gcloud meta list-files-for-upload | grep -c 'llm-orchestrator-benchmarks/src/benchmarks'
   ```

6. **Update the deploy runbook** under `docs/runbooks/deployment/` to the two-step flow, and remove any `gcloud run deploy --source` invocation:

   ```bash
   gcloud builds submit --project=agentics-dev --config=cloudbuild.yaml .
   gcloud run deploy llm-orchestrator --project=agentics-dev --region=us-central1 \
     --image=us-central1-docker.pkg.dev/agentics-dev/cloud-run-source-deploy/llm-orchestrator:latest
   ```

7. **Follow-up (separate PR): add dependency layer caching.** Either adopt `cargo-chef`, or mirror the manifest-plus-dummy-source pattern already used in `shield/Dockerfile:23-62`. Measure cold vs. warm build time before and after so the change is justified by data rather than assumption.

## Verification

The fix is verified when all of the following hold:

1. **Upload manifest (pre-build, instant).**
   - `gcloud meta list-files-for-upload` lists `Dockerfile`, `Cargo.toml`, and `Cargo.lock`.
   - Total file count is in the ~150 range, not 25.
   - `grep -c 'llm-orchestrator-benchmarks/src/benchmarks'` returns a non-zero count — this is the direct regression test for Finding 3.

   Baseline to beat: **25 files / 115.0 KiB**, containing no Rust and no Dockerfile.

2. **Context preparation succeeds.** Build logs reach `Step 3/16 : COPY Cargo.toml Cargo.lock ./` with no `lstat /workspace/Dockerfile: no such file or directory`. This is the regression test for Finding 1.

3. **Compilation completes.** Logs show `Step 5/16 : RUN cargo build --release --bin llm-orchestrator` running to completion with no `error[E0583]`, followed by the `--version` smoke check at `Dockerfile:14-15` passing.

4. **Build succeeds and is timed.**

   ```bash
   gcloud builds submit --project=agentics-dev --config=cloudbuild.yaml .
   gcloud builds list --project=agentics-dev --limit=1 --format='table(id,status,duration)'
   ```

   Must report `SUCCESS` with an image in `us-central1-docker.pkg.dev/agentics-dev/cloud-run-source-deploy/llm-orchestrator`.

   **Measured target: ~7m44s on the default machine type.** This is not an estimate — build `843006b4-1e45-4313-9740-548f16dd1827` achieved it during this investigation with exactly the `.gcloudignore` proposed here. A post-fix build materially slower than ~10 minutes indicates something else regressed. **Do not respond to a slow build by raising `machineType`** — for an uncached build, the dependency-caching follow-up (step 7) buys more than CPU does, and the measured baseline shows CPU is not the constraint.

5. **No regression in the Cloud Functions path.** If step 1 found a `gcloud functions deploy`, re-run it and confirm it still deploys from `functions/` with its own ignore file.

6. **Deploy is explicitly out of scope for this verification.** A successful image build proves the build is fixed. Promoting that image to the live `llm-orchestrator` Cloud Run service is a separate, separately-authorized step.
