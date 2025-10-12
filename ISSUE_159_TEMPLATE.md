# Issue #159: ONNX Runtime Build Dependencies in Restricted Environments

**Title:** [Build] ONNX Runtime download fails in restricted CI/sandbox environments  
**Priority:** High  
**Labels:** `build`, `dependencies`, `ci`, `p1`

---

## Problem Description

The Silero VAD feature requires ONNX Runtime libraries, which the `ort-sys` crate attempts to download during build from `cdn.pyke.io`. This download fails in restricted environments (CI runners, Docker containers, sandboxes) that lack internet access or have restricted network policies.

## Error Message

```
error: failed to run custom build command for `ort-sys v2.0.0-rc.10`

Caused by:
  process didn't exit successfully: `/path/to/build-script-build` (exit status: 101)
  
  thread 'main' panicked at build.rs:53:27:
  Failed to GET `https://cdn.pyke.io/0/pyke:ort-rs/ms@1.22.0/x86_64-unknown-linux-gnu.tgz`: 
  io: failed to lookup address information: No address associated with hostname
```

## Impact

- ❌ Cannot build project in isolated/air-gapped environments
- ❌ Blocks automated testing in GitHub Actions or other CI systems with restricted network
- ❌ Prevents reproducible builds in secure/sandboxed development environments
- ❌ Makes it impossible to verify PR #152 in current Copilot agent environment

## Current Workarounds

None that work consistently. Setting `ORT_SKIP_DOWNLOAD=1` causes the build to fail with different errors:

```
error: empty search path given via `-L`
error: could not compile `ort-sys` (lib) due to 1 previous error
```

## Affected Components

- `crates/coldvox-vad-silero` (depends on ONNX Runtime)
- `crates/app` (has `silero` in default features)
- All workspace builds (pulls in silero transitively)

## Proposed Solutions

### Option A: Make Silero VAD Truly Optional ⭐ (Recommended)
**Effort:** Low  
**Impact:** Immediate fix

1. Remove `silero` from default features in `crates/app/Cargo.toml`
2. Ensure project builds without any VAD or with a simpler VAD alternative
3. Add clear documentation on enabling Silero with proper setup

**Changes needed:**
```toml
# crates/app/Cargo.toml
[features]
default = ["text-injection", "vosk"]  # Remove "silero"
silero = ["coldvox-vad-silero/silero"]
```

**Pros:**
- Quick fix
- Allows builds in restricted environments
- Users can opt-in when they have proper setup

**Cons:**
- Changes default behavior
- Users need to explicitly enable Silero if they want it

### Option B: Pre-cache ONNX Binaries
**Effort:** Medium  
**Impact:** Fixes CI but not all sandboxes

1. Download ONNX Runtime binaries to repository or CI cache
2. Point `ort-sys` to cached location via environment variables
3. Document the setup process

**Environment variables:**
```bash
export ORT_LIB_LOCATION=/path/to/onnxruntime/lib
export ORT_SKIP_DOWNLOAD=1
```

**Pros:**
- Keeps Silero as default
- Maintains existing user experience

**Cons:**
- Increases repository size or requires complex CI setup
- Still fails in some restricted environments
- Maintenance burden (keeping binaries updated)

### Option C: Add Alternative VAD Backend
**Effort:** High  
**Impact:** Long-term solution

1. Implement a pure Rust VAD (energy-based or Level3)
2. Make it the default
3. Keep Silero as optional advanced feature

**Pros:**
- No external dependencies
- Builds everywhere
- Better for embedded/restricted environments

**Cons:**
- Significant development effort
- May have lower accuracy than Silero ML model
- Already have Level3 VAD (but it's deprecated)

### Option D: Use System-Installed ONNX Runtime
**Effort:** Medium  
**Impact:** Requires system packages

1. Update build script to check for system ONNX first
2. Fallback to download if not found
3. Document package installation per distro

**Installation examples:**
```bash
# Ubuntu/Debian
sudo apt-get install libonnxruntime-dev

# Fedora
sudo dnf install onnxruntime-devel

# Arch
yay -S onnxruntime
```

**Pros:**
- Standard package management
- System-wide updates

**Cons:**
- Not available on all distros
- Version compatibility issues
- Doesn't help in sandboxes without packages

## Recommended Approach

**Phase 1 (Immediate):** Implement Option A
- Remove `silero` from default features
- Update documentation
- Test that builds work without it
- Release patch version

**Phase 2 (Next Sprint):** Implement Option B
- Set up CI caching for ONNX binaries
- Document environment variables
- Provide setup scripts

**Phase 3 (Future):** Consider Option C
- Evaluate alternative VAD backends
- Implement if needed for embedded use cases

## Implementation Checklist

### Phase 1: Make Silero Optional
- [ ] Remove `silero` from default features in `crates/app/Cargo.toml`
- [ ] Test build without Silero: `cargo build --workspace`
- [ ] Test build with Silero: `cargo build --workspace --features silero`
- [ ] Update README with Silero setup instructions
- [ ] Update CHANGELOG
- [ ] Create migration guide for existing users
- [ ] Test in CI environment
- [ ] Release patch version

### Phase 2: Pre-cache Binaries (Optional)
- [ ] Download ONNX Runtime binaries for Linux x86_64
- [ ] Add to CI cache or artifacts
- [ ] Update CI workflows to set `ORT_LIB_LOCATION`
- [ ] Document the caching strategy
- [ ] Test in CI

## Testing Requirements

### Build Tests
```bash
# Test without Silero (should work)
cargo clean && cargo build --workspace

# Test with Silero (should work if ONNX available)
cargo clean && cargo build --workspace --features silero

# Test all features
cargo clean && cargo build --workspace --all-features

# Test in Docker (simulates CI)
docker run -v $(pwd):/work -w /work rust:latest cargo build --workspace
```

### Verification
- [ ] Builds successfully without internet
- [ ] Builds successfully in Docker
- [ ] Builds successfully in GitHub Actions
- [ ] Builds successfully with Silero when ONNX is available
- [ ] Tests pass without Silero
- [ ] Tests pass with Silero (when available)

## Documentation Updates

### README.md
```markdown
## Optional Features

### Silero VAD (Advanced ML-based Voice Activity Detection)

Silero VAD provides superior voice detection accuracy but requires ONNX Runtime.

**To enable:**
```bash
cargo build --features silero
```

**Requirements:**
- Internet access during build (to download ONNX Runtime), OR
- Pre-installed ONNX Runtime:
  - Ubuntu: `sudo apt-get install libonnxruntime-dev`
  - Fedora: `sudo dnf install onnxruntime-devel`
  - Set `ORT_LIB_LOCATION=/path/to/onnxruntime`

**Note:** If you encounter build errors related to ONNX Runtime, the project will work fine without Silero using the default VAD backend.
```

### BUILDING.md (New file)
Create comprehensive build documentation including:
- System dependencies per platform
- Optional features and their requirements
- Troubleshooting common build issues
- CI/Docker build instructions

## Related Issues

- PR #152: Could not be fully verified due to this issue
- Issue #100: CI improvements (may benefit from ONNX caching)
- Issue #63: Qt6 detection (similar system dependency challenges)

## Success Criteria

- [ ] Project builds successfully without internet access
- [ ] CI workflows complete without errors
- [ ] Users can easily enable/disable Silero
- [ ] Clear documentation on requirements and setup
- [ ] No regressions in functionality
- [ ] Silero still works when properly configured

## Timeline

- **Phase 1:** 1-2 days (remove from default, update docs)
- **Phase 2:** 3-5 days (CI caching setup)
- **Total:** 1 week for complete solution

## Additional Notes

This issue was discovered during PR #152 finalization review when attempting to run `cargo check` in a sandboxed GitHub Copilot agent environment. The build failed repeatedly despite installing system dependencies because the ONNX Runtime download was blocked.

The root cause is that `ort-sys` has built-in download logic that is incompatible with restricted environments. This is a known limitation of the `ort` crate ecosystem and affects many projects using ONNX Runtime in Rust.

---

**Priority Justification:** This is marked as HIGH priority because it:
1. Blocks verification of PRs in certain environments
2. Prevents reproducible builds
3. May affect CI reliability
4. Impacts developer experience significantly

However, it's not CRITICAL because:
1. Workarounds exist (build on different machines)
2. Only affects build time, not runtime
3. Doesn't block end users (pre-built binaries work fine)
