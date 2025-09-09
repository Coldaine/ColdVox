ColdVox Text Injection — Testing & Backend Tasks Plan

Overview
- Goal: Resolve current testing and backend inconsistencies, increase determinism, and improve coverage around the session/processor pipeline and backends.
- Outcomes: Stable unit tests (headless), clear feature gating, mockable external calls, basic CI matrix, and docs explaining how to run tests.

Phase 0 — Quick Fixes (low risk)
- Fix borrow-after-move in `StrategyManager::inject`.
  - Change `for method in method_order` to iterate by reference (`for &method in method_order.iter()`), and precompute `let total_methods = method_order.len();` for logging.
  - Acceptance: `cargo test -p coldvox-text-injection --all-features -- --list` compiles; no E0382 in `manager.rs`.

- Correct combo injector naming and feature-gating.
  - Current: file `combo_clip_atspi.rs` implements `ComboClipboardYdotool` and uses `ydotool`, but the module is gated on `wl_clipboard` + `atspi` and imported as `combo_clip_atspi`.
  - Plan: Rename file to `combo_clip_ydotool.rs` (done) and update `lib.rs`/`manager.rs` imports. Gate with `#[cfg(all(feature = "wl_clipboard", feature = "ydotool"))]`. Keep the AT-SPI paste attempt inside the module behind `#[cfg(feature = "atspi")]`.
  - Acceptance: Builds with/without features; `rg` shows no incorrect `atspi` gating in combo; manager imports updated.

- Remove or port stale tests and wire them in.
  - Files: `src/tests/test_noop.rs`, `src/tests/test_caching_and_chunking.rs` were removed; replaced with `test_allow_block.rs` and `test_async_processor.rs` using the async API and StrategyManager entry points.
  - Acceptance: `cargo test -p coldvox-text-injection` passes; no unused test files left unreferenced.

Phase 1 — Core Unit Tests (logic-first)
- Allow/block list with and without `regex` feature.
  - Add tests that validate `is_app_allowed` uses compiled regex when `regex` enabled and substring fallback otherwise.
  - Acceptance: Two tests pass under `--features regex` and under `--no-default-features`.

- Chunking and pacing coverage.
  - Expose `chunk_and_paste` and `pace_type_text` via `pub(crate)` + `#[cfg(test)]` helpers or test through a dummy injector that records calls.
  - Verify char-boundary chunking on multi-byte text; set delays to 0 in config to keep tests fast; assert metrics increments for `paste_uses`/`keystroke_uses` and `flush_size_chars`.
  - Acceptance: Deterministic tests, no sleeps > a few ms.

Phase 2 — Async Pipeline Tests
- AsyncInjectionProcessor event loop coverage (headless, deterministic).
  - Use `tokio-test` to drive deterministic async tests; wrap ops with explicit `timeout`s.
  - Start `AsyncInjectionProcessor` with channels and default config so it falls back to `NoOp` injector.
  - Send a Final transcription, allow one tick (bounded sleep or `interval.tick().await`), and assert state returns to Idle and success counter increments (or at least, no panics).
  - Add shutdown signal test to ensure graceful exit path; verify no deadlocks.
  - Race tests: concurrently send Final + shutdown to probe ordering; assert consistent final state.
  - Acceptance: Reliable, <1s per test, no external backends/GUI.

Phase 3 — Backend Command Mocking (compile-time swap)
- Prefer compile-time mocking over runtime DI to keep release builds lean and type-safe.
  - Option A (recommended): `mockall_double` with `#[double] use real_module::CommandApi;` and a `mock_module::CommandApi` in tests.
  - Option B (alias pattern):
    - `pub mod command_api { #[cfg(not(test))] pub use real_impl::CommandApi as CommandApi; #[cfg(test)] pub use mock_impl::CommandApi as CommandApi; }`
  - Refactor `ydotool_injector`, `kdotool_injector`, `window_manager`, and `combo_clip_ydotool` to call `command_api::CommandApi` directly (no `Box<dyn ...>` fields).
  - Tests: simulate success, non-zero exit, and timeout; include partial failure in combo path (clipboard OK, paste fails) and assert error propagation.
  - Acceptance: Backend tests pass headlessly; no reliance on real binaries or sockets; zero runtime DI cost.

- Clipboard injector tests cleanup.
  - Keep current basic tests; add a test that verifies `restore_clipboard` path is a no-op when `restore_clipboard=false`.
  - Optionally route wl-clipboard calls via the same runner pattern for consistency (stretch goal).
  - Acceptance: No flaky sleeps; tests finish < 1s.

Phase 4 — Metrics Assertions
- Method metrics and histograms.
  - Add tests that induce both success and failure in `StrategyManager::inject` (use tiny `max_total_latency_ms` to force `BudgetExhausted`) and assert counters in `InjectionMetrics` (attempts, successes, failures, latency histogram growth, flush sizes).
  - Acceptance: Clear assertions on metrics fields; tests do not rely on specific OS state.

Additional Coverage (new)
- Property-based testing for Unicode/chunking edge cases (add `proptest` as dev-dep).
  - Generate multi-byte, very long, and boundary-straddling inputs; assert chunking respects char boundaries and metrics invariants.
- Resource cleanup/Isolation.
  - Ensure mocks reset between tests; avoid shared global state; verify no leaked tasks.
- Timing-sensitive scenarios.
  - Add targeted small-delay tests to surface race conditions in session transitions beyond zero-delay cases.

Phase 5 — CI + Docs
- CI matrix (GitHub Actions):
  - Jobs: (1) default features, (2) `--no-default-features`, (3) `--features regex`.
  - Cache `~/.cargo` and `target`; run `cargo test -p coldvox-text-injection --locked`.
  - Acceptance: All jobs green in main branch PRs.

- Documentation:
  - Add `docs/testing.md` covering: how to run tests in each feature mode, how compile-time mocking works (`mockall_double` or alias pattern), how to run optional integration tests (skipped by default), and environment variables to flip Wayland/X11 detection.
  - Acceptance: `README.md` links to `docs/testing.md`.

Phase 6 — Optional/Stretch
- AT-SPI adapter and minimal tests (when `atspi` feature enabled):
  - Introduce thin adapter trait over AT-SPI proxies to allow mocking. Add a smoke test for `AtspiInjector::is_available` that tolerates no bus.
  - Acceptance: Tests pass both with and without a11y bus present; adapter adds no runtime cost in release.

Implementation Notes
- Keep production code paths unchanged; prefer compile-time mocking over runtime DI.
- Avoid long sleeps; use `tokio-test` and bounded `timeout`s; set delays near-zero via config in tests.
- Maintain minimal, focused changes aligned with existing style.

Test Strategy Categories (guidance)
- Pure unit: logic with no externals (chunking, pacing, allow/block evaluation).
- Component: single backends with mocked command layer (success/non-zero-exit/timeout branches).
- Integration: minimal end-to-end with NoOp fallback or controlled env; verify session → inject → idle cycle and metrics update.

Tracking & Validation
- Run locally:
  - `cargo test -p coldvox-text-injection`
  - `cargo test -p coldvox-text-injection --no-default-features`
  - `cargo test -p coldvox-text-injection --features regex`
- For feature combos with backends, prefer running in CI without GUI by relying on NoOp injector fallback and mock runners.

Acceptance Criteria (high level)
- All tests pass in the three CI modes above.
- No compile errors under `-- --list` or `cargo check --tests`.
- No lingering stale test files or misnamed modules/gates.
- New backend tests run without system binaries present.
