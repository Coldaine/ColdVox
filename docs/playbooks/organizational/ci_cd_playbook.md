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

## Build Caching

### sccache

The self-hosted runner uses sccache for Rust compilation caching. This is set up automatically via CI but can be pre-installed:

```bash
# One-time setup on runner
just setup-sccache

# Or manually
cargo install sccache --locked
```

The CI workflow (`text_injection_tests` job) will:
1. Run `just setup-sccache` to ensure sccache is installed
2. Start the sccache server
3. Set `RUSTC_WRAPPER=sccache` for all subsequent cargo commands

**Expected impact**: 30-60% faster incremental builds.

### Swatinem/rust-cache

Standard cargo cache action. Works on both hosted and self-hosted runners.

## AI-Powered CI Failure Analysis

When CI fails on a PR, the `ci-failure-analysis.yml` workflow automatically:
1. Verifies the failure (handles race conditions)
2. Gets PR number via SHA lookup (works for fork PRs)
3. Fetches logs from failed jobs
4. Analyzes the failure using **Gemini 2.5 Flash** AI (with thinking/reasoning)
5. Posts a comment on the PR with root cause analysis and fix suggestions

**Requirements**:
- `GEMINI_API_KEY` secret must be set in repository settings
- Get a free key from https://aistudio.google.com/app/apikey

**Configuration**: `.github/workflows/ci-failure-analysis.yml`

**Design Decisions** (based on research):
- Uses `ubuntu-latest` (not self-hosted) for security - auxiliary workflows handling untrusted PR data should run on ephemeral GitHub-hosted runners
- Uses SHA-based PR lookup via `listPullRequestsAssociatedWithCommit` API (GitHub bug: `workflow_run.pull_requests` is empty for fork PRs)
- Verifies `conclusion` via API call before proceeding (race condition workaround)
- Uses Gemini 2.5 Flash (`/v1beta` endpoint for latest models) with exponential backoff retry
- Limits log size to 50KB to stay within API token limits

**Cost**: Free tier (250 requests/day) is sufficient for most projects. Estimated ~$0.78/month if exceeding free tier.

This runs only on PR failures (not push failures) to avoid noise.

## Related Documentation

- [docs/dependencies.md](../../dependencies.md) - Dependency overview and tooling docs
- [deny.toml](../../../deny.toml) - cargo-deny configuration
- [runner_setup.md](./runner_setup.md) - Self-hosted runner setup guide
