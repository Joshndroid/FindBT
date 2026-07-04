param(
    [Parameter(Mandatory = $true)]
    [string[]]$Path,
    [switch]$Required
)

$ErrorActionPreference = "Stop"

function Find-MpCmdRun {
    $candidates = @()
    if (-not [string]::IsNullOrWhiteSpace(${env:ProgramFiles})) {
        $candidates += Join-Path ${env:ProgramFiles} "Windows Defender\MpCmdRun.exe"
        $candidates += Get-ChildItem -Path (Join-Path ${env:ProgramData} "Microsoft\Windows Defender\Platform") -Filter "MpCmdRun.exe" -Recurse -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName
    }
    if (-not [string]::IsNullOrWhiteSpace(${env:ProgramFiles(x86)})) {
        $candidates += Join-Path ${env:ProgramFiles(x86)} "Windows Defender\MpCmdRun.exe"
    }

    $candidates | Where-Object { -not [string]::IsNullOrWhiteSpace($_) -and (Test-Path $_) } | Select-Object -First 1
}

$scanner = Find-MpCmdRun
if ([string]::IsNullOrWhiteSpace($scanner)) {
    $message = "Microsoft Defender MpCmdRun.exe was not found."
    if ($Required) {
        throw $message
    }
    Write-Warning "$message Skipping Defender scan."
    return
}

foreach ($item in $Path) {
    $resolved = Resolve-Path $item
    Write-Host "Microsoft Defender scan: $resolved"
    & $scanner -Scan -ScanType 3 -File $resolved
    if ($LASTEXITCODE -ne 0) {
        throw "Microsoft Defender scan failed for $resolved with exit code $LASTEXITCODE."
    }
}
