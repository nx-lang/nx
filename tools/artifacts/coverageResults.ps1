$RepoRoot = [System.IO.Path]::GetFullPath("$PSScriptRoot\..\..")
$DotNetTestRoot = Join-Path $RepoRoot "bindings/dotnet/tests"

$coverageFiles = @(
    Get-ChildItem -Path $DotNetTestRoot -Filter *.cobertura.xml -Recurse |
    Where-Object { $_.FullName -notlike "*/In/*" -and $_.FullName -notlike "*\In\*" }
)

# Prepare code coverage reports for merging on another machine
$cloudRepoRoot = $env:SYSTEM_DEFAULTWORKINGDIRECTORY
if (!$cloudRepoRoot) { $cloudRepoRoot = $env:GITHUB_WORKSPACE }
if ($cloudRepoRoot) {
    Write-Host "Substituting $cloudRepoRoot with `"{reporoot}`""
    $coverageFiles |% {
        $content = Get-Content -LiteralPath $_ |% { $_ -Replace [regex]::Escape($cloudRepoRoot), "{reporoot}" }
        Set-Content -LiteralPath $_ -Value $content -Encoding UTF8
    }
} else {
    Write-Warning "coverageResults: Cloud build not detected. Machine-neutral token replacement skipped."
}

if (!((Test-Path (Join-Path $RepoRoot "bin")) -and (Test-Path (Join-Path $RepoRoot "obj")))) { return }

return @{
    $RepoRoot = (
        $coverageFiles +
        (Get-ChildItem "$RepoRoot\obj\*.cs" -Recurse)
    );
}
