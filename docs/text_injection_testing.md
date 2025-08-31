# Text Injection Testing Matrix and Scenarios

## Overview

This document outlines comprehensive testing scenarios for the text injection system, covering functional, performance, security, and edge case testing.

## Test Categories

### 1. Functional Tests

#### Backend Detection and Selection
- **Scenario**: System with multiple backends available
- **Steps**:
  1. Start with Wayland + AT-SPI available
  2. Disable AT-SPI service
  3. Verify fallback to clipboard-only mode
  4. Re-enable AT-SPI
  5. Verify preferred backend selection
- **Acceptance Criteria**:
  - Backend detection completes within 100ms
  - Correct backend selected based on availability
  - No crashes during backend switching

#### Focus State Handling
- **Scenario**: Wayland + AT-SPI off, unknown focus handling
- **Test Case 1**: `inject_on_unknown_focus = true`
  - **Steps**:
    1. Set focus to unknown state
    2. Attempt text injection
    3. Verify injection proceeds
  - **Acceptance Criteria**:
    - Injection succeeds
    - `focus_missing` metric not incremented

- **Test Case 2**: `inject_on_unknown_focus = false`
  - **Steps**:
    1. Set focus to unknown state
    2. Attempt text injection
    3. Verify injection blocked
  - **Acceptance Criteria**:
    - Injection fails with appropriate error
    - `focus_missing` metric incremented

### 2. Performance Tests

#### Large Transcript Handling
- **Scenario**: Paste chunking with large text
- **Steps**:
  1. Generate 10KB text transcript
  2. Configure `paste_chunk_chars = 1000`
  3. Inject text
  4. Monitor chunk processing
- **Acceptance Criteria**:
  - Text split into correct chunk sizes (±10%)
  - Per-chunk pacing respected (min 50ms between chunks)
  - Total budget not exceeded
  - All chunks injected successfully

#### Keystroke Pacing
- **Scenario**: High-frequency keystroke injection
- **Configuration**: `rate_cps = 30`
- **Steps**:
  1. Generate burst of 100 characters
  2. Inject using keystroke method
  3. Measure inter-keystroke timing
- **Acceptance Criteria**:
  - Average rate within 25-35 CPS
  - Jitter tolerance: ±20ms
  - No character loss
  - Rate limiting triggers correctly

### 3. Fallback Cascade Tests

#### Complete Fallback Chain
- **Scenario**: Progressive backend failure
- **Steps**:
  1. Start with AT-SPI enabled
  2. Inject text - verify AT-SPI used
  3. Disable AT-SPI paste action
  4. Inject text - verify clipboard+paste fallback
  5. Disable clipboard
  6. Inject text - verify clipboard-only fallback
  7. Enable ydotool
  8. Inject text - verify ydotool fallback
- **Acceptance Criteria**:
  - Each fallback attempted in correct order
  - Success recorded for working fallback
  - Appropriate error for failed methods
  - No infinite loops

### 4. Security and Privacy Tests

#### Allowlist/Blocklist Functionality
- **Test Case 1**: Allow-only mode
  - **Configuration**:
    ```toml
    allowlist = ["firefox", "chromium"]
    ```
  - **Steps**:
    1. Focus terminal application
    2. Attempt injection
    3. Focus Firefox
    4. Attempt injection
  - **Acceptance Criteria**:
    - Terminal injection blocked
    - Firefox injection allowed

- **Test Case 2**: Block specific applications
  - **Configuration**:
    ```toml
    blocklist = ["terminal"]
    ```
  - **Steps**:
    1. Focus terminal
    2. Attempt injection
    3. Focus text editor
    4. Attempt injection
  - **Acceptance Criteria**:
    - Terminal injection blocked
    - Text editor injection allowed

#### Regex Pattern Handling
- **Test Case 1**: Valid regex patterns
  - **Configuration**:
    ```toml
    allowlist = ["^firefox$", "chromium.*"]
    ```
  - **Steps**:
    1. Test various window class names
    2. Verify correct matching
  - **Acceptance Criteria**:
    - Valid patterns work correctly
    - Performance impact minimal

- **Test Case 2**: Invalid regex patterns
  - **Configuration**:
    ```toml
    allowlist = ["[invalid", "^firefox$"]
    ```
  - **Steps**:
    1. Attempt injection with invalid pattern
    2. Check logs for warnings
    3. Verify valid patterns still work
  - **Acceptance Criteria**:
    - Invalid pattern logged as warning
    - Invalid pattern skipped
    - Valid patterns continue working
    - No crashes

#### Privacy Logging
- **Scenario**: Log content verification
- **Steps**:
  1. Enable debug logging temporarily
  2. Inject sensitive text
  3. Review log output
  4. Disable debug logging
- **Acceptance Criteria**:
  - Normal logs show only length/hash
  - Debug logs show full text (when explicitly enabled)
  - No accidental plaintext in production logs

## Test Environment Setup

### System Requirements
- **Wayland**: GNOME/KDE Plasma
- **X11**: Fallback testing
- **AT-SPI**: Accessibility services enabled
- **Tools**: wl-clipboard, ydotool, kdotool installed

### Test Applications
- **Terminal**: gnome-terminal, konsole
- **Browser**: Firefox, Chromium
- **Editor**: gedit, kate, VS Code
- **Office**: LibreOffice

## Automated vs Manual Tests

### Automated Tests
- Backend detection
- Configuration validation
- Basic injection success/failure
- Performance metrics
- Memory usage
- Error handling

### Manual Tests
- Visual confirmation of injection
- Cross-application testing
- IME interaction
- Focus state verification
- Log content review

## Test Data

### Sample Texts
- **Short**: "Hello world"
- **Medium**: 500-character paragraph
- **Long**: 10KB technical documentation
- **Unicode**: Mixed ASCII/Unicode content
- **Special**: Control characters, newlines, tabs

### Window Classes
- `firefox`
- `chromium-browser`
- `gnome-terminal`
- `code`
- `gedit`

## Metrics and Monitoring

### Key Metrics to Verify
- `chars_buffered`: Accurate character counting
- `chars_injected`: Matches actual injected content
- `successes`/`failures`: Correct incrementing
- `latency_samples`: Realistic timing values
- `rate_limited`: Triggers on budget exhaustion

### Performance Benchmarks
- **Cold Start**: <500ms to first injection
- **Hot Path**: <50ms per injection
- **Memory**: <50MB steady state
- **CPU**: <5% during active injection

## Edge Cases

### Error Conditions
- Network clipboard services unavailable
- AT-SPI bus disconnected
- Permission changes during operation
- Window focus lost mid-injection
- System suspend/resume

### Boundary Conditions
- Empty text injection
- Maximum text size (100KB+)
- Rate limit boundary (exact CPS limit)
- Focus timeout (exact timing)
- Memory pressure scenarios

## Regression Testing

### Version Compatibility
- Test across different Wayland compositors
- Verify with different AT-SPI versions
- Check external tool compatibility

### Configuration Changes
- Hot-reload of allowlist/blocklist
- Runtime backend switching
- Dynamic rate limit adjustment

## Reporting

### Test Results Format
```
Test: Backend Fallback Cascade
Status: PASS
Duration: 2.3s
Details:
  - AT-SPI: PASS (45ms)
  - Clipboard+Paste: PASS (67ms)
  - Clipboard: PASS (23ms)
  - YdoTool: PASS (89ms)
```

### Coverage Metrics
- **Functional**: >95% code coverage
- **Performance**: All benchmarks met
- **Security**: All privacy checks pass
- **Compatibility**: Works on target platforms