param(
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Release",
    [switch]$RequireInstaller,
    [switch]$RequireDefender
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$repoRoot = Resolve-Path (Join-Path $root "..")
$version = "0.2.0"
$workspaceToml = Join-Path $root "Cargo.toml"
$inWorkspacePackage = $false
foreach ($line in Get-Content $workspaceToml) {
    if ($line -match '^\[workspace\.package\]') {
        $inWorkspacePackage = $true
        continue
    }
    if ($line -match '^\[') {
        $inWorkspacePackage = $false
    }
    if ($inWorkspacePackage -and $line -match '^version = "([^"]+)"') {
        $version = $Matches[1]
        break
    }
}

$releaseName = "FindBT-v$version-windows-x64"
$distDir = Join-Path $root "dist\windows"
$artifactsDir = Join-Path $distDir "artifacts"
$localReleasePath = Join-Path $artifactsDir "local-release.txt"
$portableDir = Join-Path $distDir "$releaseName-portable"
$offlineDir = Join-Path $distDir "$releaseName-offline-portable"
$targetProfile = if ($Configuration -eq "Release") { "release" } else { "debug" }
$builtExe = Join-Path $root "target\$targetProfile\findbt-app.exe"
$iconPath = Join-Path $root "assets\icons\FindBT.ico"

Push-Location $root
try {
    & (Join-Path $root "scripts\build-windows-app.ps1") -Configuration $Configuration

    if (-not (Test-Path $builtExe)) {
        throw "Build completed but expected executable was not found at $builtExe"
    }

    if (Test-Path $artifactsDir) {
        Remove-Item -Recurse -Force $artifactsDir
    }
    New-Item -ItemType Directory -Force -Path $artifactsDir | Out-Null
    foreach ($path in @($portableDir, $offlineDir)) {
        if (Test-Path $path) {
            Remove-Item -Recurse -Force $path
        }
        New-Item -ItemType Directory -Force -Path $path | Out-Null
    }

    $portableExe = Join-Path $portableDir "FindBT.exe"
    Copy-Item -Path $builtExe -Destination $portableExe
    Copy-Item -Path $iconPath -Destination (Join-Path $portableDir "FindBT.ico")
    Copy-Item -Path (Join-Path $repoRoot "QUICKSTART.md") -Destination (Join-Path $portableDir "quickstart.txt")
    $builtUtc = (Get-Date).ToUniversalTime().ToString("yyyy-MM-dd HH:mm:ss 'UTC'")
    @"
FindBT Local Release
====================

Version: v$version
Platform: Windows x64
Built: $builtUtc
Configuration: $Configuration

Artifacts:
- $releaseName-portable.zip
- $releaseName-offline-portable.zip
- $releaseName-installer.msi, when WiX v4 is installed

SHA256 files are generated beside each installer and portable zip.

Checks:
- cargo build -p findbt-app
- Microsoft Defender scan is run when Microsoft Defender is available.
"@ | Set-Content -Path $localReleasePath -Encoding ASCII
    Copy-Item -Path $localReleasePath -Destination (Join-Path $portableDir "local-release.txt")
    @"
FindBT Windows Portable
=======================

Version: v$version
Configuration: $Configuration
Contents:
- FindBT.exe
- FindBT.ico
- quickstart.txt
- local-release.txt

This portable build is intended to run offline from the extracted folder.
"@ | Add-Content -Path (Join-Path $portableDir "local-release.txt") -Encoding ASCII

    $portableZip = Join-Path $artifactsDir "$releaseName-portable.zip"
    if (Test-Path $portableZip) {
        Remove-Item -Force $portableZip
    }
    Compress-Archive -Path (Join-Path $portableDir "*") -DestinationPath $portableZip -CompressionLevel Optimal

    Copy-Item -Path $portableExe -Destination (Join-Path $offlineDir "FindBT.exe")
    Copy-Item -Path $iconPath -Destination (Join-Path $offlineDir "FindBT.ico")
    Copy-Item -Path (Join-Path $repoRoot "QUICKSTART.md") -Destination (Join-Path $offlineDir "quickstart.txt")
    Copy-Item -Path $localReleasePath -Destination (Join-Path $offlineDir "local-release.txt")
    @"
FindBT Windows Offline Portable
===============================

Version: v$version
Configuration: $Configuration

This package is intended for offline machines. No network access is required to run FindBT from the extracted folder.
"@ | Set-Content -Path (Join-Path $offlineDir "offline-readme.txt") -Encoding ASCII

    $offlineZip = Join-Path $artifactsDir "$releaseName-offline-portable.zip"
    if (Test-Path $offlineZip) {
        Remove-Item -Force $offlineZip
    }
    Compress-Archive -Path (Join-Path $offlineDir "*") -DestinationPath $offlineZip -CompressionLevel Optimal

    $msiPath = Join-Path $artifactsDir "$releaseName-installer.msi"
    $wix = Get-Command wix.exe -ErrorAction SilentlyContinue
    if ($wix) {
        if (Test-Path $msiPath) {
            Remove-Item -Force $msiPath
        }
        & $wix.Source build (Join-Path $root "windows\FindBT.wxs") `
            -d "SourceDir=$portableDir" `
            -d "Version=$version" `
            -o $msiPath
        if ($LASTEXITCODE -ne 0) {
            throw "WiX MSI build failed with exit code $LASTEXITCODE."
        }
    }
    elseif ($RequireInstaller) {
        throw "WiX Toolset was not found. Install WiX v4 locally, then rerun this script to produce the MSI installer."
    }
    else {
        Write-Warning "WiX Toolset was not found, so the MSI installer was skipped. Portable and offline portable artifacts were still created."
    }

    $scanTargets = @($portableExe, $portableZip, $offlineZip)
    if (Test-Path $msiPath) {
        $scanTargets += $msiPath
    }
    & (Join-Path $repoRoot "scripts\invoke-windows-defender-scan.ps1") -Path $scanTargets -Required:$RequireDefender

    Get-ChildItem -Path $artifactsDir -File | Where-Object { $_.Name -notlike "*.sha256.txt" -and $_.Name -ne "local-release.txt" } | ForEach-Object {
        $hash = Get-FileHash -Algorithm SHA256 -Path $_.FullName
        "$($hash.Hash)  $($_.Name)" | Set-Content -Path "$($_.FullName).sha256.txt" -Encoding ASCII
    }

    Write-Host "Windows artifacts:"
    Get-ChildItem -Path $artifactsDir -File | Select-Object -ExpandProperty Name
}
finally {
    Pop-Location
}
