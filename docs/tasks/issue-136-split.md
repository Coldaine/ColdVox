# Issue: Deconstruct "Crazy Enormous Issue" #136

**Status:** New (Split)
**Original Issue:** #136
**Tags:** `refactor`, `split-needed`

## Summary

This document splits the large, ambiguous Issue #136 into smaller, actionable tasks. The original issue covered a wide range of topics from CI improvements to dependency updates. These new issues can be individually prioritized and assigned.

## Split Issues

### 1. CI/CD Workflow Enhancements
- **Description:** Improve the CI/CD pipeline by adding more comprehensive build matrices, caching dependencies more effectively, and adding automated release-plz integration.
- **Acceptance Criteria:**
    - CI runs tests across stable, beta, and nightly Rust.
    - Dependency caching is optimized, reducing CI run times.
    - A new workflow automates release PRs.
- **Labels:** `ci`, `enhancement`

### 2. Dependency Audit and Update
- **Description:** Perform a full audit of all `Cargo.toml` dependencies. Update outdated crates, remove unused ones, and consolidate versions where possible.
- **Acceptance Criteria:**
    - `cargo-deny` checks pass without warnings.
    - All key dependencies (e.g., `tokio`, `clap`) are on their latest stable versions.
    - A report is produced summarizing the changes.
- **Labels:** `dependencies`, `tech-debt`

### 3. Developer Onboarding and Documentation
- **Description:** Improve the developer onboarding experience by updating the `README.md`, creating a `CONTRIBUTING.md`, and adding more detailed setup instructions for all sub-crates.
- **Acceptance Criteria:**
    - A new developer can successfully build and test the project by following the `README.md`.
    - `CONTRIBUTING.md` clearly outlines the PR and review process.
- **Labels:** `documentation`, `onboarding`
