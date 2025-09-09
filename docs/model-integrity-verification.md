# Model Integrity Verification

This document describes the model integrity verification system implemented to ensure Vosk model files are complete and uncorrupted during CI/CD pipeline execution.

## Overview

The integrity verification system provides:

- **Structural validation**: Ensures required directories and files exist
- **Size validation**: Checks that model size meets minimum requirements  
- **Checksum verification**: Validates file integrity against SHA256 hashes
- **CI integration**: Automatic verification during GitHub Actions workflows
- **Development mode**: Graceful handling when checksums are placeholder/missing

## Components

### 1. Verification Script

**Location**: `scripts/verify-model-integrity.sh`

**Usage**:
```bash
# Verify with defaults
./scripts/verify-model-integrity.sh

# Verify specific model and checksums
./scripts/verify-model-integrity.sh models/vosk-model-small-en-us-0.15 models/SHA256SUMS verify

# Generate checksums for a model
./scripts/verify-model-integrity.sh models/vosk-model-small-en-us-0.15 models/SHA256SUMS generate
```

**Environment Variables**:
- `COLDVOX_VERIFY_VERBOSE=1`: Enable detailed output
- `COLDVOX_MIN_MODEL_SIZE_MB=40`: Set minimum model size threshold

### 2. Checksums File

**Location**: `models/SHA256SUMS`

Contains SHA256 hashes for all critical model files. During development, this file may contain placeholder content, which the verification script detects and handles gracefully.

### 3. CI Integration

The verification is integrated into the `download-vosk-model` job in `.github/workflows/ci.yml`:

```yaml
- name: Verify vendored model with integrity checks
  run: |
    export COLDVOX_VERIFY_VERBOSE=1
    export COLDVOX_MIN_MODEL_SIZE_MB=40
    
    if ! ./scripts/verify-model-integrity.sh; then
      echo "FATAL: Model integrity verification failed"
      exit 1
    fi
```

## Verification Process

### 1. Structure Validation

Checks for required directories and files:

**Required Directories**:
- `am/` - Acoustic model files
- `conf/` - Configuration files  
- `ivector/` - I-vector extractor files

**Critical Files**:
- `am/final.mdl` - Main acoustic model
- `conf/mfcc.conf` - MFCC configuration
- `ivector/final.ie` - I-vector extractor

### 2. Size Validation

Ensures the model directory is at least 40MB (configurable), indicating the model is complete and not just stub files.

### 3. Checksum Verification

Validates file integrity using SHA256 hashes:

- Skips verification in development mode (when checksums are placeholder)
- Provides detailed error reporting for checksum mismatches
- Only verifies files relevant to the specific model directory

## Development Workflow

### Setting up a New Model

1. **Download and extract the model**:
   ```bash
   cd models/
   wget https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip
   unzip vosk-model-small-en-us-0.15.zip
   ```

2. **Generate checksums**:
   ```bash
   ./scripts/verify-model-integrity.sh models/vosk-model-small-en-us-0.15 models/SHA256SUMS generate
   ```

3. **Verify the setup**:
   ```bash
   ./scripts/verify-model-integrity.sh
   ```

4. **Commit both model and checksums**:
   ```bash
   git add models/vosk-model-small-en-us-0.15/ models/SHA256SUMS
   git commit -m "feat: add Vosk model with integrity verification"
   ```

### Updating an Existing Model

1. **Replace model files** with new version
2. **Regenerate checksums**:
   ```bash
   ./scripts/verify-model-integrity.sh models/vosk-model-small-en-us-0.15 models/SHA256SUMS generate
   ```
3. **Verify and commit** changes

## CI/CD Behavior

### Success Cases
- ✅ All checks pass: CI continues normally
- ⚠️ Placeholder checksums detected: Warning logged, structure/size checks still performed

### Failure Cases  
- ❌ Model directory missing: Job fails with clear error message
- ❌ Required files missing: Job fails listing missing files
- ❌ Model too small: Job fails indicating possible corruption
- ❌ Checksum mismatch: Job fails with integrity error

### Error Recovery

When integrity verification fails in CI:

1. **Check the error message** for specific issues
2. **Local verification**: Run the script locally to debug
3. **Regenerate checksums** if model was intentionally updated
4. **Re-download model** if files appear corrupted

## Benefits

### For Development
- **Early detection** of model corruption or incomplete downloads
- **Consistent verification** across different environments
- **Clear error messages** for troubleshooting

### For CI/CD
- **Fail-fast behavior** prevents wasted compute on broken models
- **Detailed logging** for debugging CI issues
- **Graceful development mode** handling

### For Production
- **Model integrity assurance** before deployment
- **Reproducible builds** with verified model checksums
- **Supply chain security** through cryptographic verification

## Troubleshooting

### Common Issues

**"Model directory not found"**
- Ensure the model is committed to the repository
- Check that the model path is correct in the script

**"Checksum verification failed"** 
- Model files may be corrupted or modified
- Regenerate checksums if model was intentionally updated
- Check if files were modified by Git LFS or similar tools

**"Model size below minimum"**
- Model download may be incomplete
- Check available disk space during model extraction
- Verify the downloaded archive wasn't truncated

### Debug Commands

```bash
# Verbose verification
COLDVOX_VERIFY_VERBOSE=1 ./scripts/verify-model-integrity.sh

# Check model directory manually
ls -la models/vosk-model-small-en-us-0.15/
du -sh models/vosk-model-small-en-us-0.15/

# Verify checksums manually
cd models/vosk-model-small-en-us-0.15/
sha256sum -c ../SHA256SUMS
```

## Future Enhancements

- Support for multiple model versions
- Automatic model download with verification
- Integration with model repository/CDN
- GPG signature verification for additional security