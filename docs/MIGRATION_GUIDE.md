# ColdVox Migration Guide

This guide helps existing users migrate to the new workspace-based ColdVox architecture.

## Overview

ColdVox has been refactored into a Cargo workspace with multiple specialized crates. This provides better modularity, clearer dependencies, and optional features.

## Quick Migration

### If you were using basic commands:

**Before:**
```bash
cargo run
cargo run --bin mic_probe  
```

**After:**
```bash
cargo run -p coldvox-app --bin coldvox
cargo run -p coldvox-app --bin mic_probe
```

### If you were using feature flags:

**Before:**
```bash
cargo run --features vosk
```

**After:**  
```bash
cargo run -p coldvox-app --features vosk
```

## Detailed Changes

### Binary Locations

| Component | Before | After |
|-----------|---------|--------|
| Main app | `cargo run` | `cargo run -p coldvox-app --bin coldvox` |
| Microphone probe | `cargo run --bin mic_probe` | `cargo run -p coldvox-app --bin mic_probe` |
| TUI dashboard | `cargo run --bin tui_dashboard` | `cargo run -p coldvox-app --bin tui_dashboard` |
| Examples | `cargo run --example <name>` | `cargo run -p coldvox-app --example <name>` |

### Feature Flags

All feature flags remain the same but must be specified with the app crate:

| Feature | Usage |
|---------|--------|
| `vosk` | `cargo run -p coldvox-app --features vosk` |
| `text-injection` | `cargo run -p coldvox-app --features text-injection` |
| `examples` | `cargo run -p coldvox-app --features examples` |
| `live-hardware-tests` | `cargo run -p coldvox-app --features live-hardware-tests` |

### Multiple features:
```bash
cargo run -p coldvox-app --features vosk,text-injection
```

### Building and Testing

**Before:**
```bash
cargo build
cargo test
cargo clippy
```

**After:**
```bash
cargo build --workspace
cargo test --workspace  
cargo clippy --workspace
```

Or for specific crates:
```bash
cargo build -p coldvox-app
cargo test -p coldvox-foundation
```

## New Capabilities

### Workspace Benefits

1. **Modular Dependencies**: Individual crates have minimal, focused dependencies
2. **Optional Features**: STT and text injection are now truly optional
3. **Better Testing**: Each crate can be tested independently
4. **Clearer Architecture**: Separation of concerns across crates

### Individual Crate Usage

You can now depend on specific ColdVox functionality in your projects:

```toml
[dependencies]
coldvox-audio = { path = "path/to/coldvox/crates/coldvox-audio" }
coldvox-foundation = { path = "path/to/coldvox/crates/coldvox-foundation" }
```

## Configuration Changes

### Environment Variables
All environment variables remain the same:
- `RUST_LOG`: Logging level control
- `VOSK_MODEL_PATH`: Vosk model directory

### CLI Arguments
Most CLI arguments are unchanged, but some STT and text-injection specific arguments now require their respective feature flags to be enabled.

## Troubleshooting Migration Issues

### "Package not found" errors
Make sure to use `-p coldvox-app` to specify the application crate.

### Missing feature errors  
Features must be specified on the app crate: `--features vosk` becomes `-p coldvox-app --features vosk`.

### Build errors
The workspace structure requires all crates to be buildable. If you encounter dependency issues:

1. Ensure you're building the workspace: `cargo build --workspace`
2. Check that optional dependencies are properly feature-gated
3. Verify system dependencies are installed (especially for STT features)

### IDE Integration

If your IDE or language server has issues with the workspace:

1. Make sure it's configured to use the workspace root (`Cargo.toml`)
2. Some IDEs may need to be restarted after the workspace migration
3. Check that your IDE supports Cargo workspaces (most modern tools do)

## Getting Help

If you encounter issues during migration:

1. Check the main README.md for updated quick start instructions
2. Review the individual crate README files for specific functionality
3. Open an issue on GitHub with details about your migration problem

## Rollback Information

If you need to temporarily roll back to a pre-workspace version, you can checkout the commit before the workspace migration. However, we recommend migrating to the new structure for better maintainability and features.

The workspace migration maintains full backward compatibility for core functionality - only the build commands have changed.