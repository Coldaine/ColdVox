---
doc_type: playbook
subsystem: general
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-12-03
---

# CI/CD Playbook

This playbook documents the continuous integration and delivery pipeline for ColdVox.

## Workflow Overview

The main CI workflow (`.github/workflows/ci.yml`) runs on:
- Push to `main`, `release/*`, `feature/*`, `feat/*`, `fix/*` branches
- Pull requests to `main`
- Daily schedule (cron)
- Manual dispatch

## CI Jobs

### 1. validate-workflows

Validates workflow YAML files using the `gh` CLI. Optional (continue-on-error).

### 2. setup-whisper-dependencies

Sets up Whisper model cache for STT tests.

### 3. security_audit

**Purpose**: Scans dependencies for security vulnerabilities and license compliance.

**Tools**:
- `cargo audit` - Checks `Cargo.lock` against [RustSec Advisory Database](https://rustsec.org/)
- `cargo deny` - Comprehensive dependency linting (licenses, bans, advisories, sources)

**Configuration**: See `deny.toml` in repository root.

**Failure handling**: Issues a warning but does not block the build. Check the job output for details on any vulnerabilities or license issues.

**Local reproduction**:
```bash
cargo install cargo-audit cargo-deny
cargo audit
cargo deny check
```

### 4. build_and_check

Main build job:
- Formatting check (advisory)
- Clippy linting
- Type check
- Build workspace
- Build documentation
- Run unit/integration tests
- Qt 6 GUI check (if available)

### 5. text_injection_tests

Tests text injection functionality in a headless X11 environment:
- Starts Xvfb + fluxbox
- Sets up D-Bus session
- Runs real-injection-tests with clipboard utilities
- Runs Golden Master pipeline test

### 6. ci_success

Aggregates results from all jobs and generates a CI report artifact.

## Adding New Security Advisories

When cargo-audit or cargo-deny reports a new advisory:

1. **Evaluate severity**: Is it a real security risk or informational?
2. **Update dependency**: If possible, update the affected crate
3. **Ignore if appropriate**: For unmaintained (not vulnerable) crates, add to `deny.toml`:
   ```toml
   [advisories]
   ignore = [
       { id = "RUSTSEC-XXXX-XXXX", reason = "No security impact, unmaintained but stable" },
   ]
   ```
4. **Document**: Add to CHANGELOG.md under Security & Tooling

## Related Documentation

- [docs/dependencies.md](../../dependencies.md) - Dependency overview and tooling docs
- [deny.toml](../../../deny.toml) - cargo-deny configuration
