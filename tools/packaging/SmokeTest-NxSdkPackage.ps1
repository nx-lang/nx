param(
    [Parameter(Mandatory = $true)]
    [string] $PackagePath,
    [string] $RuntimeIdentifier
)

$ErrorActionPreference = 'Stop'

. "$PSScriptRoot/NxSdkRids.ps1"

if (!$RuntimeIdentifier) {
    $RuntimeIdentifier = Get-NxSdkHostRuntimeIdentifier
}
Assert-NxSdkHostMatchesRuntimeIdentifier -RuntimeIdentifier $RuntimeIdentifier

$package = Get-Item $PackagePath
$workRoot = Join-Path ([System.IO.Path]::GetTempPath()) "nx-sdk-smoke-$([Guid]::NewGuid().ToString('N'))"
$feedRoot = Join-Path $workRoot 'feed'
$appRoot = Join-Path $workRoot 'app'

New-Item -ItemType Directory -Force -Path $feedRoot, $appRoot | Out-Null
Copy-Item $package.FullName $feedRoot

try {
    dotnet new console --framework net10.0 --output $appRoot | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet new failed with exit code $LASTEXITCODE."
    }

    $packageVersion = [regex]::Match($package.Name, '^NxLang\.Sdk\.(?<version>.+)\.nupkg$').Groups['version'].Value
    if (!$packageVersion) {
        throw "Could not infer package version from '$($package.Name)'."
    }

    dotnet add $appRoot package NxLang.Sdk --version $packageVersion --source $feedRoot | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet add package failed with exit code $LASTEXITCODE."
    }

    @'
using NxLang.Nx;

int value = NxRuntime.Evaluate<int>("let root() = { 42 }");
if (value != 42)
{
    throw new InvalidOperationException($"Expected 42, got {value}.");
}

Console.WriteLine(value);
'@ | Set-Content -Path (Join-Path $appRoot 'Program.cs')

    dotnet run --project $appRoot --runtime $RuntimeIdentifier --no-self-contained | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet run failed with exit code $LASTEXITCODE."
    }
} finally {
    Remove-Item $workRoot -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "Verified NX SDK package consumption for $RuntimeIdentifier."
