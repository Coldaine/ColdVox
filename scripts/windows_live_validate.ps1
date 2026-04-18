#!/usr/bin/env pwsh

param(
    [ValidateSet('Preflight', 'Smoke', 'Live')]
    [string]$Mode = 'Live',

    [ValidateRange(1, 600)]
    [int]$RuntimeSeconds = 30,

    [ValidateRange(10, 2000)]
    [int]$TailLines = 200
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepoRoot = Split-Path -Parent $PSScriptRoot
$Timestamp = Get-Date -Format 'yyyyMMdd-HHmmss-fff'
$ArtifactRoot = Join-Path $RepoRoot "logs/windows-validation/$Timestamp-$($Mode.ToLowerInvariant())"
$ConfigPath = Join-Path $RepoRoot 'config/windows-parakeet.toml'
$PluginConfigPath = Join-Path $ArtifactRoot 'plugins.json'
$LogPath = Join-Path $RepoRoot 'logs/coldvox.log'
$SmokeFeatureList = 'silero,text-injection-enigo'
$LiveFeatureList = 'parakeet,silero,text-injection-enigo'
$ParakeetDevice = if ($env:PARAKEET_DEVICE) { $env:PARAKEET_DEVICE } else { 'cuda' }

function Write-Step {
    param([string]$Message)
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Write-Ok {
    param([string]$Message)
    Write-Host "OK: $Message" -ForegroundColor Green
}

function Write-Warn {
    param([string]$Message)
    Write-Host "WARN: $Message" -ForegroundColor Yellow
}

function Assert-Command {
    param([string]$Name)

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required command not found on PATH: $Name"
    }
}

function Add-UniqueCandidate {
    param(
        [System.Collections.Generic.List[string]]$Candidates,
        [string]$Path
    )

    if (-not [string]::IsNullOrWhiteSpace($Path) -and -not $Candidates.Contains($Path)) {
        $Candidates.Add($Path)
    }
}

function Resolve-HuggingFaceSnapshotDirs {
    param(
        [string]$HubRoot,
        [string]$ModelName
    )

    $repoDir = Join-Path $HubRoot ("models--" + ($ModelName -replace '/', '--'))
    $snapshotsDir = Join-Path $repoDir 'snapshots'

    if (-not (Test-Path $snapshotsDir)) {
        return @()
    }

    return Get-ChildItem $snapshotsDir -Directory |
        Sort-Object LastWriteTime -Descending |
        Select-Object -ExpandProperty FullName
}

function Get-ParakeetModelCandidates {
    $variant = if ($env:PARAKEET_VARIANT) { $env:PARAKEET_VARIANT.ToLowerInvariant() } else { 'tdt' }
    $modelName = switch ($variant) {
        'ctc' { 'nvidia/parakeet-ctc-1.1b' }
        default { 'nvidia/parakeet-tdt-1.1b' }
    }

    $candidates = New-Object 'System.Collections.Generic.List[string]'

    Add-UniqueCandidate $candidates $env:PARAKEET_MODEL_PATH
    Add-UniqueCandidate $candidates (Join-Path (Join-Path $env:LOCALAPPDATA 'parakeet') $modelName)

    $leafName = Split-Path $modelName -Leaf
    foreach ($root in @(
        'D:\AIModels\speech\stt',
        'D:\AIModels\speech'
    )) {
        Add-UniqueCandidate $candidates (Join-Path $root ($modelName -replace '/', '\'))
        Add-UniqueCandidate $candidates (Join-Path $root $leafName)
    }

    foreach ($hubRoot in @(
        'D:\AIModels\hf\.cache\hub',
        'D:\AIModels\hf\.hf_home\hub'
    )) {
        foreach ($snapshotDir in Resolve-HuggingFaceSnapshotDirs -HubRoot $hubRoot -ModelName $modelName) {
            Add-UniqueCandidate $candidates $snapshotDir
        }
    }

    return $candidates
}

function Resolve-ParakeetModelPath {
    if ($env:PARAKEET_MODEL_PATH) {
        if (-not (Test-Path $env:PARAKEET_MODEL_PATH)) {
            throw "PARAKEET_MODEL_PATH does not exist: $($env:PARAKEET_MODEL_PATH)"
        }

        return $env:PARAKEET_MODEL_PATH
    }

    foreach ($candidate in Get-ParakeetModelCandidates) {
        if (Test-Path $candidate) {
            $env:PARAKEET_MODEL_PATH = $candidate
            return $candidate
        }
    }

    throw 'Parakeet model not found. Checked PARAKEET_MODEL_PATH, the local parakeet cache, D:\AIModels shared speech roots, and HuggingFace caches. Set PARAKEET_MODEL_PATH explicitly if your model lives elsewhere.'
}

function New-ArtifactDirectories {
    New-Item -ItemType Directory -Force -Path $ArtifactRoot | Out-Null
}

function Write-PluginConfig {
    $pluginConfig = @{
        preferred_plugin = 'parakeet'
        fallback_plugins = @()
        require_local = $true
        max_memory_mb = $null
        required_language = 'en'
        failover = @{
            failover_threshold = 5
            failover_cooldown_secs = 10
        }
        gc_policy = @{
            model_ttl_secs = 300
            enabled = $true
        }
        metrics = @{
            log_interval_secs = 30
            debug_dump_events = $false
        }
        auto_extract_model = $true
    }

    $pluginConfig | ConvertTo-Json -Depth 8 | Set-Content -Path $PluginConfigPath -Encoding UTF8
}

function Invoke-LoggedProcess {
    param(
        [string]$Name,
        [string]$FilePath,
        [string]$WorkingDirectory,
        [string[]]$ArgumentList,
        [int]$TimeoutSeconds = 0
    )

    $stdout = Join-Path $ArtifactRoot "$Name.stdout.log"
    $stderr = Join-Path $ArtifactRoot "$Name.stderr.log"

    Write-Step $Name
    $process = Start-Process `
        -FilePath $FilePath `
        -ArgumentList $ArgumentList `
        -WorkingDirectory $WorkingDirectory `
        -RedirectStandardOutput $stdout `
        -RedirectStandardError $stderr `
        -PassThru

    $timedOut = $false
    if ($TimeoutSeconds -gt 0) {
        $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
        while (-not $process.HasExited -and (Get-Date) -lt $deadline) {
            Start-Sleep -Seconds 1
            $process.Refresh()
        }

        if (-not $process.HasExited) {
            $timedOut = $true
            Write-Warn "Timeout reached after $TimeoutSeconds seconds; stopping process tree"
            & taskkill.exe /PID $process.Id /T /F | Out-Null
            $process.WaitForExit()
        }
    } else {
        $process.WaitForExit()
    }

    if ($process.ExitCode -ne 0 -and -not $timedOut) {
        if (Test-Path $stderr) {
            Write-Host ""
            Write-Host "stderr tail:" -ForegroundColor Yellow
            Get-Content $stderr | Select-Object -Last $TailLines | ForEach-Object { Write-Host $_ }
        }

        throw "Command '$Name' failed with exit code $($process.ExitCode)."
    }

    foreach ($path in @($stdout, $stderr)) {
        if (Test-Path $path) {
            $tail = Get-Content $path | Select-Object -Last $TailLines
            if ($tail) {
                Write-Host ""
                Write-Host "$([System.IO.Path]::GetFileName($path)) tail:" -ForegroundColor DarkGray
                $tail | ForEach-Object { Write-Host $_ }
            }
        }
    }

    [pscustomobject]@{
        ExitCode = $process.ExitCode
        TimedOut = $timedOut
        StdoutPath = $stdout
        StderrPath = $stderr
    }
}

function Copy-RuntimeLog {
    if (-not (Test-Path $LogPath)) {
        Write-Warning "Runtime log not found at $LogPath."
        Set-Content (Join-Path $ArtifactRoot 'coldvox.log') -Value '' -Encoding UTF8
        Set-Content (Join-Path $ArtifactRoot 'coldvox.log.tail') -Value '' -Encoding UTF8
        return
    }

    Copy-Item $LogPath (Join-Path $ArtifactRoot 'coldvox.log') -Force
    Get-Content $LogPath | Select-Object -Last 120 | Set-Content (Join-Path $ArtifactRoot 'coldvox.log.tail') -Encoding UTF8
}

Assert-Command 'cargo'
Assert-Command 'taskkill.exe'
Assert-Command 'pwsh'

if (-not $IsWindows) {
    throw 'This validation wrapper only runs on Windows.'
}

if (-not (Test-Path $ConfigPath)) {
    throw "Missing Windows live profile: $ConfigPath"
}

function Invoke-Preflight {
    param(
        [switch]$RequireModel
    )

    Write-Step 'preflight'
    New-ArtifactDirectories
    Write-PluginConfig

    $gpuNames = Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name
    $hasNvidia = [bool]($gpuNames | Where-Object { $_ -match 'NVIDIA' })
    $hasNvidiaSmi = [bool](Get-Command nvidia-smi -ErrorAction SilentlyContinue)

    if (-not $hasNvidia -and -not $hasNvidiaSmi) {
        throw 'No NVIDIA/CUDA indicator found. This validation expects an NVIDIA GPU or nvidia-smi on PATH.'
    }

    if (-not $hasNvidiaSmi) {
        throw 'nvidia-smi is not available on PATH. This validation requires NVIDIA driver tooling with nvidia-smi accessible so Parakeet GPU initialization can be validated.'
    }

    Write-Step 'nvidia-smi'
    $nvidia = Invoke-LoggedProcess -Name 'nvidia-smi' -FilePath 'nvidia-smi' -WorkingDirectory $RepoRoot -ArgumentList @('-L')

    $modelPath = $null
    try {
        $modelPath = Resolve-ParakeetModelPath
        Write-Ok "Using Parakeet model at $modelPath"
    } catch {
        if ($RequireModel) {
            throw
        }

        Write-Warn $_.Exception.Message
    }

    [pscustomobject]@{
        GpuNames = $gpuNames
        HasNvidiaSmi = $hasNvidiaSmi
        ModelPath = $modelPath
    }
}

function Invoke-Smoke {
    $preflight = Invoke-Preflight

    $env:RUST_LOG = 'info'
    $env:COLDVOX_LOG_RETENTION_DAYS = '0'
    $env:COLDVOX_CONFIG_PATH = $ConfigPath
    $env:COLDVOX_PLUGIN_CONFIG_PATH = $PluginConfigPath
    $env:PARAKEET_DEVICE = $ParakeetDevice

    Write-Step 'coldvox-help'
    $help = Invoke-LoggedProcess `
        -Name 'coldvox-help' `
        -FilePath 'cargo' `
        -WorkingDirectory $RepoRoot `
        -ArgumentList @(
            'run',
            '-p', 'coldvox-app',
            '--bin', 'coldvox',
            '--no-default-features',
            '--features', $SmokeFeatureList,
            '--quiet',
            '--',
            '--help'
        )

    if ($help.ExitCode -ne 0) {
        throw "coldvox --help failed with exit code $($help.ExitCode)."
    }

    Write-Ok 'coldvox --help passed'

    Write-Step 'coldvox-list-devices'
    $devices = Invoke-LoggedProcess `
        -Name 'coldvox-list-devices' `
        -FilePath 'cargo' `
        -WorkingDirectory $RepoRoot `
        -ArgumentList @(
            'run',
            '-p', 'coldvox-app',
            '--bin', 'coldvox',
            '--no-default-features',
            '--features', $SmokeFeatureList,
            '--quiet',
            '--',
            '--list-devices'
        )

    if ($devices.ExitCode -ne 0) {
        throw "coldvox --list-devices failed with exit code $($devices.ExitCode)."
    }

    Write-Ok 'coldvox --list-devices passed'

    Write-Step 'coldvox-gui-smoke'
    $gui = Invoke-LoggedProcess `
        -Name 'coldvox-gui-smoke' `
        -FilePath 'cargo' `
        -WorkingDirectory $RepoRoot `
        -ArgumentList @(
            'run',
            '-p', 'coldvox-gui',
            '--quiet'
        )

    if ($gui.ExitCode -ne 0) {
        throw "coldvox-gui smoke failed with exit code $($gui.ExitCode)."
    }

    Write-Ok 'coldvox-gui smoke passed'

    [pscustomobject]@{
        Preflight = $preflight
        Help = $help
        Devices = $devices
        Gui = $gui
    }
}

function Invoke-Live {
    $smoke = Invoke-Smoke

    $modelPath = Resolve-ParakeetModelPath

    $env:RUST_LOG = 'info'
    $env:COLDVOX_LOG_RETENTION_DAYS = '0'
    $env:COLDVOX_CONFIG_PATH = $ConfigPath
    $env:COLDVOX_PLUGIN_CONFIG_PATH = $PluginConfigPath
    $env:PARAKEET_DEVICE = $ParakeetDevice
    $env:PARAKEET_MODEL_PATH = $modelPath

    Write-Step 'coldvox-build'
    $build = Invoke-LoggedProcess `
        -Name 'coldvox-build' `
        -FilePath 'cargo' `
        -WorkingDirectory $RepoRoot `
        -ArgumentList @(
            'build',
            '-p', 'coldvox-app',
            '--bin', 'coldvox',
            '--no-default-features',
            '--features', $LiveFeatureList,
            '--quiet',
            '--locked'
        )

    Write-Step 'coldvox-live'
    try {
        $live = Invoke-LoggedProcess `
            -Name 'coldvox-live' `
            -FilePath 'cargo' `
            -WorkingDirectory $RepoRoot `
            -ArgumentList @(
                'run',
                '-p', 'coldvox-app',
                '--bin', 'coldvox',
                '--no-default-features',
                '--features', $LiveFeatureList,
                '--quiet',
                '--locked'
            ) `
            -TimeoutSeconds $RuntimeSeconds
    } finally {
        Copy-RuntimeLog
    }

    if (-not $live.TimedOut -and $live.ExitCode -ne 0) {
        throw "Live runtime exited early with code $($live.ExitCode)."
    }

    if (-not (Test-Path $LogPath)) {
        throw 'Live runtime did not produce a runtime log.'
    }

    @(
        "ColdVox Windows live validation"
        "Timestamp: $Timestamp"
        "Repo root: $RepoRoot"
        "Artifact root: $ArtifactRoot"
        "Config path: $ConfigPath"
        "Plugin config path: $PluginConfigPath"
        "Smoke features: $SmokeFeatureList"
        "Live features: $LiveFeatureList"
        "PARAKEET_DEVICE: $ParakeetDevice"
        "PARAKEET_MODEL_PATH: $modelPath"
        "GPU names: $($smoke.Preflight.GpuNames -join ', ')"
        "nvidia-smi present: $($smoke.Preflight.HasNvidiaSmi)"
        "help exit code: $($smoke.Help.ExitCode)"
        "list-devices exit code: $($smoke.Devices.ExitCode)"
        "gui smoke exit code: $($smoke.Gui.ExitCode)"
        "live exit code: $($live.ExitCode)"
        "live timed out: $($live.TimedOut)"
        "coldvox.log: $(Join-Path $ArtifactRoot 'coldvox.log')"
        "coldvox.log tail: $(Join-Path $ArtifactRoot 'coldvox.log.tail')"
    ) | Set-Content -Path (Join-Path $ArtifactRoot 'summary.txt') -Encoding UTF8

    Write-Ok "Artifacts written to $ArtifactRoot"
}

switch ($Mode) {
    'Preflight' { Invoke-Preflight -RequireModel | Out-Null }
    'Smoke' { Invoke-Smoke | Out-Null }
    'Live' { Invoke-Live | Out-Null }
}
