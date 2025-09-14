# Vosk STT Improvements Work Tracker

This tracker covers fixes from the Vosk STT code review critique (September 14, 2025). Focus on confirmed valid issues. Track progress here; link to [issues log](../issues/vosk-stt-issues.md).

## Priority: High (Address Before Production)
- [ ] **RUST-001: Model Path Cloning**  
  Fix unnecessary `to_string()` in init. Est. time: 15 min. Assignee: TBD.  
  Links: [Issue](../issues/vosk-stt-issues.md#rust-001)

- [ ] **RUST-004: Hot Path Allocations**  
  Refactor `WordInfo` for `Cow<str>`. Profile perf. Est. time: 1-2 hrs. Assignee: TBD.  
  Links: [Issue](../issues/vosk-stt-issues.md#rust-004)  
  Dependencies: Update parent `coldvox-stt` crate.

- [ ] **RUST-009: Typed Errors**  
  Introduce `thiserror` for `VoskError`. Est. time: 45 min. Assignee: TBD.  
  Links: [Issue](../issues/vosk-stt-issues.md#rust-009)

## Priority: Medium
- [ ] **RUST-005: Config Clone**  
  Borrow `model_path` in `update_config`. Est. time: 10 min. Assignee: TBD.  
  Links: [Issue](../issues/vosk-stt-issues.md#rust-005)

## Priority: Low (Style/Polish)
- [ ] **RUST-006: Error Formatting**  
  Standardize messages. Est. time: 10 min. Assignee: TBD.  
  Links: [Issue](../issues/vosk-stt-issues.md#rust-006)

- [ ] **RUST-007: Pattern Matching**  
  Update `ref mut` patterns. Est. time: 5 min. Assignee: TBD.  
  Links: [Issue](../issues/vosk-stt-issues.md#rust-007)

## Milestones
- **v0.1 Fixes**: Complete High priority. Run `cargo clippy --fix` where possible.
- **Testing**: After fixes, run unit tests in `vosk_transcriber.rs` and integration via examples/vosk_test.rs.
- **Validation Commands**:
  ```
  cargo clippy --all-targets --all-features
  cargo test --package coldvox-stt-vosk
  cargo bench (add if needed)
  ```

## Notes
- Ignored invalid critique points (e.g., RUST-002 docs already present, RUST-003 no error to propagate, RUST-008 docs exist).
- Total est. time: ~3 hrs.
- Update status with dates on completion.
- Individual issue files consolidated into single log for simplicity.
