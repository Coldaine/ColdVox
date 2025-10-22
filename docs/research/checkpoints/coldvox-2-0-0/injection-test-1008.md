---
doc_type: research
subsystem: text-injection
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
---

## Test Architecture for Injection Methods

Following the Pragmatic Test Architect philosophy, here's a behavior-first testing strategy that tells complete stories about text injection.

## Test Analysis

- **Current approach**: Likely unit tests with mocks for each injection method
- **Smell detected**: Fragmentation (testing AT-SPI separately from clipboard, etc.), Mock maze (mocking D-Bus, Wayland protocols)
- **Recommended approach**: Large-span tests that verify complete injection stories with real dependencies

## The Testing Layers

```yaml
# test_distribution.yaml
injection_tests:
  service_integration: 70%  # Complete injection flows with real AT-SPI, clipboard
  trace_based: 15%          # Multi-app injection verification
  contract: 10%             # Portal/EIS protocol compliance
  pure_logic: 5%            # Keymap conversion, text chunking algorithms
```

## Core Test Architecture

### 1. The Master Integration Test - Complete Injection Story

```python
def test_complete_injection_flow_with_fallbacks():
    """
    Story: As a user dictating text, I want my words to appear
    in the focused application reliably, regardless of the 
    application type or compositor, within 200ms.
    
    This ONE test replaces:
    - test_atspi_connection()
    - test_focus_detection()
    - test_clipboard_backup()
    - test_insert_method()
    - test_paste_method()
    - test_virtual_keyboard()
    - test_portal_auth()
    - test_event_confirmation()
    """
    
    # Setup: Real injection environment
    env = InjectionTestEnvironment()
    env.start_test_app("kate")  # Real Kate editor in container
    env.start_atspi_bus()       # Real AT-SPI bus
    env.start_compositor("kwin") # Real KWin in nested mode
    
    injector = TextInjector(
        atspi_conn=env.atspi_connection,
        clipboard=env.clipboard,
        portal=env.portal,
        telemetry=env.telemetry_collector
    )
    
    # Pre-warm (part of the story!)
    start = time.monotonic()
    injector.pre_warm()
    assert (time.monotonic() - start) < 0.05  # 50ms pre-warm budget
    
    # Execute: Inject text with natural fallbacks
    text = "Hello, this is a test of the emergency injection system! 🚀"
    
    result = injector.inject(text, timeout=Duration.from_millis(200))
    
    # Verify: Complete observable behavior
    
    # 1. Text appeared in application
    actual_text = env.test_app.get_text_content()
    assert actual_text == text
    
    # 2. Injection was fast
    assert result.elapsed_ms < 200
    
    # 3. AT-SPI events fired
    events = env.atspi_event_collector.get_events()
    text_changed = events.filter(type="object:text-changed:insert")
    assert len(text_changed) > 0
    assert text.startswith(text_changed[0].inserted_text[:10])
    
    # 4. Clipboard was preserved
    assert env.clipboard.get_content() == env.original_clipboard
    
    # 5. No Klipper pollution (KDE-specific)
    if env.has_klipper:
        assert text not in env.klipper.get_history()
    
    # 6. Telemetry captured the journey
    spans = env.telemetry_collector.get_spans()
    assert spans.has("injection.attempt")
    assert spans.has("atspi.insert") or spans.has("clipboard.paste")
    
    # 7. Method selection was intelligent
    if env.test_app.supports_atspi:
        assert result.method == "atspi_insert"
    elif env.compositor == "hyprland":
        assert result.method == "virtual_keyboard"
    
    # 8. Verify idempotency - inject again
    result2 = injector.inject(text, timeout=Duration.from_millis(200))
    assert env.test_app.get_text_content() == text + text  # Appended
```

### 2. The Failure Recovery Test

```python
def test_injection_graceful_degradation():
    """
    Story: When primary injection methods fail, the system should
    automatically fall back to alternative methods while maintaining
    data integrity and user experience.
    """
    
    env = InjectionTestEnvironment()
    env.start_test_app("firefox_textbox")  # Challenging target
    
    # Setup: Progressively disable methods
    failure_scenarios = [
        {
            "name": "AT-SPI unavailable",
            "setup": lambda: env.kill_atspi_bus(),
            "expected_method": "portal_eis",
            "expected_warning": "AT-SPI bus not responding"
        },
        {
            "name": "Portal requires consent",  
            "setup": lambda: env.portal.require_user_consent(),
            "expected_method": "virtual_keyboard",
            "expected_warning": "Portal authorization pending"
        },
        {
            "name": "Only clipboard works",
            "setup": lambda: env.disable_all_except_clipboard(),
            "expected_method": "clipboard_paste_synthesized",
            "expected_warning": "Using clipboard fallback"
        }
    ]
    
    for scenario in failure_scenarios:
        with env.reset():
            scenario["setup"]()
            
            injector = TextInjector(env.get_connection_params())
            text = f"Testing {scenario['name']} scenario"
            
            result = injector.inject(text, timeout=Duration.from_millis(500))
            
            # Verify graceful degradation
            assert result.success
            assert result.method == scenario["expected_method"]
            assert scenario["expected_warning"] in result.warnings
            
            # Text still arrived correctly
            assert env.test_app.get_text_content() == text
            
            # Telemetry shows the fallback journey
            spans = env.telemetry_collector.get_spans()
            assert spans.count("method.attempted") >= 2  # Tried multiple
            assert spans.last("method.succeeded").name == scenario["expected_method"]
```

### 3. The Race Condition Test

```python
def test_concurrent_injections_maintain_order():
    """
    Story: When multiple injections happen rapidly (e.g., fast speech),
    text should appear in the correct order without corruption.
    """
    
    env = InjectionTestEnvironment()
    env.start_test_app("vscode")  # Modern Electron app
    
    injector = ConcurrentInjector(
        base_injector=TextInjector(env.get_connection_params()),
        max_concurrent=3
    )
    
    # Rapid-fire injections simulating fast dictation
    texts = [
        "First sentence. ",
        "Second sentence. ",
        "Third sentence. ",
        "Fourth sentence. ",
        "Fifth sentence. "
    ]
    
    futures = []
    for text in texts:
        # Submit without waiting
        future = injector.inject_async(text)
        futures.append(future)
        time.sleep(0.01)  # 10ms between utterances
    
    # Wait for all to complete
    results = [f.wait(timeout=1.0) for f in futures]
    
    # Verify ordering and completeness
    expected = "".join(texts)
    actual = env.test_app.get_text_content()
    
    assert actual == expected, f"Got jumbled text: {actual}"
    
    # Verify no clipboard corruption from concurrent access
    assert env.clipboard.get_content() == env.original_clipboard
    
    # Each injection succeeded
    for i, result in enumerate(results):
        assert result.success, f"Injection {i} failed"
        assert result.elapsed_ms < 200
```

## Failure Matrix

```python
class InjectionFailureMatrix:
    """
    Comprehensive failure scenarios and expected behaviors.
    Each entry tells a story about resilience.
    """
    
    FAILURE_SCENARIOS = [
        {
            "id": "no_atspi_bus",
            "condition": "AT-SPI bus not running",
            "detection": "D-Bus connection timeout",
            "expected_behavior": "Fall back to Portal/EIS or virtual keyboard",
            "user_message": "Accessibility service unavailable, using alternative input",
            "recovery": "Try to restart AT-SPI bus, continue with fallback",
            "test": test_no_atspi_bus_fallback
        },
        {
            "id": "no_focused_window",
            "condition": "No application has focus",
            "detection": "AT-SPI returns no focused object",
            "expected_behavior": "Queue injection until focus detected",
            "user_message": "Please click on the target application",
            "recovery": "Poll for focus every 100ms up to 2s",
            "test": test_no_focus_queuing
        },
        {
            "id": "non_editable_focus",
            "condition": "Focused element is not text-editable",
            "detection": "No EditableText interface on focused object",
            "expected_behavior": "Try parent/child traversal, then clipboard",
            "user_message": "Target may not be editable, attempting injection",
            "recovery": "Search up to 3 levels for editable ancestor/descendant",
            "test": test_non_editable_traversal
        },
        {
            "id": "clipboard_locked",
            "condition": "Another app has clipboard locked",
            "detection": "Clipboard operation times out",
            "expected_behavior": "Skip clipboard methods, use direct input only",
            "user_message": None,  # Silent, user doesn't need to know
            "recovery": "Retry clipboard after 50ms, then abandon",
            "test": test_clipboard_locked_bypass
        },
        {
            "id": "portal_denied",
            "condition": "User denies portal permission",
            "detection": "Portal returns 'cancelled' response",
            "expected_behavior": "Respect denial, try other methods",
            "user_message": "Portal access denied, trying alternative methods",
            "recovery": "Mark portal unavailable for session, don't retry",
            "test": test_portal_denial_handling
        },
        {
            "id": "virtual_kbd_unsupported",
            "condition": "Compositor doesn't support virtual keyboard",
            "detection": "Protocol binding fails",
            "expected_behavior": "Use toolkit-specific methods or clipboard",
            "user_message": None,  # Too technical
            "recovery": "Cache as unavailable, don't retry this session",
            "test": test_vkbd_unavailable
        },
        {
            "id": "unicode_conversion_fail",
            "condition": "Character has no keycode mapping",
            "detection": "Keymap lookup returns nothing",
            "expected_behavior": "Use clipboard for this text chunk",
            "user_message": None,
            "recovery": "Split text, inject unmappable via clipboard",
            "test": test_emoji_injection
        },
        {
            "id": "rate_limited",
            "condition": "Too many rapid injections",
            "detection": "Multiple injections within 10ms",
            "expected_behavior": "Queue and batch inject",
            "user_message": None,
            "recovery": "Combine pending injections, inject as one",
            "test": test_rate_limiting
        }
    ]
    
    @staticmethod
    def run_failure_test(scenario_id: str):
        """Run specific failure scenario test."""
        scenario = next(s for s in FAILURE_SCENARIOS if s["id"] == scenario_id)
        return scenario["test"]()
```

## Behavioral Fakes

```python
class ATSPIBusFake:
    """
    Behavioral fake that simulates real AT-SPI bus behavior,
    including timing, event propagation, and failure modes.
    """
    
    def __init__(self):
        self.objects = {}
        self.focused = None
        self.event_log = []
        self.failure_mode = None
        self.latency_ms = 5  # Realistic AT-SPI latency
    
    def create_application(self, name: str) -> FakeAccessible:
        """Simulate a real application registering with AT-SPI."""
        app = FakeAccessible(
            name=name,
            role="application",
            interfaces=["Application", "Component"]
        )
        
        # Real apps have a window
        window = app.add_child(
            name=f"{name} - Main Window",
            role="frame",
            interfaces=["Window", "Component"]
        )
        
        # Add realistic text widget
        text_widget = window.add_child(
            name="editor",
            role="text",
            interfaces=["Text", "EditableText", "Component"],
            states=["enabled", "editable", "focusable"]
        )
        
        self.objects[app.path] = app
        return app
    
    async def get_focused(self) -> Optional[FakeAccessible]:
        """Simulate focus detection with realistic latency."""
        await asyncio.sleep(self.latency_ms / 1000)
        
        if self.failure_mode == "no_focus":
            return None
        
        return self.focused
    
    async def insert_text(self, obj: FakeAccessible, position: int, text: str) -> bool:
        """Simulate text insertion with events."""
        await asyncio.sleep(self.latency_ms / 1000)
        
        if self.failure_mode == "insert_fails":
            return False
        
        if "EditableText" not in obj.interfaces:
            raise NotEditableError()
        
        # Update object state
        old_text = obj.text_content
        obj.text_content = (
            old_text[:position] + 
            text + 
            old_text[position:]
        )
        
        # Fire realistic events
        self.fire_event(TextChangedEvent(
            source=obj,
            detail="insert",
            position=position,
            length=len(text),
            inserted_text=text
        ))
        
        self.fire_event(CaretMovedEvent(
            source=obj,
            position=position + len(text)
        ))
        
        return True
    
    def fire_event(self, event):
        """Propagate events like real AT-SPI."""
        self.event_log.append(event)
        
        # Simulate event propagation delay
        asyncio.create_task(self._delayed_propagation(event))
    
    async def _delayed_propagation(self, event):
        """Events don't arrive instantly in real AT-SPI."""
        await asyncio.sleep(0.001)  # 1ms propagation
```

## Logging Architecture

```python
class InjectionTelemetry:
    """
    Structured logging that tells the complete story of each injection.
    Integrates with OpenTelemetry for distributed tracing.
    """
    
    def __init__(self):
        self.tracer = trace.get_tracer("coldvox.injection")
        self.meter = metrics.get_meter("coldvox.injection")
        
        # Metrics that matter
        self.injection_duration = self.meter.create_histogram(
            "injection.duration_ms",
            description="Time from request to confirmed text appearance"
        )
        
        self.method_attempts = self.meter.create_counter(
            "injection.method.attempts",
            description="Injection attempts by method"
        )
        
        self.fallback_triggers = self.meter.create_counter(
            "injection.fallback.triggered",
            description="Times fallback was needed"
        )
    
    @contextmanager
    def injection_span(self, text: str, context: dict):
        """
        Create a span that tells the complete injection story.
        """
        with self.tracer.start_as_current_span("injection.attempt") as span:
            span.set_attributes({
                "injection.text_length": len(text),
                "injection.has_unicode": any(ord(c) > 127 for c in text),
                "injection.target_app": context.get("app_name"),
                "injection.compositor": context.get("compositor"),
                "injection.session_id": context.get("session_id")
            })
            
            try:
                yield InjectionRecorder(span)
            except Exception as e:
                span.record_exception(e)
                span.set_status(Status(StatusCode.ERROR, str(e)))
                raise
    
    def record_method_attempt(self, method: str, success: bool, duration_ms: float):
        """Record each method tried."""
        self.method_attempts.add(1, {
            "method": method,
            "success": str(success)
        })
        
        if not success:
            self.fallback_triggers.add(1, {"from_method": method})

class InjectionRecorder:
    """Records the injection journey within a span."""
    
    def __init__(self, span):
        self.span = span
        self.method_attempts = []
    
    def attempting_method(self, method: str):
        """Record method attempt with timing."""
        method_span = self.span.tracer.start_span(
            f"injection.method.{method}",
            parent=self.span
        )
        return method_span
    
    def clipboard_operation(self, op: str):
        """Record clipboard operations."""
        with self.span.tracer.start_span(f"clipboard.{op}"):
            pass
    
    def event_received(self, event_type: str, latency_ms: float):
        """Record AT-SPI event confirmation."""
        self.span.add_event(
            "atspi.event.received",
            attributes={
                "event.type": event_type,
                "event.latency_ms": latency_ms
            }
        )
```

## Test Execution Framework

```python
class InjectionTestSuite:
    """
    Orchestrates the complete test suite with proper setup/teardown.
    """
    
    @pytest.fixture(scope="session")
    def test_environment(self):
        """
        One-time setup of the complete test environment.
        This is expensive but gives us real confidence.
        """
        env = TestEnvironmentBuilder() \
            .with_compositor("kwin", nested=True) \
            .with_atspi_bus() \
            .with_test_apps(["kate", "firefox", "vscode"]) \
            .with_telemetry_collection() \
            .build()
        
        yield env
        
        env.teardown()
    
    @pytest.mark.timeout(5)  # Complete stories in <5s
    def test_complete_user_journey(self, test_environment):
        """
        The master test that verifies the entire user experience.
        """
        # This is the test that actually matters
        pass
    
    @pytest.mark.parametrize("failure", InjectionFailureMatrix.FAILURE_SCENARIOS)
    def test_failure_recovery(self, test_environment, failure):
        """
        Verify each failure mode is handled gracefully.
        """
        test_environment.induce_failure(failure["condition"])
        
        result = inject_text("Test text")
        
        assert result.success or failure["expected_behavior"] == "fail"
        assert result.user_message == failure["user_message"]
```

## Performance Assertions

```python
class PerformanceRequirements:
    """
    Define and verify performance requirements as part of tests.
    """
    
    # Maximum time budgets (milliseconds)
    BUDGETS = {
        "pre_warm": 50,
        "atspi_insert": 30,
        "clipboard_paste": 40,
        "virtual_keyboard_chunk": 50,
        "portal_eis_connect": 100,
        "total_injection": 200,
        "fallback_penalty": 50  # Additional time per fallback
    }
    
    @staticmethod
    def assert_performance(operation: str, duration_ms: float, 
                         fallback_count: int = 0):
        """
        Verify operation meets performance requirements.
        """
        budget = PerformanceRequirements.BUDGETS[operation]
        budget += fallback_count * PerformanceRequirements.BUDGETS["fallback_penalty"]
        
        assert duration_ms <= budget, (
            f"{operation} took {duration_ms}ms, "
            f"budget was {budget}ms (with {fallback_count} fallbacks)"
        )
```

## Summary

This test architecture follows the Pragmatic Test Architect philosophy by:

1. **Testing complete user journeys** rather than individual methods
2. **Using real dependencies** (actual AT-SPI bus, compositor, applications)
3. **Telling stories** about user value (text appears where expected)
4. **Eliminating mocks** in favor of behavioral fakes that simulate real behavior
5. **Verifying observable outcomes** (text in app) over implementation details

The tests prove the system works for users, not just that code executes. When these tests pass, we have confidence that dictation will work in production.