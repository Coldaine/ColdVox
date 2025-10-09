# ColdVox Master Implementation Plan

**Status:** Active  
**Version:** 1.0  
**Created:** 2025-10-08  
**Last Updated:** 2025-10-08  
**Owner:** ColdVox Team

---

## Executive Summary

This master plan synthesizes strategic planning documents for the ColdVox voice-to-text injection system. It consolidates architectural decisions, implementation strategies, testing frameworks, and execution roadmaps across multiple domains:

1. **Text Injection System** - Multi-platform input methods with graceful fallbacks
2. **Testing Architecture** - Comprehensive test strategy from unit to E2E
3. **Hardware Validation** - Non-blocking hardware test framework
4. **Implementation Roadmap** - Prioritized execution plan with clear milestones

This document serves as the single source of truth for coordinating development efforts across the text injection vNext initiative.

---

## Table of Contents

- [1. Strategic Context](#1-strategic-context)
- [2. Text Injection Architecture](#2-text-injection-architecture)
- [3. Testing Strategy](#3-testing-strategy)
- [4. Hardware Test Framework](#4-hardware-test-framework)
- [5. Implementation Roadmap](#5-implementation-roadmap)
- [6. Success Criteria](#6-success-criteria)
- [7. Risk Management](#7-risk-management)
- [8. References](#8-references)

---

## 1. Strategic Context

### 1.1 Current State

ColdVox is a voice-to-text injection system with:
- Multi-crate Rust workspace architecture
- Real-time audio capture and processing (16 kHz, 512-sample windows)
- Silero V5 ONNX-based VAD for speech detection
- Optional Vosk STT for transcription
- Cross-platform text injection capabilities

### 1.2 Core Challenge

The text injection subsystem must reliably inject transcribed text into target applications across multiple desktop environments (KDE Plasma, Hyprland, Windows) with sub-200ms latency while handling edge cases gracefully.

### 1.3 Design Principles

1. **Pre-warm on buffer entry** - Don't wait for injection time
2. **Fast-fail stages** - Each stage ≤50ms, total ≤200ms
3. **Event-based success detection** - Confirm via platform events, not sleeps
4. **Strict clipboard hygiene** - Always restore, never leak to history
5. **Clear error messages** - Actionable guidance for every failure mode
6. **Observable behavior** - Deep telemetry without compromising privacy

---

## 2. Text Injection Architecture

### 2.1 Method Rankings by Environment

#### KDE Plasma (KWin, Wayland)
1. **AT-SPI Insert** (EditableText.insert) - Fastest/safest where exposed
2. **AT-SPI Paste** (EditableText.paste after clipboard seed) - Robust when caret exists
3. **Portal/EIS "type"** (authorized input via xdg-desktop-portal + libei) - Needs consent
4. **KDE fake-input** helper (privileged; feature-flagged) - Only if explicitly allowed

#### Hyprland (wlroots)
1. **AT-SPI Insert** - Primary method
2. **AT-SPI Paste** - Secondary with clipboard
3. **wlr Virtual Keyboard** (wtype-style synthesis) - Standard wlroots path
4. **Portal/EIS "type"** (if portal supports it) - Fallback

#### Windows
1. **UI Automation** (ValuePattern/TextPattern) - Native UIA
2. **Clipboard + SendInput Ctrl+V** (with restore) - Standard fallback
3. **SendInput typing** - Last resort

### 2.2 Pre-Warm Strategy

**Trigger:** When session enters Buffering state  
**Goal:** By ReadyToInject time, know what will work

**Pre-warm actions (async, non-blocking):**
- AT-SPI bus ping and focus snapshot (30ms budget)
- Clipboard backup if allowed (15ms)
- Portal session ensure_ready if enabled (40ms)
- Compositor-specific setup (virtual keyboard connect, etc.)
- App compatibility hint lookup

**TTL:** Keep pre-warm results "hot" for ~3s since last buffer event

### 2.3 Injection Micro-Pipelines

Each method implements:
- Timeout-bounded execution (≤50ms per stage)
- Event-based confirmation (AT-SPI text-changed, caret movement)
- Graceful failure with structured diagnostics
- Telemetry capture (timing, method, outcome)

**Confirmation Strategy:**
- Preferred: AT-SPI `object:text-changed:inserted` events
- Confirm prefix only (first 3-6 chars) to avoid IME mismatches
- Timeout window: ≤75ms
- If no confirmation: immediate fail, try next method

### 2.4 Implementation Details

#### Virtual Keyboard (Wayland wlroots)
- Proper keymap upload via anonymous file/memfd
- Keysym → keycode resolution with caching
- Shift state management for uppercase/symbols
- Unicode character support via 0x01000000 fallback
- Chunked sending (10 chars) to avoid overwhelming compositor

**Key Files:**
- Implementation: `crates/coldvox-text-injection/src/backends/virtual_keyboard.rs` (to be created)
- Dependencies: `wayland-client`, `wayland-protocols-misc`, `xkbcommon`

#### Portal/EIS (xdg-desktop-portal)
- D-Bus session creation and device selection
- Restore token persistence for avoiding re-authorization
- EIS handshake and keyboard device discovery
- Proper timeout handling at each stage
- Pre-warming to hide authorization latency

**Key Files:**
- Implementation: `crates/coldvox-text-injection/src/backends/portal_eis.rs` (to be created)
- Dependencies: `zbus`, `reis`, `ashpd` (optional)

#### KWin Fake Input (KDE-specific)
- Direct D-Bus interface to `org.kde.kwin.FakeInput`
- Authentication handling with user-friendly errors
- Keycode cache for performance
- Graceful fallback for unmapped characters
- Feature flag protection (`kde-fake-input`)

**Key Files:**
- Implementation: `crates/coldvox-text-injection/src/backends/kwin_fake.rs` (to be created)
- Dependencies: `zbus`, `xkbcommon`

### 2.5 Logging & Telemetry

**Structured Log Levels:**
- `TRACE`: Raw events (AT-SPI event names, payload sizes—not content)
- `DEBUG`: Decisions, timings per stage, chosen method, confirm outcomes
- `INFO`: Success summary (chars, method, total_ms)
- `WARN/ERROR`: Explicit fix hints with actionable guidance

**Per-Attempt Fields:**
```json
{
  "ts": "ISO-8601",
  "env": "KDE|Hyprland|Windows",
  "utterance_id": "uuid",
  "app_id": "app_name",
  "title": "window_title",
  "role": "entry|text|document_*",
  "prewarm": {"a11y_ok": true, "portal": true, "vkbd": false},
  "method": "atspi_insert|atspi_paste|vkbd|portal|kde_fake|uia|clipboard_paste",
  "stage_ms": 37,
  "confirm": {"text_changed": true, "caret_moved": true},
  "clipboard": {"seeded": true, "restored": true, "manager_cleared": true},
  "result": "ok|timeout|error",
  "total_ms": 128
}
```

**Privacy:** No raw text in logs (redacted as `[TEXT:<length>]`)

---

## 3. Testing Strategy

### 3.1 Test Philosophy

Following the **Pragmatic Test Architect** approach:
1. Test complete user journeys, not individual methods
2. Use real dependencies (actual AT-SPI bus, compositor, applications)
3. Tell stories about user value (text appears where expected)
4. Eliminate mocks in favor of behavioral fakes that simulate real behavior
5. Verify observable outcomes over implementation details

### 3.2 Test Distribution

```yaml
injection_tests:
  service_integration: 70%  # Complete injection flows with real AT-SPI, clipboard
  trace_based: 15%          # Multi-app injection verification
  contract: 10%             # Portal/EIS protocol compliance
  pure_logic: 5%            # Keymap conversion, text chunking algorithms
```

### 3.3 Test Layers

#### Unit Tests
**Scope:** Individual functions (keymap, keycode resolution, D-Bus stubs)  
**Execution:** On every commit  
**Target:** <500ms per test

**Coverage:**
- Keymap creation and keysym → keycode resolution
- Shift logic validation for uppercase/symbols
- Chunking behavior and boundaries
- D-Bus stub responses
- Restore token save/load
- Authentication flows

#### Integration Tests
**Scope:** Protocol handshakes, event flows, component wiring  
**Execution:** Nightly + PR  
**Target:** <3s per test suite

**Coverage:**
- Mock compositor protocol validation (virtual keyboard)
- Mock portal session flows (create → select → start → connect)
- Mock KWin D-Bus interface
- Event sequence verification (key press → release)
- Timeout handling at each stage

#### End-to-End Tests
**Scope:** Real injection into target apps  
**Execution:** Weekly + release  
**Target:** <30s per test

**Test Matrix:**

| Method | KDE Plasma | Hyprland | Expected Success |
|--------|------------|----------|------------------|
| AT-SPI Insert | ✅ Kate, Firefox | ✅ Firefox, Alacritty | High |
| Virtual Keyboard | ❌ (no protocol) | ✅ All apps | Medium-High |
| Portal/EIS | ✅ (with consent) | ✅ (if portal supports) | Medium |
| KWin Fake Input | ✅ (if authorized) | ❌ | Medium |

**Test Cases:**
- TC1: AT-SPI + Virtual Keyboard fallback (Hyprland → Alacritty)
- TC2: Portal/EIS with consent (KDE, first run)
- TC3: KWin Fake Input (KDE, authorized)
- TC4: Electron App (VS Code with Wayland)
- TC5: Password field safety (skip injection, no leaks)

#### Failure Mode Tests
**Scope:** Graceful degradation, timeouts, error recovery  
**Execution:** Nightly  
**Target:** Full matrix in <10s

**Scenarios:**
- No AT-SPI bus → fallback to Portal/EIS or virtual keyboard
- No focused window → queue injection until focus detected
- Non-editable focus → try parent/child traversal, then clipboard
- Clipboard locked → skip clipboard methods, use direct input
- Portal denied → respect denial, try other methods
- Virtual keyboard unsupported → cache unavailable, don't retry
- Unicode conversion fail → use clipboard for unmappable chunks
- Rate limited → queue and batch inject

### 3.4 Behavioral Fakes

Instead of mocks, use **behavioral fakes** that simulate real platform behavior:

```python
class ATSPIBusFake:
    """Simulates real AT-SPI bus behavior including timing and events"""
    - Realistic latency (5ms)
    - Event propagation delays (1ms)
    - Complete object hierarchy (app → window → text widget)
    - State management (focus, text content, caret position)
    - Failure mode simulation
```

**Benefits:**
- Tests actual code paths, not mock interactions
- Catches timing bugs and race conditions
- Easy to extend with new scenarios
- Fast execution (no real IPC overhead)

---

## 4. Hardware Test Framework

### 4.1 Philosophy

Hardware tests must run **continuously** but cannot **block development**.

**Key Principles:**
- Fast feedback via pre-commit hooks (<3s)
- Hardware validation without blocking CI
- Complete E2E verification from audio to injection
- Flakiness management through automatic quarantine
- Performance tracking to catch regressions

### 4.2 Test Tiers

#### Tier 1: Pre-Commit (Fast, <3s)
**Target:** Deterministic logic validation  
**Environment:** In-memory fakes  
**Blocking:** Yes

**Included:**
- Injection fallback chain logic
- Concurrent injection ordering
- Unicode handling
- Timeout handling
- Error propagation

**Implementation:**
```bash
#!/bin/bash
# .git/hooks/pre-commit
pytest tests/injection/fast/ \
  -m "not hardware and not e2e" \
  --fail-fast \
  --timeout=0.5 \
  -q
```

#### Tier 2: Continuous Hardware (Non-Blocking)
**Target:** Real hardware validation  
**Environment:** Real audio devices, compositor  
**Blocking:** No (results logged for monitoring)

**Included:**
- Microphone array capture
- GPU/CUDA availability
- Audio device enumeration
- Compositor detection
- AT-SPI bus health

**Implementation:**
```python
async def run_non_blocking():
    """Run hardware tests in background, report to telemetry"""
    tasks = [asyncio.create_task(run_with_telemetry(test)) 
             for test in get_hardware_tests()]
    asyncio.gather(*tasks, return_exceptions=True)
    return {"status": "hardware_tests_started"}
```

#### Tier 3: Nightly E2E (Full Suite)
**Target:** Complete integration validation  
**Environment:** Real compositor, apps, audio  
**Blocking:** No (alerts on failure)

**Included:**
- Real KWin injection (Kate, Konsole, Firefox)
- Real Hyprland injection (Alacritty, terminal apps)
- Physical USB audio devices
- Multi-monitor focus handling
- Thermal throttling resilience

#### Tier 4: Release Gate (Must Pass)
**Target:** Production readiness validation  
**Environment:** Full hardware + all feature combinations  
**Blocking:** Yes (blocks release)

**Included:**
- Complete WAV → injection pipeline
- Hardware silence detection accuracy
- Bluetooth audio latency compliance
- All Tier 1-3 tests green
- Performance benchmarks met

### 4.3 WAV-to-Injection E2E Tests

**Complete User Journey Tests:**

```python
def test_wav_to_kate_injection(hardware_env):
    """Complete story: User speaks, text appears in Kate."""
    hardware_env.start_app("kate")
    hardware_env.ensure_focus("kate")
    
    wav_file = "test_data/audio/hello_world.wav"
    hardware_env.play_audio(wav_file)
    
    pipeline = CompletePipeline(
        audio_input=hardware_env.audio_device,
        transcriber=WhisperTranscriber(),
        injector=TextInjector()
    )
    pipeline.start()
    
    wait_for(
        lambda: "Hello, world!" in hardware_env.get_kate_content(),
        timeout=5.0
    )
    
    # Verify complete telemetry
    spans = hardware_env.get_trace()
    assert spans.has("audio.capture")
    assert spans.has("vad.speech_detected")
    assert spans.has("whisper.transcribe")
    assert spans.has("injection.attempt")
    assert spans.has("atspi.text_changed")
    
    total_ms = spans.total_duration_ms()
    assert total_ms < 500
```

**Test Audio Library:**
- `hello_world.wav` - "Hello, world!" (1500ms, neutral_us)
- `british_accent.wav` - "Good morning, how are you?" (2000ms, british_female)
- `speech_with_noise.wav` - "Testing one two three" (2500ms, SNR 10dB)
- `technical_terms.wav` - "Initialize Kubernetes deployment with PostgreSQL" (3000ms)

### 4.4 Flakiness Management

**Automatic Detection:**
```python
class FlakinessDetector:
    def __init__(self):
        self.failure_history = defaultdict(list)
        self.quarantine_threshold = 0.1  # 10% failure rate
    
    def record_run(self, test_name: str, passed: bool):
        self.failure_history[test_name].append(passed)
        if self.is_flaky(test_name):
            self.quarantine(test_name)
```

**Quarantine Actions:**
- Add pytest marker: `@pytest.mark.quarantine`
- Alert team via Slack/email
- Create tracking issue automatically
- Exclude from blocking CI runs
- Include in investigation dashboard

### 4.5 Performance Regression Detection

**Automatic Tracking:**
```python
@pytest.fixture(autouse=True)
def track_performance(request, benchmark):
    start = time.perf_counter()
    yield
    duration = time.perf_counter() - start
    
    test_name = request.node.nodeid
    baseline = load_baseline(test_name)
    
    if baseline and duration > baseline * 1.2:  # 20% regression
        warnings.warn(f"Performance regression: {test_name} took {duration:.2f}s")
```

**Performance Budgets:**
```python
BUDGETS = {
    "pre_warm": 50,                # ms
    "atspi_insert": 30,
    "clipboard_paste": 40,
    "virtual_keyboard_chunk": 50,
    "portal_eis_connect": 100,
    "total_injection": 200,
    "fallback_penalty": 50         # Additional per fallback
}
```

---

## 5. Implementation Roadmap

### 5.1 Phase 1: Foundation (Weeks 1-2)

**Objective:** Core infrastructure and AT-SPI refinement

**Deliverables:**
- [ ] Refine existing AT-SPI injector with pre-warm support
- [ ] Implement structured telemetry framework
- [ ] Create behavioral fake for AT-SPI bus
- [ ] Set up test data library (WAV files with known transcripts)
- [ ] Implement pre-commit hook with fast tests

**Success Criteria:**
- Pre-commit tests complete in <3s
- AT-SPI injector has event-based confirmation
- Telemetry captures all required fields
- Zero text leaks in logs

### 5.2 Phase 2: Virtual Keyboard (Weeks 3-4)

**Objective:** Wayland virtual keyboard implementation for Hyprland

**Deliverables:**
- [ ] Implement `VirtualKeyboard` struct with Wayland protocol
- [ ] Keymap creation and upload logic
- [ ] Keysym → keycode resolution with caching
- [ ] Shift state management
- [ ] Unicode fallback handling
- [ ] Integration tests with mock compositor
- [ ] E2E tests with real Hyprland session

**Success Criteria:**
- Successful injection into Alacritty (no AT-SPI)
- <50ms per 10-char chunk
- 95%+ success rate on ASCII text
- 80%+ success rate on Unicode text
- Graceful handling of unmapped characters

### 5.3 Phase 3: Portal/EIS (Weeks 5-6)

**Objective:** xdg-desktop-portal keyboard input implementation

**Deliverables:**
- [ ] D-Bus portal session management
- [ ] EIS handshake and device discovery
- [ ] Restore token persistence
- [ ] Pre-warm integration
- [ ] Consent flow handling
- [ ] Mock portal for integration tests
- [ ] E2E tests with real portal

**Success Criteria:**
- First-run consent flow completes in <500ms
- Subsequent injections use cached session
- Pre-warm reduces injection latency by 40%+
- Clear error messages for denied consent
- Restore token survives system reboot

### 5.4 Phase 4: KWin Fake Input (Weeks 7-8)

**Objective:** KDE-specific privileged input method

**Deliverables:**
- [ ] D-Bus interface to `org.kde.kwin.FakeInput`
- [ ] Authentication flow with error handling
- [ ] Keycode cache for performance
- [ ] Feature flag (`kde-fake-input`)
- [ ] Mock KWin interface for tests
- [ ] E2E tests on KDE Plasma
- [ ] User documentation for authorization

**Success Criteria:**
- Successful injection into Konsole (terminal)
- <40ms per character
- Clear instructions for enabling in System Settings
- Graceful degradation when not authorized
- No bypass of authentication

### 5.5 Phase 5: Integration & Testing (Weeks 9-10)

**Objective:** Full pipeline integration and comprehensive testing

**Deliverables:**
- [ ] Complete WAV → injection E2E tests
- [ ] Hardware test orchestration framework
- [ ] Flakiness detection and quarantine system
- [ ] Performance regression tracking
- [ ] Cross-platform validation (KDE, Hyprland, Windows)
- [ ] Documentation suite (architecture, user guide, troubleshooting)

**Success Criteria:**
- All E2E tests green on target platforms
- Hardware tests running non-blocking in CI
- Performance budgets met across all methods
- Zero blocking flaky tests
- Complete user documentation

### 5.6 Phase 6: Polish & Release (Weeks 11-12)

**Objective:** Production readiness and release preparation

**Deliverables:**
- [ ] Performance optimization pass
- [ ] Error message refinement
- [ ] Release gate test validation
- [ ] Migration guide for existing users
- [ ] Release notes and changelog
- [ ] Demo videos showing functionality

**Success Criteria:**
- All release gate tests pass
- Performance p95 <200ms end-to-end
- Zero critical bugs
- Documentation complete and reviewed
- User acceptance testing positive

---

## 6. Success Criteria

### 6.1 Functional Requirements

**Injection Success Rates:**
- AT-SPI apps (Kate, Firefox): ≥95% success
- Non-AT-SPI apps (Alacritty, Konsole): ≥80% success via fallbacks
- Electron apps (VS Code): ≥90% success
- Overall cross-platform: ≥85% success

**Performance:**
- Pre-warm: <50ms
- Per-method injection: <50ms per stage
- Total end-to-end: <200ms p95
- Fallback penalty: +50ms per method tried
- Audio → text → injection: <500ms p95

**Robustness:**
- Zero crashes on focus race or permission denial
- Graceful degradation through fallback chain
- Proper error messages for all failure modes
- Clipboard always restored (unless app crashes)
- No text leaked to clipboard history

### 6.2 Quality Gates

**Build & Test:**
- All unit tests pass in <10s
- All integration tests pass in <30s
- Pre-commit tests complete in <3s
- No flaky tests in blocking suite
- Hardware tests report to telemetry

**Code Quality:**
- No unsafe code without justification
- Comprehensive error handling (Result types)
- Privacy-compliant logging (no raw text)
- Performance budgets documented and enforced
- API contracts clearly defined

**Documentation:**
- Architecture diagrams up-to-date
- User guide for each platform
- Troubleshooting guide with common issues
- API documentation for all public interfaces
- Example code for each injection method

### 6.3 Release Readiness

**Must Pass:**
- All Tier 4 release gate tests green
- Performance benchmarks met
- Security review complete (no credential leaks)
- User acceptance testing positive
- Migration guide tested with real users

**Nice to Have:**
- Demo video showing all platforms
- Blog post explaining architecture
- Conference talk submission
- Community feedback incorporated

---

## 7. Risk Management

### 7.1 Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Wayland protocol changes break virtual keyboard | Medium | High | Version lock dependencies, monitor upstream |
| Portal API incompatibility across distros | Medium | Medium | Test on multiple distros, provide fallbacks |
| KWin fake input disabled by default | High | Low | Document clearly, use as optional enhancement |
| Unicode characters unmappable | Medium | Medium | Clipboard fallback for unmapped chars |
| AT-SPI bus unreliable on some systems | Medium | High | Multiple fallback methods, clear diagnostics |
| Performance regression from telemetry | Low | Medium | Async telemetry, sampling, budget enforcement |

### 7.2 Process Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Scope creep from new platform requests | Medium | Medium | Clear roadmap, defer non-core platforms |
| Test flakiness blocks development | Medium | High | Automatic quarantine, non-blocking hardware tests |
| Hardware test infrastructure unavailable | Low | High | CI/CD with real hardware runners (Nobara) |
| Documentation lags implementation | High | Medium | Doc updates in same PRs as code |

### 7.3 Mitigation Strategies

**Continuous:**
- Weekly architecture review meetings
- Performance benchmarking in CI
- User feedback collection via issues
- Upstream dependency monitoring

**Per-Phase:**
- Phase retrospectives with lessons learned
- Risk assessment updates
- Scope adjustment as needed
- User testing checkpoints

---

## 8. References

### 8.1 Source Documents

This master plan synthesizes the following planning documents:

1. **InjectionMaster.md** - Fast-fail injection design with pre-warm, sub-50ms stages, event-based confirmation
2. **InjectionTest1008.md** - Behavior-first testing architecture following Pragmatic Test Architect philosophy
3. **OpusCodeInject.md** - Complete implementations for Virtual Keyboard, Portal/EIS, and KWin Fake Input
4. **OpusTestInject2.md** - Hardware test matrix and pre-commit strategy with non-blocking execution
5. **QwenTestMerge.md** - vNext test strategy with unit/integration/E2E layers and complete WAV→injection validation

### 8.2 Related Documentation

**Architecture:**
- `docs/architecture.md` - TUI architecture and robustness plan
- `CLAUDE.md` - Workspace overview and key components
- `README.md` - Project overview and setup

**Implementation:**
- `crates/coldvox-text-injection/` - Text injection subsystem
- `crates/coldvox-audio/` - Audio capture and processing
- `crates/coldvox-vad/` - VAD core traits and configurations
- `crates/coldvox-stt/` - STT abstractions and plugins

**Configuration:**
- `.github/copilot-instructions.md` - AI workspace instructions
- `Cargo.toml` - Workspace dependencies and features

### 8.3 External References

**Protocols & Standards:**
- Wayland virtual keyboard protocol: `zwp_virtual_keyboard_v1`
- xdg-desktop-portal RemoteDesktop: https://flatpak.github.io/xdg-desktop-portal/
- AT-SPI specification: https://www.freedesktop.org/wiki/Accessibility/AT-SPI2/
- KWin D-Bus interfaces: https://invent.kde.org/plasma/kwin

**Dependencies:**
- wayland-client: https://crates.io/crates/wayland-client
- zbus: https://crates.io/crates/zbus
- reis (libei bindings): https://crates.io/crates/reis
- xkbcommon: https://crates.io/crates/xkbcommon

---

## Appendix A: Quick Reference Commands

### Development
```bash
# Fast tests (pre-commit)
make test-fast

# Hardware tests (non-blocking)
make test-hardware

# Complete E2E
make test-e2e-audio

# Release gate
make test-release

# Install git hooks
make install-hooks

# Dev setup
make dev-setup
```

### Debugging
```bash
# Enable debug logging
export RUST_LOG=debug

# TUI dashboard with logging
cargo run --bin tui_dashboard -- --log-level debug --device "YOUR_DEVICE"

# Mic probe for audio diagnostics
cargo run --bin mic_probe -- --duration 30 --device "YOUR_DEVICE"

# STT test (requires Vosk model)
cargo run --features vosk,examples --example vosk_test
```

### Build Configurations
```bash
# Default (with STT)
cargo run --features vosk

# Minimal (no STT, no text injection)
cargo run --no-default-features --features silero

# With text injection
cargo run --features vosk,text-injection

# All features
cargo run --all-features
```

---

## Appendix B: Decision Log

### 2025-10-08: Consolidated Text Injection Strategy
**Decision:** Single paste approach with AT-SPI→ydotool fallback  
**Rationale:** Reduces complexity, improves reliability, clearer UX  
**Impact:** Removed standalone clipboard-only and ydotool-paste methods

### 2025-10-08: Non-Blocking Hardware Tests
**Decision:** Hardware tests don't block CI, report to telemetry  
**Rationale:** Fast developer feedback without hardware dependencies  
**Impact:** 3s pre-commit, weekly E2E validation, continuous monitoring

### 2025-10-08: vNext Method Priorities
**Decision:** Prioritize AT-SPI, then platform-specific optimal fallbacks  
**Rationale:** Maximum compatibility with graceful degradation  
**Impact:** Clear fallback chains per platform, better error messages

---

**End of Master Plan**

*This document is maintained by the ColdVox team. For questions or updates, please open an issue or contact the project maintainers.*
