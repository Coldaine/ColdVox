# Windows Live Runbook

This runbook is the operator path for the current Windows validation wave.

## Prerequisites

- Windows 11
- NVIDIA GPU with working CUDA support
- A downloaded local Parakeet model directory
- `PARAKEET_MODEL_PATH` pointing at that model directory

## Commands

Preflight:

```powershell
just windows-run-preflight
```

Smoke:

```powershell
just windows-smoke
```

Required local test gate:

```powershell
just test
```

Opt into the live runtime during the test gate:

```powershell
$env:COLDVOX_RUN_WINDOWS_LIVE = '1'
just test
```

Run the live runtime directly:

```powershell
just windows-live
```

## Artifacts

Each validation run writes artifacts to:

```text
logs/windows-validation/<timestamp>-<mode>/
```

That directory contains:

- captured stdout
- captured stderr
- `summary.txt`
- copied runtime log files when the live runtime starts

## Review / Merge Protocol

For this wave, local artifacts are the review gate.

1. Run the relevant local Windows commands and keep the artifact path.
2. Put the exact commands, hardware assumptions, and artifact path in the PR description.
3. Wait 5 minutes for review comments before merging.
4. Re-run the relevant local gate after addressing review feedback.

CI is not the release gate for this wave.
