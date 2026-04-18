# Windows Live Runbook

This runbook is the operator path for the current Windows validation wave.

## Prerequisites

- Windows 11
- NVIDIA GPU with working CUDA support
- A downloaded local Parakeet model directory
- Either:
  - `PARAKEET_MODEL_PATH` pointing at that directory, or
  - the model living in one of the shared/local discovery roots that the validator checks first

## Model Discovery

`just windows-run-preflight` and `just windows-live` search for Parakeet in this order:

1. `PARAKEET_MODEL_PATH`
2. The local Parakeet cache
3. Shared `D:\AIModels\speech\...` roots
4. Standard Hugging Face cache roots such as `HF_HUB_CACHE`, `HF_HOME\hub`, and the user's local Hugging Face cache

Use `PARAKEET_MODEL_PATH` only when the model lives somewhere outside those normal roots.

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
