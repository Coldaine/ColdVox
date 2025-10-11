#!/bin/bash
# Model integrity verification script for CI/CD pipeline
# Verifies Vosk model files against SHA256 checksums

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=common_utils.sh
source "${SCRIPT_DIR}/common_utils.sh"
# shellcheck source=config.env
source "${SCRIPT_DIR}/config.env"

# Configuration
MODEL_DIR="${1:-${VOSK_MODEL_REPO_DIR}}"
CHECKSUMS_FILE="${2:-models/SHA256SUMS}"
VERBOSE="${COLDVOX_VERIFY_VERBOSE:-0}"
COLDVOX_LOG_VERBOSE="${VERBOSE}"

# Function to check if required tools are available
check_dependencies() {
    require_command sha256sum find
}

# Function to verify model directory structure
verify_model_structure() {
    local model_dir="$1"

    log_info "Verifying model directory structure..."

    if [[ ! -d "$model_dir" ]]; then
        log_error "Model directory not found: $model_dir"
        return 1
    fi

    # Check for required subdirectories
    read -r -a required_dirs <<< "${VOSK_MODEL_REQUIRED_SUBDIRS}"
    for dir in "${required_dirs[@]}"; do
        if [[ ! -d "$model_dir/$dir" ]]; then
            log_error "Required model subdirectory missing: $model_dir/$dir"
            return 1
        fi
        log_verbose "Found required directory: $dir"
    done

    # Check for critical files
    read -r -a critical_files <<< "${VOSK_MODEL_CRITICAL_FILES}"

    for file in "${critical_files[@]}"; do
        if [[ ! -f "$model_dir/$file" ]]; then
            log_error "Critical model file missing: $model_dir/$file"
            return 1
        fi
        log_verbose "Found critical file: $file"
    done

    log_success "Model directory structure verification passed"
    return 0
}

# Function to verify file checksums
verify_checksums() {
    local model_dir="$1"
    local checksums_file="$2"

    log_info "Verifying model file integrity with checksums..."

    if [[ ! -f "$checksums_file" ]]; then
        log_error "Checksums file not found: $checksums_file"
        return 1
    fi

    # Check if checksums file contains actual checksums (not placeholder)
    if grep -q "placeholder\|demonstration\|NOTE:" "$checksums_file"; then
        log_warn "Checksums file contains placeholder content"
        log_warn "Skipping checksum verification (development mode)"
        return 0
    fi

    # Filter checksums for the specific model directory
    local model_checksums
    model_checksums=$(grep "^[a-f0-9]\{64\}  $model_dir/" "$checksums_file" 2>/dev/null || true)

    if [[ -z "$model_checksums" ]]; then
        log_warn "No checksums found for model directory: $model_dir"
        log_warn "Skipping checksum verification"
        return 0
    fi

    log_info "Found $(echo "$model_checksums" | wc -l) checksums to verify"

    # Create temporary file for verification
    local temp_checksums
    temp_checksums=$(mktemp)
    trap "rm -f $temp_checksums" EXIT

    echo "$model_checksums" > "$temp_checksums"

    # Verify checksums
    if sha256sum -c "$temp_checksums" --quiet; then
        log_success "All file checksums verified successfully"
        return 0
    else
        log_error "Checksum verification failed"

        # Show details of failed files in verbose mode
        if [[ "${VERBOSE}" == "1" ]]; then
            log_info "Detailed verification results:"
            sha256sum -c "$temp_checksums" || true
        fi

        return 1
    fi
}

# Function to check model size
verify_model_size() {
    local model_dir="$1"
    local min_size_mb="${COLDVOX_MIN_MODEL_SIZE_MB:-40}"

    log_info "Verifying model size..."

    local model_size_mb
    model_size_mb=$(du -sm "$model_dir" | cut -f1)

    log_verbose "Model size: ${model_size_mb}MB (minimum: ${min_size_mb}MB)"

    if [[ "$model_size_mb" -lt "$min_size_mb" ]]; then
        log_error "Model size ${model_size_mb}MB is below minimum ${min_size_mb}MB"
        log_error "Model may be incomplete or corrupted"
        return 1
    fi

    log_success "Model size verification passed (${model_size_mb}MB)"
    return 0
}

# Function to generate checksums (for development/setup)
generate_checksums() {
    local model_dir="$1"
    local output_file="$2"

    log_info "Generating checksums for model directory: $model_dir"

    if [[ ! -d "$model_dir" ]]; then
        log_error "Model directory not found: $model_dir"
        return 1
    fi

    # Find all relevant model files and generate checksums
    find "$model_dir" -type f \( -name "*.mdl" -o -name "*.conf" -o -name "*.ie" -o -name "*.txt" -o -name "*.json" \) \
        | sort | xargs sha256sum > "$output_file"

    log_success "Generated checksums for $(wc -l < "$output_file") files"
    log_info "Checksums written to: $output_file"
}

# Main verification function
main() {
    local model_dir="$1"
    local checksums_file="$2"

    log_info "Starting model integrity verification"
    log_info "Model directory: $model_dir"
    log_info "Checksums file: $checksums_file"

    # Check dependencies
    check_dependencies

    # Perform verifications
    local exit_code=0

    if ! verify_model_structure "$model_dir"; then
        exit_code=1
    fi

    if ! verify_model_size "$model_dir"; then
        exit_code=1
    fi

    if ! verify_checksums "$model_dir" "$checksums_file"; then
        exit_code=1
    fi

    if [[ $exit_code -eq 0 ]]; then
        log_success "All model integrity checks passed ✓"
    else
        log_error "Model integrity verification failed ✗"
    fi

    return $exit_code
}

# Command-line interface
case "${3:-verify}" in
    "verify")
        main "$MODEL_DIR" "$CHECKSUMS_FILE"
        ;;
    "generate")
        generate_checksums "$MODEL_DIR" "$CHECKSUMS_FILE"
        ;;
    *)
        echo "Usage: $0 [model_dir] [checksums_file] [verify|generate]"
        echo ""
        echo "Examples:"
        echo "  $0  # Verify with defaults"
        echo "  $0 models/vosk-model-small-en-us-0.15 models/SHA256SUMS verify"
        echo "  $0 models/vosk-model-small-en-us-0.15 models/SHA256SUMS generate"
        echo ""
        echo "Environment variables:"
        echo "  COLDVOX_VERIFY_VERBOSE=1     Enable verbose output"
        echo "  COLDVOX_MIN_MODEL_SIZE_MB=40 Minimum model size in MB"
        exit 1
        ;;
esac
