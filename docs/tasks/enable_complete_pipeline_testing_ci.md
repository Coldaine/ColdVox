# Enable Complete Pipeline Testing in CI

## Execution Plan
**SCHEDULED FOR EXECUTION:** Evening of 09/04/25
**STATUS:** Pending Implementation

## Objective

## Current Situation
- `vosk-integration.yml` workflow already exists and runs the end-to-end test
- It only triggers on specific paths, not all PRs
- The test is marked `#[ignore]` and needs `--ignored` flag
- Test expects WAV files that don't exist in the expected location
- With 2000 CI minutes/month available, there's plenty of room for comprehensive testing

## Simple Fixes Needed

### 1. Make vosk-integration.yml Run on All PRs
Change trigger from path-specific to all PRs:
```yaml
on:
  pull_request:  # Remove path restrictions
  push:
    branches: [main, develop]
  schedule:
    - cron: "0 0 * * 0"
  workflow_dispatch:
```

### 2. Fix Test Data Location
The test expects either:
- `TEST_WAV` environment variable
- `test_data/sample.wav`
- Or uses existing WAV files in root

Create proper test data:
- Move `test_audio_16k.wav` to `crates/app/test_data/test_audio_16k.wav`
- Update test to use correct path

### 3. Remove #[ignore] Attribute
In `end_to_end_wav.rs`, change:
```rust
#[tokio::test]
#[ignore]  // Remove this
async fn test_end_to_end_wav_pipeline() {
```

### 4. Update CI Test Command
Remove `--ignored` flag since test won't be ignored:
```yaml
cargo test --locked -p coldvox-app --features vosk test_end_to_end_wav_pipeline -- --nocapture
```

### 5. Add to Main CI Workflow
Add Vosk test variant to main CI:
```yaml
test:
  strategy:
    matrix:
      features:
        - "silero"
        - "silero,text-injection"
        - "silero,vosk,text-injection"  # Add full pipeline
```

## Resource Impact
With 2000 CI minutes/month, running the full pipeline test on every PR is completely reasonable:
- Vosk model download: cached after first run
- Test execution: ~30 seconds
- Total CI time per PR: minimal impact

## Summary
No need to overthink this - just enable the existing test properly. The infrastructure is already there, it just needs to be activated.

## Implementation Instructions

### Branch and PR Management
1. **Create a dedicated branch for this work:**
   ```bash
   git checkout -b feature/enable-complete-pipeline-ci-testing
   ```

2. **Make all changes in this branch** following the plan above

3. **Create a pull request** with the following:
   - Title: "Enable complete pipeline testing in CI"
   - Description: Reference this documentation and summarize all changes
   - Ensure all CI checks pass before requesting review

4. **Test the changes** by triggering the updated workflows manually before merging

### Post-Implementation Requirements
1. **This documentation requires signoff after implementation**
2. **Update STATUS above** to "Completed" with implementation date
3. **Add implementation notes** documenting any deviations from the plan
4. **Verify** that the complete pipeline tests are running successfully on PRs

### Signoff Section
- [ ] Implementation completed by: ________________ Date: __________
- [ ] All changes tested and verified by: ________________ Date: __________
- [ ] Documentation reviewed and approved by: ________________ Date: __________
