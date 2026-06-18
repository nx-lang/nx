[CmdletBinding()]
Param(
)

$result = @{}

$testRoot = (Resolve-Path "$PSScriptRoot\..\..\bindings\dotnet\tests").Path
$result[$testRoot] = (
    Get-ChildItem -Path $testRoot -Recurse -Directory -Filter TestResults |
    Get-ChildItem -Recurse -File
)

$artifactStaging = & "$PSScriptRoot/../Get-ArtifactsStagingDirectory.ps1"
$testlogsPath = Join-Path $artifactStaging "test_logs"
if (Test-Path $testlogsPath) {
    $result[$testlogsPath] = Get-ChildItem $testlogsPath -Recurse;
}

$result
