# AGENTS.md

Canonical AI agent instructions for ColdVox.

## Repo Truth

- Windows 11 is the priority environment.
- The checked-in startup config is intentionally test-friendly: `config/default.toml` defaults to `mock`.
- The supported Windows live path for this wave is `COLDVOX_CONFIG_PATH=config/windows-parakeet.toml` plus `PARAKEET_MODEL_PATH` on NVIDIA/CUDA hardware.
- `config/plugins.json` is plugin-manager persistence, not the operator-facing startup source of truth.
- The Windows GUI is out of scope for this wave. `coldvox-gui` is a stub smoke target only.
- CI is not the authoritative gate for this wave. Local Windows validation is.

## Recent Branch Reality

- `main` contains the current trunk we should build on.
- The old Qt remediation line from closed PR `#389` is not a valid base for new work.
- Start fresh branches from `origin/main` or from the current stacked branch tip, never from the old remediation line.

## Commands

Crate-scoped commands are still preferred for iteration:

```powershell
cargo check -p coldvox-stt
cargo test -p coldvox-foundation --lib --locked
cargo test -p coldvox-app --test golden_master --locked
cargo fmt --all -- --check
```

Windows local validation:

```powershell
just windows-run-preflight
just windows-smoke
just test
```

Optional live validation during the Windows test gate:

```powershell
$env:COLDVOX_RUN_WINDOWS_LIVE = '1'
just test
```

Direct live validation:

```powershell
just windows-live
```

## Working Rules

- Prefer the local Windows gate over CI status for this wave.
- Keep the default path deterministic for tests; do not flip the checked-in default away from `mock`.
- Only claim the Windows live path is validated when you have local artifacts under `logs/windows-validation/`.
- Do not describe the GUI as Windows-ready.
