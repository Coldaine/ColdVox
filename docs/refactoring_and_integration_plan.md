# Strategic Plan: Integrating Refactoring and Stabilizing `main`

## 1. Executive Summary

Recent development has been split between two conflicting efforts: a stable, behavior-preserving refactoring campaign (PR #115) and an unstable, incomplete feature introduction (PRs #112, #114). The attempt to merge these resulted in a broken `main` branch with a failing test suite, blocking all forward progress.

This document outlines the critical path forward. The strategy is to prioritize repository health by immediately merging the stable refactoring to fix the test suite and improve code quality. The valuable but unstable feature work will be re-introduced separately on this newly stabilized foundation. Adhering to this plan is vital to unblock development and ensure future changes are built on a reliable codebase.

---

## 2. Analysis of Recent Pull Requests

### PRs #113 & #115: The Successful Refactoring Campaign

-   **Goal:** A behavior-preserving refactoring of the stable codebase to improve maintainability, readability, and code quality.
-   **Outcome:** **Success.** This effort resulted in a stable, verified, and high-quality pull request (#115).
-   **Key Changes:**
    1.  **Baseline Test Fixes:** Resolved critical race conditions in the test suite by centralizing `tracing` initialization, turning a failing test suite into a green one.
    2.  **Theme A (Typed Constants):** Replaced hardcoded "magic numbers" (e.g., `16000`) with named, typed constants in `coldvox-audio`, making the code self-documenting.
    3.  **Theme B (UI Extraction):** Refactored the monolithic `draw_status()` TUI function into smaller, single-responsibility helper functions, dramatically improving readability.
    4.  **Theme C (STT Boilerplate Consolidation):** Created a `common.rs` module for STT plugins to share logic, reducing code duplication and adhering to the DRY principle.

### PRs #112 & #114: The Unstable Feature Campaign

-   **Goal:** Introduce a major new feature—a streaming STT pipeline—and advanced error handling.
-   **Outcome:** **Failure.** While the architectural vision was superior, the implementation was unstable, non-functional, and left the `main` branch with a broken build.
-   **Key Changes:**
    1.  **Full Streaming Pipeline:** Contained the logic for a real-time, chunk-based transcription engine. This is a valuable architectural goal.
    2.  **Comprehensive Error Handling:** Introduced `anyhow` and `thiserror` to provide better error context, a significant improvement for reliability.
    3.  **Build & Dependency Breakage:** The implementation relied on complex, unresolved feature flag configurations (`--features=whisper-cpp`) that broke the Cargo build process. The code was checked in a non-compiling state.

---

## 3. The Path Forward: A Three-Step Plan

This plan must be executed in order to restore repository health.

### Step 1: Immediately Merge Pull Request #115

-   **Why:** This PR is a pure win. It is verified, stable, and its changes are objective improvements. Most importantly, it **fixes the broken test suite**, establishing the green baseline required for all future development.
-   **Edits to Keep:** All of them.
    -   The centralized test setup.
    -   The typed constants.
    -   The refactored TUI functions.
    -   The consolidated STT plugin helpers.

### Step 2: Close and Abandon PRs #112, #113, and #114

-   **Why:** These pull requests are now obsolete, contaminated, or represent a failed development history. Their valuable ideas must be re-introduced cleanly. Keeping them open creates confusion.

### Step 3: Re-introduce Feature Work on a New Branch

-   **Why:** The streaming STT pipeline and improved error handling are valuable goals. They must be pursued on a stable foundation.
-   **Action Plan:**
    1.  Create a new `feature/stt-streaming` branch from `main` *after* PR #115 has been merged.
    2.  On this new, clean branch, re-implement the changes from PR #114 incrementally.
    3.  **Task 1:** Integrate `anyhow` and `thiserror`. Get the project building and passing all tests with the new error handling.
    4.  **Task 2:** Implement the streaming pipeline architecture. Write and update tests as you go. Ensure CI remains green at every step.
    5.  **Task 3:** Properly resolve the feature flag and dependency issues for `whisper-cpp` in isolation, ensuring all build configurations (default, CPU, GPU) work as expected.
    6.  Only when this new feature branch is fully functional, stable, and has a passing CI suite should a new pull request be opened.
