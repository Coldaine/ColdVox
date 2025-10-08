# Execution Guide: Domain-Based Refactor Split (Plan 2)

**Goal:** Convert the monolithic `anchor/oct-06-2025` refactor into a clean, reviewable stack using Plan 2's domain-based approach.

---

## Pre-Flight Checklist

- [ ] Install Graphite CLI: `npm install -g @withgraphite/graphite-cli@latest`
- [ ] Configure Graphite: `gt user config --set`
- [ ] Verify clean working tree: `git status`
- [ ] Create backup branch: `git branch backup/anchor-oct-06-2025-$(date +%Y%m%d-%H%M%S)`
- [ ] Checkout refactor branch: `git checkout anchor/oct-06-2025`

---

## Phase 1: Branch Setup & Tracking

```bash
# Ensure Graphite knows about this branch
gt track

# Verify tracking
gt log
```

**Expected Output:**
```
◯ anchor/oct-06-2025 (current)
  └─ main
```

---

## Phase 2: Interactive Split by Hunk

### Step 2.1: Start Interactive Split

```bash
gt split --by-hunk
```

### Step 2.2: Hunk Assignment Strategy

Graphite will present each changed hunk interactively. Use these path-based rules:

| File Path Pattern | Assign to Branch | Priority |
|------------------|------------------|----------|
| `config/**` | `01-config-settings` | 1 |
| `crates/app/src/lib.rs` (Settings-related) | `01-config-settings` | 1 |
| `crates/app/tests/settings_test.rs` | `01-config-settings` | 1 |
| `crates/coldvox-audio/src/**` | `02-audio-capture` | 2 |
| `crates/coldvox-vad/**` | `03-vad` | 3 |
| `crates/coldvox-vad-silero/**` | `03-vad` | 3 |
| `crates/coldvox-stt/**` | `04-stt` | 4 |
| `crates/coldvox-stt-vosk/**` | `04-stt` | 4 |
| `crates/app/src/runtime.rs` | `05-app-runtime-wav` | 5 |
| `crates/app/src/audio/wav_file_loader.rs` | `05-app-runtime-wav` | 5 |
| `crates/app/src/stt/tests/end_to_end_wav.rs` | `05-app-runtime-wav` | 5 |
| `crates/coldvox-text-injection/**` | `06-injection` | 6 |
| `crates/app/tests/**` (non-settings) | `07-testing` | 7 |
| `test/**` | `07-testing` | 7 |
| `crates/coldvox-telemetry/**` | `08-logging-observability` | 8 |
| Logging changes (scattered) | `08-logging-observability` | 8 |
| `docs/**`, `CHANGELOG*`, `README*` | `09-docs-changelog` | 9 |
| Ambiguous/mixed | `10-glue` (if needed) | 10 |

### Step 2.3: Interactive Split Example

```
Graphite will show:
───────────────────────────────────────
File: crates/app/src/lib.rs
Hunk 1 of 5

- pub fn load_settings() -> Result<Settings> {
+ pub fn load_settings(path: Option<PathBuf>) -> Result<Settings> {
+     let config_path = path.unwrap_or_else(|| {
+         std::env::var("COLDVOX_CONFIG_PATH")
+             .map(PathBuf::from)
+             .unwrap_or_else(|_| PathBuf::from("config/default.toml"))
+     });
+     Settings::from_file(&config_path)
  }

Select target branch:
1. Create new branch
2. Existing branch (if any)
───────────────────────────────────────

Response: 1 [Enter]
Branch name: 01-config-settings [Enter]
```

Continue for each hunk, following the path pattern table above.

---

## Phase 3: Verify & Reorder Stack

### Step 3.1: Visualize the Stack

```bash
gt log --oneline
```

**Expected Output:**
```
◯ 09-docs-changelog
  └─ 08-logging-observability
    └─ 07-testing
      └─ 06-injection
        └─ 05-app-runtime-wav
          └─ 04-stt
            └─ 03-vad
              └─ 02-audio-capture
                └─ 01-config-settings
                  └─ main
```

### Step 3.2: Reorder if Needed

If the order is incorrect:

```bash
gt reorder
```

This opens an editor showing:
```
01-config-settings
02-audio-capture
03-vad
04-stt
05-app-runtime-wav
06-injection
07-testing
08-logging-observability
09-docs-changelog
```

Rearrange lines, save, and exit. Graphite will rebase automatically.

### Step 3.3: Ensure Proper Base

```bash
# Make sure 01-config-settings is based on main
gt checkout 01-config-settings
gt move --onto main
```

---

## Phase 4: Per-Branch Validation

For **each branch** in the stack (01 → 09):

```bash
# Checkout the branch
gt checkout 01-config-settings  # replace with actual branch name

# Validate build
cargo check --workspace

# Run tests
cargo test --workspace

# Check for warnings
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt -- --check

# Document findings
echo "✓ 01-config-settings: build passes, tests pass" >> /tmp/validation.log

# Move to next branch
gt up
```

**Critical:** If a branch fails validation:
- Fix issues in that branch
- Commit fixes: `git commit -am "fix: resolve validation issues"`
- Re-run validation
- Continue to next branch

---

## Phase 5: Push & Create PRs

### Step 5.1: Push All Branches

```bash
# From any branch in the stack:
git push --all
```

**Expected Output:**
```
Counting objects: 245, done.
Delta compression using up to 8 threads.
Compressing objects: 100% (134/134), done.
Writing objects: 100% (245/245), 67.23 KiB | 6.72 MiB/s, done.
Total 245 (delta 98), reused 0 (delta 0)
To github.com:Coldaine/ColdVox.git
 * [new branch]      01-config-settings -> 01-config-settings
 * [new branch]      02-audio-capture -> 02-audio-capture
 * [new branch]      03-vad -> 03-vad
 * [new branch]      04-stt -> 04-stt
 * [new branch]      05-app-runtime-wav -> 05-app-runtime-wav
 * [new branch]      06-injection -> 06-injection
 * [new branch]      07-testing -> 07-testing
 * [new branch]      08-logging-observability -> 08-logging-observability
 * [new branch]      09-docs-changelog -> 09-docs-changelog
```

### Step 5.2: Create PRs with Graphite (Option A)

If using Graphite Cloud:

```bash
gt submit
```

This automatically creates PRs with correct base branches.

### Step 5.3: Create PRs Manually (Option B)

If not using Graphite Cloud:

```bash
# PR #1: 01-config-settings
gh pr create \
  --base main \
  --head 01-config-settings \
  --title "[01] config: centralize Settings + path-aware load" \
  --body "$(cat <<'EOF'
## Summary
Centralizes configuration loading with path-aware logic and environment variable overrides.

## Scope
- `crates/app/src/lib.rs`: Settings API
- `config/**`: TOML files
- `crates/app/tests/settings_test.rs`: Test updates

## Dependencies
- Base: `main`
- Blocks: PR #02 (audio-capture)

## Testing
- [x] `cargo test --test settings_test`
- [x] Config file validation

## Checklist
- [x] Build passes
- [x] Tests pass
- [x] Clippy clean
- [x] Documentation updated
EOF
)"

# PR #2: 02-audio-capture
gh pr create \
  --base 01-config-settings \
  --head 02-audio-capture \
  --title "[02] audio: capture lifecycle fix + ALSA stderr suppression" \
  --body "$(cat <<'EOF'
## Summary
Fixes audio capture thread lifecycle and suppresses ALSA stderr noise.

## Scope
- `crates/coldvox-audio/src/**`

## Dependencies
- Base: PR #01 (config-settings)
- Blocks: PR #03 (vad), PR #04 (stt)

## Testing
- [x] `cargo run --bin mic_probe -- --duration 30`
- [x] PipeWire FPS check

## Checklist
- [x] Build passes
- [x] Tests pass
- [x] Clippy clean
EOF
)"

# PR #3: 03-vad
gh pr create \
  --base 02-audio-capture \
  --head 03-vad \
  --title "[03] vad: windowing/debounce consistency" \
  --body "$(cat <<'EOF'
## Summary
Frame-based VAD debouncing for deterministic testing.

## Scope
- `crates/coldvox-vad/**`
- `crates/coldvox-vad-silero/**`

## Dependencies
- Base: PR #02 (audio-capture)
- Blocks: PR #05 (app-runtime-wav)

## Testing
- [x] `cargo run --example test_silero_wav`
- [x] VAD determinism tests

## Checklist
- [x] Build passes
- [x] Tests pass
- [x] Clippy clean
EOF
)"

# PR #4: 04-stt
gh pr create \
  --base 02-audio-capture \
  --head 04-stt \
  --title "[04] stt: finalize handling + helpers" \
  --body "$(cat <<'EOF'
## Summary
STT finalization behavior and helper utilities.

## Scope
- `crates/coldvox-stt/**`
- `crates/coldvox-stt-vosk/**`

## Dependencies
- Base: PR #02 (audio-capture)
- Blocks: PR #05 (app-runtime-wav)

## Testing
- [x] `cargo run --features vosk --example vosk_test`
- [x] STT processor tests

## Checklist
- [x] Build passes
- [x] Tests pass
- [x] Clippy clean
EOF
)"

# PR #5: 05-app-runtime-wav
# NOTE: This PR depends on BOTH #03 and #04. Wait for both to merge before creating,
# or base on whichever merges first, then rebase after the second merges.
gh pr create \
  --base 02-audio-capture \
  --head 05-app-runtime-wav \
  --title "[05] app: unify VAD↔STT runtime + real WAV loader" \
  --body "$(cat <<'EOF'
## Summary
Unifies VAD/STT pipeline in runtime and adds deterministic WAV streaming.

## Scope
- `crates/app/src/runtime.rs`
- `crates/app/src/audio/wav_file_loader.rs`
- E2E test integration

## Dependencies
- Base: PR #02 (audio-capture) - Will need rebase after #03 and #04 merge
- Requires: PR #03 (vad), PR #04 (stt) BOTH merged
- Blocks: PR #06 (injection)

## Testing
- [x] `cargo test -p coldvox-app test_end_to_end_wav --nocapture`
- [x] Runtime integration tests

## Checklist
- [x] Build passes
- [x] Tests pass
- [x] Clippy clean
EOF
)"

# PR #6: 06-injection
gh pr create \
  --base 05-app-runtime-wav \
  --head 06-injection \
  --title "[06] injection: clipboard-preserve + Wayland-first strategy" \
  --body "$(cat <<'EOF'
## Summary
Refactors text injection with clipboard preservation and Wayland-first strategy.

## Scope
- `crates/coldvox-text-injection/**`

## Dependencies
- Base: PR #05 (app-runtime-wav)
- Blocks: PR #07 (testing)

## Testing
- [x] `cargo test -p coldvox-text-injection`
- [x] Integration tests with strategy manager

## Checklist
- [x] Build passes
- [x] Tests pass
- [x] Clippy clean
EOF
)"

# PR #7: 07-testing
gh pr create \
  --base 06-injection \
  --head 07-testing \
  --title "[07] tests: deterministic E2E + integration suites" \
  --body "$(cat <<'EOF'
## Summary
Consolidates deterministic testing infrastructure.

## Scope
- `**/tests/**`
- E2E WAV tests

## Dependencies
- Base: PR #06 (injection)
- Blocks: PR #08 (logging)

## Testing
- [x] Full test suite: `cargo test --workspace`
- [x] E2E tests with WAV files

## Checklist
- [x] Build passes
- [x] Tests pass
- [x] Clippy clean
EOF
)"

# PR #8: 08-logging-observability
gh pr create \
  --base 07-testing \
  --head 08-logging-observability \
  --title "[08] logs: prune noisy hot paths; telemetry tweaks" \
  --body "$(cat <<'EOF'
## Summary
Reduces hot-path logging noise and improves observability.

## Scope
- `crates/coldvox-telemetry/**`
- Scattered logging changes

## Dependencies
- Base: PR #07 (testing)
- Blocks: PR #09 (docs)

## Testing
- [x] `cargo run --bin tui_dashboard -- --log-level debug`
- [x] Log output validation

## Checklist
- [x] Build passes
- [x] Tests pass
- [x] Clippy clean
EOF
)"

# PR #9: 09-docs-changelog
gh pr create \
  --base 08-logging-observability \
  --head 09-docs-changelog \
  --title "[09] docs: changelog + guides + fixes" \
  --body "$(cat <<'EOF'
## Summary
Updates documentation, changelog, and deployment guides.

## Scope
- `docs/**`
- `CHANGELOG.md`
- `README.md`

## Dependencies
- Base: PR #08 (logging)
- Blocks: None (final PR)

## Testing
- [x] Link validation
- [x] Documentation accuracy review

## Checklist
- [x] Build passes
- [x] Docs accurate
- [x] Links valid
EOF
)"
```

---

## Phase 6: Post-Merge Maintenance

### After PR #1 Merges

```bash
# Sync local branches
gt sync

# If conflicts occur:
gt checkout 02-audio-capture
gt restack

# Resolve conflicts manually, then:
git add -A
gt continue

# Push updates
git push --force-with-lease
```

### After PR #2 Merges

```bash
gt sync

# PRs #3 and #4 may need rebase (both depend on #2)
gt checkout 03-vad
gt restack
# resolve conflicts if any
git add -A && gt continue

gt checkout 04-stt
gt restack
# resolve conflicts if any
git add -A && gt continue

git push --force-with-lease --all
```

### Continue Pattern

Repeat for each merge until all PRs are landed.

---

## Troubleshooting

### Issue: `gt split` creates unexpected branches

**Solution:**
```bash
# Undo split (safe, no data loss)
gt fold  # merges child back into parent

# Re-run split with --by-commit first
gt split --by-commit  # if commits are already clustered by domain
```

### Issue: Branch order is wrong after split

**Solution:**
```bash
gt reorder  # interactive editor

# Or manually re-parent:
gt checkout 03-vad
gt move --onto 02-audio-capture
```

### Issue: Validation fails on a branch

**Solution:**
```bash
# Fix in that branch
gt checkout 02-audio-capture
# make fixes
git commit -am "fix: resolve clippy warnings"

# Re-validate
cargo test

# Continue to next branch
gt up
```

### Issue: Merge conflict during `gt restack`

**Solution:**
```bash
# Graphite pauses for manual resolution
git status  # shows conflicted files

# Resolve conflicts manually in editor
# Then:
git add -A
gt continue  # resumes restack operation
```

### Issue: Need to insert a new branch mid-stack

**Solution:**
```bash
# Say you need to insert "00-hotfix-clipboard" before "01-config"
gt checkout 01-config-settings
gt create --insert --message "hotfix: clipboard P0 fix"

# This inserts new branch between current and parent
# Result: main → 00-hotfix → 01-config → ...
```

---

## Success Criteria

- [ ] All 9 branches pushed to GitHub
- [ ] All 9 PRs created with correct base branches
- [ ] Each PR has clear title, description, and scope
- [ ] Each PR passes CI (build + tests + lint)
- [ ] Dependency graph matches Plan 2 architecture
- [ ] No cross-cutting changes (each PR modifies 1-2 crates)
- [ ] Stack visualized correctly with `gt log`

---

## Timeline Estimate

| Phase | Time | Notes |
|-------|------|-------|
| Pre-flight setup | 15 min | Install Graphite, create backup |
| Interactive split | 60-90 min | Requires careful hunk assignment |
| Stack reordering | 10 min | Usually automatic |
| Per-branch validation | 90 min | 9 branches × 10 min each |
| Push & PR creation | 30 min | 9 PRs with descriptions |
| **Total** | **3.5-4 hours** | First-time execution |

**Repeat execution:** ~90 minutes (familiarity with tools + patterns)

---

## Next Steps After Stack Creation

1. **Assign reviewers** per domain expertise
2. **Monitor CI** for each PR
3. **Address feedback** incrementally (use `gt amend` for fixups)
4. **Merge bottom-up**: PR #1 → PR #2 → ... → PR #9
5. **Run `gt sync`** after each merge
6. **Celebrate** when all PRs land! 🎉

---

**Author:** GitHub Copilot Coding Agent
**Last Updated:** 2024-10-07
