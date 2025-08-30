# Text Injection Privacy Policy and Logging Guidelines

## Overview

This document outlines the privacy considerations and logging practices for the text injection system in ColdVox.

## Privacy Principles

### Data Handling
- **No Persistence**: Injected text content is never stored to disk or persisted in any form
- **In-Memory Only**: All text processing occurs in memory and is discarded after injection
- **No Telemetry**: Text content is not included in any telemetry or metrics collection

### Logging Behavior

#### Default Logging (Privacy-Safe)
- **Redacted by Default**: All logs containing text content show only metadata:
  - Text length (character count)
  - Hash/SHA256 of content (for debugging correlation)
  - Injection method used
  - Success/failure status
- **No Plaintext**: Actual text content never appears in logs under normal operation

#### Debug Logging (Opt-in)
- **Trace Level Required**: Full text logging requires:
  - Log level set to `trace`
  - Configuration option `redact_logs = false`
- **Explicit Consent**: Users must explicitly enable this for debugging purposes
- **Temporary Use**: Debug logging should only be enabled for troubleshooting and disabled afterwards

## Configuration

### redact_logs Setting
```toml
[logging]
# Default: true (recommended for privacy)
redact_logs = true

# For debugging only - set to false temporarily
# redact_logs = false
```

### Log Level Configuration
```bash
# Normal operation
RUST_LOG=info

# Debug with full text (use with caution)
RUST_LOG=trace
```

## Log Examples

### Safe Logging (Default)
```
INFO: Injected text (42 chars, hash: a1b2c3...) using method AtspiInsert - success
WARN: Injection failed for text (128 chars, hash: d4e5f6...) - timeout
```

### Debug Logging (Opt-in)
```
TRACE: Injecting text: "Hello, world!" using method AtspiInsert
TRACE: Injection successful for "Hello, world!"
```

## Rationale

### Why Redact by Default?
- **Privacy Protection**: Prevents accidental exposure of sensitive information
- **Compliance**: Aligns with data protection best practices
- **Security**: Reduces risk of log-based data leaks

### Why Allow Full Logging?
- **Debugging**: Essential for troubleshooting injection failures
- **Development**: Required during feature development and testing
- **Transparency**: Users can inspect what text is being injected when needed

## Best Practices

1. **Keep Redaction Enabled**: Only disable for specific debugging sessions
2. **Monitor Log Files**: Regularly review and secure log file access
3. **Temporary Debug Mode**: Enable trace logging only when actively debugging
4. **Clean Up**: Remove or rotate debug logs containing full text after use

## Implementation Notes

- Redaction is implemented at the logging macro level
- Hash calculation uses SHA256 for correlation without revealing content
- Configuration is checked at runtime for each log statement
- No performance impact from redaction in normal operation