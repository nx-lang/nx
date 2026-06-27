param(
    [Parameter(Mandatory = $true)]
    [string] $PackagePath,
    [string[]] $RuntimeIdentifiers
)

$ErrorActionPreference = 'Stop'

if (!$RuntimeIdentifiers -or $RuntimeIdentifiers.Length -eq 0) {
    . "$PSScriptRoot/NxSdkRids.ps1"
    $RuntimeIdentifiers = $NxSdkSupportedRids
} else {
    . "$PSScriptRoot/NxSdkRids.ps1"
}

Add-Type -AssemblyName System.IO.Compression.FileSystem

$archive = [System.IO.Compression.ZipFile]::OpenRead((Resolve-Path $PackagePath))
try {
    $entries = @($archive.Entries | ForEach-Object { $_.FullName })

    $required = @('lib/net10.0/NxLang.Sdk.dll')
    foreach ($rid in $RuntimeIdentifiers) {
        $required += "runtimes/$rid/native/$(Get-NxSdkNativeLibraryName -RuntimeIdentifier $rid)"
    }

    foreach ($entry in $required) {
        if ($entries -notcontains $entry) {
            throw "Package '$PackagePath' is missing required entry '$entry'."
        }
    }

    $cliEntries = @($entries | Where-Object {
        $fileName = [System.IO.Path]::GetFileName($_)
        $fileName -eq 'nxlang' -or $fileName -eq 'nxlang.exe'
    })

    if ($cliEntries.Length -gt 0) {
        throw "Package '$PackagePath' must not contain nxlang CLI entries: $($cliEntries -join ', ')"
    }

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

    $metadata = $nuspec.package.metadata
    $expectedProjectUrl = 'https://github.com/nx-lang/nx'
    if ($metadata.projectUrl -ne $expectedProjectUrl) {
        throw "Package '$PackagePath' has projectUrl '$($metadata.projectUrl)'; expected '$expectedProjectUrl'."
    }

    $repositoryUrl = $metadata.repository.url
    if ($repositoryUrl -ne $expectedProjectUrl) {
        throw "Package '$PackagePath' has repository URL '$repositoryUrl'; expected '$expectedProjectUrl'."
    }

    $metadataText = $metadata.OuterXml
    if ($metadataText -match 'github\.com/bret/nx') {
        throw "Package '$PackagePath' contains stale GitHub metadata URL github.com/bret/nx."
    }

    $requiredMetadata = @{
        id = 'NxLang.Sdk'
        title = 'NX .NET SDK'
        license = 'MIT'
        readme = 'README.md'
    }

    foreach ($item in $requiredMetadata.GetEnumerator()) {
        $value = if ($item.Key -eq 'license') { [string] $metadata.license.InnerText } else { [string] $metadata.($item.Key) }
        if ($value -ne $item.Value) {
            throw "Package '$PackagePath' has metadata '$($item.Key)' value '$value'; expected '$($item.Value)'."
        }
    }

    if ($metadata.description -notmatch 'Managed \.NET SDK' -or $metadata.description -notmatch 'native nx_ffi SDK assets') {
        throw "Package '$PackagePath' description must describe the managed .NET SDK and packaged native NX SDK assets."
    }
} finally {
    $archive.Dispose()
}

Write-Host "Verified NX SDK package: $PackagePath"
