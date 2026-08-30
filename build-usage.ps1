<#
.SYNOPSIS
  AlphaTA — one-click build + verify for all 7 language bindings (PowerShell 7+).

.DESCRIPTION
  Thin forwarder to scripts/build-usage-packages.ps1. Mirrors the bash wrapper
  build-usage.sh so the same command-line works on Windows PowerShell.

.PARAMETER Languages
  Optional subset of: python, node, java, go, c, dotnet, wasm.
  When omitted, all 7 languages are built.

.PARAMETER BenchTalib
  Switch: run scripts/bench-vs-talib.ps1 instead of the language build.
  Emits dist/bench/finkit-vs-talib.md + dist/bench/results.json.

.PARAMETER NoBundle
  Switch: skip the final usage-bundle.zip packaging step.

.PARAMETER NoVerify
  Switch: skip the install-and-verify smoke test for each language.

.PARAMETER Json
  Switch: emit the manifest as JSON to stdout (in addition to dist/manifest.json).

.PARAMETER Help
  Switch: print this synopsis.

.EXAMPLE
  pwsh ./build-usage.ps1 python node

.EXAMPLE
  pwsh ./build-usage.ps1 -BenchTalib

.EXAMPLE
  pwsh ./build-usage.ps1 -NoBundle
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string[]] $Languages = @(),

    [switch] $BenchTalib,
    [switch] $NoBundle,
    [switch] $NoVerify,
    [switch] $Json,
    [switch] $Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Get-Help $MyInvocation.MyCommand.Path -Full
    exit 0
}

$root = (Resolve-Path "$PSScriptRoot").Path
$unified = Join-Path $root "scripts\build-usage-packages.ps1"

if (-not (Test-Path $unified)) {
    throw "missing $unified — make sure scripts/build-usage-packages.ps1 is on disk"
}

# Splat the original bound parameters (preserves switch types). Strip
# the script-level switches that the unified builder doesn't accept.
$forward = @{}
foreach ($k in $PSBoundParameters.Keys) {
    if ($k -in @('BenchTalib', 'Help')) { continue }
    $forward[$k] = $PSBoundParameters[$k]
}

if ($BenchTalib) {
    $bench = Join-Path $root "scripts\bench-vs-talib.ps1"
    if (-not (Test-Path $bench)) {
        throw "missing $bench — implement Step 5 of the plan first"
    }
    & $bench @PSBoundParameters
    exit $LASTEXITCODE
}

& $unified @forward
exit $LASTEXITCODE
