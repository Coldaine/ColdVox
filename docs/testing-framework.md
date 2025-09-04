# ColdVox Feature Testing Framework

## Overview

ColdVox uses a comprehensive feature testing framework to systematically test all possible feature combinations across the Rust workspace. This framework is designed to identify feature-gated bugs, platform-specific issues, and conditional compilation problems that standard CI might miss.

The testing framework consists of:
1. **Python test runner script** (`test-features.py`) - Automated local testing tool
2. **GitHub Actions workflow** (`.github/workflows/feature-matrix.yml`) - CI/CD integration
3. **Curated test strategies** - Intelligently selected feature combinations

## Problem Statement

ColdVox is a complex Rust workspace with multiple crates that use feature flags extensively:

- **VAD engines**: `silero` (default), `level3` (energy-based)
- **STT backends**: `vosk` for offline speech recognition
- **Text injection**: Platform-specific backends (`atspi`, `wl_clipboard`, `enigo`, etc.)
- **Platform detection**: Automatic feature selection based on OS and desktop environment

Without systematic testing, bugs can hide in rarely-used feature combinations or platform-specific code paths that aren't covered by standard CI.

## Architecture

### Test Runner Script (`test-features.py`)

The core of the framework is a Python script that:

- **Dynamically discovers features** by parsing `Cargo.toml` files
- **Implements multiple testing strategies** from simple to exhaustive
- **Provides clear output** with success/failure status for each combination
- **Aggregates results** with comprehensive summaries
- **Supports JSON output** for CI integration and analysis

#### Testing Strategies

1. **`default-only`**: Test only the default feature set
2. **`no-default`**: Test with all default features disabled
3. **`each-feature`**: Test each feature individually (with defaults disabled)
4. **`curated`** (recommended): Hand-picked combinations most likely to reveal issues
5. **`powerset`**: Test all possible combinations (expensive, for comprehensive testing)

#### Curated Combinations

The curated strategy includes intelligently selected feature combinations for each crate:

**coldvox-app**:
- Core combinations: `[]`, `[silero]`, `[level3]`, `[vosk]`, `[text-injection]`
- Integration pairs: `[silero,vosk]`, `[level3,text-injection]`
- Platform-specific: `[text-injection-atspi]`, `[text-injection-enigo]`
- Full stack: `[silero,vosk,text-injection]`

**coldvox-text-injection**:
- Backend isolation: `[atspi]`, `[wl_clipboard]`, `[enigo]`, `[ydotool]`
- Backend combinations: `[atspi,wl_clipboard]`, `[enigo,mki]`
- Convenience features: `[all-backends]`, `[linux-desktop]`

**coldvox-vad**:
- Feature variants: `[]` (none), `[level3]` (energy-based VAD)

### GitHub Actions Workflow

The CI/CD workflow provides three types of testing:

#### 1. Feature Matrix Testing
- **Multi-platform**: Tests on Ubuntu, Windows, and macOS
- **Multi-package**: Tests core crates (`coldvox-app`, `coldvox-vad`, `coldvox-text-injection`)
- **Curated combinations**: Focused on high-value feature combinations
- **Platform-specific exclusions**: Intelligently excludes irrelevant combinations

#### 2. Comprehensive Testing
- **Python script integration**: Uses `test-features.py` for exhaustive testing
- **Manual trigger support**: Can be triggered via GitHub UI with strategy selection
- **Result artifacts**: Uploads JSON results for analysis
- **Summary reports**: Generates test summaries in GitHub step output

#### 3. Smoke Testing
- **PR validation**: Quick tests on pull requests
- **Format checking**: Ensures code formatting compliance
- **Basic builds**: Tests default and no-default configurations
- **Lint validation**: Runs clippy for code quality

## Usage Guide

### Local Testing

#### Prerequisites
```bash
# Install Python dependencies (Python 3.11+ recommended)
pip install tomllib  # Built into Python 3.11+

# Ensure you're in the workspace root
cd /path/to/ColdVox
```

#### Basic Usage
```bash
# Make script executable
chmod +x test-features.py

# Test all feature combinations for the main app (curated strategy)
./test-features.py -p coldvox-app

# Test each feature individually
./test-features.py -p coldvox-vad --strategy each-feature

# Test specific package with verbose output
./test-features.py -p coldvox-text-injection --strategy curated --verbose

# Test entire workspace with powerset strategy (warning: slow!)
./test-features.py --strategy powerset

# Save results to JSON file
./test-features.py -p coldvox-app --json-output results.json
```

#### Advanced Examples
```bash
# Test with additional cargo test arguments
./test-features.py -p coldvox-app -- --test integration

# Test specific test pattern
./test-features.py -p coldvox-app -- --test "*vad*"

# Test with timeout and specific test binary
./test-features.py -p coldvox-app -- --bin tui_dashboard --lib
```

### CI/CD Integration

#### Automatic Triggers
- **Push to main**: Runs comprehensive testing + feature matrix
- **Pull requests**: Runs quick smoke tests only
- **Scheduled**: Weekly comprehensive testing (can be configured)

#### Manual Triggers
1. Go to **Actions** tab in GitHub repository
2. Select **Feature Matrix Testing** workflow
3. Click **Run workflow**
4. Choose testing strategy and options
5. Monitor results in the workflow output

#### Reading Results
- **Matrix view**: See results for each OS/package/feature combination
- **Artifacts**: Download JSON results for detailed analysis
- **Step summary**: View aggregated test results and failure summaries

## Configuration

### Adding New Features
When adding new features to any crate:

1. **Update curated combinations** in `test-features.py`:
   ```python
   curated = {
       "your-crate-name": [
           [],  # Always test with no features
           ["new-feature"],
           ["new-feature", "existing-feature"],
           # ... other combinations
       ]
   }
   ```

2. **Update CI matrix** if needed in `.github/workflows/feature-matrix.yml`:
   ```yaml
   features:
     - name: "new-feature"
       flags: "--no-default-features --features new-feature"
   ```

### Platform-Specific Testing
For platform-specific features:

1. **Add platform exclusions** in the CI workflow
2. **Update curated combinations** to include platform-specific variants
3. **Test locally** on target platforms before pushing

### Performance Tuning
- **Curated strategy**: ~10-20 combinations per crate (fast, recommended)
- **Each-feature**: 1 + number of features (moderate)
- **Powerset**: 2^n combinations (slow, use sparingly)

## Best Practices

### Development Workflow
1. **Before implementing features**: Run curated tests to establish baseline
2. **During development**: Test specific packages with relevant strategies
3. **Before PR**: Run comprehensive tests locally if making feature changes
4. **PR review**: Check CI results for any new failures

### Debugging Feature Issues
1. **Identify failing combination**: Check test output for specific feature flags
2. **Reproduce locally**: Use exact command from test output
3. **Isolate the issue**: Test subsets of the failing combination
4. **Check platform dependencies**: Verify all required libraries are available

### Maintaining Test Quality
- **Review curated combinations quarterly**: Add new important combinations
- **Monitor CI performance**: Adjust strategies if tests become too slow
- **Update exclusions**: Keep platform-specific exclusions current
- **Document feature interactions**: Note any discovered feature conflicts

## Troubleshooting

### Common Issues

#### Test Script Errors
```bash
# Permission denied
chmod +x test-features.py

# Python dependencies
pip install tomllib  # or use Python 3.11+

# Cargo.toml not found
./test-features.py --workspace-root /path/to/workspace
```

#### CI Failures
- **Timeout**: Reduce test combinations or increase timeout
- **Platform dependencies**: Check if all required system libraries are installed
- **Feature conflicts**: Some features may be mutually exclusive
- **Model downloads**: Vosk model downloads may fail; check network connectivity

#### Feature Compilation Issues
- **Missing dependencies**: Check platform-specific dependencies in Cargo.toml
- **Feature flag typos**: Verify feature names match exactly
- **Conditional compilation**: Ensure `#[cfg(feature = "...")]` attributes are correct

### Performance Optimization
- **Parallel testing**: The script runs tests sequentially; consider parallel execution for large test suites
- **Caching**: CI uses Rust dependency caching; local testing can benefit from incremental builds
- **Test filtering**: Use specific test patterns to reduce execution time during development

## Metrics and Monitoring

### Test Coverage Metrics
- **Feature combination coverage**: Track which combinations are tested
- **Platform coverage**: Ensure all supported platforms are tested
- **Failure patterns**: Monitor which combinations fail most often

### Performance Metrics
- **Test duration**: Track test execution time per strategy
- **CI resource usage**: Monitor GitHub Actions minutes consumption
- **Success rates**: Track test pass rates over time

## Future Enhancements

### Planned Improvements
1. **Parallel execution**: Run tests in parallel for faster local testing
2. **Interactive mode**: TUI for selecting specific combinations to test
3. **Regression detection**: Compare results against previous runs
4. **Integration with cargo-hack**: Leverage existing Rust tooling for feature testing
5. **Custom test filters**: More granular control over which tests to run

### Advanced Features
- **Dependency analysis**: Identify which features affect which code paths
- **Minimal reproduction**: Automatically find the smallest failing feature set
- **Performance regression**: Track build and test times across feature combinations
- **Documentation generation**: Auto-generate feature compatibility matrices

## Contributing

When contributing to the testing framework:

1. **Test your changes**: Run the framework on itself before submitting
2. **Update documentation**: Keep this document current with any changes
3. **Consider performance**: Avoid adding expensive default test combinations
4. **Maintain compatibility**: Ensure changes work across all supported platforms

### Adding New Test Strategies
To add a new testing strategy:

1. **Update `TestStrategy` enum** in `test-features.py`
2. **Implement strategy logic** in `generate_test_combinations()`
3. **Add CLI option** in argument parser
4. **Update documentation** with strategy description and use cases
5. **Test the strategy** on existing crates

This framework ensures ColdVox maintains high quality across all its feature combinations and platforms, catching issues early and providing confidence in the robustness of the codebase.
