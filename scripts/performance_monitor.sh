#!/bin/bash

# ColdVox CI Performance Monitoring Script
# Tracks build times, resource usage, and job outcomes for self-hosted runner

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_DIR="$SCRIPT_DIR/../logs/performance"
TIMESTAMP=$(date '+%Y%m%d_%H%M%S')
MONITOR_LOG="$LOG_DIR/performance_${TIMESTAMP}.log"

# Create log directory
mkdir -p "$LOG_DIR"

# Configuration
RUNNER_SERVICE="actions.runner.Coldaine-ColdVox.laptop-extra.service"
GITHUB_REPO="Coldaine/ColdVox"
SAMPLE_INTERVAL=5  # seconds
MAX_RUNTIME=3600   # 1 hour max monitoring

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$MONITOR_LOG"
}

get_system_metrics() {
    local load_avg memory_usage disk_usage
    
    # CPU usage (1-minute average from /proc/loadavg)
    load_avg=$(cut -d' ' -f1 /proc/loadavg)
    
    # Memory usage in MB
    memory_usage=$(free -m | awk '/^Mem:/ {printf "%.1f", $3}')
    
    # Disk usage for workspace
    disk_usage=$(df /home/coldaine/actions-runner/_work 2>/dev/null | awk 'NR==2 {print $5}' | sed 's/%//')
    
    # Runner process CPU and memory
    local runner_pid runner_cpu runner_mem
    runner_pid=$(pgrep -f "Runner.Listener" || echo "")
    if [[ -n "$runner_pid" ]]; then
        runner_stats=$(ps -p "$runner_pid" -o %cpu,%mem --no-headers 2>/dev/null || echo "0.0 0.0")
        runner_cpu=$(echo "$runner_stats" | awk '{print $1}')
        runner_mem=$(echo "$runner_stats" | awk '{print $2}')
    else
        runner_cpu="0.0"
        runner_mem="0.0"
    fi
    
    echo "$load_avg,$memory_usage,$disk_usage,$runner_cpu,$runner_mem"
}

get_runner_status() {
    if systemctl is-active "$RUNNER_SERVICE" >/dev/null 2>&1; then
        echo "active"
    else
        echo "inactive"
    fi
}

get_workflow_runs() {
    # Get recent workflow runs with timing data
    gh run list \
        --repo "$GITHUB_REPO" \
        --limit 5 \
        --json status,conclusion,createdAt,updatedAt,name,workflowName,databaseId \
        2>/dev/null || echo "[]"
}

calculate_duration() {
    local start_time="$1"
    local end_time="$2"
    
    # Convert ISO timestamps to seconds since epoch
    local start_epoch end_epoch duration
    start_epoch=$(date -d "$start_time" +%s 2>/dev/null || echo "0")
    end_epoch=$(date -d "$end_time" +%s 2>/dev/null || echo "0")
    
    if [[ $start_epoch -gt 0 && $end_epoch -gt 0 ]]; then
        duration=$((end_epoch - start_epoch))
        echo "$duration"
    else
        echo "0"
    fi
}

monitor_workflows() {
    log "Starting workflow performance monitoring..."
    log "Monitor log: $MONITOR_LOG"
    log "Sample interval: ${SAMPLE_INTERVAL}s, Max runtime: ${MAX_RUNTIME}s"
    
    # CSV header
    echo "timestamp,load_avg,memory_mb,disk_percent,runner_cpu_percent,runner_mem_percent,runner_status" >> "$MONITOR_LOG"
    
    local start_time=$(date +%s)
    local sample_count=0
    
    while [[ $(($(date +%s) - start_time)) -lt $MAX_RUNTIME ]]; do
        local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
        local metrics=$(get_system_metrics)
        local runner_status=$(get_runner_status)
        
        # Log metrics in CSV format
        echo "$timestamp,$metrics,$runner_status" >> "$MONITOR_LOG"
        
        # Progress update every 10 samples
        ((sample_count++))
        if [[ $((sample_count % 10)) -eq 0 ]]; then
            log "Sample $sample_count: Load ${metrics#*,}"
        fi
        
        sleep "$SAMPLE_INTERVAL"
    done
    
    log "Monitoring completed after $sample_count samples"
}

analyze_recent_performance() {
    log "Analyzing recent workflow performance..."
    
    local workflows
    workflows=$(get_workflow_runs)
    
    if [[ "$workflows" == "[]" ]]; then
        log "No recent workflow data available"
        return 1
    fi
    
    # Parse and analyze workflow data
    echo "$workflows" | jq -r '.[] | 
        select(.status != null and .createdAt != null) |
        [.workflowName, .status, .conclusion, .createdAt, .updatedAt, .databaseId] | 
        @csv' | while IFS=',' read -r workflow status conclusion created updated run_id; do
        
        # Remove quotes from jq CSV output
        workflow=${workflow//\"/}
        status=${status//\"/}
        conclusion=${conclusion//\"/}
        created=${created//\"/}
        updated=${updated//\"/}
        run_id=${run_id//\"/}
        
        if [[ "$status" == "completed" && -n "$updated" ]]; then
            local duration
            duration=$(calculate_duration "$created" "$updated")
            log "WORKFLOW: $workflow | Duration: ${duration}s | Result: $conclusion | Run: $run_id"
        else
            log "WORKFLOW: $workflow | Status: $status | Run: $run_id"
        fi
    done
}

generate_performance_report() {
    log "Generating performance report..."
    
    local report_file="$LOG_DIR/performance_report_${TIMESTAMP}.md"
    
    cat > "$report_file" << EOF
# ColdVox Self-Hosted Runner Performance Report

**Generated**: $(date)
**Monitoring Period**: $TIMESTAMP
**Runner**: laptop-extra (Nobara Linux 42)

## System Configuration
- **CPU**: 13th Gen Intel Core i7-1365U (10 cores, 12 threads)
- **RAM**: 30GB
- **Storage**: 238.5GB NVMe SSD
- **OS**: Nobara Linux 42 (Fedora-based)

## Workflow Analysis
EOF
    
    # Add workflow performance data
    analyze_recent_performance >> "$report_file"
    
    # Add system metrics summary
    if [[ -f "$MONITOR_LOG" ]]; then
        echo "" >> "$report_file"
        echo "## Resource Utilization Summary" >> "$report_file"
        echo "" >> "$report_file"
        
        # Calculate averages from monitoring data
        local avg_load avg_memory avg_disk
        avg_load=$(tail -n +2 "$MONITOR_LOG" | awk -F',' '{sum+=$2; count++} END {if(count>0) printf "%.2f", sum/count; else print "N/A"}')
        avg_memory=$(tail -n +2 "$MONITOR_LOG" | awk -F',' '{sum+=$3; count++} END {if(count>0) printf "%.1f", sum/count; else print "N/A"}')
        avg_disk=$(tail -n +2 "$MONITOR_LOG" | awk -F',' '{sum+=$4; count++} END {if(count>0) printf "%.1f", sum/count; else print "N/A"}')
        
        cat >> "$report_file" << EOF
- **Average Load**: $avg_load
- **Average Memory Usage**: ${avg_memory}MB
- **Average Disk Usage**: ${avg_disk}%

## Raw Monitoring Data
See: \`$MONITOR_LOG\`

## Recommendations
EOF
        
        # Add performance recommendations based on data
        if (( $(echo "$avg_load > 8.0" | bc -l 2>/dev/null || echo 0) )); then
            echo "- ⚠️  **High CPU Load**: Consider enabling parallel job limits" >> "$report_file"
        fi
        
        if [[ "$avg_memory" != "N/A" ]] && (( $(echo "$avg_memory > 20000" | bc -l 2>/dev/null || echo 0) )); then
            echo "- ⚠️  **High Memory Usage**: Monitor for memory leaks in long-running jobs" >> "$report_file"
        fi
        
        if [[ "$avg_disk" != "N/A" ]] && (( $(echo "$avg_disk > 70" | bc -l 2>/dev/null || echo 0) )); then
            echo "- ⚠️  **High Disk Usage**: Implement aggressive workspace cleanup" >> "$report_file"
        fi
    fi
    
    log "Performance report generated: $report_file"
    echo "$report_file"
}

main() {
    case "${1:-monitor}" in
        "monitor")
            monitor_workflows
            generate_performance_report
            ;;
        "analyze")
            analyze_recent_performance
            ;;
        "report")
            generate_performance_report
            ;;
        *)
            echo "Usage: $0 [monitor|analyze|report]"
            echo "  monitor  - Start continuous monitoring (default)"
            echo "  analyze  - Analyze recent workflow performance"  
            echo "  report   - Generate performance report"
            exit 1
            ;;
    esac
}

# Trap for cleanup
trap 'log "Monitoring interrupted"; exit 1' INT TERM

main "$@"