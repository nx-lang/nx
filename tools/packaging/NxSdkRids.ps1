$NxSdkSupportedRids = @(
    'linux-x64',
    'osx-arm64',
    'win-x64'
)

$NxSdkNativeLibraryByRid = @{
    'linux-x64' = 'libnx_ffi.so'
    'osx-arm64' = 'libnx_ffi.dylib'
    'win-x64' = 'nx_ffi.dll'
}

$NxSdkExpectedArchitectureByRid = @{
    'linux-x64' = [System.Runtime.InteropServices.Architecture]::X64
    'osx-arm64' = [System.Runtime.InteropServices.Architecture]::Arm64
    'win-x64' = [System.Runtime.InteropServices.Architecture]::X64
}

$NxSdkExpectedPlatformByRid = @{
    'linux-x64' = 'Linux'
    'osx-arm64' = 'macOS'
    'win-x64' = 'Windows'
}

function Get-NxSdkNativeLibraryName {
    param(
        [Parameter(Mandatory = $true)]
        [string] $RuntimeIdentifier
    )

    if (!$NxSdkNativeLibraryByRid.ContainsKey($RuntimeIdentifier)) {
        $supported = $NxSdkSupportedRids -join ', '
        throw "Unsupported NX SDK RID '$RuntimeIdentifier'. Supported RIDs: $supported."
    }

    return $NxSdkNativeLibraryByRid[$RuntimeIdentifier]
}

function Get-NxSdkHostPlatform {
    if ($IsWindows) {
        return 'Windows'
    }

    if ($IsMacOS) {
        return 'macOS'
    }

    return 'Linux'
}

function Get-NxSdkHostRuntimeIdentifier {
    $hostPlatform = Get-NxSdkHostPlatform
    $hostArchitecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture

    foreach ($runtimeIdentifier in $NxSdkSupportedRids) {
        if ($NxSdkExpectedPlatformByRid[$runtimeIdentifier] -eq $hostPlatform -and
            $NxSdkExpectedArchitectureByRid[$runtimeIdentifier] -eq $hostArchitecture) {
            return $runtimeIdentifier
        }
    }

    throw "Unsupported NX SDK host '$hostPlatform/$hostArchitecture'. Pass -RuntimeIdentifier explicitly after adding RID support."
}

function Assert-NxSdkHostMatchesRuntimeIdentifier {
    param(
        [Parameter(Mandatory = $true)]
        [string] $RuntimeIdentifier
    )

    Get-NxSdkNativeLibraryName -RuntimeIdentifier $RuntimeIdentifier | Out-Null

    $hostPlatform = Get-NxSdkHostPlatform
    $expectedPlatform = $NxSdkExpectedPlatformByRid[$RuntimeIdentifier]
    if ($hostPlatform -ne $expectedPlatform) {
        throw "Cannot use '$RuntimeIdentifier' on host platform '$hostPlatform'. Use a matching host or add explicit cross-compilation support."
    }

    $hostArchitecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    $expectedArchitecture = $NxSdkExpectedArchitectureByRid[$RuntimeIdentifier]
    if ($hostArchitecture -ne $expectedArchitecture) {
        throw "Cannot use '$RuntimeIdentifier' on host architecture '$hostArchitecture'. Use a matching host or add explicit cross-compilation support."
    }
}
