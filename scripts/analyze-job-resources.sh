#!/bin/bash
# Analyze GitHub Actions job resource usage from logs

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=== GitHub Actions Job Resource Analysis ==="
echo "Analyzing recent workflow runs to classify job resource usage..."
echo

# Get recent workflow runs
RUNS=$(gh run list --limit 10 --json databaseId,name,conclusion,createdAt 2>/dev/null)

if [[ -z "$RUNS" ]]; then
    echo "No recent workflow runs found"
    exit 1
fi

# Analyze each workflow
echo "$RUNS" | jq -r '.[] | "\(.databaseId) \(.name)"' | while read -r run_id run_name; do
    echo "Analyzing run: $run_name (ID: $run_id)"
    
    # Get job details
    JOBS=$(gh run view "$run_id" --json jobs 2>/dev/null || echo "{}")
    
    if [[ "$JOBS" == "{}" ]]; then
        continue
    fi
    
    # Parse job information
    echo "$JOBS" | jq -r '.jobs[] | "\(.name)|\(.startedAt)|\(.completedAt)|\(.conclusion)"' | while IFS='|' read -r job_name started completed conclusion; do
        if [[ -n "$started" && -n "$completed" ]]; then
            # Calculate duration
            start_epoch=$(date -d "$started" +%s 2>/dev/null || echo 0)
            end_epoch=$(date -d "$completed" +%s 2>/dev/null || echo 0)
            duration=$((end_epoch - start_epoch))
            duration_min=$((duration / 60))
            
            # Classify based on duration and name patterns
            classification="medium"
            color="$YELLOW"
            
            # Heavy classification rules
            if [[ "$job_name" =~ (build|test|integration|e2e) ]] && [[ $duration_min -gt 5 ]]; then
                classification="heavy"
                color="$RED"
            # Light classification rules  
            elif [[ "$job_name" =~ (validate|format|lint|success) ]] || [[ $duration_min -lt 2 ]]; then
                classification="light"
                color="$GREEN"
            fi
            
            printf "  ${color}%-40s %3d min  [%s]${NC}\n" "$job_name" "$duration_min" "$classification"
        fi
    done
done

echo
echo "=== Resource Classification Summary ==="
echo
echo "Based on analysis, here's the recommended job classification:"
echo
echo "HEAVY JOBS (1 concurrent, 6+ cores):"
echo "  - build_and_check"
echo "  - text_injection_tests"
echo "  - msrv-check"
echo
echo "MEDIUM JOBS (2 concurrent, 3 cores):"
echo "  - setup-vosk-model"
echo "  - gui-groundwork"
echo "  - security"
echo
echo "LIGHT JOBS (4 concurrent, 1 core):"
echo "  - validate-workflows"
echo "  - ci-success"
echo "  - Individual lint/format steps"
echo
echo "To implement, add these labels to your workflow jobs:"
echo "  runs-on: [self-hosted, Linux, X64, fedora, nobara, heavy]"
echo "  runs-on: [self-hosted, Linux, X64, fedora, nobara, medium]"
echo "  runs-on: [self-hosted, Linux, X64, fedora, nobara, light]"