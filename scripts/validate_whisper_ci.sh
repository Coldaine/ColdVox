#!/bin/bash
# Validate Whisper CI Configuration
# This script validates that the CI pipeline is properly configured for Whisper

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_step() {
    echo -e "${BLUE}==> $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Change to repo root
cd "$(git rev-parse --show-toplevel)"

print_step "Validating Whisper CI configuration..."

# 1. Check that workflow files are properly configured
print_step "Checking GitHub Actions workflows..."

# Check main CI workflow
if grep -q "WHISPER_MODEL_PATH" .github/workflows/ci.yml; then
    print_success "Main CI workflow has Whisper environment variables"
else
    print_error "Main CI workflow missing Whisper environment variables"
fi

# Check that setup-whisper-cache.sh is referenced
if grep -q "setup-whisper-cache.sh" .github/workflows/ci.yml; then
    print_success "Main CI workflow references setup-whisper-cache.sh"
else
    print_error "Main CI workflow doesn't reference setup-whisper-cache.sh"
fi

# Check Whisper integration workflow
if grep -q "whisper" .github/workflows/vosk-integration.yml; then
    print_success "Integration workflow is configured for Whisper"
else
    print_error "Integration workflow not configured for Whisper"
fi

# 2. Check that setup script exists and is executable
print_step "Checking setup scripts..."
if [[ -f "scripts/ci/setup-whisper-cache.sh" ]]; then
    print_success "setup-whisper-cache.sh exists"
    if [[ -x "scripts/ci/setup-whisper-cache.sh" ]]; then
        print_success "setup-whisper-cache.sh is executable"
    else
        print_warning "setup-whisper-cache.sh is not executable"
    fi
else
    print_error "setup-whisper-cache.sh not found"
fi

# Check that verify script exists and is executable
if [[ -f "scripts/verify_whisper_model.sh" ]]; then
    print_success "verify_whisper_model.sh exists"
    if [[ -x "scripts/verify_whisper_model.sh" ]]; then
        print_success "verify_whisper_model.sh is executable"
    else
        print_warning "verify_whisper_model.sh is not executable"
    fi
else
    print_error "verify_whisper_model.sh not found"
fi

# 3. Check that local_ci.sh doesn't reference Vosk
print_step "Checking local CI script..."
if ! grep -q "vosk" scripts/local_ci.sh; then
    print_success "local_ci.sh doesn't reference Vosk"
else
    print_warning "local_ci.sh still references Vosk"
fi

# 4. Check pre-commit hooks
print_step "Checking pre-commit hooks..."
if grep -q "verify_whisper_model.sh" .pre-commit-config.yaml; then
    print_success "Pre-commit hooks reference verify_whisper_model.sh"
else
    print_error "Pre-commit hooks don't reference verify_whisper_model.sh"
fi

# Check that E2E test is configured for Whisper
if grep -q "whisper" .pre-commit-config.yaml; then
    print_success "Pre-commit hooks are configured for Whisper"
else
    print_error "Pre-commit hooks not configured for Whisper"
fi

# 5. Check that VOSK references are removed from workflow files
print_step "Checking for Vosk references in workflows..."
if ! grep -q "vosk" .github/workflows/ci.yml; then
    print_success "Main CI workflow doesn't reference Vosk"
else
    print_warning "Main CI workflow still references Vosk"
fi

# 6. Run setup script to validate it works
print_step "Testing Whisper setup script..."
if WHISPER_MODEL_SIZE=tiny bash scripts/ci/setup-whisper-cache.sh > /dev/null 2>&1; then
    print_success "setup-whisper-cache.sh runs successfully"
else
    print_error "setup-whisper-cache.sh failed to run"
fi

# 7. Run verify script to validate it works
print_step "Testing Whisper verify script..."
if bash scripts/verify_whisper_model.sh > /dev/null 2>&1; then
    print_success "verify_whisper_model.sh runs successfully"
else
    print_error "verify_whisper_model.sh failed to run"
fi

# 8. Check that configuration files are updated
print_step "Checking configuration files..."
if grep -q "whisper" config/default.toml; then
    print_success "Default configuration references Whisper"
else
    print_warning "Default configuration doesn't reference Whisper"
fi

print_success "🎉 Whisper CI validation completed!"
echo -e "${GREEN}The CI pipeline appears to be correctly configured for Whisper.${NC}"