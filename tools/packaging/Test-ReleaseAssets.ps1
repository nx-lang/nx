# Checks the assets downloaded from a GitHub Release made by release.yml: the manifest names this
# release, and every asset matches its checksum. Each publish job then checks the artifacts it
# publishes.
param(
    [Parameter(Mandatory = $true)]
    [string] $Path,
    [Parameter(Mandatory = $true)]
    [string] $Tag,
    [Parameter(Mandatory = $true)]
    [string] $Version
)

$ErrorActionPreference = 'Stop'

$manifestPath = Join-Path $Path 'release-manifest.json'
$checksumPath = Join-Path $Path 'release-checksums.txt'

if (!(Test-Path $manifestPath) -or !(Test-Path $checksumPath)) {
    throw 'Release assets must include release-manifest.json and release-checksums.txt.'
}

$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($manifest.schema -ne 'nx.release-manifest.v1' -or $manifest.tag -ne $Tag -or $manifest.version -ne $Version) {
    throw 'Release manifest does not match the release being published.'
}

Push-Location $Path
try {
    sha256sum -c release-checksums.txt
    if ($LASTEXITCODE -ne 0) {
        throw 'Release assets do not match release-checksums.txt.'
    }
} finally {
    Pop-Location
}
