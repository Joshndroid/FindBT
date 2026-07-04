param(
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Debug"
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")

Push-Location $root
try {
    $args = @("build", "-p", "findbt-app")
    if ($Configuration -eq "Release") {
        $args += "--release"
    }

    & cargo @args
    if ($LASTEXITCODE -ne 0) {
        throw "cargo $($args -join ' ') failed with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}
