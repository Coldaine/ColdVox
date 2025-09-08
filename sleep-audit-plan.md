# Sleep Audit Plan - ColdVox Codebase Analysis

## Executive Summary

The ColdVox codebase contains 108 `sleep()` calls across 38 Rust files. While our recent CI fixes addressed the most critical hanging issues (infinite waits on AT-SPI connections), a comprehensive audit of all sleep operations is needed to:

1. **Identify potential CI issues** - sleeps that could contribute to test execution time or flakiness
2. **Assess necessity and timing** - whether each sleep is required and appropriately configured
3. **Risk assessment** - classify sleeps by potential for causing CI timeouts or flakiness
4. **Understanding opportunities** - document sleeps that might benefit from alternative approaches

## Complete Sleep Inventory

### High-Risk Test Files (Potential CI Impact)

#### `crates/app/src/stt/tests/end_to_end_wav.rs` - **13 sleeps**
- **Risk Level**: HIGH - End-to-end tests with multiple timing dependencies
- **Concern**: Longest test file with most sleeps, requires analysis of timing necessity

#### `crates/app/tests/unit/watchdog_test.rs` - **12 sleeps**
- **Risk Level**: HIGH - Watchdog timing tests require precise delays
- **Concern**: Multiple sequential sleeps testing timeout behavior

#### `crates/coldvox-text-injection/src/tests/real_injection.rs` - **7 sleeps**
- **Risk Level**: MEDIUM - Real injection tests with app startup waits
- **Concern**: UI synchronization delays, worth examining for necessity

#### `crates/app/tests/integration/capture_integration_test.rs` - **7 sleeps**
- **Risk Level**: MEDIUM - Audio capture integration timing
- **Concern**: Hardware timing dependencies

### Text Injection Components (Production Code)

#### `crates/coldvox-text-injection/src/session.rs` - **4 sleeps**
- **Risk Level**: LOW - Production delays for UI coordination
- **Concern**: User-facing delays, should be minimal but necessary

#### `crates/coldvox-text-injection/src/combo_clip_atspi.rs` - **3 sleeps**
- **Risk Level**: MEDIUM - AT-SPI coordination delays
- **Concern**: External service coordination timing

#### `crates/coldvox-text-injection/src/combo_clip_ydotool.rs` - **3 sleeps**
- **Risk Level**: MEDIUM - Wayland tool coordination
- **Concern**: External process synchronization

#### `crates/coldvox-text-injection/src/manager.rs` - **2 sleeps**
- **Risk Level**: LOW - Strategic delays for text injection
- **Concern**: Chunking and rate limiting, likely necessary

### Audio Processing Components

#### `crates/app/src/stt/processor.rs` - **4 sleeps**
- **Risk Level**: LOW - STT processing coordination
- **Concern**: Processing pipeline timing

#### `crates/coldvox-audio/src/capture.rs` - **4 sleeps**
- **Risk Level**: LOW - Audio hardware coordination
- **Concern**: Device initialization timing

### Lower-Risk Files (1-3 sleeps each)
- `crates/app/tests/integration/mock_injection_tests.rs` - 4 sleeps
- `crates/app/src/probes/mic_capture.rs` - 3 sleeps
- `crates/app/src/probes/vad_mic.rs` - 2 sleeps
- `crates/coldvox-audio/src/monitor.rs` - 2 sleeps
- `crates/coldvox-text-injection/src/tests/real_injection_smoke.rs` - 2 sleeps
- Multiple files with 1 sleep each (18 files)

### Example/Demo Code (Low Priority)
- `examples/stt_performance_metrics_demo.rs` - 3 sleeps
- `examples/inject_demo.rs` - 3 sleeps
- `examples/foundation_probe.rs` - 4 sleeps
- `crates/coldvox-telemetry/examples/demo.rs` - 3 sleeps
- Other example files with 1-2 sleeps each

## Agent Evaluation Prompt

---

# Sleep Audit Analysis Task

You are tasked with conducting a comprehensive evaluation of all sleep operations in the ColdVox codebase. Your goal is to assess each sleep for necessity, optimization potential, and CI impact.

## Your Mission

Analyze each sleep operation in the following files, providing:

1. **Necessity Assessment** - Is this sleep required for correct operation?
2. **Timing Evaluation** - Is the duration appropriate and well-tuned?
3. **CI Impact Score** - How much does this sleep affect CI execution time?
4. **Risk Classification** - Potential for causing flakiness or timeouts
5. **Optimization Opportunities** - Can this be improved or eliminated?

## Analysis Framework

For each file, examine every `sleep()` call and categorize using this framework:

### Necessity Grades (A-F)
- **A (Essential)** - Critical for correctness, cannot be removed
- **B (Important)** - Improves reliability, difficult to replace
- **C (Useful)** - Prevents issues but could potentially be optimized
- **D (Questionable)** - May be unnecessary or overly conservative
- **F (Harmful)** - Definitely should be removed or replaced

### CI Impact Scores (1-10)
- **1-2 (Minimal)** - <10ms, negligible CI impact
- **3-4 (Low)** - 10-50ms, acceptable for necessary operations
- **5-6 (Medium)** - 50-200ms, should be justified
- **7-8 (High)** - 200ms-1s, needs strong justification
- **9-10 (Critical)** - >1s, likely problematic for CI

### Risk Classifications
- **GREEN** - Stable, predictable timing, low flakiness risk
- **YELLOW** - May occasionally cause delays but generally reliable
- **RED** - High potential for CI flakiness or timeouts

### Optimization Categories
- **Event-Driven** - Can be replaced with event polling/waiting
- **Configurable** - Should respect environment variables (CI vs development)
- **Conditional** - Only needed in certain conditions
- **Hardware-Dependent** - Required for hardware coordination
- **Test-Only** - Only affects test execution time
- **User-Facing** - Impacts user experience if changed

## Detailed Analysis Instructions

### Step 1: File-by-File Analysis
For each file in the inventory above:

1. **Read the complete file** to understand context
2. **Locate each sleep() call** and examine surrounding code
3. **Determine the purpose** - what is the sleep trying to achieve?
4. **Assess alternatives** - could this be done differently?
5. **Grade using the framework above**

### Step 2: Pattern Recognition
Look for common patterns:
- **Sequential sleeps** in test files (accumulating delays)
- **Fixed vs variable timing** (hardcoded vs configurable)
- **Test synchronization** vs **production timing**
- **Hardware coordination** vs **arbitrary delays**

### Step 3: Priority Recommendations
Based on your analysis, create prioritized recommendations:

**Immediate Actions** (High Impact, Low Effort)
- Sleeps that should be removed or reduced immediately
- Quick wins for CI performance

**Short-Term Improvements** (Medium Impact, Medium Effort)
- Sleeps that can be made configurable for CI environments
- Event-driven replacements

**Long-Term Optimizations** (Variable Impact, High Effort)
- Architectural changes to eliminate timing dependencies
- Hardware abstraction improvements

## Expected Deliverable Format

Create a comprehensive report with the following sections:

### 1. Executive Summary
- Total sleeps analyzed: 108
- Overall health assessment
- Top 3 recommendations

### 2. Critical Findings
- Highest impact sleeps (Grade F or Score 9-10)
- Immediate action items
- CI bottlenecks

### 3. File-by-File Analysis
For each file, provide:
```
### crates/path/to/file.rs (X sleeps)
**Overall File Grade**: A-F
**Total CI Impact**: Sum of individual scores
**Risk Level**: GREEN/YELLOW/RED

#### Sleep 1 (Line N): sleep(Duration::from_millis(X))
- **Purpose**: Why this sleep exists
- **Necessity Grade**: A-F with justification
- **CI Impact Score**: 1-10 with justification
- **Risk**: GREEN/YELLOW/RED
- **Optimization**: Category and suggestion
- **Context**: Code excerpt and explanation

#### Sleep 2 (Line N): ...
[Continue for all sleeps in file]

**File Recommendations**: Specific actionable improvements
```

### 4. Summary Statistics
- Grade distribution (how many A, B, C, D, F)
- CI impact distribution (how many 1-2, 3-4, etc.)
- Risk distribution (GREEN/YELLOW/RED counts)
- Optimization category breakdown

### 5. Implementation Roadmap
- **Phase 1 (Immediate)**: Critical fixes
- **Phase 2 (Short-term)**: Medium-impact improvements
- **Phase 3 (Long-term)**: Architectural optimizations

### 6. Testing Strategy
- How to validate that sleep modifications don't break functionality
- CI performance benchmarks to track
- Regression testing approach

## Success Criteria

Your analysis is successful if it:

1. **Comprehensively covers all 108 sleeps** with justified grades
2. **Identifies actionable improvements** that can speed up CI
3. **Maintains system correctness** - no recommendations that break functionality
4. **Provides clear prioritization** for implementation
5. **Includes concrete examples** of how to implement changes

## Key Questions to Answer

1. **Which sleeps are the biggest CI bottlenecks?**
2. **Which sleeps can be safely reduced or eliminated?**
3. **Which sleeps should be configurable for CI vs development?**
4. **Are there patterns of problematic sleep usage?**
5. **What's the potential total CI time savings?**

## Tools and Approach

- Use the Read tool to examine each file thoroughly
- Use the Grep tool to find specific sleep patterns
- Focus on high-impact files first (those with 4+ sleeps)
- Look for comments explaining why sleeps are needed
- Consider the broader context of each component

Remember: The goal is not to eliminate all sleeps, but to ensure each one is necessary, appropriately timed, and not causing CI issues. Some sleeps are genuinely required for hardware coordination, user experience, or system stability.

---

Begin your analysis with the highest-risk files first, and provide regular progress updates as you work through the inventory.
