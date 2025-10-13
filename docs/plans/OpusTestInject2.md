## Hardware Test Matrix & Pre-Commit Strategy

### Live Hardware Tests (Non-Blocking but Mandatory)

```yaml
# hardware_test_matrix.yaml
hardware_tests:
  continuous:  # Runs on every push, non-blocking
    - microphone_array_capture
    - gpu_cuda_availability  
    - audio_device_enumeration
    - compositor_detection
    - atspi_bus_health
    
  nightly:  # Full hardware suite
    - real_kwin_injection
    - real_hyprland_injection
    - physical_usb_audio
    - nvidia_tensorrt_inference
    - multi_monitor_focus
    
  required_for_release:  # Must pass before prod
    - complete_wav_to_injection
    - hardware_silence_detection
    - bluetooth_audio_latency
    - thermal_throttling_resilience
```

### Pre-Commit Hook Implementation

```bash
#!/bin/bash
# .git/hooks/pre-commit
# Fast-fail injection tests that MUST pass before commit

set -e

echo "🚀 Running fast injection tests (target: <3 seconds)..."

# Set timeout for entire test suite
export PYTEST_TIMEOUT=3

# Run only the fast, deterministic tests
pytest tests/injection/fast/ \
  -m "not hardware and not e2e" \
  --fail-fast \
  --timeout=0.5 \
  --timeout-method=thread \
  -q \
  --tb=short \
  --benchmark-disable

if [ $? -ne 0 ]; then
  echo "❌ Fast injection tests failed. Fix before committing."
  echo "To bypass (NOT recommended): git commit --no-verify"
  exit 1
fi

echo "✅ Fast tests passed in $(($SECONDS))s"

# Warning for skipped hardware tests
echo "⚠️  Hardware tests skipped (will run in CI)"
echo "   Run manually: make test-hardware"
```

```python
# tests/injection/fast/test_injection_core.py
# These MUST execute in <500ms each

import pytest
from unittest.mock import patch
import time

@pytest.mark.timeout(0.5)
def test_injection_fallback_chain():
    """
    Complete injection story with simulated failures.
    Uses behavioral fakes, not real hardware.
    """
    env = FakeInjectionEnvironment()  # <5ms setup
    
    # Simulate AT-SPI failure
    env.atspi.fail_next()
    
    start = time.perf_counter()
    result = inject_text_with_fallback("Hello world", env)
    duration = time.perf_counter() - start
    
    assert result.success
    assert result.method == "clipboard_paste"  # Fell back
    assert duration < 0.1  # 100ms max for fake env
    
@pytest.mark.timeout(0.3)
def test_concurrent_injection_ordering():
    """Fast version using in-memory fakes."""
    env = FakeInjectionEnvironment()
    
    # Rapid concurrent injections
    futures = []
    for i in range(5):
        futures.append(inject_async(f"Text {i}", env))
    
    results = [f.result(timeout=0.2) for f in futures]
    
    # Verify ordering preserved
    assert env.get_injected_text() == "Text 0Text 1Text 2Text 3Text 4"
    
@pytest.mark.timeout(0.2)
def test_unicode_handling():
    """Complex text handling without real compositor."""
    env = FakeInjectionEnvironment()
    
    test_cases = [
        "Simple ASCII",
        "Émoji 🚀 support",
        "中文字符",
        "RTL: العربية",
        "Math: ∑∫∂",
    ]
    
    for text in test_cases:
        result = inject_text_with_fallback(text, env)
        assert result.success, f"Failed on: {text}"
```

### Expanded E2E Tests (WAV → Injection)

```python
# tests/e2e/test_complete_voice_pipeline.py

@pytest.mark.e2e
@pytest.mark.hardware
@pytest.mark.timeout(30)  # Full E2E gets more time
class TestCompleteVoicePipeline:
    """
    These tests verify the COMPLETE user experience from
    speaking into microphone to text appearing in application.
    """
    
    def test_wav_to_kate_injection(self, hardware_env):
        """
        Complete story: User speaks, text appears in Kate.
        This is what actually matters to users.
        """
        # Setup: Real Kate, real audio
        hardware_env.start_app("kate")
        hardware_env.ensure_focus("kate")
        
        # Play test audio through virtual cable
        wav_file = "test_data/audio/hello_world.wav"  # "Hello, world!"
        hardware_env.play_audio(wav_file)
        
        # Complete pipeline should handle it
        pipeline = CompletePipeline(
            audio_input=hardware_env.audio_device,
            transcriber=WhisperTranscriber(),
            injector=TextInjector()
        )
        
        # Start pipeline
        pipeline.start()
        
        # Wait for complete processing
        wait_for(
            lambda: "Hello, world!" in hardware_env.get_kate_content(),
            timeout=5.0,
            error="Text never appeared in Kate"
        )
        
        # Verify complete telemetry
        spans = hardware_env.get_trace()
        assert spans.has("audio.capture")
        assert spans.has("vad.speech_detected")
        assert spans.has("whisper.transcribe")
        assert spans.has("injection.attempt")
        assert spans.has("atspi.text_changed")
        
        # Total pipeline latency
        total_ms = spans.total_duration_ms()
        assert total_ms < 500, f"Pipeline took {total_ms}ms"
    
    def test_continuous_dictation_flow(self, hardware_env):
        """
        Story: User dictates multiple sentences with pauses.
        System should handle speech segments naturally.
        """
        hardware_env.start_app("libreoffice_writer")
        
        # Simulate natural dictation
        dictation_segments = [
            ("test_data/audio/first_sentence.wav", "This is the first sentence."),
            ("test_data/audio/second_sentence.wav", "This is the second sentence."),  
            ("test_data/audio/third_sentence.wav", "This is the third sentence."),
        ]
        
        pipeline = CompletePipeline(
            audio_input=hardware_env.audio_device,
            transcriber=WhisperTranscriber(),
            injector=TextInjector()
        )
        
        pipeline.start()
        
        for wav_file, expected_text in dictation_segments:
            hardware_env.play_audio(wav_file)
            time.sleep(0.5)  # Natural pause between sentences
        
        # Verify complete text arrived in order
        final_text = hardware_env.get_libreoffice_content()
        expected = " ".join([text for _, text in dictation_segments])
        assert final_text == expected
        
        # Verify no text was lost or duplicated
        assert final_text.count("sentence") == 3
    
    def test_noisy_environment_injection(self, hardware_env):
        """
        Story: User dictates with background noise.
        System should still inject correct text.
        """
        hardware_env.start_app("vscode")
        hardware_env.add_background_noise("office_chatter.wav", volume=0.3)
        
        # Speak over noise
        hardware_env.play_audio("test_data/audio/clear_speech_with_noise.wav")
        
        pipeline = CompletePipeline(
            audio_input=hardware_env.audio_device,
            transcriber=WhisperTranscriber(model="large-v3"),  # Better for noise
            injector=TextInjector()
        )
        
        pipeline.start()
        
        # Should still get the text
        wait_for(
            lambda: "The quick brown fox" in hardware_env.get_vscode_content(),
            timeout=10.0  # More time for larger model
        )
    
    def test_accent_and_speed_variations(self, hardware_env):
        """
        Story: Different users with various accents and speaking speeds
        should all get accurate injection.
        """
        test_cases = [
            ("british_accent_fast.wav", "I'd like a cup of tea, please."),
            ("southern_drawl_slow.wav", "Y'all come back now, you hear?"),
            ("technical_jargon.wav", "Initialize the Kubernetes deployment."),
            ("numbers_and_symbols.wav", "The total is $1,234.56."),
        ]
        
        hardware_env.start_app("firefox_textbox")
        pipeline = CompletePipeline(
            audio_input=hardware_env.audio_device,
            transcriber=WhisperTranscriber(),
            injector=TextInjector()
        )
        pipeline.start()
        
        for wav_file, expected in test_cases:
            hardware_env.clear_firefox_textbox()
            hardware_env.play_audio(f"test_data/audio/{wav_file}")
            
            wait_for(
                lambda: expected in hardware_env.get_firefox_content(),
                timeout=5.0,
                error=f"Failed to inject: {expected}"
            )
```

### Hardware Test Orchestration

```python
# tests/hardware/test_orchestration.py

class HardwareTestRunner:
    """
    Runs hardware tests continuously without blocking CI.
    Results are collected for monitoring but don't fail builds.
    """
    
    def __init__(self):
        self.results_queue = Queue()
        self.telemetry = HardwareTestTelemetry()
    
    async def run_non_blocking(self):
        """
        Run hardware tests in background, report to telemetry.
        """
        tasks = []
        
        # Start all hardware tests concurrently
        for test in self.get_hardware_tests():
            task = asyncio.create_task(
                self.run_with_telemetry(test)
            )
            tasks.append(task)
        
        # Don't wait - let them run in background
        asyncio.gather(*tasks, return_exceptions=True)
        
        # Return immediately
        return {"status": "hardware_tests_started"}
    
    async def run_with_telemetry(self, test_func):
        """
        Run test and report results without failing CI.
        """
        test_name = test_func.__name__
        
        try:
            start = time.perf_counter()
            await test_func()
            duration = time.perf_counter() - start
            
            self.telemetry.record_success(test_name, duration)
            
        except Exception as e:
            self.telemetry.record_failure(test_name, str(e))
            
            # Alert but don't fail
            if self.is_critical_hardware(test_name):
                self.send_alert(
                    f"Critical hardware test failed: {test_name}",
                    level="warning",  # Not error - doesn't block
                    details=str(e)
                )
    
    def is_critical_hardware(self, test_name: str) -> bool:
        """Some hardware is more critical than others."""
        critical = [
            "test_microphone_capture",
            "test_gpu_availability",
            "test_atspi_bus_health"
        ]
        return test_name in critical
```

### Makefile Targets

```makefile
# Makefile

# Fast tests for pre-commit (< 3 seconds)
.PHONY: test-fast
test-fast:
	@echo "Running fast injection tests..."
	@pytest tests/injection/fast/ -m "not hardware" --timeout=0.5 -q

# Hardware tests (non-blocking)
.PHONY: test-hardware
test-hardware:
	@echo "Starting hardware tests (non-blocking)..."
	@python -m tests.hardware.runner --non-blocking

# Complete E2E with real audio (blocking, for release)
.PHONY: test-e2e-audio
test-e2e-audio:
	@echo "Running complete WAV→injection tests..."
	@pytest tests/e2e/ -m "e2e and audio" --timeout=30

# Pre-release gate
.PHONY: test-release
test-release: test-fast
	@echo "Running release tests..."
	@pytest tests/e2e/ -m "release_gate" --timeout=60
	@python -m tests.hardware.runner --blocking --required-only

# Install git hooks
.PHONY: install-hooks
install-hooks:
	@cp hooks/pre-commit .git/hooks/
	@chmod +x .git/hooks/pre-commit
	@echo "Pre-commit hook installed"

# Developer setup
.PHONY: dev-setup
dev-setup: install-hooks
	@pip install -e ".[dev]"
	@echo "Development environment ready"
```

## Additional Critical Pieces

### 1. Flakiness Detection & Quarantine

```python
class FlakinessDetector:
    """
    Automatically detect and quarantine flaky tests.
    """
    
    def __init__(self):
        self.failure_history = defaultdict(list)
        self.quarantine_threshold = 0.1  # 10% failure rate
    
    def record_run(self, test_name: str, passed: bool):
        self.failure_history[test_name].append(passed)
        
        # Keep last 100 runs
        if len(self.failure_history[test_name]) > 100:
            self.failure_history[test_name].pop(0)
        
        # Check for flakiness
        if self.is_flaky(test_name):
            self.quarantine(test_name)
    
    def is_flaky(self, test_name: str) -> bool:
        history = self.failure_history[test_name]
        if len(history) < 10:
            return False
        
        failure_rate = history.count(False) / len(history)
        return 0 < failure_rate < self.quarantine_threshold
    
    def quarantine(self, test_name: str):
        """Move test to quarantine, alert team."""
        print(f"⚠️  Quarantining flaky test: {test_name}")
        
        # Add pytest mark
        with open("pytest.ini", "a") as f:
            f.write(f"\n# Auto-quarantined\nmarkers = quarantine: {test_name}")
        
        # Alert team
        send_slack_alert(
            f"Test {test_name} quarantined for flakiness. "
            f"Failure rate: {self.get_failure_rate(test_name):.1%}"
        )
```

### 2. Performance Regression Detection

```python
@pytest.fixture(autouse=True)
def track_performance(request, benchmark):
    """
    Automatically track performance of every test.
    """
    if "benchmark" in request.keywords:
        return
    
    start = time.perf_counter()
    yield
    duration = time.perf_counter() - start
    
    # Record and check for regression
    test_name = request.node.nodeid
    baseline = load_baseline(test_name)
    
    if baseline and duration > baseline * 1.2:  # 20% regression
        warnings.warn(
            f"Performance regression detected: "
            f"{test_name} took {duration:.2f}s "
            f"(baseline: {baseline:.2f}s)"
        )
```

### 3. Test Data Management

```python
class TestAudioLibrary:
    """
    Centralized test audio management.
    """
    
    # Standard test files with known content
    TEST_AUDIO = {
        "simple": {
            "file": "hello_world.wav",
            "text": "Hello, world!",
            "duration_ms": 1500,
            "speaker": "neutral_us"
        },
        "accented": {
            "file": "british_accent.wav", 
            "text": "Good morning, how are you?",
            "duration_ms": 2000,
            "speaker": "british_female"
        },
        "noisy": {
            "file": "speech_with_noise.wav",
            "text": "Testing one two three",
            "duration_ms": 2500,
            "snr_db": 10  # Signal-to-noise ratio
        },
        "technical": {
            "file": "technical_terms.wav",
            "text": "Initialize the Kubernetes deployment with PostgreSQL",
            "duration_ms": 3000,
            "speaker": "neutral_us"
        }
    }
    
    @classmethod
    def get_test_wav(cls, scenario: str) -> TestAudioFile:
        """Get test audio with metadata."""
        return TestAudioFile(**cls.TEST_AUDIO[scenario])
```

The test architecture ensures:
1. **Fast feedback** via pre-commit hooks (<3s)
2. **Hardware validation** without blocking development  
3. **Complete E2E verification** from audio to injection
4. **Flakiness management** through automatic quarantine
5. **Performance tracking** to catch regressions

The key insight: Hardware tests must run continuously but can't block development. The pre-commit suite proves core logic works, while E2E tests with real audio prove the complete system works for users.