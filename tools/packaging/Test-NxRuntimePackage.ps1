param(
    [Parameter(Mandatory = $true)]
    [string] $PackagePath,
    [string[]] $RuntimeIdentifiers
)

$ErrorActionPreference = 'Stop'

if (!$RuntimeIdentifiers -or $RuntimeIdentifiers.Length -eq 0) {
    . "$PSScriptRoot/NxRuntimeRids.ps1"
    $RuntimeIdentifiers = $NxRuntimeSupportedRids
} else {
    . "$PSScriptRoot/NxRuntimeRids.ps1"
}

Add-Type -AssemblyName System.IO.Compression.FileSystem

$archive = [System.IO.Compression.ZipFile]::OpenRead((Resolve-Path $PackagePath))
try {
    $entries = @($archive.Entries | ForEach-Object { $_.FullName })

    $required = @('lib/net10.0/NxLang.Runtime.dll')
    foreach ($rid in $RuntimeIdentifiers) {
        $required += "runtimes/$rid/native/$(Get-NxRuntimeNativeLibraryName -RuntimeIdentifier $rid)"
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
} finally {
    $archive.Dispose()
}

Write-Host "Verified NX runtime package: $PackagePath"
