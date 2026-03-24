# ColdVox Health Recovery: Execution Plan

Provide this exact document to the execution agent. Its goal is to resolve the remaining blockers on the `test-pr-365` branch to produce a stable, verified project foundation.

We are already on the `test-pr-365` branch, which bounds Python to `<3.13` and removes the local pip mess in favor of `uv`. However, the branch does not compile due to a breaking change in the `rubato` audio library.

## Step 1: Fix the `rubato` 1.0 Breaking Change
The `test-pr-365` branch fails to compile with:
```
error[E0432]: unresolved import `rubato::SincFixedIn`
  --> crates\app\src\audio\vad_adapter.rs:85:16
```
This is because `rubato` bumped from `0.16` to `1.0.1`. In version `1.0.1`, `rubato` requires input and output data to be wrapped in the `audioadapter` crate's `Adapter`/`AdapterMut` objects (specifically `SequentialSliceOfVecs` or similar structures) rather than passing raw `Vec` arrays like `0.16` did, which breaks the current signature.

**Agent Objective:**
1. Read the `rubato` 1.0.1 and `audioadapter` documentation to understand the migration path.
2. Edit `crates/app/src/audio/vad_adapter.rs` to fix the API usage by wrapping our buffers correctly.
3. Verify with `cargo check -p coldvox-app`. The pipeline **must** compile cleanly.

## Step 2: Port the STT Garbage Collection Fix (PR #366)
The STT plugins are incredibly fragile right now due to early garbage collection (tracked in PR #366).
Since we are fixing the foundation on this branch, we should manually pull the changes from PR #366 or cherry-pick them so that the plugin stops unloading during use.

**Agent Objective:**
1. Fetch PR #366 (`git fetch origin pull/366/head:pr-366`).
2. Examine the changes it makes to the STT plugin manager.
3. Apply those exact changes to our current workspace (`crates/coldvox-text-injection/src/manager.rs` or related STT files).

## Step 3: Prune the Remaining "Vaporware" Stubs
PR #365 removed the `whisper` feature flag, but left several others that do not actually exist in the codebase.

**Agent Objective:**
1. Open `crates/coldvox-stt/Cargo.toml`.
2. Remove the defined feature flags for `coqui`, `leopard`, and `silero-stt`. They have no implementation logic.
3. Ensure the project still compiles (`cargo check --workspace`).

## Step 4: Rewrite the "Source of Truth" Documentation
The `docs/plans/critical-action-plan.md` claims that `parakeet` does not compile, which is factually incorrect and wastes developer time.

**Agent Objective:**
1. Edit `docs/plans/critical-action-plan.md`.
2. Update the document to state that `parakeet` *does* compile via `cargo check -p coldvox-app --features parakeet`.
3. Shift the documented focus for `parakeet` away from "compilation errors" and toward "runtime validation and STT accuracy testing".

## Final Verification
Run `./scripts/local_ci.sh` (or `cargo clippy --workspace --all-targets --locked` and `cargo test --workspace --locked`). Confirm the entire project passes before committing these changes to the `test-pr-365` branch.
