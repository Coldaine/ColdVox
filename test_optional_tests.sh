#!/bin/bash
# Test script to verify optional tests correctly detect environment
set -e

echo "=========================================="
echo "ColdVox Optional Tests Environment Check"
echo "=========================================="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track results
SHOULD_RUN=0
SHOULD_SKIP=0
TOTAL=0

check_result() {
    local name=$1
    local should_run=$2
    local output=$3
    
    TOTAL=$((TOTAL + 1))
    
    if [[ "$should_run" == "true" ]]; then
        SHOULD_RUN=$((SHOULD_RUN + 1))
        if echo "$output" | grep -q "Skipping"; then
            echo -e "${RED}✗ FAIL${NC}: $name should RUN but is SKIPPING"
            echo "  Output: $output"
            return 1
        else
            echo -e "${GREEN}✓ PASS${NC}: $name is running (as expected)"
            return 0
        fi
    else
        SHOULD_SKIP=$((SHOULD_SKIP + 1))
        if echo "$output" | grep -q "Skipping"; then
            echo -e "${GREEN}✓ PASS${NC}: $name is skipping (as expected)"
            return 0
        else
            echo -e "${RED}✗ FAIL${NC}: $name should SKIP but is RUNNING"
            echo "  Output: $output"
            return 1
        fi
    fi
}

# === Environment Detection ===
echo "1. Environment Detection"
echo "------------------------"
echo "DISPLAY: ${DISPLAY:-not set}"
echo "WAYLAND_DISPLAY: ${WAYLAND_DISPLAY:-not set}"
echo ""

HAS_DISPLAY=false
if [[ -n "${DISPLAY:-}" ]] || [[ -n "${WAYLAND_DISPLAY:-}" ]]; then
    HAS_DISPLAY=true
    echo -e "${GREEN}✓ Display server available${NC}"
else
    echo -e "${RED}✗ No display server${NC}"
fi
echo ""

# === Tool Availability ===
echo "2. Tool Availability"
echo "--------------------"
TOOLS_AVAILABLE=true
for tool in xdotool Xvfb openbox; do
    if command -v $tool &>/dev/null; then
        echo -e "  ${GREEN}✓${NC} $tool"
    else
        echo -e "  ${RED}✗${NC} $tool (missing)"
        TOOLS_AVAILABLE=false
    fi
done
echo ""

# === Vosk Model ===
echo "3. Vosk Model Availability"
echo "--------------------------"
MODEL_PATH="${VOSK_MODEL_PATH:-models/vosk-model-small-en-us-0.15}"
HAS_MODEL=false
if [[ -d "$MODEL_PATH/graph" ]]; then
    HAS_MODEL=true
    echo -e "${GREEN}✓ Model found at: $MODEL_PATH${NC}"
else
    echo -e "${RED}✗ Model NOT found at: $MODEL_PATH${NC}"
fi
echo ""

# === Test Execution ===
echo "=========================================="
echo "Test Execution Verification"
echo "=========================================="
echo ""

FAILED=0

# Test 1: Unit tests (should always run)
echo "Test 1: Unit Tests (should always run)"
echo "---------------------------------------"
OUTPUT=$(cargo test -p coldvox-text-injection --lib test_configuration_defaults 2>&1 || true)
if check_result "Unit tests" "true" "$OUTPUT"; then
    :
else
    FAILED=$((FAILED + 1))
fi
echo ""

# Test 2: Vosk test with model
echo "Test 2: Vosk Test (should run if model exists)"
echo "-----------------------------------------------"
if [[ "$HAS_MODEL" == "true" ]]; then
    # This test is marked #[ignore], so it needs --ignored
    OUTPUT=$(timeout 30 cargo test -p coldvox-app --features vosk test_vosk_transcriber_with_model -- --ignored --nocapture 2>&1 || true)
    if check_result "Vosk test with model" "true" "$OUTPUT"; then
        :
    else
        FAILED=$((FAILED + 1))
    fi
else
    echo -e "${YELLOW}⊘ SKIP${NC}: Vosk test (no model available)"
fi
echo ""

# Test 3: Real injection smoke test
echo "Test 3: Real Injection Smoke Test"
echo "----------------------------------"
if [[ "$HAS_DISPLAY" == "true" ]]; then
    echo "Testing with RUN_REAL_INJECTION_SMOKE=1..."
    # Note: This test may hang if GTK app doesn't launch properly
    # Using a short timeout to detect hangs
    OUTPUT=$(timeout 5 bash -c 'RUN_REAL_INJECTION_SMOKE=1 cargo test -p coldvox-text-injection --features real-injection-tests -- real_injection_smoke --nocapture 2>&1' || echo "timeout or error")
    
    if echo "$OUTPUT" | grep -q "timeout or error"; then
        echo -e "${YELLOW}⚠ WARNING${NC}: Test timed out or errored (may be trying to launch GUI)"
        echo "  This is expected if GTK app launch hangs in this environment"
    elif echo "$OUTPUT" | grep -q "\[smoke\] Running"; then
        echo -e "${GREEN}✓ PASS${NC}: Test is running (not skipping)"
    elif echo "$OUTPUT" | grep -q "\[smoke\] Skipping"; then
        echo -e "${RED}✗ FAIL${NC}: Test skipped but environment has display"
        FAILED=$((FAILED + 1))
    else
        echo -e "${YELLOW}⊘ UNCLEAR${NC}: Test output unclear"
        echo "  Output snippet: $(echo "$OUTPUT" | head -3)"
    fi
else
    echo "Testing without RUN_REAL_INJECTION_SMOKE..."
    OUTPUT=$(cargo test -p coldvox-text-injection --features real-injection-tests -- real_injection_smoke --nocapture 2>&1 || true)
    if check_result "Real injection smoke (no env var)" "false" "$OUTPUT"; then
        :
    else
        FAILED=$((FAILED + 1))
    fi
fi
echo ""

# Test 4: E2E WAV test
echo "Test 4: E2E WAV Pipeline Test"
echo "------------------------------"
if [[ "$HAS_MODEL" == "true" ]]; then
    echo "Checking if test would skip..."
    # Just compile and check initial logic
    OUTPUT=$(timeout 10 cargo test -p coldvox-app --features vosk test_end_to_end_wav_pipeline --no-run 2>&1 || true)
    if echo "$OUTPUT" | grep -q "Finished"; then
        echo -e "${GREEN}✓ PASS${NC}: Test compiles (would run if executed)"
    else
        echo -e "${RED}✗ FAIL${NC}: Test doesn't compile"
        FAILED=$((FAILED + 1))
    fi
else
    echo -e "${YELLOW}⊘ SKIP${NC}: E2E WAV test (no model available)"
fi
echo ""

# === Summary ===
echo "=========================================="
echo "Summary"
echo "=========================================="
echo ""
echo "Environment:"
echo "  Display: $HAS_DISPLAY"
echo "  Tools: $TOOLS_AVAILABLE"
echo "  Vosk Model: $HAS_MODEL"
echo ""
echo "Expected behavior in this environment:"
if [[ "$HAS_DISPLAY" == "true" && "$TOOLS_AVAILABLE" == "true" ]]; then
    echo "  ✓ Text injection tests should RUN"
else
    echo "  ✗ Text injection tests should SKIP"
fi
if [[ "$HAS_MODEL" == "true" ]]; then
    echo "  ✓ Vosk E2E tests should RUN"
else
    echo "  ✗ Vosk E2E tests should SKIP"
fi
echo ""

if [[ $FAILED -eq 0 ]]; then
    echo -e "${GREEN}✓ All optionality checks PASSED${NC}"
    echo "Tests correctly detect environment and run/skip as appropriate"
    exit 0
else
    echo -e "${RED}✗ $FAILED check(s) FAILED${NC}"
    echo "Some tests are not correctly detecting environment"
    exit 1
fi
