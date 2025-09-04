# Feature Testing Framework

ColdVox includes a comprehensive testing framework for validating all feature combinations across the workspace. This framework helps identify feature-gated bugs, platform-specific issues, and conditional compilation problems.

## Quick Start

### Prerequisites
```bash
# Python 3.11+ (for tomllib support)
python3 --version

# Ensure you're in the workspace root
cd /path/to/ColdVox
```

### Running Tests Locally

```bash
# Make test script executable
chmod +x test-features.py

# Test main app with curated feature combinations (recommended)
./test-features.py -p coldvox-app

# Test VAD crate with all individual features
./test-features.py -p coldvox-vad --strategy each-feature

# Test text injection with verbose output
./test-features.py -p coldvox-text-injection --verbose

# Test entire workspace (warning: can be slow)
./test-features.py --strategy curated
```

### Testing Strategies

- **`curated`** (default): Hand-picked combinations most likely to reveal issues
- **`each-feature`**: Test each feature individually
- **`powerset`**: Test all possible combinations (expensive)
- **`default-only`**: Test only default features
- **`no-default`**: Test with no default features

### Example Output

```
================================================================================
Running: cargo test -p coldvox-app --no-default-features --features silero,vosk
================================================================================
✅ SUCCESS (took 12.34s)

[3/8] Testing combination...
================================================================================
Running: cargo test -p coldvox-app --no-default-features --features level3,text-injection
================================================================================
❌ FAILURE (took 5.67s)

TEST SUMMARY
================================================================================

Total tests: 8
✅ Passed: 7
❌ Failed: 1

FAILED COMBINATIONS
--------------------------------------------------------------------------------
  Features: level3,text-injection
  Command:  cargo test -p coldvox-app --no-default-features --features level3,text-injection
```

## CI/CD Integration

The framework integrates with GitHub Actions to provide automated testing:

### Automatic Testing
- **Pull Requests**: Quick smoke tests (formatting, basic builds, clippy)
- **Main Branch**: Comprehensive feature matrix testing across platforms
- **Manual Trigger**: Full customizable testing via GitHub Actions UI

### Manual Workflow Trigger
1. Go to **Actions** tab → **Feature Matrix Testing**
2. Click **Run workflow**
3. Select testing strategy and options
4. Monitor results and download artifacts

## Key Features

### Multi-Platform Support
- Tests run on Linux, Windows, and macOS
- Platform-specific feature exclusions
- Automatic dependency installation

### Intelligent Test Selection
- **Curated combinations**: Focus on high-value feature interactions
- **Platform awareness**: Skip irrelevant combinations per OS
- **Performance optimized**: Balance thoroughness with execution time

### Rich Output
- Clear pass/fail indicators with timing
- Detailed failure summaries
- JSON output for integration and analysis
- CI artifact uploads for result preservation

## Advanced Usage

### Custom Test Arguments
```bash
# Pass arguments to cargo test
./test-features.py -p coldvox-app -- --test integration_tests

# Test specific patterns
./test-features.py -p coldvox-app -- --test "*audio*"
```

### JSON Output
```bash
# Save results for analysis
./test-features.py -p coldvox-app --json-output results.json

# Use with other tools
./test-features.py --strategy powerset --json-output full-results.json
```

## Supported Packages

The framework tests these core packages with curated feature combinations:

- **`coldvox-app`**: Main application with VAD, STT, and text injection features
- **`coldvox-vad`**: Voice activity detection with energy-based alternatives
- **`coldvox-text-injection`**: Platform-specific text injection backends
- **`coldvox-vad-silero`**: Silero ML-based VAD engine
- **`coldvox-stt-vosk`**: Vosk speech-to-text integration

## Troubleshooting

### Common Issues

#### Permission Denied
```bash
chmod +x test-features.py
```

#### Python Dependencies
```bash
# Python 3.11+ includes tomllib
python3 -c "import tomllib"

# For older Python versions
pip install tomli
```

#### Workspace Detection
```bash
# Specify workspace root explicitly
./test-features.py --workspace-root /path/to/ColdVox
```

## Documentation

For detailed information about the testing framework, including:
- Architecture and design decisions
- CI/CD workflow configuration
- Adding new features and test strategies
- Performance optimization tips
- Contributing guidelines

See the complete documentation at [`docs/testing-framework.md`](docs/testing-framework.md).

## Contributing

When adding new features:
1. Update curated combinations in `test-features.py`
2. Add relevant CI matrix entries if needed
3. Test locally before pushing
4. Update documentation as appropriate

This framework ensures robust feature compatibility and helps catch integration issues early in the development cycle.
