# Live Test Dashboard – Plan and Probe Migration

## Goals

- Replace ad-hoc probe binaries with a unified, ergonomic test surface for manual checks and regression prevention.
- Keep fast, reliable headless runs for CI and scripted checks.
- Reduce default build weight/clutter by excluding probes from normal builds while preserving utility on demand.

## Scope

- Consolidate existing bins (`foundation_probe`, `mic_probe`, `record_10s`, `vad_demo`) into a shared `probes` module.
- Provide two frontends interfaces:
  - TUI dashboard (default): interactive manual testing.
  - Headless CLI modes: single-test runners for CI and scripts.
- Optional GUI dashboard behind a feature flag (deferred).

## Architecture

```
                ┌───────────────────────────────────────────────────┐
                │                    coldvox-app                    │
                ├──────────────────────────┬────────────────────────┤
                │        Library           │     Dashboards         │
                │  (audio, vad, stt, …)   │                        │
                ├──────────────────────────┼────────────────────────┤
                │        probes/           │  tui_dashboard (bin)   │
                │  - LiveTest trait        │  - ratatui+crossterm   │
                │  - MicCaptureCheck       │  - Panels + actions    │
                │  - VadFromMicCheck       │  - Results view        │
                │  - RecordToWav           │  - Log pane            │
                │  - FoundationHealth      │                        │
                ├──────────────────────────┼────────────────────────┤
                │        headless cli      │  gui_dashboard (bin?)  │
                │  - Subcommands per test  │  - egui/eframe (opt)     │
                │  - JSON results          │  - Feature-gated       │
                └──────────────────────────┴────────────────────────┘
```

### Core contracts

- `LiveTest` trait
  - `name()` -> `&'static str`
  - `run(ctx: &mut TestContext) -> Result<LiveTestResult, TestError>`
    - Distinguishes "could not run" (infra/config error) vs. "ran and failed" (assertion thresholds not met).

- `LiveTestResult`
  - metrics: map of string -> number/string/bool
  - pass/fail: bool
  - notes: string (optional)
  - artifacts: file paths (e.g., WAV) (optional)

- `TestContext` (expanded)
  - device selection (name/index)
  - timeouts (per test)
  - thresholds (loaded, per-test)
  - output dir + retention policy
  - feature flags (e.g., `use_vosk`)
  - audio config: sample_rate_hz, channels, chunker_frame_size (default 512), ring buffer sizes, VAD thresholds (e.g., Silero threshold)

### Reuse from library

- Mic capture: `audio::AudioCapture`
- VAD from mic: `audio::chunker::AudioChunker` (512@16k) + `audio::VadProcessor` via `vad::VadAdapter` (do not hardcode Silero; keep engine-switchable)
- Health/foundation: `foundation::{StateManager, ShutdownHandler, HealthMonitor}`

Note on frame sizes:
- Capture callbacks are variable-sized, but chunker outputs standardized 512-sample frames.
- Both Silero and Level3 VAD now use 512-sample frames at 16 kHz (~32 ms) consistently.

## Tests Implemented (Initial set)

- `MicCaptureCheck`
  - Starts capture for N seconds.
  - Metrics: frames_captured/sec, drop_rate, silent_frames, last_frame_age, watchdog_triggered.
  - Thresholds: drop_rate <= X%, frames/sec in [expected±delta], watchdog_triggered == false.

- `VadFromMicCheck`
  - Starts capture + chunker (512@16k) + VAD (Silero).
  - Metrics: event_counts (start/end), avg_probability (if exposed), latency estimates.
  - Thresholds: event_counts > 0 when speaking into mic, acceptable idle false-positives.

- `RecordToWav`
  - Writes 10s of mic to WAV; returns file path.
  - Metrics: file size, sample count.
  - Thresholds: sample_count == 10s*16k ± tolerance.

- `FoundationHealth`
  - Exercises state transitions, shutdown guard, health monitor stubs.
  - Metrics: transitions_ok, panic_hook_ok.

### Critical regression tests (Phase 1 priority)

- Watchdog timeout detection
- Ring buffer overflow scenarios
- Sample format negotiation failures (CPAL)
- Device disconnection/recovery cycles

## Frontends

### TUI Dashboard (Phase 1)

- Tech: `ratatui` + `crossterm` (portable, minimal deps).
- Panels:
  - Devices: list + selection (using `audio::DeviceManager`).
  - Live level meter: RMS bar from a small tap on `AudioCapture`.
  - Actions: keys/buttons to run tests above.
  - Results: last run metrics, pass/fail, colored status.
  - Logs: tail of recent `tracing` lines (optional simple channel/pipe).
- Shortcuts:
  - M: Mic capture check
  - V: VAD from mic
  - R: Record 10s
  - F: Foundation health
  - S: Save results to JSON
  - Q: Quit

### Headless CLI (Phase 1)

- `cargo run --bin tui_dashboard -- --ci mic-capture --duration 10 --thresholds thresholds.toml --json out.json`
- Behavior: no UI; run test once, write JSON, exit code = pass?0:1.
- Subcommands: `mic-capture`, `vad-mic`, `record-wav`, `foundation`.

### GUI Dashboard (Phase 2 – optional)

- Tech: `egui/eframe` behind feature `dashboard-gui`.
- Same actions, with waveform/level visualization.
- Deferred until TUI stabilizes.

## Thresholds, Results, and Artifacts

### Thresholds.toml (example with severities and ranges)

```
[mic_CAPTURE]
max_drop_rate.error = 0.10
max_drop_rate.warn = 0.05
frames_per_sec.min = 45
frames_per_sec.max = 55
watchdog_must_be_false = true

[vad_mic]
min_event_count.error = 1
max_false_positives.warn = 0
expected_engine = "silero|level3"  # adapter-validated
```

### JSON result (example)

```
{
  "test": "mic_capture",
  "pass": true,
  "metrics": {
    "frames_captured": 832,
    "drop_rate": 0.01,
    "frames_per_sec": 52.0,
    "watchdog_triggered": false
  },
  "notes": "OK",
  "artifacts": []
}
```

### Artifacts policy

- Default retention: keep last 20 runs under `.coldox/test_runs/`; prune older.
- Artifacts per test (e.g., WAVs) capped (e.g., 2 per run) and compressed if large.

### Performance metrics

- Collect CPU% and RSS during each test (sampled every 250 ms) using `/proc` on Linux.
- Latency probes: measure capture-to-VAD event dispatch time for VAD test.
- Report as metrics: `cpu_avg`, `cpu_p95`, `rss_max`, `latency_avg_ms`, `latency_p95_ms`.

### Regression detection

- Maintain rolling history (last N=50) of key metrics per test.
- Flag regressions using:
  - Deviation beyond thresholds, and
  - Trend checks (e.g., EWMA slope) indicating gradual degradation.
- Store small `history.json` alongside results to enable local trend checks.

## Migration plan

1) Create `src/probes/` module
2) Add TUI dashboard bin
3) Wire thresholds + results
4) Deprecate old bins in build
5) CI integration (optional first cut)
6) Documentation

### Work items (granular)

- Probes extraction
  - [x] Create `src/probes/mod.rs` and files; move logic from bins.
  - [x] Design `LiveTest` trait and `LiveTestResult`.
  - [x] Implement `MicCaptureCheck` using `AudioCapture` metrics.
  - [x] Implement `VadFromMicCheck` using `AudioChunker` + `VadProcessor` (512@16k).
  - [x] Implement `RecordToWav` writing via `hound`.
  - [x] Implement `FoundationHealth` using `foundation`.

- TUI dashboard
  - [x] Add new bin `tui_dashboard` with ratatui UI skeleton.
  - [x] Device list and selection UI.
  - [x] Action bindings for each test.
  - [x] Results display (table + pass/fail color).
  - [x] Headless `--ci` subcommands with JSON output and exit codes.

- Thresholds & persistence
  - [x] Define `thresholds.toml` schema.
  - [x] Implement evaluation and per-test criteria.
  - [x] Persist last N results; compare for regressions.

- Migration & cleanup
  - [x] Move `src/bin/{mice_prob,record_10s,vad_demo,foundation_prob}.rs` to `examples/` (or gate with feature).
  - [x] Remove `[[bin]]` entries from `Cargo.toml` for these.
  - [x] README/docs update.

- GUI (optional)
  - [ ] Feature `dashboard-gui` with egui/eframe.
  - [ ] Simple indicator + charts.

## Risks & mitigations

- Wayland/GUI friction: start with TUI to avoid compositor issues.
- Audio device access in CI: use file-based tests or container privileges only where needed; keep CI optional.
- Flaky hardware metrics: use tolerances and reasonable thresholds; persist history to spot trends.

## Timeline (suggested)

- Week 1: probes extraction + TUI skeleton + mic capture test + migrate bins to examples.
- Week2: VAD from mic test, thresholds/JSON, headless CLI, docs.
- Week3: Record-to-WAV + foundation health + CI hook + polish.
- Week4: Optional GUI; refine thresholds and add more tests as needed.

## Commands (examples)

- Manual dashboard: `cargo run -p coldvox-app --bin tui_dashboard`
- Headless mic capture: `cargo run -p coldvox-app --bin tui_dashboard -- --ci mic-capture --duration 10 --thresholdhs thresholds.toml --json out.json`
- Examples (if moved): `cargo run -p coldvox-app --example mic_probe -- --duration 10`

---
Decision point: examples vs. feature-gated bins. Recommendation: move to `examples/` to keep usage easy and remove default build cost. Feature-gate the optional GUI.