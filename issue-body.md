## Summary
Based on recent Copilot analysis (September 16, 2025), here's a plan to polish the CI pipeline. Setup is already strong (selective Rust caching in release/text_injection, Vosk local persistent cache via setup-vosk-cache.sh, MSRV matrix in ci.yml, automated pre-commit with .pre-commit-config.yaml including Vosk verify/E2E/GPU hooks, real E2E tests with nextest/headless). Focus on minor extensions for speed/quality as solo dev.

Key risks low (reliable runner, local gates), but these reduce re-runs and catch vulns.

## Proposed Improvements (Low Effort, 1-2 Hours Total)

### 1. Universal Rust Caching
- **Why:** Selective now (Swatinem/rust-cache@v2 in release/text_injection); missing in build_and_check (ci.yml) and vosk-tests (vosk-integration.yml)—rebuilds deps/target.
- **Fix:** Add after checkout in those jobs:
  ```
  - name: Cache Rust dependencies
    uses: Swatinem/rust-cache@v2.8.0
    with:
      workspaces: 'crates -> target'
      cache-on-failure: true
  ```
- **Impact:** 2-5min faster runs; adapt local_ci.sh if needed.

### 2. Security Scans Enforcement
- **Why:** deny.toml exists but unenforced; no audit/secrets scans—minor supply-chain risk.
- **Fix:** New job in ci.yml after validate-workflows (or step in build_and_check):
  ```
  security-scan:
    runs-on: [self-hosted, ...]
    needs: validate-workflows
    steps:
      - uses: actions/checkout@v4
      - name: Cargo Deny
        uses: EmbarkStudios/cargo-deny-action@v1
        with:
          manifest-path: Cargo.toml
          cmd: check
      - name: Cargo Audit
        uses: actions-rs/audit-check@v1
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
      - name: Secret Scan
        uses: trufflesecurity/trufflehog@v5
        with:
          path: ./
          base: HEAD
          head: HEAD
  ```
- **Impact:** ~1min; run locally with cargo deny check.

### 3. CI Pre-Commit Enforcement
- **Why:** Local auto-runs (YAML/Vosk/E2E/GPU); CI doesn't—slight bypass risk on push.
- **Fix:** Step in build_and_check (ci.yml) after cache:
  ```
  - name: Run pre-commit hooks
    uses: pre-commit/action@v5
    with:
      extra_args: '--all-files --show-diff-on-failure'
  ```
- **Impact:** ~30s; ensures consistency.

### 4. Nightly Matrix & Coverage (Optional)
- **Why:** MSRV good (stable/1.75); no nightly (async regressions); no reports.
- **Fix:** Update build_and_check matrix: `rust-version: [stable, 1.75, nightly]`. In stable (after tests):
  ```
  - name: Coverage
    uses: actions-rs/tarpaulin@v0.26.0
    with:
      args: --ignore-tests --out Xml
  - name: Upload Coverage
    uses: codecov/codecov-action@v4
    with:
      file: ./cobertura.xml
      fail_ci_if_error: true  # Customize threshold
  ```
- **Impact:** ~2min; skip if not needed.

## References
- Updated docs: /docs/devops/ (overview, ci-workflows.md, etc.—details jobs/matrix/caching).
- Code review: /docs/reviews/code_review.md (ties to STT refactor).
- Local test: act workflow_dispatch -j build_and_check (install act via cargo install act).

Implementation: Start with caching + deny-action in ci.yml; test manually.