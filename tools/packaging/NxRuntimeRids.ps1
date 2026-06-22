$NxRuntimeSupportedRids = @(
    'linux-x64',
    'osx-arm64',
    'win-x64'
)

$NxRuntimeNativeLibraryByRid = @{
    'linux-x64' = 'libnx_ffi.so'
    'osx-arm64' = 'libnx_ffi.dylib'
    'win-x64' = 'nx_ffi.dll'
}

$NxRuntimeExpectedArchitectureByRid = @{
    'linux-x64' = [System.Runtime.InteropServices.Architecture]::X64
    'osx-arm64' = [System.Runtime.InteropServices.Architecture]::Arm64
    'win-x64' = [System.Runtime.InteropServices.Architecture]::X64
}

$NxRuntimeExpectedPlatformByRid = @{
    'linux-x64' = 'Linux'
    'osx-arm64' = 'macOS'
    'win-x64' = 'Windows'
}

function Get-NxRuntimeNativeLibraryName {
    param(
        [Parameter(Mandatory = $true)]
        [string] $RuntimeIdentifier
    )

    if (!$NxRuntimeNativeLibraryByRid.ContainsKey($RuntimeIdentifier)) {
        $supported = $NxRuntimeSupportedRids -join ', '
        throw "Unsupported NX runtime RID '$RuntimeIdentifier'. Supported RIDs: $supported."
    }

    return $NxRuntimeNativeLibraryByRid[$RuntimeIdentifier]
}

function Get-NxRuntimeHostPlatform {
    if ($IsWindows) {
        return 'Windows'
    }

    if ($IsMacOS) {
        return 'macOS'
    }

    return 'Linux'
}

function Get-NxRuntimeHostRuntimeIdentifier {
    $hostPlatform = Get-NxRuntimeHostPlatform
    $hostArchitecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture

    foreach ($runtimeIdentifier in $NxRuntimeSupportedRids) {
        if ($NxRuntimeExpectedPlatformByRid[$runtimeIdentifier] -eq $hostPlatform -and
            $NxRuntimeExpectedArchitectureByRid[$runtimeIdentifier] -eq $hostArchitecture) {
            return $runtimeIdentifier
        }
    }

    throw "Unsupported NX runtime host '$hostPlatform/$hostArchitecture'. Pass -RuntimeIdentifier explicitly after adding RID support."
}

function Assert-NxRuntimeHostMatchesRuntimeIdentifier {
    param(
        [Parameter(Mandatory = $true)]
        [string] $RuntimeIdentifier
    )

    Get-NxRuntimeNativeLibraryName -RuntimeIdentifier $RuntimeIdentifier | Out-Null

    $hostPlatform = Get-NxRuntimeHostPlatform
    $expectedPlatform = $NxRuntimeExpectedPlatformByRid[$RuntimeIdentifier]
    if ($hostPlatform -ne $expectedPlatform) {
        throw "Cannot use '$RuntimeIdentifier' on host platform '$hostPlatform'. Use a matching host or add explicit cross-compilation support."
    }

    $hostArchitecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    $expectedArchitecture = $NxRuntimeExpectedArchitectureByRid[$RuntimeIdentifier]
    if ($hostArchitecture -ne $expectedArchitecture) {
        throw "Cannot use '$RuntimeIdentifier' on host architecture '$hostArchitecture'. Use a matching host or add explicit cross-compilation support."
    }
}
