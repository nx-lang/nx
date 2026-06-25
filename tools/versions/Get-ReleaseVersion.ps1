#!/usr/bin/env pwsh

[CmdletBinding()]
param (
    [ValidateSet('package', 'vscode')]
    [string] $Track = 'package',

    [ValidateSet('auto', 'pull_request', 'main', 'tag')]
    [string] $Context = 'auto',

    [string] $RefName = $env:GITHUB_REF_NAME,

    [string] $EventName = $env:GITHUB_EVENT_NAME,

    [string] $PullRequestNumber,

    [string] $RunNumber = $env:GITHUB_RUN_NUMBER,

    [string] $RunAttempt = $env:GITHUB_RUN_ATTEMPT,

    [string] $RepositoryRoot = (Resolve-Path "$PSScriptRoot/../..").Path,

    [string] $GitHubEnvPath,

    [string] $GitHubOutputPath,

    [switch] $Json
)

$ErrorActionPreference = 'Stop'

$packageTagPattern = '^v(?<Version>\d+\.\d+\.\d+)$'
$vscodeTagPattern = '^vscode-v(?<Version>\d+\.\d+\.\d+)$'

function Normalize-RefName([string] $Value) {
    if ($Value -like 'refs/heads/*') {
        return $Value.Substring('refs/heads/'.Length)
    }

    if ($Value -like 'refs/tags/*') {
        return $Value.Substring('refs/tags/'.Length)
    }

    return $Value
}

function Resolve-ReleaseContext {
    if ($Context -ne 'auto') {
        return $Context
    }

    if ($EventName -in @('pull_request', 'pull_request_target')) {
        return 'pull_request'
    }

    $normalizedRef = Normalize-RefName $RefName
    if ($normalizedRef -match $packageTagPattern -or $normalizedRef -match $vscodeTagPattern) {
        return 'tag'
    }

    return 'main'
}

function Invoke-MinVer([string] $TagPrefix) {
    $output = & dotnet minver $RepositoryRoot --tag-prefix $TagPrefix --default-pre-release-identifiers alpha.0 --verbosity error
    if ($LASTEXITCODE -ne 0) {
        throw "MinVer failed with exit code $LASTEXITCODE."
    }

    $version = ($output | Select-Object -Last 1).Trim()
    if (!$version) {
        throw 'MinVer did not emit a version.'
    }

    return $version
}

function Get-CoreVersion([string] $Version) {
    if ($Version -notmatch '^(?<Core>\d+\.\d+\.\d+)') {
        throw "Version '$Version' does not start with major.minor.patch."
    }

    return $Matches.Core
}

function Assert-ValidSemVer([string] $Version, [string] $Name) {
    if ($Version -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z]+(\.[0-9A-Za-z]+)*)?(\+[0-9A-Za-z.-]+)?$') {
        throw "$Name '$Version' is not a SemVer-compatible package version."
    }
}

function Get-RequiredNumber([string] $Value, [string] $Name, [string] $Fallback) {
    if (!$Value) {
        $Value = $Fallback
    }

    if ($Value -notmatch '^\d+$') {
        throw "$Name '$Value' must be numeric."
    }

    return $Value
}

function Get-PullRequestNumberFromEvent {
    if (!$env:GITHUB_EVENT_PATH -or !(Test-Path -LiteralPath $env:GITHUB_EVENT_PATH)) {
        return ''
    }

    try {
        $event = Get-Content -LiteralPath $env:GITHUB_EVENT_PATH -Raw | ConvertFrom-Json
    } catch {
        return ''
    }

    if ($event.pull_request -and $event.pull_request.number) {
        return [string] $event.pull_request.number
    }

    if ($event.number -and $EventName -in @('pull_request', 'pull_request_target')) {
        return [string] $event.number
    }

    return ''
}

$resolvedContext = Resolve-ReleaseContext
$normalizedRefName = Normalize-RefName $RefName
$tagPrefix = if ($Track -eq 'vscode') { 'vscode-v' } else { 'v' }
$minVerVersion = Invoke-MinVer $tagPrefix
$baseVersion = Get-CoreVersion $minVerVersion
$releaseTag = ''
$releaseVersion = $baseVersion

if ($resolvedContext -eq 'tag') {
    if ($Track -eq 'package') {
        if ($normalizedRefName -match $vscodeTagPattern) {
            throw "VS Code release tag '$normalizedRefName' cannot run the package release track."
        }

        $tagMatch = [regex]::Match($normalizedRefName, $packageTagPattern)
        if (!$tagMatch.Success) {
            throw "Package release tags must match v<major>.<minor>.<patch>. Actual tag: '$normalizedRefName'."
        }
    } else {
        if ($normalizedRefName -match $packageTagPattern) {
            throw "Package release tag '$normalizedRefName' cannot run the VS Code release track."
        }

        $tagMatch = [regex]::Match($normalizedRefName, $vscodeTagPattern)
        if (!$tagMatch.Success) {
            throw "VS Code release tags must match vscode-v<major>.<minor>.<patch>. Actual tag: '$normalizedRefName'."
        }
    }

    $releaseTag = $normalizedRefName
    $releaseVersion = $tagMatch.Groups['Version'].Value

    if ($env:GITHUB_ACTIONS -eq 'true' -and $minVerVersion -ne $releaseVersion) {
        throw "MinVer emitted '$minVerVersion' for tag '$releaseTag', but the tag requires '$releaseVersion'."
    }
}

$run = Get-RequiredNumber $RunNumber 'RunNumber' '0'
$attempt = Get-RequiredNumber $RunAttempt 'RunAttempt' '1'

if (!$PullRequestNumber) {
    $PullRequestNumber = Get-PullRequestNumberFromEvent
}

if ($resolvedContext -eq 'pull_request') {
    if (!$PullRequestNumber) {
        throw 'PullRequestNumber is required for pull request version calculation.'
    }

    $pr = Get-RequiredNumber $PullRequestNumber 'PullRequestNumber' '0'
    $packageVersion = "$baseVersion-pr.$pr.$run.$attempt"
} elseif ($resolvedContext -eq 'main') {
    $packageVersion = "$baseVersion-ci.$run.$attempt"
} else {
    $packageVersion = $releaseVersion
}

$vsixVersion = if ($resolvedContext -eq 'tag' -and $Track -eq 'vscode') { $releaseVersion } else { Get-CoreVersion $packageVersion }

Assert-ValidSemVer $packageVersion 'Package version'
if ($vsixVersion -notmatch '^\d+\.\d+\.\d+$') {
    throw "VS Code extension version '$vsixVersion' must be major.minor.patch."
}

$values = [ordered] @{
    RELEASE_TRACK = $Track
    RELEASE_CONTEXT = $resolvedContext
    RELEASE_TAG = $releaseTag
    RELEASE_VERSION = $releaseVersion
    MINVER_VERSION = $minVerVersion
    PACKAGE_VERSION = $packageVersion
    NUGET_PACKAGE_VERSION = $packageVersion
    MSBUILD_VERSION = $packageVersion
    NPM_PACKAGE_VERSION = $packageVersion
    VSCODE_EXTENSION_VERSION = $vsixVersion
}

if ($GitHubEnvPath) {
    foreach ($entry in $values.GetEnumerator()) {
        Add-Content -LiteralPath $GitHubEnvPath -Value "$($entry.Key)=$($entry.Value)"
    }
}

if ($GitHubOutputPath) {
    foreach ($entry in $values.GetEnumerator()) {
        Add-Content -LiteralPath $GitHubOutputPath -Value "$($entry.Key.ToLowerInvariant())=$($entry.Value)"
    }
}

if ($Json) {
    $values | ConvertTo-Json -Depth 4
} else {
    foreach ($entry in $values.GetEnumerator()) {
        Write-Host "$($entry.Key)=$($entry.Value)"
    }
}
