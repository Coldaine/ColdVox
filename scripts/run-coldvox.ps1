#!/usr/bin/env pwsh

# Ensure the Python DLL from the UV-managed environment is in the PATH.
# This fixes the STATUS_DLL_NOT_FOUND (0xc0000135) crash.

Write-Host "==> Detecting Python environment..." -ForegroundColor Blue
$base = uv run python -c "import sys; print(sys.base_prefix)"

if (-not $base) {
    Write-Error "Could not find UV-managed Python. Run 'uv sync' first."
    exit 1
}

Write-Host "==> Adding $base to PATH..." -ForegroundColor Blue
$env:PATH = "$base;$env:PATH"

Write-Host "==> Starting ColdVox with Moonshine STT..." -ForegroundColor Green
Set-Location crates/app
cargo run --bin coldvox --features moonshine,text-injection
