## Summary

- What does this PR change and why?
- Link to related issues/ADRs/notes.

## Highlights

- [ ] Audio/VAD/STT pipeline stability improvements
- [ ] Real WAV E2E test with Vosk + Silero VAD
- [ ] Text injection ordering on Wayland/X11 with fast fallbacks
- [ ] Capture lifecycle and watchdog fixes
- [ ] Docs/telemetry updates

## Detailed Changes

- Per-crate summary of changes (app, audio, vad, stt, text-injection, telemetry)
- Any config/feature flag additions or behavior changes

## Risk/Impact

- Breaking changes? Migrations or config updates needed?
- User-facing behavior changes (defaults, ordering, timeouts)

## Test Plan

- How was this validated? Include commands and expected outputs.
- Key tests/examples to run locally (with feature flags as needed)

## Checklist

- [ ] Build passes: cargo check/build across workspace
- [ ] Tests pass locally (unit/integration/examples as relevant)
- [ ] Docs/CHANGELOG updated
- [ ] Logs are actionable at debug level; no noisy spam
- [ ] Feature flags documented (vosk, silero, text-injection, gui)

## Screenshots/Logs (optional)

Paste relevant excerpts to aid reviewers.
