# Revised Agentic Workflow: Reviewer-Driven Evidence Adjudication

Based on the discussion, the application of the Portable Agentic Evidence Standard has been reframed. The standard is not a heavy pre-merge bureaucracy that demands micro-PRs. Instead, it is an **evidence-gathering mandate driven by the Agent Reviewer**.

## The Course Correction

1.  **PR Size Reality:** You pointed out that demanding tiny PRs to appease an AI is backward. If the system is good, it should handle normal or even larger PRs.
2.  **Continuous Readiness:** PRs should be created *ready for review*. Draft PRs are an anti-pattern when working with autonomous agents.
3.  **The Role of the Reviewer:** The burden of proof isn't too high; the *direction* of the proof was wrong. The developer (or generating agent) shouldn't be forced to proactively write a massive YAML dossier of evidence before submitting a PR. Instead, **the Agent Reviewer should demand and verify the evidence.**

## The Three-Tier Implementation

This is how the standard should be pragmatically enforced without crushing velocity:

### Tier 1: Deterministic Fast-Fail (The Baseline)
This remains standard CI.
*   **What it does:** `cargo check`, `cargo fmt`, `cargo clippy`, and high-value invariant tests (A-Grade unit tests).
*   **Why it matters:** It filters out syntax errors and broken builds before wasting LLM API calls.
*   **Status:** Always runs on every push.

### Tier 2: The Agentic Reviewer (The Inquisitor)
This is where the Portable Standard actually lives. It doesn't run as a passive CI check; it runs as an **active PR Reviewer**.
*   **What it does:** When a PR is created, the Agent Reviewer reads the diff and the `contract.yaml` manifest.
*   **The Inquisitor Pattern:** 
    *   Instead of failing the PR with "You didn't provide a Waiver ID," the Reviewer *asks* for the evidence: 
        *   *"I see you changed the default audio format. `docs/northstar.md` says we prioritize stability over latency here. Can you provide evidence (a log or test result) that stability is maintained?"*
    *   If the PR author (human or agent) provides a link to an integration test result or a pasted log snippet in the comments, the Reviewer assesses it. If the evidence satisfies the claim, the Reviewer approves.
*   **Why it works:** It shifts the standard from a "blocking wall" to a "guided conversation." It naturally scales with PR size because the reviewer only asks for evidence on *material claims*, ignoring trivial refactors.

### Tier 3: The Release Adjudicator (The Final Artifact)
This runs on merge to main or during a release cut.
*   **What it does:** It generates the **Delivery Artifact** (e.g., executing `cargo run` and capturing the stdout, or verifying the published package).
*   **Why it matters:** It guarantees the final product is livable, catching the "Green CI, Broken Product" drift that Tier 1 tests miss.

## Summary

The standard is brilliant, but it is a **Review Strategy**, not a build-script. By placing the burden of evidence-gathering into an asynchronous conversation with an Agent Reviewer, we maintain high velocity, allow normal-sized PRs, and still enforce rigorous semantic consistency.
