<#
.SYNOPSIS
  Clean transient debug / build-test logs and temp directories that
  accumulate at the repo root when building or debugging.

.DESCRIPTION
  The build scripts under scripts/ are supposed to be the only thing that
  writes into dist/ and .test_venv/. Any *.log / *.err / *.obj file at the
  repo root, or stray extraction directories under .test_venv/, is almost
  always a debugging artifact. This script removes them.

  Run it any time the working tree gets noisy, or wire it into your editor
  / pre-commit as desired.

.EXAMPLE
  pwsh ./scripts/clean-logs.ps1
#>
[CmdletBinding()]
param(
    [switch] $DryRun
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path "$PSScriptRoot\..").Path

# Files that are not part of the project and exist only as debug output.
# Each entry is a literal name (no extension filter — many of the old logs
# were saved without a .log suffix).
$RootArtifacts = @(
    'build_c','build_dotnet','build_err','build_full','build_full2',
    'build_go','build_java','build_lib','build_node','build_py',
    'build_short','build_verify','build','bundle_run','cargo_check',
    'clippy_verify','debug','debug2','deep_test','doc_test',
    'dotnet_cargo','doc_warnings.txt','fix1.err','fix_test','fix1',
    'fix2','full_test','go_ta_build','int_test','int_test2','int_test3',
    'int_test4','pt_build','pt_build2','pt_test','pt_test2','pt_test3',
    'pt_test4','publish_dry_run','simd_test','test_core_verify',
    'test_link.obj','test_link2.obj','test_output','test_verify','test1',
    'verify_all','wbench.txt','winget_gcc','winget_msys'
)

# Per-subdirectory clean-up: remove *.log files that the build scripts may
# have left behind when the developer piped a single command to a file.
$LogDirs = @(
    'core','scripts','wasm'
)

# Temp directories under .test_venv/ that get created when inspecting
# NuGet/nupkg layouts; they're always reproducible.
$TempDirs = @(
    'dotnet_unpack',
    'logs',
    '_usage_dotnet_feed'
)

function Remove-Path {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) { return }
    if ($DryRun) {
        Write-Host "would remove $Path"
    } else {
        Remove-Item -LiteralPath $Path -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "removed $Path"
    }
}

foreach ($name in $RootArtifacts) {
    Remove-Path -Path (Join-Path $Root $name)
}

foreach ($sub in $LogDirs) {
    $dir = Join-Path $Root $sub
    if (-not (Test-Path -LiteralPath $dir)) { continue }
    Get-ChildItem -LiteralPath $dir -File -Filter '*.log' -ErrorAction SilentlyContinue |
        ForEach-Object { Remove-Path -Path $_.FullName }
}

$TestVenv = Join-Path $Root '.test_venv'
foreach ($name in $TempDirs) {
    Remove-Path -Path (Join-Path $TestVenv $name)
}

if ($DryRun) {
    Write-Host "dry-run only — no files were removed"
} else {
    Write-Host "clean complete"
}
