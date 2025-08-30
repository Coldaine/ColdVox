# Text Injection Operations Runbook

## Overview

This runbook provides operational procedures for deploying, troubleshooting, and maintaining the text injection system in ColdVox.

## Pre-Deployment Checks

### Environment Verification

#### Desktop Session
```bash
# Check desktop environment
echo $XDG_SESSION_TYPE  # Should be "wayland" or "x11"
echo $WAYLAND_DISPLAY   # Should exist for Wayland
echo $DISPLAY          # Should exist for X11
```

#### AT-SPI Services
```bash
# Check AT-SPI bus
echo $AT_SPI_BUS_ADDRESS
# Should show: unix:path=/run/user/1000/at-spi/bus

# Verify AT-SPI registry
busctl --user list | grep org.a11y
# Should show AT-SPI services
```

#### Clipboard Tools
```bash
# Wayland clipboard
which wl-copy wl-paste
# Should be available in PATH

# X11 clipboard (fallback)
which xclip xsel
# At least one should be available
```

#### External Tools
```bash
# ydotool
which ydotool
ls -la /tmp/ydotool.socket  # Should exist

# kdotool
which kdotool

# uinput access
ls -la /dev/uinput
groups | grep uinput  # User should be in uinput group
```

### Feature Flag Configuration

#### Minimal Configuration
```toml
[features]
text-injection-clipboard = true

[text_injection]
injection_mode = "auto"
inject_on_unknown_focus = false
```

#### Full Configuration
```toml
[features]
text-injection-atspi = true
text-injection-clipboard = true
text-injection-ydotool = true
text-injection-regex = true

[text_injection]
injection_mode = "auto"
inject_on_unknown_focus = true
allowlist = ["firefox", "chromium", "code"]
paste_chunk_chars = 1000
rate_cps = 30
```

## Deployment Procedures

### 1. Feature Flag Activation

#### Enable Core Features
```bash
# Build with minimal features
cargo build --features text-injection-clipboard

# Build with full features
cargo build --features text-injection-atspi,text-injection-clipboard,text-injection-ydotool,text-injection-regex
```

#### Verify Build
```bash
# Check enabled features
cargo build --features text-injection 2>&1 | grep -i "feature"

# Verify binary capabilities
./target/debug/coldvox --help | grep -i injection
```

### 2. Runtime Verification

#### Capability Probe
```bash
# Run the probe example
cargo run --example text_injection_probe

# Expected output:
# ✓ Desktop Environment: Wayland
# ✓ AT-SPI Available: true
# ✓ Preferred Backend: Wayland+AT-SPI
```

#### Test Injection
```bash
# Start with minimal logging
RUST_LOG=info ./target/debug/coldvox

# Test basic injection (requires running application)
# Use the TUI to verify status
```

## Troubleshooting Guide

### Common Issues

#### Issue: No Backend Available
**Symptoms:**
- Probe shows "No backends available"
- Injection fails immediately

**Checks:**
```bash
# Verify desktop session
echo $XDG_SESSION_TYPE

# Check clipboard tools
which wl-copy

# Verify AT-SPI (if enabled)
busctl --user list | grep a11y
```

**Solutions:**
1. Install missing tools: `sudo apt install wl-clipboard`
2. Enable AT-SPI: `gsettings set org.gnome.desktop.a11y.applications screen-reader-enabled true`
3. Restart session for Wayland changes

#### Issue: Permission Denied
**Symptoms:**
- "Permission denied" errors
- uinput access failures

**Checks:**
```bash
# Check uinput permissions
ls -la /dev/uinput
groups | grep uinput

# Check AT-SPI permissions
busctl --user status
```

**Solutions:**
```bash
# Add to uinput group
sudo usermod -a -G uinput $USER

# Restart session
# Or use ydotool with sudo (not recommended)
```

#### Issue: AT-SPI Not Working
**Symptoms:**
- AT-SPI probe fails
- Fallback to clipboard-only

**Checks:**
```bash
# Check AT-SPI environment
echo $AT_SPI_BUS_ADDRESS

# Verify accessibility services
gsettings get org.gnome.desktop.a11y.applications screen-reader-enabled
```

**Solutions:**
```bash
# Enable accessibility
gsettings set org.gnome.desktop.a11y.applications screen-reader-enabled true

# Restart AT-SPI registry
killall at-spi-bus-launcher
at-spi-bus-launcher --launch-immediately
```

#### Issue: High Latency/Low Success Rate
**Symptoms:**
- Injection takes >200ms
- Success rate <80%

**Checks:**
```bash
# Monitor system load
uptime
top -b -n1 | head -20

# Check for competing processes
ps aux | grep -E "(ydotool|kdotool|input)"
```

**Solutions:**
1. Reduce `rate_cps` in config
2. Increase `paste_chunk_chars`
3. Disable competing input tools
4. Check system performance

### Diagnostic Commands

#### System Information
```bash
# Full system probe
cat << 'EOF' > diagnose_text_injection.sh
#!/bin/bash
echo "=== Text Injection Diagnostics ==="
echo "Desktop: $XDG_SESSION_TYPE"
echo "Wayland: $WAYLAND_DISPLAY"
echo "X11: $DISPLAY"
echo "AT-SPI: $AT_SPI_BUS_ADDRESS"
echo ""
echo "=== Tool Availability ==="
which wl-copy && echo "✓ wl-clipboard" || echo "✗ wl-clipboard"
which ydotool && echo "✓ ydotool" || echo "✗ ydotool"
which kdotool && echo "✓ kdotool" || echo "✗ kdotool"
echo ""
echo "=== Permissions ==="
ls -la /dev/uinput 2>/dev/null && echo "✓ uinput accessible" || echo "✗ uinput not accessible"
groups | grep -q uinput && echo "✓ uinput group" || echo "✗ not in uinput group"
echo ""
echo "=== Services ==="
busctl --user list 2>/dev/null | grep -q a11y && echo "✓ AT-SPI services" || echo "✗ AT-SPI services"
EOF

chmod +x diagnose_text_injection.sh
./diagnose_text_injection.sh
```

## Rollback Procedures

### Emergency Rollback

#### Immediate Disable
```bash
# Kill running processes
pkill -f coldvox

# Disable injection features
# Edit config to disable injection
```

#### Conservative Mode
```toml
[text_injection]
# Force paste-only mode
injection_mode = "paste"

# Disable external tools
allow_ydotool = false
allow_enigo = false
allow_mki = false

# Strict focus requirements
inject_on_unknown_focus = false
require_focus = true
```

#### Minimal Feature Set
```bash
# Rebuild with minimal features
cargo build --features text-injection-clipboard

# Disable advanced features
cargo build --no-default-features --features text-injection-clipboard
```

### Gradual Rollback Steps

1. **Step 1: Disable Advanced Features**
   ```toml
   allow_ydotool = false
   allow_enigo = false
   allow_mki = false
   text_injection_regex = false
   ```

2. **Step 2: Conservative Injection**
   ```toml
   injection_mode = "paste"
   inject_on_unknown_focus = false
   rate_cps = 10
   ```

3. **Step 3: Minimal Backend**
   ```toml
   # Keep only clipboard
   text_injection_atspi = false
   ```

4. **Step 4: Complete Disable**
   ```toml
   text_injection = false
   ```

## Monitoring and Maintenance

### Key Metrics to Monitor
- Success rate (>90% target)
- Average latency (<100ms target)
- Error rate trends
- Backend preference changes

### Log Analysis
```bash
# Search for injection errors
grep -i "injection.*fail" /var/log/coldvox.log

# Monitor success rates
grep "Successfully injected" /var/log/coldvox.log | wc -l

# Check for permission issues
grep -i "permission denied" /var/log/coldvox.log
```

### Performance Tuning

#### Optimal Settings by Use Case
```toml
# High-reliability (default)
[text_injection]
injection_mode = "auto"
paste_chunk_chars = 1000
rate_cps = 30

# High-performance
[text_injection]
injection_mode = "keystroke"
paste_chunk_chars = 2000
rate_cps = 50

# IME-heavy environments
[text_injection]
injection_mode = "paste"
paste_chunk_chars = 500
rate_cps = 20
```

## Support Procedures

### User Issue Triage
1. Run diagnostic script
2. Check system compatibility
3. Review configuration
4. Test with minimal features
5. Escalate if needed

### Escalation Paths
- **Configuration Issues**: Update documentation
- **Permission Issues**: System administration
- **Performance Issues**: Code optimization
- **Compatibility Issues**: Platform-specific fixes

## Version Compatibility

### Breaking Changes
- Monitor for AT-SPI API changes
- Test with new Wayland compositors
- Verify external tool compatibility

### Upgrade Testing
```bash
# Test upgrade path
cargo update
cargo build --features text-injection-full
cargo run --example text_injection_probe