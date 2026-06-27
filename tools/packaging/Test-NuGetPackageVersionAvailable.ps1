param(
    [Parameter(Mandatory = $true)]
    [string] $PackagePath,
    [string] $PackageId = 'NxLang.Sdk',
    [string] $RegistryUrl = 'https://api.nuget.org/v3/index.json'
)

$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.IO.Compression.FileSystem

$archive = [System.IO.Compression.ZipFile]::OpenRead((Resolve-Path $PackagePath))
try {
    $nuspecEntry = $archive.Entries | Where-Object { $_.FullName -like '*.nuspec' } | Select-Object -First 1
    if (!$nuspecEntry) {
        throw "Package '$PackagePath' is missing a nuspec metadata file."
    }

    $stream = $nuspecEntry.Open()
    try {
        $reader = [System.IO.StreamReader]::new($stream)
        try {
            [xml] $nuspec = $reader.ReadToEnd()
        } finally {
            $reader.Dispose()
        }
    } finally {
        $stream.Dispose()
    }

    $packageVersion = [string] $nuspec.package.metadata.version
} finally {
    $archive.Dispose()
}

if (!$packageVersion) {
    throw "Package '$PackagePath' does not declare a version."
}

if ($RegistryUrl -ne 'https://api.nuget.org/v3/index.json') {
    Write-Host "Skipping NuGet.org duplicate check for non-default registry '$RegistryUrl'."
    return
}

$versionIndexUrl = "https://api.nuget.org/v3-flatcontainer/$($PackageId.ToLowerInvariant())/index.json"
try {
    $versionIndex = Invoke-RestMethod -Uri $versionIndexUrl
    $publishedVersions = @($versionIndex.versions)
} catch {
    if ($_.Exception.Response.StatusCode.value__ -eq 404) {
        $publishedVersions = @()
    } else {
        throw
    }
}

if ($publishedVersions -contains $packageVersion.ToLowerInvariant()) {
    Write-Warning "$PackageId $packageVersion already exists on NuGet.org; treating this run as an idempotent retry."
    return
}

Write-Host "$PackageId $packageVersion is available on NuGet.org."
