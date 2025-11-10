# ColdVox Issue Clearance Master Plan

**Created**: 2025-11-10
**Status**: Active Roadmap
**Remaining Issues**: 26 open

## Executive Summary

This document provides atomic, step-by-step instructions to systematically clear all remaining GitHub issues. Issues are grouped by priority tier and dependencies, with specific implementation steps for each.

---

## Priority Tier 1: Critical Path (Do First)

### Issue #221: Implement WhisperEngine API (Candle Migration)
**Priority**: P1 - CRITICAL
**Blocks**: #222, #223, #224
**Time Estimate**: 2-4 weeks
**Complexity**: Large

**Why Critical**: Enables pure-Rust architecture, eliminates Python dependency, unblocks benchmarking and documentation work.

#### Atomic Steps

**Phase 1: Audio Preprocessing (Week 1)**
```
1. Create directory: crates/coldvox-stt/src/candle/
2. Create file: audio.rs
3. Port log_mel_spectrogram from Candle examples:
   - URL: https://github.com/huggingface/candle/tree/main/candle-examples/examples/whisper
   - Function: mel_filters(), pcm_to_mel()
4. Add dependencies to Cargo.toml:
   - candle-core
   - candle-nn
   - hf-hub (for model downloads)
5. Write unit tests for mel spectrogram generation
6. Verify: cargo test -p coldvox-stt candle::audio
```

**Phase 2: Model Loading (Week 1-2)**
```
7. Create file: loader.rs
8. Implement ModelLoader struct:
   - load_safetensors() - for standard models
   - load_gguf() - for quantized models (defer to Phase 4)
9. Download base.en model for testing:
   - Use hf-hub to download from HuggingFace
10. Implement model config parsing (n_vocab, n_audio_ctx, etc.)
11. Test model loading with base.en
12. Verify: Model loads without panic, dimensions match spec
```

**Phase 3: Core Decoding (Week 2-3)**
```
13. Create file: decode.rs
14. Port decoder loop from Candle examples:
   - Token-by-token generation
   - Greedy sampling (no beam search initially)
15. Create file: types.rs
16. Define public types:
   - Segment { start, end, text }
   - Transcript { segments, language }
17. Implement basic transcription:
   - Input: mel spectrogram
   - Output: Transcript
18. Test with sample audio (silence, single word)
19. Verify: Can transcribe "hello" from test audio
```

**Phase 4: Timestamp Generation (Week 3)**
```
20. Create file: timestamps.rs
21. Port timestamp extraction:
   - Token -> time mapping
   - Segment boundary detection
22. Integrate with decode.rs
23. Test timestamp accuracy with known audio
24. Verify: Timestamps within 100ms of expected
```

**Phase 5: WhisperEngine Facade (Week 3-4)**
```
25. Create file: mod.rs (main facade)
26. Implement WhisperEngine struct:
   - new(model_path, device) -> Result<Self>
   - transcribe(audio: &[f32]) -> Result<Transcript>
27. Implement WhisperEngineInit config struct
28. Wire into plugin system:
   - Update plugins/whisper_plugin.rs
   - Replace stub with WhisperEngine
29. Update plugin to report is_available: true
30. Integration test: Full audio -> text pipeline
31. Verify: cargo run --features whisper transcribes real audio
```

**Acceptance Criteria**:
- [ ] All 5 modules implemented (audio, loader, decode, timestamps, mod)
- [ ] WhisperEngine transcribes test audio correctly
- [ ] Plugin reports as available
- [ ] Compatible with existing config (WHISPER_MODEL_PATH, etc.)
- [ ] Proper error handling throughout
- [ ] Code includes attribution to Candle source

**Next Issue After Completion**: #222 (Benchmarking)

---

## Priority Tier 2: Quick Wins (High ROI)

### Issue #171: Complete AT-SPI Focus Backend #jules
**Priority**: P2
**Time Estimate**: 30-45 minutes
**Complexity**: Small

#### Atomic Steps
```
1. Open: crates/coldvox-text-injection/src/focus.rs
2. Locate SystemFocusAdapter::query_focus() (line 92)
3. Copy AT-SPI connection code from manager.rs:295-339
4. Modify to query focused object state instead of name:
   - Use State::Focused + State::Editable
   - Return FocusStatus::EditableText if both present
   - Return FocusStatus::NonEditable if only focused
5. Add error handling for AT-SPI unavailable
6. Test with real apps:
   - Open text editor, verify EditableText
   - Focus terminal, verify EditableText
   - Focus browser window, verify Mixed/EditableText
7. cargo test -p coldvox-text-injection focus
8. Update issue #171 with completion comment
```

**Acceptance Criteria**:
- [ ] Returns EditableText for text fields
- [ ] Returns NonEditable for buttons/labels
- [ ] Falls back to Unknown when AT-SPI unavailable
- [ ] No panics on connection failure

---

## Priority Tier 3: Documentation & Testing

### Issue #224: Update README.md (Remove Faster-Whisper) #jules
**Priority**: P2
**Blocks**: None
**Depends On**: #221 completion
**Time Estimate**: 1-2 hours

#### Atomic Steps
```
1. Wait for #221 to be merged
2. Open: README.md
3. Find and replace:
   - "faster-whisper" -> "Candle Whisper"
   - "Python Package: Install the faster-whisper..." -> Remove section
   - "Faster-Whisper STT is the default" -> "Candle Whisper STT is the default"
4. Update Quick Start section:
   - Remove "pip install faster-whisper"
   - Update cargo run examples
5. Update Technology Stack:
   - Remove Python dependency mentions
   - Add "Pure Rust STT via Candle"
6. Update Model Setup section:
   - Keep model identifiers (base.en, etc.)
   - Update download mechanism description
7. Check for Python 3.13 compatibility warnings -> Remove
8. Verify all links work
9. cargo build --features whisper to verify examples
10. Create PR with README updates
```

**Acceptance Criteria**:
- [ ] All faster-whisper references removed
- [ ] Python dependency removed from STT context
- [ ] Model setup accurate for Candle
- [ ] Quick start examples work
- [ ] No broken links

---

### Issue #222: Build Benchmarking Harness #jules
**Priority**: P2
**Depends On**: #221 completion
**Time Estimate**: 1 week

#### Atomic Steps
```
1. Create: crates/coldvox-benchmarks/ (new crate)
2. Add Cargo.toml with dependencies:
   - criterion (for benchmarking)
   - coldvox-stt
   - candle-whisper engine
3. Create benchmark suite structure:
   - benches/stt_comparison.rs
4. Implement comparison metrics:
   - Transcription accuracy (WER - Word Error Rate)
   - Processing speed (RTF - Real Time Factor)
   - Memory usage
   - Cold start time
5. Create test audio corpus:
   - 10 samples, 5-30 seconds each
   - Various accents, noise levels
6. Implement faster-whisper baseline:
   - Keep old Python implementation for comparison
   - Run via Python subprocess
7. Implement Candle benchmark:
   - Use new WhisperEngine
8. Generate comparison report:
   - CSV output with metrics
   - Markdown summary
9. Add CI job to run benchmarks nightly
10. Document results in docs/benchmarks/
```

**Acceptance Criteria**:
- [ ] Compares both implementations on same audio
- [ ] Measures WER, RTF, memory, cold start
- [ ] Generates report with comparison
- [ ] CI runs benchmarks nightly
- [ ] Results documented

---

### Issue #223: Research Word-Level Timestamps #jules
**Priority**: P2
**Depends On**: #221 completion
**Time Estimate**: 3-5 days (research + prototype)

#### Atomic Steps
```
1. Research Whisper token-level timestamps:
   - Read Candle whisper examples
   - Review OpenAI Whisper source
   - Document token -> time mapping
2. Create prototype:
   - Extend timestamps.rs from #221
   - Implement token-level extraction
3. Create heuristics for word boundaries:
   - Space tokens
   - Punctuation handling
   - Language-specific rules
4. Test with sample audio:
   - "The quick brown fox"
   - Measure word timestamp accuracy
5. Document findings in docs/research/word-timestamps.md
6. If successful, integrate into WhisperEngine
7. Otherwise, document limitations
```

**Acceptance Criteria**:
- [ ] Research documented
- [ ] Prototype working or limitations documented
- [ ] If working: word timestamps within 50ms
- [ ] If not: clear explanation of challenges

---

### Issue #40: Platform-Specific Text Injection Testing #jules
**Priority**: P2
**Time Estimate**: 2-3 weeks (ongoing)
**Complexity**: Large

#### Atomic Steps
```
1. Create test matrix spreadsheet:
   - Rows: Backends (AT-SPI, clipboard, ydotool, kdotool, enigo)
   - Columns: DEs (GNOME/Wayland, GNOME/X11, KDE/Wayland, KDE/X11, Sway, i3)
2. Set up test environments (one at a time):
   - Install GNOME Wayland on test system
   - Configure auto-login
   - Install test apps (gnome-terminal, gedit, firefox)
3. For each environment:
   - Run: cargo test -p coldvox-text-injection --test live_tests -- --ignored
   - Document results in test matrix
   - Note failures with error messages
4. Create per-platform documentation:
   - docs/platforms/gnome-wayland.md
   - Include: required packages, permissions, known issues
5. Update README with platform compatibility table
6. Create CI job template for future VM-based testing
```

**Acceptance Criteria**:
- [ ] Test matrix complete for 5+ environments
- [ ] Known issues documented per platform
- [ ] Setup instructions for each environment
- [ ] Recommended backends per platform documented

---

### Issue #162: Testing Infrastructure #jules
**Priority**: P2
**Time Estimate**: Ongoing (epic)
**Complexity**: Large

**Note**: This is a large epic. Break into sub-issues:

#### Sub-Issue 1: Unit Test Coverage
```
1. Run: cargo tarpaulin --workspace
2. Identify crates <80% coverage:
   - List uncovered modules
3. For each module:
   - Add unit tests for public functions
   - Add edge case tests
4. Target: 80% coverage across workspace
```

#### Sub-Issue 2: Integration Test Suite
```
1. Create: crates/app/tests/integration/
2. Add tests for:
   - Full audio pipeline (mic -> VAD -> STT -> injection)
   - Plugin failover scenarios
   - Device hotplug handling
3. Use test fixtures:
   - Sample WAV files
   - Mock devices
4. Ensure tests run in CI
```

#### Sub-Issue 3: Property-Based Testing
```
1. Add proptest to dev-dependencies
2. Create property tests for:
   - Audio format conversions (all formats -> i16)
   - Ring buffer operations
   - Text injection encoding
3. Target: 1000 iterations per property
```

**Acceptance Criteria**:
- [ ] 80% code coverage
- [ ] Integration tests for main workflows
- [ ] Property tests for core algorithms
- [ ] All tests pass in CI

---

### Issue #173: VM-Based Compositor Testing Matrix #jules
**Priority**: P2
**Time Estimate**: 2-3 weeks
**Complexity**: Large

**Note**: This requires significant infrastructure work. Consider deferring until #40 is complete.

#### Atomic Steps
```
1. Install Packer on CI runner:
   - sudo dnf install packer
2. Create Packer config for Fedora XFCE:
   - packer/fedora-xfce.pkr.hcl
   - Auto-login enabled
   - ColdVox dependencies pre-installed
3. Build initial VM image:
   - packer build packer/fedora-xfce.pkr.hcl
4. Create libvirt orchestration script:
   - scripts/vm-test-runner.sh
   - Launches VM, runs tests, collects results
5. Test locally first:
   - ./scripts/vm-test-runner.sh fedora-xfce
   - Verify test execution
6. Create GitHub Actions workflow:
   - .github/workflows/vm-matrix.yml
   - Runs nightly on self-hosted runner
7. Add additional compositor images (one at a time):
   - KDE Plasma, GNOME, Sway
8. Document in docs/testing/vm-infrastructure.md
```

**Acceptance Criteria**:
- [ ] VM images build automatically
- [ ] Tests run in VMs successfully
- [ ] Results collected and reported
- [ ] Nightly runs complete in <4 hours
- [ ] Clear logs for debugging

---

## Priority Tier 4: Enhancements (Future Work)

### Issue #42: Long Utterance Processing #jules
**Priority**: P3
**Time Estimate**: 1-2 weeks
**Depends On**: #221, #222

#### Atomic Steps
```
1. Research Whisper long-form transcription:
   - Review Candle examples
   - Check for chunking strategies
2. Implement audio chunking for >30s utterances:
   - Split on silence detection
   - Overlap chunks by 1-2 seconds
3. Modify WhisperEngine to handle chunks:
   - Process each chunk
   - Merge transcripts with timestamp alignment
4. Test with long audio (5+ minutes)
5. Benchmark performance vs single-chunk
6. Document limitations (if any)
```

**Acceptance Criteria**:
- [ ] Transcribes audio >30 seconds
- [ ] Maintains timestamp accuracy across chunks
- [ ] Performance acceptable for real-time use

---

### Issue #44: STT Performance Metrics #jules
**Priority**: P3
**Time Estimate**: 3-5 days

#### Atomic Steps
```
1. Extend PipelineMetrics in coldvox-telemetry:
   - Add STT-specific metrics struct
2. Track metrics:
   - Transcription latency (time from audio end to text)
   - Processing time per second of audio (RTF)
   - Queue depth
   - Error rates per plugin
3. Expose via logging:
   - Log every 100 transcriptions
   - Include p50, p95, p99 latencies
4. Add /metrics endpoint (future):
   - JSON format for monitoring
5. Document metrics in docs/metrics.md
```

**Acceptance Criteria**:
- [ ] Latency tracked and logged
- [ ] RTF calculated correctly
- [ ] Error rates per plugin visible
- [ ] Metrics help identify bottlenecks

---

### Issue #45: Optimize Audio Format Conversions #jules
**Priority**: P3
**Time Estimate**: 2-3 days

**Note**: Related to #36 (now fixed). May already be optimal.

#### Atomic Steps
```
1. Benchmark current conversion performance:
   - Use criterion for micro-benchmarks
   - Test all formats: F32, U16, U32, F64 -> I16
2. Identify bottlenecks:
   - Profile with perf or flamegraph
3. Optimize hot paths:
   - SIMD vectorization (if applicable)
   - Lookup tables for U16/U32 conversions
4. Re-benchmark to measure improvement
5. If <10% improvement, close as "already optimal"
```

**Acceptance Criteria**:
- [ ] Benchmark baseline established
- [ ] Optimization attempted
- [ ] Results documented (even if minimal gain)

---

### Issue #46: Harden STT Model Loading #jules
**Priority**: P3
**Time Estimate**: 2-3 days

#### Atomic Steps
```
1. Add model validation to WhisperEngine:
   - Check file exists
   - Verify file signature (magic bytes)
   - Validate tensor dimensions
2. Add security checks:
   - Max file size limit (10GB)
   - Prevent path traversal
3. Improve error messages:
   - "Model not found: /path/to/model.safetensors"
   - "Invalid model format, expected safetensors"
4. Add tests for error cases:
   - Missing file
   - Corrupted file
   - Wrong format
5. Document model requirements in docs/
```

**Acceptance Criteria**:
- [ ] Model validation prevents bad inputs
- [ ] Clear error messages on failure
- [ ] No crashes from malformed models
- [ ] Security checks prevent exploits

---

### Issue #47: Async Processing for Non-Blocking STT #jules
**Priority**: P3
**Time Estimate**: 3-5 days

**Note**: STT already runs in async tasks. May be "already done".

#### Atomic Steps
```
1. Review current STT architecture:
   - Verify transcription runs in tokio::spawn
   - Check if blocking any async tasks
2. If blocking found:
   - Use tokio::task::spawn_blocking for CPU-heavy work
   - Ensure UI/TUI remains responsive
3. Add tests:
   - Verify UI responsive during long transcription
4. If already async, document and close
```

**Acceptance Criteria**:
- [ ] Transcription doesn't block UI
- [ ] TUI responsive during processing
- [ ] Architecture documented

---

## Priority Tier 5: Documentation & CI

### Issue #208: Refactor Tests to New Paradigm #jules
**Priority**: P3
**Time Estimate**: 1-2 weeks

#### Atomic Steps
```
1. Define "new testing paradigm":
   - Document in docs/testing/paradigm.md
   - Standards: naming, structure, fixtures
2. Create test migration checklist:
   - List all existing test files
3. Migrate tests one module at a time:
   - Update test names to new convention
   - Use common test utilities
   - Add missing setup/teardown
4. Create test templates:
   - examples/test_template.rs
5. Update CONTRIBUTING.md with test guidelines
```

**Acceptance Criteria**:
- [ ] Testing paradigm documented
- [ ] 80%+ tests follow new paradigm
- [ ] Template available for new tests

---

### Issue #209: Improve Logging (Verifiable & Context-Aware) #jules
**Priority**: P3
**Time Estimate**: 3-5 days

#### Atomic Steps
```
1. Add structured logging fields:
   - correlation_id (for request tracing)
   - component (audio, stt, text-injection)
   - operation (transcribe, inject, etc.)
2. Implement log correlation:
   - Generate ID on audio capture
   - Pass through pipeline
3. Add context to errors:
   - Include state (device name, plugin, etc.)
4. Create log analysis tools:
   - scripts/analyze-logs.sh (find errors by correlation_id)
5. Document logging patterns in docs/logging.md
```

**Acceptance Criteria**:
- [ ] Logs include correlation IDs
- [ ] Can trace request through pipeline
- [ ] Errors include relevant context
- [ ] Log analysis tools available

---

### Issue #210: LLM-Assisted Test Debugging Strategy #jules
**Priority**: P3
**Time Estimate**: 2-3 days (research + doc)

#### Atomic Steps
```
1. Research LLM test debugging:
   - How to format test output for LLMs
   - Effective prompt patterns
2. Create prompt templates:
   - "Analyze this test failure: ..."
   - "Suggest fixes for: ..."
3. Write integration guide:
   - docs/testing/llm-debugging.md
4. Create helper scripts:
   - scripts/format-test-output.sh (for LLM input)
5. Test with real failures
6. Document best practices
```

**Acceptance Criteria**:
- [ ] Prompt templates created
- [ ] Integration guide written
- [ ] Scripts to prepare test output for LLMs
- [ ] Examples of successful debugging

---

### Issue #211: Add Code Coverage Job to CI #jules
**Priority**: P3
**Time Estimate**: 1 day

#### Atomic Steps
```
1. Add tarpaulin to dev-dependencies
2. Create coverage workflow:
   - .github/workflows/coverage.yml
3. Configure tarpaulin:
   - Run on all crates
   - Generate HTML + lcov reports
4. Upload to Codecov or similar:
   - Add CODECOV_TOKEN to secrets
5. Add badge to README.md
6. Set coverage threshold (80%)
7. Fail PR if coverage drops >5%
```

**Acceptance Criteria**:
- [ ] Coverage runs in CI
- [ ] Report uploaded to Codecov
- [ ] Badge in README
- [ ] PRs show coverage diff

---

### Issue #212: Explore Test Parallelization #jules
**Priority**: P3
**Time Estimate**: 1-2 days

#### Atomic Steps
```
1. Benchmark current test runtime:
   - cargo test --workspace -- --nocapture
   - Record total time
2. Try nextest:
   - cargo install cargo-nextest
   - cargo nextest run --workspace
   - Compare time
3. If faster (>20% improvement):
   - Add to CI workflow
   - Document in docs/testing/
4. Configure parallel settings:
   - --test-threads=auto
5. Identify non-parallelizable tests:
   - Mark with #[serial] if needed
```

**Acceptance Criteria**:
- [ ] Test runtime measured
- [ ] Parallelization strategy chosen
- [ ] CI updated if beneficial
- [ ] Speedup documented

---

### Issue #213: Use GitHub-Hosted Runners for Certain Jobs #jules
**Priority**: P3
**Time Estimate**: 1 day

#### Atomic Steps
```
1. Identify jobs suitable for GitHub runners:
   - Clippy, rustfmt (no hardware needed)
   - Docs builds
   - Simple unit tests
2. Update workflows:
   - Change runs-on: ubuntu-latest (for suitable jobs)
   - Keep self-hosted for hardware tests
3. Test workflows on fork
4. Monitor cost (check GitHub Actions usage)
5. Document runner strategy in docs/ci/runners.md
```

**Acceptance Criteria**:
- [ ] CI jobs categorized (self-hosted vs GitHub)
- [ ] Workflows updated
- [ ] Cost monitored
- [ ] No loss in functionality

---

### Issue #215: Enhance Docs Cross-Reference Analyzer #jules
**Priority**: P3
**Time Estimate**: 2-3 days

#### Atomic Steps
```
1. Create script: scripts/check-docs-links.sh
2. Implement checks:
   - Find all [link](file.md) in docs/
   - Verify target exists
   - Check for broken anchors
   - Find orphaned docs (not linked anywhere)
3. Detect stale docs:
   - Compare doc last-modified vs code last-modified
   - Flag docs >3 months older than code
4. Add to CI:
   - .github/workflows/docs-check.yml
5. Generate report:
   - docs/LINK_REPORT.md (auto-generated)
```

**Acceptance Criteria**:
- [ ] Detects broken links
- [ ] Finds orphaned docs
- [ ] Flags stale documentation
- [ ] Runs in CI
- [ ] Report generated

---

## Priority Tier 6: Roadmap Items (Defer)

### Issue #226: GUI Integration Roadmap #jules
**Priority**: P4 (Future)
**Complexity**: Epic

**Action**: This is a large multi-milestone roadmap. Keep open as planning doc. No immediate action needed until backend work stabilizes.

### Issue #228: CI/CD Workflow Enhancements #jules
**Priority**: P3
**Time Estimate**: 1-2 weeks

**Action**: Break into sub-issues as needed. Many items may already be addressed by other issues (#211, #212, #213).

### Issue #229: Dependency Audit and Update #jules
**Priority**: P3
**Time Estimate**: 1 day recurring

#### Atomic Steps
```
1. Install cargo-outdated:
   - cargo install cargo-outdated
2. Run audit:
   - cargo outdated --workspace
3. Update dependencies:
   - cargo update
4. Test after update:
   - cargo test --workspace
5. Create PR with updates
6. Set up monthly reminder (GitHub Actions schedule)
```

### Issue #230: Developer Onboarding Documentation #jules
**Priority**: P3
**Time Estimate**: 2-3 days

#### Atomic Steps
```
1. Create CONTRIBUTING.md (if not exists)
2. Add sections:
   - Getting Started (build, test, run)
   - Architecture Overview
   - Coding Standards
   - Testing Guidelines
   - PR Process
3. Create docs/onboarding/:
   - architecture.md
   - development-workflow.md
   - debugging-guide.md
4. Record architecture walkthrough video (optional)
5. Test with new contributor
```

---

## Execution Strategy

### Phase 1: Foundation (Weeks 1-4)
**Goal**: Complete critical path + quick wins

1. **Week 1**: Start #221 (Candle audio.rs + loader.rs)
2. **Week 1**: Complete #171 (AT-SPI focus backend) - quick win
3. **Week 2-3**: Continue #221 (decode.rs, timestamps.rs)
4. **Week 4**: Complete #221 (WhisperEngine facade, integration)

### Phase 2: Consolidation (Weeks 5-6)
**Goal**: Update docs, add benchmarks

1. Complete #224 (README updates)
2. Complete #222 (Benchmarking harness)
3. Research #223 (Word-level timestamps)

### Phase 3: Quality (Weeks 7-10)
**Goal**: Improve testing and reliability

1. Platform testing #40 (ongoing)
2. Testing infrastructure #162 (unit tests, integration)
3. CI improvements #211, #212, #213

### Phase 4: Polish (Weeks 11-12)
**Goal**: Documentation and minor enhancements

1. #208, #209, #210 (Testing/logging improvements)
2. #215 (Docs analyzer)
3. #44, #45, #46, #47 (Performance/security enhancements)

### Phase 5: Advanced (Future)
**Goal**: Complex infrastructure

1. #173 (VM-based testing)
2. #226 (GUI integration)
3. #228, #229, #230 (Maintenance)

---

## Success Metrics

### Milestone 1 (End of Phase 1)
- [ ] #221 complete - Candle Whisper working
- [ ] #171 complete - AT-SPI focus backend working
- [ ] 2-3 quick wins closed

### Milestone 2 (End of Phase 2)
- [ ] README updated (#224)
- [ ] Benchmarks comparing old vs new STT (#222)
- [ ] Word timestamp research complete (#223)

### Milestone 3 (End of Phase 3)
- [ ] Code coverage >80% (#162, #211)
- [ ] Test matrix for 3+ platforms (#40)
- [ ] CI improvements deployed (#212, #213)

### Milestone 4 (End of Phase 4)
- [ ] <10 open issues remaining
- [ ] All P1/P2 issues closed
- [ ] Documentation complete

---

## Daily Workflow Template

Each day, follow this pattern:

```
1. Review open issues (sorted by priority)
2. Pick highest priority issue not blocked by dependencies
3. Follow atomic steps for that issue
4. Create feature branch: fix/issue-{number}-{short-name}
5. Implement, test, commit
6. Create PR with reference to issue
7. Update issue with progress comment
8. Close issue when PR merged
9. Repeat
```

---

## Notes

- **Don't batch issues**: Complete each issue fully before starting next
- **Update as you go**: Mark completed items immediately
- **Test everything**: Every issue requires tests passing
- **Document decisions**: Add comments to issues explaining approach
- **Ask for help**: Flag blockers immediately

---

## Quick Reference: Issue Priority List

**P1 (Must Do)**: #221
**P2 (Should Do)**: #171, #222, #223, #224, #40, #162, #173
**P3 (Nice to Have)**: #42, #44, #45, #46, #47, #208-215
**P4 (Future)**: #226, #228, #229, #230

**Start Here**: #221 (Candle migration) and #171 (AT-SPI focus)
