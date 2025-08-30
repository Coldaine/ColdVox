# Text Injection Security Notes

## Overview

This document outlines the security considerations, threat model, and best practices for the text injection system in ColdVox.

## Security Principles

### Least Privilege
- **Default Security**: System operates with minimal required permissions
- **Opt-in Elevation**: Advanced features require explicit user consent
- **Graceful Degradation**: Security restrictions don't break core functionality

### Defense in Depth
- **Multiple Injection Methods**: Fallback mechanisms reduce single-point failures
- **Input Validation**: All text input validated before injection
- **Access Controls**: Application allowlist/blocklist enforcement

## Threat Model

### Attack Vectors

#### 1. Unauthorized Text Injection
- **Risk**: Malicious text injection into sensitive applications
- **Mitigation**:
  - Application allowlist/blocklist
  - Focus validation before injection
  - User confirmation for high-risk operations

#### 2. Information Disclosure
- **Risk**: Sensitive text exposed in logs or memory
- **Mitigation**:
  - Log redaction by default
  - Memory clearing after injection
  - No persistent storage of injected content

#### 3. Privilege Escalation
- **Risk**: Injection mechanisms used to gain elevated access
- **Mitigation**:
  - Sandboxed injection processes
  - Limited system API access
  - User permission requirements

#### 4. System Stability Attacks
- **Risk**: Injection causing system hangs or crashes
- **Mitigation**:
  - Rate limiting and budget enforcement
  - Timeout mechanisms
  - Error recovery procedures

## Permission Model

### Backend Security Levels

#### Level 1: Minimal (Default)
- **Clipboard + AT-SPI**: Standard system APIs
- **Permissions**: None required
- **Security**: High (uses system-provided mechanisms)
- **Reliability**: High (well-tested system components)

#### Level 2: Elevated (Opt-in)
- **YdoTool/KdoTool**: External process execution
- **Permissions**: uinput group membership
- **Security**: Medium (external process isolation)
- **Reliability**: Medium (depends on external tool stability)

#### Level 3: Advanced (Expert Only)
- **Enigo/MKI**: Direct input device access
- **Permissions**: Root or input group access
- **Security**: Low (direct hardware access)
- **Reliability**: Low (potential system interference)

### Permission Requirements

#### uinput Access
```bash
# Check current permissions
ls -la /dev/uinput
# crw-rw---- 1 root input 10, 223 Dec  1 12:00 /dev/uinput

# Add user to input group
sudo usermod -a -G input $USER

# Verify group membership
groups $USER
# Should include 'input'
```

#### AT-SPI Permissions
```bash
# Enable accessibility services
gsettings set org.gnome.desktop.a11y.applications screen-reader-enabled true

# Verify AT-SPI bus access
busctl --user status
```

## Data Protection

### Log Security

#### Why Redaction is Critical
- **Privacy Protection**: Prevents accidental exposure of sensitive information
- **Compliance**: Meets data protection requirements
- **Forensics**: Maintains audit trails without compromising privacy

#### Redaction Implementation
```rust
// Safe logging - default behavior
info!("Injected text ({} chars, hash: {})", text.len(), hash);

// Unsafe logging - debug only
trace!("Injected text: {}", text);  // NEVER in production
```

#### Production Log Policy
- **Level**: INFO or WARN only
- **Content**: Metadata only (length, hash, method, success/failure)
- **PII**: Never logged
- **Debug Mode**: Temporary use only, with explicit user consent

### Memory Security

#### Data Lifecycle
1. **Input**: Text received from speech processing
2. **Processing**: Text validated and prepared for injection
3. **Injection**: Text sent to target application
4. **Cleanup**: Memory cleared immediately after injection

#### Memory Protection
- **No Persistence**: Text never written to disk
- **Immediate Cleanup**: Memory freed after use
- **No Caching**: Sensitive text not cached
- **Secure Zeroing**: Memory overwritten before deallocation

## Access Control

### Application Filtering

#### Allowlist Mode
```toml
[text_injection]
# Only allow specific applications
allowlist = ["firefox", "chromium", "code", "gedit"]
```

#### Blocklist Mode
```toml
[text_injection]
# Block specific applications
blocklist = ["terminal", "password-manager"]
```

#### Regex Support
```toml
[text_injection]
# Advanced pattern matching
allowlist = ["^firefox$", "chromium.*", "code-.*"]
```

### Focus Validation

#### Strict Mode
```toml
[text_injection]
# Require confirmed focus
require_focus = true
inject_on_unknown_focus = false
```

#### Permissive Mode
```toml
[text_injection]
# Allow injection with unknown focus
inject_on_unknown_focus = true
```

## Operational Security

### Configuration Security

#### Secure Defaults
```toml
[text_injection]
# Conservative security settings
injection_mode = "paste"  # Safer than keystroke
rate_cps = 30            # Reasonable rate limiting
max_total_latency_ms = 5000  # Budget enforcement
```

#### High-Security Mode
```toml
[text_injection]
# Maximum security settings
allowlist = ["trusted-app"]
inject_on_unknown_focus = false
require_focus = true
redact_logs = true
```

### Monitoring and Auditing

#### Security Metrics
- **Injection Attempts**: Track all injection operations
- **Failure Patterns**: Monitor for suspicious failure rates
- **Permission Changes**: Audit permission modifications
- **Configuration Changes**: Log security setting changes

#### Audit Logging
```rust
// Security events
info!("Security: Injection blocked for unauthorized app: {}", app_id);
warn!("Security: Rate limit exceeded, possible attack");
error!("Security: Permission denied for injection method: {:?}", method);
```

## Compliance Considerations

### Data Protection Regulations
- **GDPR**: Personal data handling requirements
- **CCPA**: California privacy law compliance
- **HIPAA**: Healthcare data protection (if applicable)

### Enterprise Security
- **Zero Trust**: Verify every injection request
- **Least Privilege**: Minimal required permissions
- **Audit Trails**: Complete operation logging
- **Incident Response**: Security event handling procedures

## Best Practices

### Development Security
1. **Code Review**: All injection code requires security review
2. **Input Validation**: Validate all text input
3. **Error Handling**: Secure error messages (no information leakage)
4. **Testing**: Comprehensive security testing

### Deployment Security
1. **Minimal Features**: Enable only required features
2. **Secure Configuration**: Use restrictive allowlists
3. **Monitoring**: Enable security monitoring
4. **Updates**: Keep dependencies updated

### User Security
1. **User Education**: Explain permission requirements
2. **Consent**: Obtain user consent for elevated permissions
3. **Transparency**: Clear indication of active injection
4. **Control**: Easy disable/enable controls

## Incident Response

### Security Incident Procedure
1. **Detection**: Monitor for suspicious activity
2. **Containment**: Disable injection immediately
3. **Investigation**: Review logs and system state
4. **Recovery**: Restore secure configuration
5. **Lessons Learned**: Update security measures

### Emergency Controls
```bash
# Immediate disable
pkill -f coldvox

# Remove permissions
sudo gpasswd -d $USER input

# Clear logs
truncate -s 0 /var/log/coldvox.log
```

## Future Security Enhancements

### Planned Improvements
- **Sandboxing**: Isolated injection processes
- **Encryption**: Encrypted text in transit
- **Authentication**: User verification for sensitive operations
- **Rate Limiting**: Advanced rate limiting algorithms
- **Anomaly Detection**: ML-based security monitoring

### Research Areas
- **Side-channel Attacks**: Timing-based information leakage
- **Memory Attacks**: Heap spraying or use-after-free
- **UI Redressing**: Clickjacking-style attacks
- **IME Vulnerabilities**: Input method security issues