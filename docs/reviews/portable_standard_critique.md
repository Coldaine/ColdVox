# Critique: The ColdVox Test Review vs. The Portable Agentic Evidence Standard

This document evaluates the recent effort to clean up the ColdVox test suite ("Consolidated Test Improvement Plan") against the newly established **Portable Agentic Evidence Standard**.

## The Verdict

The recent test cleanup effort, while directionally sound in removing tautologies, was fundamentally operating on a **legacy paradigm**. It attempted to fix the test suite by classifying tests as "Unit" vs "Integration" or "Fragile" vs "Resilient."

According to the Portable Evidence Standard, this is the wrong axis entirely. **The problem with ColdVox isn't that its unit tests are bad; the problem is that its evidence-collection machinery is blind to semantic drift and operational reality.**

Here is how our previous test plan failed against the Portable Standard, and how it must be corrected.

---

## Failure 1: We focused on "Tests" instead of "Claims and Evidence"

**The Portable Standard Rule:** Every change unit must be justified by a *claim-to-evidence map*.

**Our Failure:** We graded tests in a vacuum. We looked at `f32_to_i16_basic` and said "this is an A-grade test because it checks math." But under the new standard, a test is only valuable if it provides an **Artifact** that supports a specific **Claim**.
If the claim is "The audio pipeline accurately preserves bit depth," then the test is evidence. If that claim is never made, the test is orphaned context. We were optimizing the *runners* without defining the *claims*.

## Failure 2: We ignored "Semantic Drift" (The Contradiction Engine)

**The Portable Standard Rule:** Contradictions are first-class failures. If two authoritative artifacts disagree, the contradiction itself is a blocker.

**Our Failure:** We spent hours deciding whether `test_transcription_config_default` (asserting `buffer_size == 512`) was a "Sanity Canary" or a "Tautology."
Under the new standard, this debate is obsolete. The test is useless not just because it's tautological, but because it doesn't use the **Contradiction Engine**.
The correct agentic approach isn't to write a unit test checking if a variable equals `512`. The correct approach is for a Cartographer Agent to read `docs/architecture.md` (which says "default buffer is 512ms") and compare it against `config/default.toml` and `src/stt/config.rs`. If they drift, the agent blocks the PR. **We were trying to use code to solve a semantic documentation problem.**

## Failure 3: We failed to recognize "Skip is not success"

**The Portable Standard Rule:** Pass counts as positive evidence, fail counts as negative evidence, skip counts as no evidence.

**Our Failure:** In our "Transformation (Phase 3)" plan, we proposed moving hardware tests behind `#[ignore]` and runtime skips (e.g., `if !is_audio_available() { return; }`).
Under the Portable Standard, this is a **catastrophic failure.** By silently skipping the test at runtime, we produce a "Green CI" without actually producing a **Primary Artifact** for the claim "Audio hardware capture works." We essentially designed a system to hide missing evidence.
Instead of silently skipping, the test must emit a `skip` artifact. If the repo manifest requires the claim `primary_supported_behavior` (which relies on audio), the Adjudicator must block the merge because "skip counts as no evidence."

## Failure 4: We ignored "Delivery Evidence"

**The Portable Standard Rule:** Pre-acceptance automation is not enough. There must be a post-build validation phase.

**Our Failure:** We assumed that if `cargo test --workspace` passed, the software was good. We completely missed the historical ColdVox failure where the tests passed because they skipped the real STT path, but `cargo run` crashed immediately in production.
The standard demands an `automation_entrypoint_validity` artifact. We need an agent to actually execute `just run` and capture an exit-zero artifact, rather than relying purely on internal rust test harnesses.

## Failure 5: We missed "Authority Policy" entirely

**The Portable Standard Rule:** Authorities must be declared with explicit precedence (e.g., Product Docs > Build Config).

**Our Failure:** We had no mechanism to detect when a low-level engineer changed `justfile` to default to an experimental remote STT backend, directly contradicting the `northstar.md` product vision. We were trying to catch regressions using Rust unit tests, completely blind to the fact that the project's operational defaults had mutinied against the product documentation.

---

## Course Correction: Aligning ColdVox with the Standard

To truly modernize ColdVox under this standard, we must abandon the "grading" of individual unit tests and implement the **Portable Tooling Architecture**:

1.  **Stop writing "Canary" unit tests.** Delete them all. Replace them with a Semantic Drift Agent (Cartographer) that runs on every PR, comparing `Cargo.toml`, `justfile`, `default.toml`, and the `.md` files in `docs/`.
2.  **Create a `contract.yaml` manifest.** Define `Windows 11` as the primary support target and `docs/northstar.md` as the highest authority.
3.  **Halt the Runtime Skip rollout.** Do not implement silent skips. Implement an Artifact Collector that explicitly logs skipped scenarios so the Adjudicator can fail the build for lack of evidence.
4.  **Add a Scenario Synthesizer step.** Before merge, an agent must literally execute `cargo run` in a headless environment and provide the `stdout` as a required Primary Artifact demonstrating startup liveness.

The previous test plan was trying to build a better mousetrap. The Portable Agentic Evidence Standard realizes we shouldn't be hunting mice; we should be securing the perimeter.
