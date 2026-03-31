# PyO3 Dependency Audit Script for ColdVox (Windows PowerShell)
# This script automates the dependency audit process outlined in PYO3_DEPENDENCY_AUDIT_PLAN.md
#
# Usage: .\scripts\pyo3_audit.ps1 [-Phase PHASE] [-Output FILE] [-Verbose]
#
# Options:
#   -Phase PHASE    Run specific phase (1-6, or 'all')
#   -Output FILE    Output file for report (default: audit_report.md)
#   -Verbose        Enable verbose output
#   -Help           Show this help message

param(
    [string]$Phase = "all",
    [string]$Output = "audit_report.md",
    [switch]$Verbose,
    [switch]$Help
)

# Set error action preference
$ErrorActionPreference = "Continue"

# Colors for output
$Colors = @{
    Info = "Cyan"
    Success = "Green"
    Warning = "Yellow"
    Error = "Red"
    Verbose = "Gray"
}

# Logging functions
function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor $Colors.Info
}

function Write-Success {
    param([string]$Message)
    Write-Host "[SUCCESS] $Message" -ForegroundColor $Colors.Success
}

function Write-Warning {
    param([string]$Message)
    Write-Host "[WARNING] $Message" -ForegroundColor $Colors.Warning
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor $Colors.Error
}

function Write-VerboseLog {
    param([string]$Message)
    if ($Verbose) {
        Write-Host "[VERBOSE] $Message" -ForegroundColor $Colors.Verbose
    }
}

# Help message
function Show-Help {
    @"
PyO3 Dependency Audit Script for ColdVox (Windows PowerShell)

Usage: .\pyo3_audit.ps1 [OPTIONS]

Options:
    -Phase PHASE    Run specific phase (1-6, or 'all')
                    1: Environment Snapshot
                    2: Dependency Tree Analysis
                    3: DLL & Shared Library Mapping
                    4: PyO3 Environment Checks
                    5: DLL_NOT_FOUND Troubleshooting
                    6: Generate Report
                    all: Run all phases (default)
    
    -Output FILE    Output file for report (default: audit_report.md)
    -Verbose        Enable verbose output
    -Help           Show this help message

Examples:
    .\pyo3_audit.ps1                          # Run all phases
    .\pyo3_audit.ps1 -Phase 1                 # Run only phase 1
    .\pyo3_audit.ps1 -Phase 1-3               # Run phases 1-3
    .\pyo3_audit.ps1 -Output my_report.md     # Custom output file

"@
}

# Get script directory and project root
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

# Initialize report
function Initialize-Report {
    Write-Info "Initializing audit report: $Output"
    
    $reportContent = @"
# ColdVox PyO3 Dependency Audit Report

**Date:** $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')
**Auditor:** Automated Audit Script (PowerShell)
**Environment:** $($env:OS) $($env:PROCESSOR_ARCHITECTURE)

---

"@
    
    Set-Content -Path $Output -Value $reportContent -Encoding UTF8
}

# Phase 1: Environment Snapshot
function Invoke-Phase1 {
    Write-Info "Phase 1: Capturing environment snapshot..."
    
    $phaseContent = @"

## 1. Environment Snapshot

### Python Environment

"@
    
    Add-Content -Path $Output -Value $phaseContent -Encoding UTF8
    
    # Python version and architecture
    Write-VerboseLog "Checking Python version..."
    try {
        $pythonVersion = python --version 2>&1
        Add-Content -Path $Output -Value $pythonVersion -Encoding UTF8
        
        $pythonInfo = python -c "
import sys
print(f'Executable: {sys.executable}')
print(f'Version: {sys.version}')
print(f'Architecture: {\"64-bit\" if sys.maxsize > 2**32 else \"32-bit\"}')
print(f'Platform: {sys.platform}')
" 2>&1
        Add-Content -Path $Output -Value "`n$pythonInfo" -Encoding UTF8
    }
    catch {
        Add-Content -Path $Output -Value "Python not found in PATH" -Encoding UTF8
    }
    
    # Virtual environment detection
    Add-Content -Path $Output -Value "`n### Virtual Environment`n" -Encoding UTF8
    try {
        $venvInfo = python -c "
import sys
print(f'Prefix: {sys.prefix}')
print(f'Base Prefix: {sys.base_prefix}')
print(f'In venv: {sys.prefix != sys.base_prefix}')
" 2>&1
        Add-Content -Path $Output -Value $venvInfo -Encoding UTF8
    }
    catch {
        Add-Content -Path $Output -Value "Could not detect virtual environment" -Encoding UTF8
    }
    
    # Environment variables
    Add-Content -Path $Output -Value "`n### Environment Variables`n" -Encoding UTF8
    Add-Content -Path $Output -Value "PYTHONHOME: $($env:PYTHONHOME ?? '<unset>')" -Encoding UTF8
    Add-Content -Path $Output -Value "PYTHONPATH: $($env:PYTHONPATH ?? '<unset>')" -Encoding UTF8
    Add-Content -Path $Output -Value "VIRTUAL_ENV: $($env:VIRTUAL_ENV ?? '<unset>')" -Encoding UTF8
    Add-Content -Path $Output -Value "CONDA_PREFIX: $($env:CONDA_PREFIX ?? '<unset>')" -Encoding UTF8
    
    # Python packages
    Add-Content -Path $Output -Value "`n### Installed Python Packages`n" -Encoding UTF8
    try {
        if (Get-Command uv -ErrorAction SilentlyContinue) {
            Write-VerboseLog "Using uv to list packages..."
            $packages = uv pip list 2>&1
            Add-Content -Path $Output -Value $packages -Encoding UTF8
        }
        elseif (Get-Command pip -ErrorAction SilentlyContinue) {
            Write-VerboseLog "Using pip to list packages..."
            $packages = pip list 2>&1
            Add-Content -Path $Output -Value $packages -Encoding UTF8
        }
        else {
            Add-Content -Path $Output -Value "No package manager found" -Encoding UTF8
        }
    }
    catch {
        Add-Content -Path $Output -Value "Could not list packages: $_" -Encoding UTF8
    }
    
    # Rust toolchain
    Add-Content -Path $Output -Value "`n### Rust Toolchain`n" -Encoding UTF8
    try {
        $rustVersion = rustc --version 2>&1
        $cargoVersion = cargo --version 2>&1
        Add-Content -Path $Output -Value $rustVersion -Encoding UTF8
        Add-Content -Path $Output -Value $cargoVersion -Encoding UTF8
    }
    catch {
        Add-Content -Path $Output -Value "Rust not found in PATH" -Encoding UTF8
    }
    
    # System information
    Add-Content -Path $Output -Value "`n### System Information`n" -Encoding UTF8
    $sysInfo = systeminfo | Select-String "OS Name", "OS Version", "System Type"
    Add-Content -Path $Output -Value ($sysInfo -join "`n") -Encoding UTF8
    
    Write-Success "Phase 1 completed"
}

# Phase 2: Dependency Tree Analysis
function Invoke-Phase2 {
    Write-Info "Phase 2: Analyzing dependency tree..."
    
    Add-Content -Path $Output -Value "`n---`n" -Encoding UTF8
    Add-Content -Path $Output -Value "## 2. Dependency Tree Analysis`n" -Encoding UTF8
    Add-Content -Path $Output -Value "### Python Dependency Tree`n" -Encoding UTF8
    
    # Install pipdeptree if not present
    if (-not (Get-Command pipdeptree -ErrorAction SilentlyContinue)) {
        Write-VerboseLog "Installing pipdeptree..."
        try {
            if (Get-Command uv -ErrorAction SilentlyContinue) {
                uv pip install pipdeptree 2>&1 | Out-Null
            }
            else {
                pip install pipdeptree 2>&1 | Out-Null
            }
        }
        catch {
            Add-Content -Path $Output -Value "Could not install pipdeptree" -Encoding UTF8
        }
    }
    
    # Generate dependency tree
    if (Get-Command pipdeptree -ErrorAction SilentlyContinue) {
        $depTree = pipdeptree --warn silence 2>&1
        Add-Content -Path $Output -Value $depTree -Encoding UTF8
    }
    else {
        Add-Content -Path $Output -Value "pipdeptree not available" -Encoding UTF8
    }
    
    # Identify PyO3 dependencies
    Add-Content -Path $Output -Value "`n### PyO3/Rust Dependencies`n" -Encoding UTF8
    if (Test-Path "$ProjectRoot\Cargo.lock") {
        $pyo3Deps = Select-String -Path "$ProjectRoot\Cargo.lock" -Pattern "pyo3" -SimpleMatch 2>&1
        if ($pyo3Deps) {
            Add-Content -Path $Output -Value ($pyo3Deps -join "`n") -Encoding UTF8
        }
        else {
            Add-Content -Path $Output -Value "No PyO3 found in Cargo.lock" -Encoding UTF8
        }
    }
    else {
        Add-Content -Path $Output -Value "Cargo.lock not found" -Encoding UTF8
    }
    
    # List Python packages with native extensions
    Add-Content -Path $Output -Value "`n### Packages with Native Extensions`n" -Encoding UTF8
    try {
        $nativePackages = python -c "
import pkg_resources
native_indicators = ['numpy', 'torch', 'scipy', 'librosa', 'transformers', 'cffi', 'pycparser']
for pkg in pkg_resources.working_set:
    if any(ind in pkg.project_name.lower() for ind in native_indicators):
        print(f'{pkg.project_name} {pkg.version}')
" 2>&1
        Add-Content -Path $Output -Value $nativePackages -Encoding UTF8
    }
    catch {
        Add-Content -Path $Output -Value "Could not identify native packages" -Encoding UTF8
    }
    
    Write-Success "Phase 2 completed"
}

# Phase 3: DLL & Shared Library Mapping
function Invoke-Phase3 {
    Write-Info "Phase 3: Mapping native libraries..."
    
    Add-Content -Path $Output -Value "`n---`n" -Encoding UTF8
    Add-Content -Path $Output -Value "## 3. Native Library Mapping`n" -Encoding UTF8
    Add-Content -Path $Output -Value "### Python Extension Modules`n" -Encoding UTF8
    
    # Find Python extension modules
    try {
        $pythonPrefix = python -c "import sys; print(sys.prefix)" 2>&1
        Write-VerboseLog "Searching for native libraries in: $pythonPrefix"
        
        # Find .dll and .pyd files
        $dllFiles = Get-ChildItem -Path $pythonPrefix -Recurse -Include "*.dll", "*.pyd" -ErrorAction SilentlyContinue | 
            Select-Object -First 50 FullName, Length, LastWriteTime
        
        if ($dllFiles) {
            Add-Content -Path $Output -Value "| Library | Size | Last Modified |" -Encoding UTF8
            Add-Content -Path $Output -Value "|---------|------|---------------|" -Encoding UTF8
            foreach ($dll in $dllFiles) {
                $size = [math]::Round($dll.Length / 1KB, 2)
                Add-Content -Path $Output -Value "| $($dll.FullName) | ${size} KB | $($dll.LastWriteTime) |" -Encoding UTF8
            }
        }
        else {
            Add-Content -Path $Output -Value "No .dll/.pyd files found" -Encoding UTF8
        }
    }
    catch {
        Add-Content -Path $Output -Value "Could not determine Python prefix" -Encoding UTF8
    }
    
    # Check Rust build output
    Add-Content -Path $Output -Value "`n### Rust/PyO3 Build Output`n" -Encoding UTF8
    if (Test-Path "$ProjectRoot\target") {
        $rustLibs = Get-ChildItem -Path "$ProjectRoot\target" -Recurse -Include "*.dll", "*.pyd" -ErrorAction SilentlyContinue |
            Select-Object -First 20 FullName
        
        if ($rustLibs) {
            Add-Content -Path $Output -Value ($rustLibs.FullName -join "`n") -Encoding UTF8
        }
        else {
            Add-Content -Path $Output -Value "No PyO3 libraries found" -Encoding UTF8
        }
    }
    else {
        Add-Content -Path $Output -Value "target directory not found" -Encoding UTF8
    }
    
    Write-Success "Phase 3 completed"
}

# Phase 4: PyO3 Environment Checks
function Invoke-Phase4 {
    Write-Info "Phase 4: Verifying PyO3 environment..."
    
    Add-Content -Path $Output -Value "`n---`n" -Encoding UTF8
    Add-Content -Path $Output -Value "## 4. PyO3 Environment Verification`n" -Encoding UTF8
    Add-Content -Path $Output -Value "### Python Interpreter Consistency`n" -Encoding UTF8
    
    # Create temporary Python script
    $tempScript = [System.IO.Path]::GetTempFileName() + ".py"
    @'
import sys
import os

print("Executable:", sys.executable)
print("Version:", sys.version)
print("Version Info:", sys.version_info)
print("Platform:", sys.platform)
print("Architecture:", "64-bit" if sys.maxsize > 2**32 else "32-bit")

print("\n=== Environment Variables ===")
print("PYTHONHOME:", os.environ.get('PYTHONHOME', '<unset>'))
print("PYTHONPATH:", os.environ.get('PYTHONPATH', '<unset>'))
print("VIRTUAL_ENV:", os.environ.get('VIRTUAL_ENV', '<unset>'))

print("\n=== Path Analysis ===")
python_exe = sys.executable
python_prefix = sys.prefix
print("Python executable:", python_exe)
print("Python prefix:", python_prefix)

# Check for multiple Python installations
import subprocess
try:
    result = subprocess.run(['where', 'python'], capture_output=True, text=True)
    print("\n=== All Python executables in PATH ===")
    print(result.stdout)
except Exception as e:
    print(f"Could not check PATH: {e}")
'@ | Set-Content -Path $tempScript -Encoding UTF8
    
    try {
        $pythonEnv = python $tempScript 2>&1
        Add-Content -Path $Output -Value $pythonEnv -Encoding UTF8
    }
    catch {
        Add-Content -Path $Output -Value "Python environment check failed" -Encoding UTF8
    }
    finally {
        Remove-Item -Path $tempScript -ErrorAction SilentlyContinue
    }
    
    # Check PYTHONHOME
    Add-Content -Path $Output -Value "`n### PYTHONHOME Check`n" -Encoding UTF8
    if ($env:PYTHONHOME) {
        Add-Content -Path $Output -Value "WARNING: PYTHONHOME is set to: $env:PYTHONHOME" -Encoding UTF8
        Add-Content -Path $Output -Value "PyO3 may fail to initialize Python correctly." -Encoding UTF8
        Add-Content -Path $Output -Value "Recommendation: Remove PYTHONHOME environment variable" -Encoding UTF8
    }
    else {
        Add-Content -Path $Output -Value "PYTHONHOME is not set (good)" -Encoding UTF8
    }
    
    Write-Success "Phase 4 completed"
}

# Phase 5: DLL_NOT_FOUND Troubleshooting
function Invoke-Phase5 {
    Write-Info "Phase 5: Troubleshooting DLL_NOT_FOUND..."
    
    Add-Content -Path $Output -Value "`n---`n" -Encoding UTF8
    Add-Content -Path $Output -Value "## 5. DLL_NOT_FOUND Troubleshooting`n" -Encoding UTF8
    Add-Content -Path $Output -Value "### Missing Library Detection`n" -Encoding UTF8
    
    Add-Content -Path $Output -Value "Windows: Use Process Monitor or Dependency Walker for DLL analysis" -Encoding UTF8
    Add-Content -Path $Output -Value "See PYO3_DEPENDENCY_AUDIT_PLAN.md Phase 5 for instructions`n" -Encoding UTF8
    
    # Check Visual C++ Redistributables
    Add-Content -Path $Output -Value "### Visual C++ Redistributables`n" -Encoding UTF8
    $vcRedist = Get-WmiObject -Class Win32_Product | Where-Object {$_.Name -like "*Visual C++*"} | 
        Select-Object Name, Version
    
    if ($vcRedist) {
        Add-Content -Path $Output -Value "| Name | Version |" -Encoding UTF8
        Add-Content -Path $Output -Value "|------|---------|" -Encoding UTF8
        foreach ($vc in $vcRedist) {
            Add-Content -Path $Output -Value "| $($vc.Name) | $($vc.Version) |" -Encoding UTF8
        }
    }
    else {
        Add-Content -Path $Output -Value "No Visual C++ Redistributables found" -Encoding UTF8
    }
    
    # Check for required VC++ runtime DLLs
    Add-Content -Path $Output -Value "`n### Required VC++ Runtime DLLs`n" -Encoding UTF8
    $vcDlls = @("msvcp140.dll", "vcruntime140.dll", "vcruntime140_1.dll")
    foreach ($dll in $vcDlls) {
        $found = Get-ChildItem -Path "C:\Windows\System32", "C:\Windows\SysWOW64" -Filter $dll -ErrorAction SilentlyContinue
        if ($found) {
            Add-Content -Path $Output -Value "✓ Found: $dll at $($found.FullName)" -Encoding UTF8
        }
        else {
            Add-Content -Path $Output -Value "✗ MISSING: $dll" -Encoding UTF8
        }
    }
    
    # Check system PATH
    Add-Content -Path $Output -Value "`n### System PATH Analysis`n" -Encoding UTF8
    $relevantPaths = $env:PATH -split ';' | Where-Object {$_ -match "(python|torch|cuda|msvc)"}
    if ($relevantPaths) {
        Add-Content -Path $Output -Value ($relevantPaths -join "`n") -Encoding UTF8
    }
    else {
        Add-Content -Path $Output -Value "No relevant paths found" -Encoding UTF8
    }
    
    Write-Success "Phase 5 completed"
}

# Phase 6: Generate Report
function Invoke-Phase6 {
    Write-Info "Phase 6: Generating final report..."
    
    $summaryContent = @"

---

## 6. Summary and Recommendations

### Critical Issues Found
[Review the sections above and list critical issues here]

### Recommended Actions
1. **Verify Python Environment**
   - Ensure PYTHONHOME is unset
   - Use uv to manage Python dependencies
   - Verify Python 3.10-3.12 is used

2. **Check Native Dependencies**
   - Install Visual C++ Redistributables
   - Verify CUDA libraries (if using GPU)
   - Check for missing .dll files

3. **Rebuild if Necessary**
   ``````bash
   cargo clean -p coldvox-stt
   cargo build -p coldvox-stt --features moonshine
   ``````

4. **Verify Installation**
   ``````bash
   uv sync
   cargo check -p coldvox-stt --features moonshine
   ``````

### Verification Commands
``````bash
# Check Python environment
uv run python -c "import sys; print(sys.executable)"

# Check PyTorch
uv run python -c "import torch; print(f'PyTorch {torch.__version__}')"

# Check ColdVox STT
cargo check -p coldvox-stt --features moonshine
``````

---

*Report generated by pyo3_audit.ps1 on $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')*
"@
    
    Add-Content -Path $Output -Value $summaryContent -Encoding UTF8
    
    Write-Success "Phase 6 completed"
    Write-Success "Report saved to: $Output"
}

# Main execution
function Main {
    Write-Info "Starting ColdVox PyO3 Dependency Audit"
    Write-Info "Project root: $ProjectRoot"
    Write-Info "Output file: $Output"
    Write-Info "Phase: $Phase"
    
    Set-Location $ProjectRoot
    
    # Initialize report
    Initialize-Report
    
    # Run requested phases
    switch ($Phase) {
        "all" {
            Invoke-Phase1
            Invoke-Phase2
            Invoke-Phase3
            Invoke-Phase4
            Invoke-Phase5
            Invoke-Phase6
        }
        "1" { Invoke-Phase1 }
        "2" { Invoke-Phase2 }
        "3" { Invoke-Phase3 }
        "4" { Invoke-Phase4 }
        "5" { Invoke-Phase5 }
        "6" { Invoke-Phase6 }
        default {
            # Handle range like 1-3
            if ($Phase -match "^(\d+)-(\d+)$") {
                $start = [int]$Matches[1]
                $end = [int]$Matches[2]
                for ($i = $start; $i -le $end; $i++) {
                    & "Invoke-Phase$i"
                }
            }
            else {
                Write-Error "Invalid phase: $Phase"
                Show-Help
                exit 1
            }
        }
    }
    
    Write-Success "Audit completed successfully!"
    Write-Info "Review the report at: $Output"
}

# Show help if requested
if ($Help) {
    Show-Help
    exit 0
}

# Run main function
Main
