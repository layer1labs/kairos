#Requires -Version 7
<#
.SYNOPSIS
    Build and launch the Kairos terminal from the repository root.

.DESCRIPTION
    Ensures the Rust toolchain and protoc are on PATH, then builds and runs
    the kairos binary. Always targets --bin kairos (never warp/stable/dev/preview,
    which require Warp's private channel-config binary).

.PARAMETER Release
    Build with --release optimisations (slower build, faster runtime).
    Defaults to debug build for faster iteration.

.PARAMETER NoBuild
    Skip the cargo build step and run the last compiled binary directly.

.EXAMPLE
    .\Open-Kairos.ps1
    # Debug build + launch

.EXAMPLE
    .\Open-Kairos.ps1 -Release
    # Release build + launch

.EXAMPLE
    .\Open-Kairos.ps1 -NoBuild
    # Run the last compiled binary without rebuilding
#>
param(
    [switch]$Release,
    [switch]$NoBuild
)

Set-StrictMode -Off
$ErrorActionPreference = "Stop"

# ── Resolve repo root ────────────────────────────────────────────────────────
$RepoRoot = $PSScriptRoot
if (-not $RepoRoot) { $RepoRoot = $PWD.Path }

# ── Ensure cargo is on PATH ──────────────────────────────────────────────────
$CargoBin = "$env:USERPROFILE\.cargo\bin"
if (Test-Path $CargoBin) {
    $env:PATH = "$CargoBin;$env:PATH"
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "cargo not found. Install Rust via: winget install Rustlang.Rustup"
    exit 1
}

# ── Ensure protoc is on PATH (required for proto API crates) ─────────────────
$env:PATH = [System.Environment]::GetEnvironmentVariable("PATH","Machine") + ";" +
            [System.Environment]::GetEnvironmentVariable("PATH","User")

# ── Build ────────────────────────────────────────────────────────────────────
if (-not $NoBuild) {
    $BuildArgs = @("build", "--bin", "kairos")
    if ($Release) { $BuildArgs += "--release" }

    Write-Host "Building Kairos$(if ($Release) { ' (release)' })..." -ForegroundColor Cyan
    Push-Location $RepoRoot
    try {
        & cargo @BuildArgs
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Build failed (exit $LASTEXITCODE). See output above."
            exit $LASTEXITCODE
        }
    } finally {
        Pop-Location
    }
}

# ── Locate binary ────────────────────────────────────────────────────────────
$Profile  = if ($Release) { "release" } else { "debug" }
$Binary   = Join-Path $RepoRoot "target\$Profile\kairos.exe"

if (-not (Test-Path $Binary)) {
    Write-Error "Binary not found at: $Binary`nRun without -NoBuild to compile first."
    exit 1
}

# ── Launch ───────────────────────────────────────────────────────────────────
Write-Host "Launching Kairos ($Profile)..." -ForegroundColor Green
& $Binary
