---
doc_type: history
subsystem: stt
status: archived
freshness: historical
preservation: permanent
last_reviewed: 2026-02-12
owners: Coldaine
version: 1.0.0
---

# TranscriptionConfig.enabled Bug Discovery (2025-11-06)

## Context

Branch: `claude/compare-pr-204-205-011CUpVR7VEoCMEGGtChozsX`
Purpose: Implement golden test functionality

## Problem Discovered

Golden master test `test_short_phrase_pipeline` was hanging until 60s timeout, then failing. Investigation revealed a critical configuration bug.

### Root Cause

**`TranscriptionConfig::default()` sets `enabled: false`**

```rust
// crates/coldvox-stt/src/types.rs:L80-L101
impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,  // ← Problem
            // ... other fields
        }
    }
}
```

The golden master test created a custom config but never set `enabled: true`:

```rust
// crates/app/tests/golden_master.rs:L176-L193
let transcription_config = TranscriptionConfig {
    model_path: model_path.to_string(),
    // enabled field inherited from Default, = false
    ..Default::default()
};
```

**Result**: STT processor short-circuited and never published transcription events, causing test to timeout waiting for events that would never arrive.

## The Deeper Anti-Pattern

### Configuration Architecture Conflict

The runtime **unconditionally forces** `enabled: true`:

```rust
// runtime.rs:534-539
let stt_config = opts.transcription_config.clone().unwrap_or_else(|| TranscriptionConfig {
    enabled: true,  // ← Runtime forces this!
    streaming: true,
    ..Default::default()
});
```

**This creates a contradiction**:
- `TranscriptionConfig::default()` says disabled
- Runtime always sets enabled
- Default value is meaningless except in tests

### Why This Field Exists (But Shouldn't)

Investigation found this is **vestigial code** from earlier architecture:

1. **Copy-paste from PersistenceConfig**: `PersistenceConfig` legitimately has an `enabled` flag (toggle saving transcripts to disk). Someone copied the pattern without thinking.

2. **Legacy comment**: `"This streaming flag is now legacy. Behavior is controlled by Settings."` - indicates refactoring left dead fields.

3. **No legitimate use case**: Whether STT runs is controlled by `AppRuntimeOptions.stt_selection` (is it `Some` or `None`), not a field inside the config.

### What Should Control STT Enabling

Single responsibility: `AppRuntimeOptions.stt_selection`
- `Some(...)` = STT enabled
- `None` = STT disabled

Configuration in config files: `plugins.json` controls plugin selection, not scattered fields.

## Immediate Fix Applied

**High severity**: Set `enabled: true` in golden_master.rs

```rust
let transcription_config = TranscriptionConfig {
    enabled: true,  // ← Fixed
    model_path: model_path.to_string(),
    ..Default::default()
};
```

**Medium severity**: Restore test skip in justfile when Whisper model missing

```bash
# justfile else branch
cargo test --workspace --locked --skip test_end_to_end_wav_pipeline --skip test_short_phrase_pipeline
```

## Recommended Cleanup (Not Yet Done)

Remove the vestigial `enabled` field entirely:

**Files to update**:
- `crates/coldvox-stt/src/types.rs` - remove field from struct
- `crates/coldvox-stt/src/processor.rs` - remove check on lines 73, 96-98
- `crates/app/src/runtime.rs` - remove forced override on line 537
- `crates/app/tests/golden_master.rs` - remove explicit `enabled: true`

**Also consider removing**:
- `TranscriptionConfig.streaming` field (marked "now legacy")
- Other boolean toggles that should be in top-level config, not buried in feature configs

## Lessons Learned

1. **Defaults that are always overridden are code smells** - If runtime always sets a field, why have a default?

2. **Legacy comments are warnings** - "This flag is now legacy" means "I should have deleted this"

3. **Configuration should have one source of truth** - Mixing feature selection (`stt_selection`) with feature configuration (`enabled` flag) creates confusion

4. **Tests reveal production contradictions** - The golden test failed because it used the "real" default value that production code never uses

## Impact

Without this fix:
- Golden master tests would fail 100% of the time
- STT would never activate when using default TranscriptionConfig
- Confusing for users: "I enabled STT in plugins but nothing happens"

With fix:
- Tests pass (after addressing missing model skip logic)
- Still leaves architectural debt for future cleanup

## Related Anti-Patterns Found

During investigation, found similar patterns elsewhere:
- Empty feature flags: `whisper = []`, `coqui = []`, `leopard = []`, `no-stt = []`
- Stub backends with no implementation
- Documentation referencing non-functional features
- `compat.rs` module (547 lines, introduced commit `ee16f06` 2025-10-09) — migration/compatibility layers never used by any code, only by their own unit tests

All point to same underlying issue: **code/config that persisted after refactoring but lost its purpose**.

## References

- Branch: `claude/compare-pr-204-205-011CUpVR7VEoCMEGGtChozsX`
- Discussion: 2025-11-06 Claude session on branch status
