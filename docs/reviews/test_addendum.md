# Test Evaluation Addendum: Testing Invariants vs. Testing State

This document re-evaluates the testing strategy for ColdVox. It discards superficial distinctions between "Unit" and "Integration" tests and instead focuses on the fundamental computer science metric: **Does the test assert a system invariant, or does it merely test state and wiring?**

## The Core Metric: Invariants vs. Wiring

A test's value is derived entirely from its ability to prevent a real-world system failure. The scale (unit vs. integration) is irrelevant if the test is conceptually hollow.

### The F-Grade: Tautologies and Wiring Tests
Tests that provide zero value fall into two categories, regardless of whether they are micro-unit tests or macro-integration tests.

1. **Testing State (The Unit Tautology):**
   * *Example:* `test_transcription_config_default` asserting `buffer_size == 512`.
   * *The Flaw:* This tests memory assignment and developer recall. It asserts that a hardcoded constant is equal to itself. If the constant is changed to improve the system, the test fails. It is a "Change Detector" that provides negative value.

2. **Testing Wiring (The Integration Tautology):**
   * *Example:* A pipeline test that feeds a perfectly clean, pre-recorded WAV file into the STT engine and asserts it gets a `200 OK`.
   * *The Flaw:* This does not test the application's logic. It tests that the standard library's networking functions work and that the external STT service is online. Because there is no noise or edge-case introduced, it tests the "happy path" wiring, which is already validated by the type system.

### The A-Grade: Asserting Invariants
A valuable test asserts a mathematical, architectural, or physical invariant that must hold true regardless of how the internal code is refactored.

1. **Mathematical/Physical Boundaries (Micro Invariant):**
   * *Example:* `test_rms_silence` in `coldvox-audio-quality`.
   * *Why it's A-Grade:* Calculating the log of pure silence (`0.0`) yields negative infinity. If this boundary isn't clamped, the application panics. This test asserts a mathematical invariant against a physical hardware state (a hardware mute). It is impossible to reliably trigger this absolute boundary in a standard integration test using analog audio files.

2. **Algorithmic Integrity (Micro Invariant):**
   * *Example:* Audio sample conversions (`f32_to_i16_basic`).
   * *Why it's A-Grade:* This asserts bitwise integrity. A "happy path" integration test will pass even if there is an off-by-one bitshift error, because downstream STT AI models will smooth over the resulting harmonic distortion. This test asserts the algorithm itself is flawless, preventing silent degradation.

3. **Architectural Liveness (Macro Invariant):**
   * *Example:* `test_noop_inject_success` (The fallback text injector).
   * *Why it's A-Grade:* This tests the system's "last resort" safety valve. It asserts the invariant: *If all primary systems fail, the application must not panic.* Testing this deterministically in a unit test guarantees the architectural safety-valve works without requiring a convoluted, destructive integration test environment.

## Conclusion
We must stop evaluating tests based on their scope (Unit vs. E2E). We evaluate them by their rigor. 
* If a test only verifies that data can be passed from Point A to Point B without crashing, it is **F-Grade**.
* If a test forces the system against a hard boundary (mathematical limits, structural safety-valves, invalid configurations) and proves it survives, it is **A-Grade**.
