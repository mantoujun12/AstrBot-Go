# Builds (optional) and packages a portable Windows zip containing only the
# main executable. The WebView2 runtime is a system component, so no other
# files are needed.
#
# Usage:
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/portable.ps1
#     [-Target <rust-triple>] [-SkipBuild] [-Version <string>]
#
# No third-party dependencies; built-in commands only.

param(
    [string]$Target,
    [switch]$SkipBuild,
    [string]$Version
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot

if (-not $Target) {
    # Auto-detect the host architecture.
    if ([System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -eq [System.Runtime.InteropServices.Architecture]::Arm64) {
        $Target = "aarch64-pc-windows-msvc"
    } else {
        $Target = "x86_64-pc-windows-msvc"
    }
}

if ($Target -notin @("x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc")) {
    throw "Unsupported target '$Target'. Expected x86_64-pc-windows-msvc or aarch64-pc-windows-msvc."
}

$targetDir = Join-Path $repoRoot "src-tauri/target/$Target/release"
$exePath = Join-Path $targetDir "astrbot-go.exe"

if (-not $SkipBuild) {
    Push-Location $repoRoot
    try {
        # The Tauri 2 CLI has no `--bundles none` value; `--no-bundle` is the
        # equivalent way to skip bundling.
        pnpm tauri build --target $Target --no-bundle
        if ($LASTEXITCODE -ne 0) {
            throw "pnpm tauri build failed with exit code $LASTEXITCODE."
        }
    } finally {
        Pop-Location
    }
}

if (-not (Test-Path $exePath)) {
    throw "Executable not found: $exePath. Build the app first (omit -SkipBuild) or check the target triple."
}

if (-not $Version) {
    $conf = Get-Content (Join-Path $repoRoot "src-tauri/tauri.conf.json") -Raw | ConvertFrom-Json
    $Version = $conf.version
    if (-not $Version) {
        throw "Could not read 'version' from src-tauri/tauri.conf.json. Pass -Version explicitly."
    }
}

$arch = if ($Target -like "x86_64*") { "x64" } else { "arm64" }
$portableDir = Join-Path $targetDir "portable"
$zipPath = Join-Path $portableDir "AstrBot-Go_${Version}_${arch}_portable.zip"

if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
New-Item -ItemType Directory -Force -Path $portableDir | Out-Null

# Use the bundled bsdtar instead of Compress-Archive: older Compress-Archive
# versions write backslash entry separators, producing invalid zips.
tar -a -c -f $zipPath -C $targetDir astrbot-go.exe
if ($LASTEXITCODE -ne 0) {
    throw "tar failed with exit code $LASTEXITCODE."
}

# Verify entry names use forward slashes.
$entries = tar -tf $zipPath
Write-Host "Zip entries:"
$entries | ForEach-Object { Write-Host "  $_" }
if ($entries -notcontains "astrbot-go.exe") {
    throw "Zip content verification failed; expected entry 'astrbot-go.exe'."
}

Write-Host "Portable zip created: $zipPath"
