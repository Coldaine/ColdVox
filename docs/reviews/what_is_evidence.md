# Demystifying "Evidence" in the Agentic Standard

The language of the standard ("Claims," "Artifacts," "Dossiers") makes it sound like a legal proceeding. In reality, **"Evidence" is just a copy-paste of what you already look at on your screen when you are working.**

It shouldn't be excruciating to assemble. It's just a habit of capturing the output you already generate to prove to yourself that your code works.

Here is exactly what "proof" means, practically, for different types of changes.

## 1. Low-Risk Changes (Trivial Refactors, Typos, UI Tweaks)
*   **The Claim:** "I cleaned up the code and nothing broke."
*   **The Proof Required:** The standard Tier 1 CI run.
*   **What you actually provide:** Nothing. The automated `cargo check` and `cargo test` passing is the artifact. The Agent Reviewer sees the green checkmark and approves.

## 2. Medium-Risk Changes (Adding a Feature, Changing a Config)
*   **The Claim:** "I added a new configuration option for the audio buffer."
*   **The Proof Required:** A unit test OR a runtime log.
*   **What you actually provide:** 
    *   *Option A:* You added a test that fails if the config is invalid. (The CI log is your evidence).
    *   *Option B:* You run the app locally with the new config. You copy the terminal output where it says `[INFO] Loaded buffer size: 2048`. You paste that block into the PR comment.
*   **The Agent Reviewer:** Reads the pasted log in your comment, verifies it matches the new feature, and approves.

## 3. High-Risk Changes (Changing the Default Engine, Modifying the API, Shifting Platforms)
*   **The Claim:** "I swapped the default STT engine from Whisper to Parakeet."
*   **The Proof Required:** Independent corroboration that the *product* still works as intended.
*   **What you actually provide:**
    1.  **The CI passes** (Unit/Integration tests).
    2.  **A Delivery Artifact:** You run the full application. You talk into your microphone. You copy the terminal output showing the app booting, connecting to Parakeet, and printing the transcription of what you just said. You paste this entire block into the PR.
*   **The Agent Reviewer:** Sees that the default changed. Checks your pasted log. Verifies that the log shows a successful end-to-end transcription using the *new* default engine. Approves.

## Why this is fundamentally different (and better)

In a legacy workflow, if you swapped the default engine to Parakeet, you might just ensure `cargo test` passes. But if `cargo test` was accidentally hardcoded to only test the *old* Whisper engine, the CI would be green, you would merge, and the live app would crash.

In the Agentic Evidence workflow, the Agent Reviewer says: *"You changed a primary default. Show me the app running with the new default."*

By forcing you to paste that one terminal log, it forces you to actually boot the app. It takes you 10 seconds to copy and paste the log, but it completely eliminates the class of bugs where "the tests pass but the app is broken."

## Summary
You don't need to write YAML files or complex JSON schemas to provide evidence. 

**"Evidence" is just pasting the terminal output, the test log, or the screenshot that proves the thing you changed actually works in the real world.** The Agent Reviewer is smart enough to read the paste and understand what it means.