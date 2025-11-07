---
doc_type: plan
subsystem: general
version: 0.1.0
status: draft
owners: [Documentation Working Group]
last_reviewed: 2025-11-07
---

# Documentation Analysis Enhancement Plan

> STUB NOTICE: This plan documents intended future work. The implementation is not complete. Treat current analyzer outputs as provisional for cross-reference only. Do not rely on lag/coverage/stale metrics until this plan is implemented and validated. A tracking issue has been opened to complete this work.

## Goal
Evolve the documentation analysis tooling beyond simple temporal correlations to produce actionable metrics: documentation lag per feature/PR, coverage scores, stale/abandoned feature detection, and reduced noise in doc↔code correlations.

## Current State (Baseline)
- `docs_history_analyzer.py`: timeline, file size evolution, raw commit stats.
- `docs_cross_reference_analyzer.py`: enriches doc commits referencing PR numbers with PR metadata (title/author/state) and correlates doc commits to code commits via a ±7 day window (produces ~90k correlations; high noise).
- Missing: direct PR doc coverage, documentation lag calculation, symbol-level mapping, stale documentation detection, per-PR feature gap analysis.

## Problems
1. High correlation volume obscures true relationships (±7 day brute window).
2. Lag between code introduction and documentation not measured.
3. PRs without doc updates invisible unless doc commit message references the PR number.
4. No mapping of code entities (public structs/functions/config keys) to doc references.
5. Stale documentation (docs referencing removed code) undetected.
6. Abandoned features (code added then never documented, or docs added then code removed) not surfaced.

## Scope of Enhancements
### Phase 1 (Instrumentation & Flags)
- Add CLI flags: `--window DAYS` (int), `--since YYYY-MM-DD`, `--pr-scan all|mentioned`.
- Reduce default correlation window to ±3 days.
- Expose an optional `--output <dir>`.
- Persist configuration metadata in JSON output (`analysis_config`).

### Phase 2 (PR Lag & Coverage)
- Fetch PR list (all or since date) via GitHub API when token + owner/name provided.
- For each PR:
  - Gather merge date, changed files, and classification of changed files (code vs docs).
  - Compute doc coverage boolean: did any doc file change in the PR diff?
  - After merge, search doc commits (within +N days; default 14) for references to changed code paths -> mark first doc update date.
  - Compute doc lag days = first_doc_update_date − merge_date (or null if missing).
- Output `pr_lag_summary.csv` and embed details under `prs` in JSON.

### Phase 3 (Symbol Extraction)
- Lightweight extraction of public Rust symbols using regex for `pub (struct|enum|fn|trait)` in changed code files.
- Maintain a symbol inventory per PR.
- Doc symbol search: grep docs for symbol names; record presence.
- Coverage metric: symbols_documented / symbols_added.

### Phase 4 (Stale & Abandoned Detection)
- Build HEAD symbol set; compare with doc symbol references.
  - Stale docs: symbol referenced in docs but absent in code.
  - Undocumented additions: symbol present in code added >14 days ago with no doc reference.
- Output `stale_symbols.csv` and `undocumented_symbols.csv`.

### Phase 5 (Noise Reduction & Scoring)
- Replace time-window correlations with:
  - Direct PR-based correlation (doc commits referencing PR or touching files changed by that PR).
  - Post-merge doc commits only (0..window days).
- Provide correlation score per doc commit: (#matching code commits / total code commits in window).

## Data Model Additions
```jsonc
{
  "analysis_config": {"window": 3, "since": "2025-01-01", "pr_scan": "mentioned"},
  "prs": {
    "190": {
      "merged_at": "2025-10-19T12:34:56Z",
      "changed_files": ["src/lib.rs", "docs/architecture.md"],
      "doc_files_changed_in_pr": ["docs/architecture.md"],
      "symbols_added": ["VoicePipeline"],
      "symbols_documented": ["VoicePipeline"],
      "coverage": 1.0,
      "first_doc_update_date": "2025-10-19",
      "doc_lag_days": 0
    }
  },
  "stale_symbols": ["OldPipeline"],
  "undocumented_symbols": ["NewConfigKey"],
  "pr_lag_summary": [
    {"pr": 190, "merge_date": "2025-10-19", "first_doc_date": "2025-10-19", "lag_days": 0}
  ]
}
```

## Acceptance Criteria
1. Running cross-reference analyzer with flags produces enriched JSON including `analysis_config` and (if token supplied) PR lag metrics.
2. `pr_lag_summary.csv`, `stale_symbols.csv`, `undocumented_symbols.csv` generated when relevant.
3. Correlation count drops significantly (<5k) with narrower window and PR-based filtering.
4. Symbols added vs documented ratio available per PR.
5. Stale and undocumented symbol CSVs highlight real examples (or are empty if none).

## Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Regex symbol extraction false positives | Provide opt-in flag `--symbol-mode regex|none`; later upgrade to syn parsing. |
| GitHub API rate limit | Batch requests, backoff, allow `--no-pr-details` flag. |
| Large repo performance | Scope with `--since`, lazy file scanning, caching. |
| Over-reporting stale symbols | Require symbol absence for >=14 days before marking stale. |
| Windows path edge cases | Normalize paths to POSIX when comparing. |

## Implementation Order
1. Flags & config recording.
2. PR lag + coverage basic (no symbols yet).
3. Symbol extraction & coverage ratio.
4. Stale/undocumented detection.
5. Refined correlation logic and scoring.
6. Documentation updates (`DOCS_ANALYSIS_README.md`).

## Out of Scope (Future)
- Semantic similarity clustering of doc vs code changes.
- GraphQL bulk queries optimization.
- Natural language “feature summaries” using LLM.

## Next Step
Implement Phase 1 + Phase 2 minimal subset to produce lag metrics; iterate.
