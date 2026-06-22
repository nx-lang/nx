param(
    [string] $RuntimeIdentifier,
    [string] $Configuration = 'Release',
    [string] $OutputRoot,
    [switch] $SkipBuild
)

$ErrorActionPreference = 'Stop'

$RepoRoot = [System.IO.Path]::GetFullPath("$PSScriptRoot/../..")
. "$PSScriptRoot/NxRuntimeRids.ps1"

if (!$RuntimeIdentifier) {
    $RuntimeIdentifier = Get-NxRuntimeHostRuntimeIdentifier
}

$libraryName = Get-NxRuntimeNativeLibraryName -RuntimeIdentifier $RuntimeIdentifier
Assert-NxRuntimeHostMatchesRuntimeIdentifier -RuntimeIdentifier $RuntimeIdentifier

if (!$OutputRoot) {
    $OutputRoot = "$RepoRoot/artifacts/nx-runtime"
}

$cargoProfile = if ($Configuration -eq 'Release') { 'release' } else { 'debug' }

if (!$SkipBuild) {
    $cargoArgs = @('build', '-p', 'nx-ffi')
    if ($Configuration -eq 'Release') {
        $cargoArgs += '--release'
    } elseif ($Configuration -ne 'Debug') {
        throw "Unsupported configuration '$Configuration'. Supported values are Debug and Release."
    }

    Push-Location $RepoRoot
    try {
        & cargo @cargoArgs
        if ($LASTEXITCODE -ne 0) {
            throw "cargo $($cargoArgs -join ' ') failed with exit code $LASTEXITCODE."
        }
    } finally {
        Pop-Location
    }
}

$sourcePath = Join-Path $RepoRoot "target/$cargoProfile/$libraryName"
if (!(Test-Path $sourcePath)) {
    throw "Expected NX native runtime at '$sourcePath'."
}

$destinationDirectory = Join-Path $OutputRoot "runtimes/$RuntimeIdentifier/native"
New-Item -ItemType Directory -Force -Path $destinationDirectory | Out-Null
$destinationPath = Join-Path $destinationDirectory $libraryName
Copy-Item $sourcePath $destinationPath -Force

Write-Host "Staged $RuntimeIdentifier native runtime asset: $destinationPath"
