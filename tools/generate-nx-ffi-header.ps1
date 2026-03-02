Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$cbindgen = Get-Command cbindgen -ErrorAction SilentlyContinue

if ($null -eq $cbindgen) {
    throw "cbindgen is required to generate bindings/c/nx.h. Install it with: cargo install cbindgen --locked"
}

& $cbindgen.Path `
    (Join-Path $repoRoot 'crates/nx-ffi') `
    --config (Join-Path $repoRoot 'crates/nx-ffi/cbindgen.toml') `
    --output (Join-Path $repoRoot 'bindings/c/nx.h')
