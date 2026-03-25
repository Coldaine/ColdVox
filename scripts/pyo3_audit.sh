#!/bin/bash
# PyO3 Dependency Audit Script for ColdVox
# This script automates the dependency audit process outlined in PYO3_DEPENDENCY_AUDIT_PLAN.md
#
# Usage: ./scripts/pyo3_audit.sh [--phase PHASE] [--output FILE]
#
# Options:
#   --phase PHASE    Run specific phase (1-6, or 'all')
#   --output FILE    Output file for report (default: audit_report.md)
#   --verbose        Enable verbose output
#   --help           Show this help message

set -euo pipefail

# Default values
PHASE="all"
OUTPUT_FILE="audit_report.md"
VERBOSE=false
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_verbose() {
    if [ "$VERBOSE" = true ]; then
        echo -e "${NC}[VERBOSE]${NC} $1"
    fi
}

# Help message
show_help() {
    cat << EOF
PyO3 Dependency Audit Script for ColdVox

Usage: $0 [OPTIONS]

Options:
    --phase PHASE    Run specific phase (1-6, or 'all')
                    1: Environment Snapshot
                    2: Dependency Tree Analysis
                    3: DLL & Shared Library Mapping
                    4: PyO3 Environment Checks
                    5: DLL_NOT_FOUND Troubleshooting
                    6: Generate Report
                    all: Run all phases (default)
    
    --output FILE    Output file for report (default: audit_report.md)
    --verbose        Enable verbose output
    --help           Show this help message

Examples:
    $0                          # Run all phases
    $0 --phase 1                # Run only phase 1
    $0 --phase 1-3              # Run phases 1-3
    $0 --output my_report.md    # Custom output file

EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --phase)
            PHASE="$2"
            shift 2
            ;;
        --output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            show_help
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Initialize report
init_report() {
    log_info "Initializing audit report: $OUTPUT_FILE"
    cat > "$OUTPUT_FILE" << EOF
# ColdVox PyO3 Dependency Audit Report

**Date:** $(date '+%Y-%m-%d %H:%M:%S')
**Auditor:** Automated Audit Script
**Environment:** $(uname -s) $(uname -r)

---

EOF
}

# Phase 1: Environment Snapshot
phase1() {
    log_info "Phase 1: Capturing environment snapshot..."
    
    cat >> "$OUTPUT_FILE" << EOF
## 1. Environment Snapshot

### Python Environment
EOF

    # Python version and architecture
    log_verbose "Checking Python version..."
    if command -v python &> /dev/null; then
        python --version >> "$OUTPUT_FILE" 2>&1
        echo "" >> "$OUTPUT_FILE"
        
        python -c "
import sys
print(f'Executable: {sys.executable}')
print(f'Version: {sys.version}')
print(f'Architecture: {\"64-bit\" if sys.maxsize > 2**32 else \"32-bit\"}')
print(f'Platform: {sys.platform}')
" >> "$OUTPUT_FILE" 2>&1
    else
        echo "Python not found in PATH" >> "$OUTPUT_FILE"
    fi

    # Virtual environment detection
    cat >> "$OUTPUT_FILE" << EOF

### Virtual Environment
EOF
    python -c "
import sys
print(f'Prefix: {sys.prefix}')
print(f'Base Prefix: {sys.base_prefix}')
print(f'In venv: {sys.prefix != sys.base_prefix}')
" >> "$OUTPUT_FILE" 2>&1 || echo "Could not detect virtual environment" >> "$OUTPUT_FILE"

    # Environment variables
    cat >> "$OUTPUT_FILE" << EOF

### Environment Variables
EOF
    echo "PYTHONHOME: \${PYTHONHOME:-<unset>}" >> "$OUTPUT_FILE"
    echo "PYTHONPATH: \${PYTHONPATH:-<unset>}" >> "$OUTPUT_FILE"
    echo "VIRTUAL_ENV: \${VIRTUAL_ENV:-<unset>}" >> "$OUTPUT_FILE"

    # Python packages
    cat >> "$OUTPUT_FILE" << EOF

### Installed Python Packages
EOF
    if command -v uv &> /dev/null; then
        log_verbose "Using uv to list packages..."
        uv pip list >> "$OUTPUT_FILE" 2>&1 || echo "uv pip list failed" >> "$OUTPUT_FILE"
    elif command -v pip &> /dev/null; then
        log_verbose "Using pip to list packages..."
        pip list >> "$OUTPUT_FILE" 2>&1 || echo "pip list failed" >> "$OUTPUT_FILE"
    else
        echo "No package manager found" >> "$OUTPUT_FILE"
    fi

    # Rust toolchain
    cat >> "$OUTPUT_FILE" << EOF

### Rust Toolchain
EOF
    if command -v rustc &> /dev/null; then
        rustc --version >> "$OUTPUT_FILE" 2>&1
        cargo --version >> "$OUTPUT_FILE" 2>&1
    else
        echo "Rust not found in PATH" >> "$OUTPUT_FILE"
    fi

    # System information
    cat >> "$OUTPUT_FILE" << EOF

### System Information
EOF
    uname -a >> "$OUTPUT_FILE" 2>&1
    
    log_success "Phase 1 completed"
}

# Phase 2: Dependency Tree Analysis
phase2() {
    log_info "Phase 2: Analyzing dependency tree..."
    
    cat >> "$OUTPUT_FILE" << EOF

---

## 2. Dependency Tree Analysis

### Python Dependency Tree
EOF

    # Install pipdeptree if not present
    if ! command -v pipdeptree &> /dev/null; then
        log_verbose "Installing pipdeptree..."
        if command -v uv &> /dev/null; then
            uv pip install pipdeptree >> "$OUTPUT_FILE" 2>&1
        else
            pip install pipdeptree >> "$OUTPUT_FILE" 2>&1
        fi
    fi

    # Generate dependency tree
    if command -v pipdeptree &> /dev/null; then
        pipdeptree --warn silence >> "$OUTPUT_FILE" 2>&1 || echo "pipdeptree failed" >> "$OUTPUT_FILE"
    else
        echo "pipdeptree not available" >> "$OUTPUT_FILE"
    fi

    # Identify PyO3 dependencies
    cat >> "$OUTPUT_FILE" << EOF

### PyO3/Rust Dependencies
EOF
    if [ -f "$PROJECT_ROOT/Cargo.lock" ]; then
        grep -i "pyo3" "$PROJECT_ROOT/Cargo.lock" >> "$OUTPUT_FILE" 2>&1 || echo "No PyO3 found in Cargo.lock" >> "$OUTPUT_FILE"
    else
        echo "Cargo.lock not found" >> "$OUTPUT_FILE"
    fi

    # List Python packages with native extensions
    cat >> "$OUTPUT_FILE" << EOF

### Packages with Native Extensions
EOF
    python -c "
import pkg_resources
native_indicators = ['numpy', 'torch', 'scipy', 'librosa', 'transformers', 'cffi', 'pycparser']
for pkg in pkg_resources.working_set:
    if any(ind in pkg.project_name.lower() for ind in native_indicators):
        print(f'{pkg.project_name} {pkg.version}')
" >> "$OUTPUT_FILE" 2>&1 || echo "Could not identify native packages" >> "$OUTPUT_FILE"

    log_success "Phase 2 completed"
}

# Phase 3: DLL & Shared Library Mapping
phase3() {
    log_info "Phase 3: Mapping native libraries..."
    
    cat >> "$OUTPUT_FILE" << EOF

---

## 3. Native Library Mapping

### Python Extension Modules
EOF

    # Find Python extension modules
    PYTHON_PREFIX=$(python -c "import sys; print(sys.prefix)" 2>/dev/null || echo "")
    if [ -n "$PYTHON_PREFIX" ]; then
        log_verbose "Searching for native libraries in: $PYTHON_PREFIX"
        
        case "$(uname -s)" in
            Linux*)
                find "$PYTHON_PREFIX" -name "*.so" 2>/dev/null | head -50 >> "$OUTPUT_FILE" || echo "No .so files found" >> "$OUTPUT_FILE"
                ;;
            Darwin*)
                find "$PYTHON_PREFIX" -name "*.dylib" 2>/dev/null | head -50 >> "$OUTPUT_FILE" || echo "No .dylib files found" >> "$OUTPUT_FILE"
                ;;
            CYGWIN*|MINGW*|MSYS*)
                find "$PYTHON_PREFIX" -name "*.dll" -o -name "*.pyd" 2>/dev/null | head -50 >> "$OUTPUT_FILE" || echo "No .dll/.pyd files found" >> "$OUTPUT_FILE"
                ;;
        esac
    else
        echo "Could not determine Python prefix" >> "$OUTPUT_FILE"
    fi

    # Check Rust build output
    cat >> "$OUTPUT_FILE" << EOF

### Rust/PyO3 Build Output
EOF
    if [ -d "$PROJECT_ROOT/target" ]; then
        find "$PROJECT_ROOT/target" -name "*.pyd" -o -name "*.so" -o -name "*.dylib" 2>/dev/null | head -20 >> "$OUTPUT_FILE" || echo "No PyO3 libraries found" >> "$OUTPUT_FILE"
    else
        echo "target directory not found" >> "$OUTPUT_FILE"
    fi

    log_success "Phase 3 completed"
}

# Phase 4: PyO3 Environment Checks
phase4() {
    log_info "Phase 4: Verifying PyO3 environment..."
    
    cat >> "$OUTPUT_FILE" << EOF

---

## 4. PyO3 Environment Verification

### Python Interpreter Consistency
EOF

    # Create temporary Python script
    TEMP_SCRIPT=$(mktemp)
    cat > "$TEMP_SCRIPT" << 'PYTHON_SCRIPT'
import sys
import os

print("Executable:", sys.executable)
print("Version:", sys.version)
print("Version Info:", sys.version_info)
print("Platform:", sys.platform)
print("Architecture:", "64-bit" if sys.maxsize > 2**32 else "32-bit")

print("\n=== Environment Variables ===")
print("PYTHONHOME:", os.environ.get('PYTHONHOME', '<unset>'))
print("PYTHONPATH:", os.environ.get('PYTHONPATH', '<unset>'))
print("VIRTUAL_ENV:", os.environ.get('VIRTUAL_ENV', '<unset>'))

print("\n=== Path Analysis ===")
python_exe = sys.executable
python_prefix = sys.prefix
print("Python executable:", python_exe)
print("Python prefix:", python_prefix)

# Check for multiple Python installations
import subprocess
try:
    result = subprocess.run(['where', 'python'] if sys.platform == 'win32' else ['which', '-a', 'python'], 
                          capture_output=True, text=True)
    print("\n=== All Python executables in PATH ===")
    print(result.stdout)
except Exception as e:
    print(f"Could not check PATH: {e}")
PYTHON_SCRIPT

    python "$TEMP_SCRIPT" >> "$OUTPUT_FILE" 2>&1 || echo "Python environment check failed" >> "$OUTPUT_FILE"
    rm -f "$TEMP_SCRIPT"

    # Check PYTHONHOME
    cat >> "$OUTPUT_FILE" << EOF

### PYTHONHOME Check
EOF
    if [ -n "${PYTHONHOME:-}" ]; then
        echo "WARNING: PYTHONHOME is set to: $PYTHONHOME" >> "$OUTPUT_FILE"
        echo "PyO3 may fail to initialize Python correctly." >> "$OUTPUT_FILE"
        echo "Recommendation: unset PYTHONHOME" >> "$OUTPUT_FILE"
    else
        echo "PYTHONHOME is not set (good)" >> "$OUTPUT_FILE"
    fi

    log_success "Phase 4 completed"
}

# Phase 5: DLL_NOT_FOUND Troubleshooting
phase5() {
    log_info "Phase 5: Troubleshooting DLL_NOT_FOUND..."
    
    cat >> "$OUTPUT_FILE" << EOF

---

## 5. DLL_NOT_FOUND Troubleshooting

### Missing Library Detection
EOF

    # Check for missing dependencies
    case "$(uname -s)" in
        Linux*)
            log_verbose "Checking for missing .so dependencies..."
            if command -v ldd &> /dev/null; then
                # Check Python executable
                echo "Python executable dependencies:" >> "$OUTPUT_FILE"
                ldd "$(which python)" 2>&1 | grep "not found" >> "$OUTPUT_FILE" || echo "No missing dependencies for Python" >> "$OUTPUT_FILE"
                
                # Check critical Python packages
                echo "" >> "$OUTPUT_FILE"
                echo "Critical package dependencies:" >> "$OUTPUT_FILE"
                for pkg in torch numpy scipy; do
                    PKG_PATH=$(python -c "import $pkg; print($pkg.__file__)" 2>/dev/null | sed 's/__init__.py//')
                    if [ -n "$PKG_PATH" ]; then
                        find "$PKG_PATH" -name "*.so" 2>/dev/null | while read lib; do
                            MISSING=$(ldd "$lib" 2>&1 | grep "not found" || true)
                            if [ -n "$MISSING" ]; then
                                echo "  $lib:" >> "$OUTPUT_FILE"
                                echo "$MISSING" >> "$OUTPUT_FILE"
                            fi
                        done
                    fi
                done
            else
                echo "ldd not available" >> "$OUTPUT_FILE"
            fi
            ;;
        Darwin*)
            log_verbose "Checking for missing .dylib dependencies..."
            if command -v otool &> /dev/null; then
                echo "Python executable dependencies:" >> "$OUTPUT_FILE"
                otool -L "$(which python)" 2>&1 | grep "not found" >> "$OUTPUT_FILE" || echo "No missing dependencies for Python" >> "$OUTPUT_FILE"
            else
                echo "otool not available" >> "$OUTPUT_FILE"
            fi
            ;;
        CYGWIN*|MINGW*|MSYS*)
            log_verbose "Windows detected - manual DLL check required"
            echo "Windows: Use Process Monitor or Dependency Walker for DLL analysis" >> "$OUTPUT_FILE"
            echo "See PYO3_DEPENDENCY_AUDIT_PLAN.md Phase 5 for instructions" >> "$OUTPUT_FILE"
            ;;
    esac

    # Check system PATH
    cat >> "$OUTPUT_FILE" << EOF

### System PATH Analysis
EOF
    echo "$PATH" | tr ':' '\n' | grep -i -E "(python|torch|cuda|msvc)" >> "$OUTPUT_FILE" 2>&1 || echo "No relevant paths found" >> "$OUTPUT_FILE"

    log_success "Phase 5 completed"
}

# Phase 6: Generate Report
phase6() {
    log_info "Phase 6: Generating final report..."
    
    cat >> "$OUTPUT_FILE" << EOF

---

## 6. Summary and Recommendations

### Critical Issues Found
[Review the sections above and list critical issues here]

### Recommended Actions
1. **Verify Python Environment**
   - Ensure PYTHONHOME is unset
   - Use uv to manage Python dependencies
   - Verify Python 3.10-3.12 is used

2. **Check Native Dependencies**
   - Install Visual C++ Redistributables (Windows)
   - Verify CUDA libraries (if using GPU)
   - Check for missing .so/.dll files

3. **Rebuild if Necessary**
   \`\`\`bash
   cargo clean -p coldvox-stt
   cargo build -p coldvox-stt --features moonshine
   \`\`\`

4. **Verify Installation**
   \`\`\`bash
   uv sync
   cargo check -p coldvox-stt --features moonshine
   \`\`\`

### Verification Commands
\`\`\`bash
# Check Python environment
uv run python -c "import sys; print(sys.executable)"

# Check PyTorch
uv run python -c "import torch; print(f'PyTorch {torch.__version__}')"

# Check ColdVox STT
cargo check -p coldvox-stt --features moonshine
\`\`\`

---

*Report generated by pyo3_audit.sh on $(date '+%Y-%m-%d %H:%M:%S')*
EOF

    log_success "Phase 6 completed"
    log_success "Report saved to: $OUTPUT_FILE"
}

# Main execution
main() {
    log_info "Starting ColdVox PyO3 Dependency Audit"
    log_info "Project root: $PROJECT_ROOT"
    log_info "Output file: $OUTPUT_FILE"
    log_info "Phase: $PHASE"
    
    cd "$PROJECT_ROOT"
    
    # Initialize report
    init_report
    
    # Run requested phases
    case "$PHASE" in
        all)
            phase1
            phase2
            phase3
            phase4
            phase5
            phase6
            ;;
        1)
            phase1
            ;;
        2)
            phase2
            ;;
        3)
            phase3
            ;;
        4)
            phase4
            ;;
        5)
            phase5
            ;;
        6)
            phase6
            ;;
        *)
            # Handle range like 1-3
            if [[ "$PHASE" =~ ^([0-9]+)-([0-9]+)$ ]]; then
                START="${BASH_REMATCH[1]}"
                END="${BASH_REMATCH[2]}"
                for ((i=START; i<=END; i++)); do
                    phase$i
                done
            else
                log_error "Invalid phase: $PHASE"
                show_help
                exit 1
            fi
            ;;
    esac
    
    log_success "Audit completed successfully!"
    log_info "Review the report at: $OUTPUT_FILE"
}

# Run main function
main
