---
doc_type: proposal
subsystem: general
status: draft
freshness: current
preservation: preserve
last_reviewed: 2026-03-31
owners: Patrick MacLyman
version: 1.0.0
summary: Portable standard for change acceptance in agentic software workflows, with ColdVox failure mapping
---

# Portable Agentic Evidence Standard

## Purpose

Define a change-acceptance standard that applies across repositories of different languages, architectures, and product types.

This is not a test strategy for one codebase.
It is a portable evidence system for projects that use both deterministic tooling and agentic LLM workflows.

## Executive Summary

Deterministic tooling is necessary but not sufficient.

It is good at enforcing known invariants:

- build succeeds
- types are valid
- required files exist
- declared tests pass

It is bad at detecting drift in meaning:

- docs and code disagree about the canonical path
- CI still enforces a no-longer-relevant platform
- default build and default product have diverged
- tests pass because they skip the real path

The portable answer is not "replace deterministic gates with agents."
It is:

> Require an evidence bundle for every material claim, let agents generate and challenge that bundle adaptively, and adjudicate readiness using a fixed evidence policy.

This standard works across repos because the fixed part is the acceptance policy, not the implementation details of any one test suite.

## Why This Is Needed

Recent industrial evidence points in the same direction:

- Meta's "Just-in-Time Catching Test Generation" shows that change-aware generated tests can catch serious regressions that traditional pre-existing suites miss, but also shows the central problem is false positives and the need for assessment layers.  
  Source: https://arxiv.org/abs/2601.22832
- Meta's mutation-guided LLM test generation work shows LLMs are useful for generating targeted hardening tests around specific concerns rather than only broad generic tests.  
  Source: https://arxiv.org/abs/2501.12862
- NIST SSDF is intentionally outcome-oriented and designed to be integrated into different SDLC implementations rather than prescribing a single fixed workflow.  
  Source: https://csrc.nist.gov/pubs/sp/800/218/final
- Google SRE release engineering emphasizes that the same test targets that gate release should be the ones run continuously, and that releases should be backed by audit trails and system-level testing on the packaged artifacts.  
  Source: https://sre.google/sre-book/release-engineering/
- Google SRE canary guidance emphasizes that release safety requires an evaluation process integrated into deployment, not just earlier acceptance-stage tests.  
  Source: https://sre.google/workbook/canarying-releases/
- AWS test observability guidance reinforces that execution evidence needs logs, traces, metrics, and correlation, not just binary pass/fail status.  
  Source: https://docs.aws.amazon.com/prescriptive-guidance/latest/performance-engineering-aws/test-observability.html

Taken together, these support a portable standard based on:

- explicit claims
- artifact-backed evidence
- adaptive test generation and review
- delivery and publication evidence
- auditable adjudication

## Core Principle

Every **change unit** should be justified by a **claim-to-evidence map**.

A change unit may be:

- a pull request
- a merge request
- a Gerrit CL
- a commit range
- a release candidate
- a package promotion
- a content publication
- an infrastructure rollout

Not:

- "the CI was green"
- "two agents agreed"
- "nothing obvious broke"

But:

- what changed
- what could regress
- what environments matter
- what evidence was produced
- who or what independently challenged that evidence

## The Standard

### 1. Every change declares claims

A change unit does not get accepted as a raw diff. It gets accepted as a set of claims.

Examples:

- "Default startup still works."
- "The canonical API path has changed."
- "This is documentation-only."
- "Windows is now the required runtime target."
- "This feature is optional and not part of product-default."

These claims can be authored by humans, inferred by agents, or both.
They should be normalized into a machine-readable bundle.

Portable claim schema:

- `claim_id`
- `statement`
- `claim_type`
- `affected_surfaces`
- `risk`
- `authoritative_source`
- `required_evidence_classes`
- `status`
- `disposition`

Portable rule:

- if an assessor discovers a missing material claim, that omission becomes an explicit blocker until the claim is added, waived, or disproven

### 2. Every material claim needs evidence

The evidence can vary by repo and by change type.
The standard does not require one specific test harness.

Allowed evidence types:

- build artifact evidence
- static analysis evidence
- unit or integration test results
- generated catching tests
- mutation or fault-injection results
- scenario execution traces
- screenshots or recordings for UI claims
- logs, traces, and metrics
- canary or staged-delivery observations
- configuration and documentation consistency checks
- independent reviewer findings

The policy requirement is:

> A material claim is not accepted unless the repo has at least one relevant primary artifact for it.

Portable evidence envelope:

- `evidence_id`
- `claim_id`
- `artifact_type`
- `producer_role`
- `reviewer_role`
- `executor`
- `model_id`
- `run_id`
- `parent_run_id`
- `revision`
- `diff_base`
- `working_tree_state`
- `feature_flags`
- `config_hash`
- `environment_fingerprint`
- `input_artifact_ids`
- `result`
- `timestamp`
- `artifact_uri`
- `artifact_digest`

Portable rule:

- adjudication should operate on evidence records with this minimum envelope, not on free-form agent narratives

### 3. Medium- and high-risk claims require independent corroboration

This is where agents belong.

Corroboration can come from:

- a second execution path
- a second artifact type
- a second agent with a different role
- a human reviewer
- a production-like canary

Two agents agreeing with no inspectable artifact is not sufficient.
Two agents agreeing over the same artifact is useful but still weaker than independent evidence.

Portable rule:

- low risk: one primary artifact
- medium risk: one primary artifact plus one independent corroboration
- high risk: multiple artifacts plus independent corroboration plus artifact-appropriate delivery evidence, rollback clarity, or downstream-consumer evidence

Independence policy:

- a producer may not be the sole assessor of its own evidence for medium- or high-risk claims
- medium- and high-risk corroboration must come from a distinct role, distinct run, and distinct provenance chain
- self-assessment may be kept as advisory evidence, but not as the only corroborating signal
- evidence should retain `producer_role`, `reviewer_role`, `run_id`, and `parent_run_id` so collapsed independence is detectable

### 3a. Risk floors and escalation triggers

Portable rule:

the following changes carry a minimum risk floor even if an author or agent underrates them:

- default entrypoint changed
- support matrix changed
- runtime config or feature defaults changed
- delivery or publication path changed
- authoritative docs changed
- automation runner or environment changed
- auth, security, or policy path changed

Suggested minimums:

- default surface changed: high
- support matrix changed: high
- automation surface changed: medium
- authoritative contradiction introduced: high
- publication or deployment path changed: high

### 4. Skip is not success

This should be universal.

If a required claim is validated by a check that skipped, that claim is unresolved.

The portable rule is:

- pass counts as positive evidence
- fail counts as negative evidence
- skip counts as no evidence

This single rule prevents a large class of false greens across every repo type.

### 5. Freshness matters

Evidence must be tied to the revision being considered.

Portable rule:

- old results do not satisfy new claims
- artifacts must reference the current diff, current head, or current release candidate
- if the change alters packaging, branch state, config, or deployment path, evidence must be re-produced for the built artifact or release candidate, not only for mainline source

### 6. Delivery evidence matters too

Pre-acceptance automation is not enough for many repos.

Portable rule:

- if the change affects runtime behavior, deployment semantics, publication semantics, or downstream-consumer interfaces, there must be a post-build, pre-publication, or promotion-stage validation phase
- artifact-appropriate delivery evidence can include canary evaluation, package install/import checks, downstream compatibility tests, rendered-site preview, firmware flash/boot, model export/inference sanity, archive verification, or store-submission dry runs

### 7. Contradictions are first-class failures

This is the biggest gap in many repos.

Portable rule:

if two authoritative artifacts disagree about the same concept, the contradiction itself is a blocker until explicitly resolved or waived.

Authority policy:

- authorities must be declared with explicit precedence
- when two authorities conflict, the higher-precedence authority wins by default
- lower-precedence operational defaults, plans, or commands become blockers until aligned or explicitly waived

Waiver policy:

- waivers require a structured record, not a casual approval
- required fields:
  - `waiver_id`
  - `scope`
  - `authority`
  - `reason`
  - `linked_risk`
  - `compensating_evidence`
  - `expiry`
  - `required_follow_up`
- some claim classes should be non-waivable by policy, for example `security` or `product_default`
- expired waivers revert to blockers automatically

Unknown-surface policy:

- if a new normative surface appears and is not classified as authority, advisory, validation, or publication material, it is a blocker until mapped or explicitly exempted

Typical contradiction classes:

- docs vs config
- config vs build defaults
- CI vs declared support matrix
- commands vs actual scripts
- tests in tree vs tests registered
- release target vs required runner platform

## Portable Tooling Architecture

This standard is portable if the tooling is built around generic interfaces rather than repo-specific assumptions.

### A. Contract Manifest

Each repo supplies a small declarative manifest.
This is the only repo-specific onboarding requirement.

Example:

```yaml
version: 1

project:
  name: example
  artifact_types:
    - service
    - package
    - publication

support_targets:
  required:
    - primary-supported-target
  optional:
    - compatibility-target

validation_surfaces:
  - name: primary_behavior
    kind: runtime_or_consumer
  - name: automation_entrypoint
    kind: automation
  - name: rendered_preview
    kind: publication

publication_surfaces:
  - name: release_candidate
    kind: deliverable

claims:
  primary_supported_behavior:
    description: "normal supported path for intended consumers"
    required_evidence_classes:
      - primary_artifact
      - behavior_artifact
  automation_entrypoint_validity:
    description: "documented automation entrypoints remain valid"
    required_evidence_classes:
      - primary_artifact

risk_floors:
  default_surface_changed: high
  support_matrix_changed: high
  automation_surface_changed: medium

authorities:
  - path: docs/product.md
    precedence: 1
  - path: docs/status.md
    precedence: 2
  - path: docs/architecture.md
    precedence: 3

waiver_policy:
  forbidden_claim_types:
    - security
    - product_default
```

This works for any stack because the repo only declares:

- authoritative docs
- supported targets
- validation or publication surfaces
- what counts as primary supported behavior

### B. Surface Inventory Engine

A portable agent or static tool inventories:

- build or package targets
- validation surfaces
- automation surfaces
- publication surfaces
- config files
- support matrix declarations
- test surfaces, when tests exist
- externally hosted contract artifacts
- docs that contain normative phrases like `default`, `canonical`, `supported`, `required`

This is repo-agnostic.
Only the parsers differ by ecosystem.

### C. Drift and Contradiction Engine

This engine compares semantic statements across artifacts.

Examples of universal checks:

- a documented entrypoint is invalid on the primary supported environment
- declared support matrix conflicts with required CI runner labels
- default runtime config selects a backend not compiled by default
- docs say "supported now: X" while commands default to Y

This layer is exactly where agentic reasoning adds value over deterministic unit tests.

### D. Scenario Synthesizer

An agent turns change claims and diffs into candidate scenarios:

- startup smoke
- API request/response validation
- UI happy path
- packaging/build path
- migration path
- rollback path
- environment-specific path
- downstream consumer compatibility
- schema, data, or stored-state compatibility
- rendered or publication correctness
- reproducibility and provenance
- policy or compliance validation

These scenarios can be executed by deterministic runners, browser automation, service probes, or local commands.

### E. Artifact Collector

Every execution emits structured artifacts:

- command or action
- environment
- revision and diff base
- working tree state
- feature flags and config hash
- input artifact identifiers
- stdout/stderr
- exit code
- logs, traces, and metrics
- screenshots, transcript samples, or published outputs
- skip, pass, fail, and flake classification
- artifact URI and digest

Without this layer, agent agreement is not auditable.

### F. Independent Verifier Agents

At least two generic roles are portable:

- **Cartographer**: discovers surfaces, contradictions, and missing checks
- **Assessor**: judges whether the evidence bundle actually supports the claims

Optional roles:

- **Scenario Generator**: proposes targeted regression checks
- **Mutation/Catching Agent**: generates change-aware tests for risky diffs
- **Policy Agent**: enforces manifest and organizational standards

The point is not "more agents."
The point is role separation so that generation and assessment are not collapsed into one viewpoint.

Operational requirement:

- the same agent lineage must not both generate and independently validate the same high-risk evidence bundle

### G. Adjudicator

The adjudicator is allowed to be deterministic or policy-driven, but it decides over artifacts and claims, not vibes.

Portable adjudication rules:

- every required claim has evidence
- no required claim is backed only by skip results
- required corroboration exists when risk demands it
- unresolved contradictions are either fixed or explicitly waived under policy
- required support targets have coverage somewhere in the bundle
- unknown normative surfaces are classified before acceptance
- waivers are structured, time-bounded, and visible in the evidence ledger

This is the stable part of the system.

## What This Looks Like In Practice

### Minimal adoption

1. Add a repo contract manifest.
2. Inventory validation surfaces, automation surfaces, support targets, and authoritative docs.
3. Add skip-detection and orphan-test detection.
4. Require claim-to-evidence output for every change unit.
5. Add one verifier agent to check for contradictions and insufficient evidence.

### Mature adoption

1. Diff-aware scenario generation
2. Mutation-guided or catching-test generation on risky changes
3. Runtime observability collection for test and canary flows
4. Independent assessor agents
5. Delivery or publication evidence collection
6. Persistent memory of past failure patterns for risk scoring

## How This Standard Would Have Caught the ColdVox Problems

### 1. Default app build had no STT backend compiled in

Observed problem:

- default app features enabled VAD and text injection but no STT backend
- default runtime config still required STT

Detector:

- contradiction engine compares default build features against default runtime config

Required evidence:

- startup smoke for `product_default`

Failure point:

- manifest says product-default requires one real STT path
- default startup artifact shows zero available plugins
- claim fails immediately

### 2. Default app startup failed at runtime

Observed problem:

- `cargo run -p coldvox-app --bin coldvox` failed with `No STT plugin available`

Detector:

- scenario synthesizer generates startup smoke for declared default entrypoint

Required evidence:

- exit-zero startup artifact

Failure point:

- runtime artifact negative
- product-default claim blocked

### 3. Golden test targeted non-default backends and skipped the real STT path

Observed problem:

- test status green
- actual output said no real STT backend feature enabled
- the registered product-path test only exercised `moonshine` or `parakeet`
- the operational path had drifted to `http-remote`, so the test selector no longer matched the default runtime path

Detector:

- scenario-to-default-path mismatch detection
- artifact collector classifies skip output separately from pass
- adjudicator forbids skip-only satisfaction of required claims

Required evidence:

- a passing, non-skipped behavior artifact for the real STT path

Failure point:

- "golden" test targets the wrong backend class and then produces no positive evidence
- claim remains unresolved

### 4. `autotests = false` hid test files in tree

Observed problem:

- tests existed under `crates/app/tests/` but were not registered in `Cargo.toml`
- files can exist under `tests/` and still never run when auto-discovery is disabled and the manifest omits them

Detector:

- surface inventory engine compares files on disk with registered test targets

Required evidence:

- test inventory consistency report

Failure point:

- unregistered tests flagged
- repo inconsistency warning elevated to blocker if coverage-critical files are orphaned

### 5. Registered `pipeline_integration` target contained zero tests

Observed problem:

- target existed
- execution listed `0 tests`

Detector:

- artifact collector records zero-test targets
- assessor marks them as dead evidence

Required evidence:

- each required target must contribute actual executed tests or be removed/reclassified

Failure point:

- dead target flagged as misleading

### 6. `just ci` was Bash-only and invalid on the primary Windows environment

Observed problem:

- `just ci` invoked a Bash script
- the repo's primary supported environment had shifted to Windows
- the documented local CI entrypoint was therefore not valid for the primary supported environment

Detector:

- platform-aware entrypoint execution detector
- contradiction engine compares support targets with required local automation surfaces

Required evidence:

- `automation_entrypoint_validity` artifact on the primary supported environment

Failure point:

- broken or platform-invalid command immediately detected

### 7. CI required a Fedora/Nobara Linux runner after the repo shifted to Windows-first

Observed problem:

- current product direction says Windows 11 priority
- active hardware CI still depends on Fedora/Nobara KDE Wayland runner

Detector:

- support-target consistency check compares authoritative docs and required CI runner matrix

Required evidence:

- support matrix alignment artifact

Failure point:

- required hardware CI platform no longer matches declared primary target
- contradiction becomes visible before the runner silently rots

### 8. Lower-authority operational defaults overrode higher-authority product docs

Observed problem:

- `northstar` said Moonshine supported now, Parakeet later
- config and `justfile` defaulted to HTTP remote Parakeet CPU

Detector:

- authority-weighted contradiction detection scans authoritative docs and normative operational files for `default`, `canonical`, `supported`, and `priority`

Required evidence:

- semantic consistency report

Failure point:

- the higher-precedence product authority wins
- lower-precedence defaults and plans become blockers until aligned or explicitly waived

### 9. Live remote services worked, but the app wiring did not

Observed problem:

- CPU and GPU services were healthy and transcribed correctly
- default app build still failed to reach any STT backend

Detector:

- evidence bundle requires both dependency/service health and application-path health when the claim is end-to-end behavior

Required evidence:

- service health probe
- application-path execution artifact

Failure point:

- dependency health alone does not satisfy end-to-end claim
- missing app-path success blocks acceptance

### 10. GUI checks were validating a demo seam, not the real product runtime

Observed problem:

- GUI/demo checks could pass while the real runtime path for STT, injection, hotkeys, and persistence was not wired through the product UI

Detector:

- claim-to-surface coverage detector compares UI claims against backend-bound runtime evidence

Required evidence:

- at least one UI artifact that exercises the real product-backed runtime path for any claim that the GUI validates end-to-end behavior

Failure point:

- demo-only UI evidence is classified as partial coverage
- end-to-end GUI claims remain unresolved until a backend-bound artifact exists

## Why This Standard Is Portable

It does not assume:

- Rust
- Cargo
- GitHub Actions
- UI applications
- backend services
- monorepo or polyrepo
- tests written by humans
- tests written by agents

It only assumes every project has:

- some authoritative statements
- some meaningful validation or publication surfaces
- some risk-bearing changes
- some observable artifacts

Those assumptions hold for effectively every software project.

## Recommended Minimum Policy For Any Repo

If only one policy is adopted, it should be this:

> No material claim may be accepted unless it has fresh artifact-backed evidence, and no required claim may be satisfied by a skipped check or by agent agreement without inspectable artifacts.

If a repo can adopt three policies, use these:

1. Define authoritative docs, support targets, and default entrypoints in a manifest.
2. Block on contradictions between docs, config, CI, commands, and packaged behavior.
3. Require independent corroboration for medium- and high-risk claims.

## Final Recommendation

For broad applicability, do not standardize on a specific test suite.
Standardize on:

- claim extraction
- evidence classes
- artifact freshness
- contradiction detection
- independent corroboration
- adjudication rules

Deterministic tooling remains one evidence producer.
Agentic systems become:

- scouts
- generators
- contradiction finders
- assessors

But the acceptance decision is made over the evidence bundle, not over confidence alone.

That is the portable standard.
