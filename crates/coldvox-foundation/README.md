# coldvox-foundation

Foundation types, errors, and core functionality for the ColdVox voice processing pipeline.

## Purpose

This crate provides the essential building blocks used across all ColdVox components:

- **Error Types**: Unified error handling with `ColdVoxError` and domain-specific error types
- **Core Types**: Common data structures and type definitions
- **Shared Utilities**: Helper functions and utilities used by multiple crates
- **Configuration**: Base configuration structures and validation

## API Overview

```rust
use coldvox_foundation::{ColdVoxError, Result};

// Unified error handling
fn example() -> Result<()> {
    // Your code here
    Ok(())
}
```

## Features

- `default`: Standard functionality (currently empty, ready for future flags)

## Usage

This crate is typically used as a dependency by other ColdVox crates rather than directly by end users. If you're building applications with ColdVox, you'll likely want to use the main `coldvox-app` crate instead.

## Dependencies

- `tokio`: Async runtime support
- `tracing`: Logging and instrumentation
- `thiserror`: Error handling macros
- `serde`: Serialization support