# Security and Best Practices in ColdVox DevOps

**Last Updated:** September 16, 2025  

## Overview

Security focuses on supply-chain (crates, deps), runner protection, and secret handling. Best practices cover reproducibility, efficiency, and maintainability. Current setup uses `deny.toml` but lacks CI enforcement; self-hosted risks are high without isolation. Strengths: Vosk model SHA256 verification in `setup-vosk-cache.sh` (prevents tampering).

## Security Practices

### Current Measures
- **Dependency Policy:** deny.toml (Cargo-deny config) blocks unsafe crates/licenses (not enforced in CI; no cargo-deny-action).  
- **Vosk Model Integrity:** SHA256SUMS in models/; setup-vosk-cache.sh verifies download (SHA256: 57919d20a3f03582a7a5b754353b3467847478b7d4b3ed2a3495b545448a44b9) before caching/symlinking to /home/coldaine/ActionRunnerCache/vosk/.  
- **Tokens/Perms:** Workflows limit to read/write where needed (e.g., `contents: read` in `ci.yml`).  
- **No Secrets in Code:** Env vars (e.g., `VOSK_MODEL_PATH`) over hardcodes.  

### Risks and Gaps
- **Self-Hosted Exposure:** Sudo in workflows (`dnf install`) risks escalation; no containerization.  
- **No Scanning:** Missing `cargo-audit` (vulns), `cargo-deny check` in CI (despite `deny.toml`), secret scanning (e.g., TruffleHog—no trufflehog in workflows).  
- **Supply-Chain:** Vosk verified (SHA256) but not signed; no SBOM.  
- **Secrets:** No rotation policy; runner tokens could leak if machine compromised.  

### Recommendations
1. **Add Scans to CI (`ci.yml`):**  
   - Vulns/Licenses: `uses: EmbarkStudios/cargo-deny-action@v1 with: { manifest-path: Cargo.toml, cmd: check }` (enforces deny.toml).  
   - Audit: `uses: actions-rs/audit-check@v1 with: { token: ${{ secrets.GITHUB_TOKEN }} }`.  
   - Secrets: `uses: trufflesecurity/trufflehog@v5`.  

2. **Runner Security:**  
   - Containers: `container: fedora:latest` in jobs (isolates from host).  
   - Non-Root: Run runner as `runner` user; restrict sudo to specific cmds.  
   - Firewall: Allow only GH IPs (see GitHub docs).  
   - Monitoring: Integrate `runner_health_check.sh` as scheduled job.  

3. **Model/Dep Verification:**  
   - Already Strong: Vosk SHA256 in script (setup-vosk-cache.sh).  
   - SBOM: Add `cargo-generate-sbom` to release.  

4. **Token Management:**  
   - Use fine-grained PATs; rotate quarterly.  
   - No secrets in workflows; use OIDC for auth.  

## Best Practices

### Reproducibility
- **Local Mirroring:** `just ci` or `local_ci.sh` (fmt/clippy/check/build/test; Vosk skip if absent).  
- **Pins:** `--locked` in Cargo; `Cargo.lock` committed.  
- **Fix:** `rust-toolchain.toml` for pinning.  

### Efficiency
- **Caching:** Selective Rust (`Swatinem/rust-cache@v2` in release/text_injection); Vosk local (persistent script). Add to build/vosk jobs.  
- **Parallelism:** Jobs parallel; use `cargo-nextest` (already in Vosk).  
- **Timeouts:** Per-test 10s; reduce job to 15min.  

### Maintainability
- **Docs:** This folder; add badges to README.md.  
- **Enforcement:** Pre-commit in CI (`pre-commit/action@v5`).  
- **Audits:** Quarterly: Run scans, review runner logs.  

### Compliance
- **Licenses:** `cargo-deny` ready (enforce in CI).  
- **Accessibility:** Vosk THIRDPARTY.md credits.  

Adopt incrementally: Start with deny-action in CI, then containerize runners.