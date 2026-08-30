<#
.SYNOPSIS
  AlphaTA — install + smoke test every built artifact on Windows.

.DESCRIPTION
  PowerShell 7+ equivalent of scripts/install-and-test.sh.
  For each language in {python, node, java, go, c, dotnet, wasm}:
    1. Skip if dist/<lang>/<platform>/ is already populated.
    2. Otherwise run scripts/build-usage-packages.ps1 to build.
    3. Run the same script with --no-bundle to install + verify.
    4. Capture logs in .test_venv/logs/<lang>.log.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$ScriptDir = (Resolve-Path "$PSScriptRoot").Path
$Root      = (Resolve-Path "$ScriptDir\..").Path
$Unified   = Join-Path $Root "scripts\build-usage-packages.ps1"
$LogDir    = Join-Path $Root ".test_venv\logs"
$null      = New-Item -ItemType Directory -Force -Path $LogDir

$Languages = @("python", "node", "java", "go", "c", "dotnet", "wasm")

# platform detection — use PowerShell 7 idiomatic checks, not $env:OS
# (which is "Windows_NT" in CMD, useless for our purpose).
$Platform = if     ($IsWindows) { "windows-x64" }
            elseif ($IsLinux)   { if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "linux-arm64" } else { "linux-x64" } }
            elseif ($IsMacOS)  { if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "macos-arm64" } else { "macos-x64" } }
            else   { throw "unsupported platform: $($PSVersionTable.OS)" }

function Write-Hdr  ([string]$m) { Write-Host ""; Write-Host "=== $m ===" -ForegroundColor DarkCyan }
function Write-Ok   ([string]$m) { Write-Host "  [OK]   $m" -ForegroundColor Green }
function Write-Err  ([string]$m) { Write-Host "  [FAIL] $m" -ForegroundColor Red }
function Write-Warn ([string]$m) { Write-Host "  [SKIP] $m" -ForegroundColor Yellow }

# 1. figure out who needs a build
$needsBuild = @()
foreach ($lang in $Languages) {
    $dist = Join-Path $Root "dist\$lang\$Platform"
    if (Test-Path $dist) {
        $hasFiles = (Get-ChildItem -Path $dist -Force -ErrorAction SilentlyContinue | Select-Object -First 1)
        if (-not $hasFiles) { $needsBuild += $lang }
    } else {
        $needsBuild += $lang
    }
}

if ($needsBuild.Count -gt 0) {
    Write-Hdr "pre-build: $($needsBuild -join ', ')"
    & $Unified @needsBuild -NoBundle -NoVerify | Out-Null
}

$ok = 0
$fail = 0
$failed = @()

# 2. verify each language in isolation
foreach ($lang in $Languages) {
    Write-Hdr "[$lang] install + smoke"
    $log = Join-Path $LogDir "$lang.log"
    # Run the unified builder and capture its output. The script returns
    # 0 on success (1+ ok, 0 fail) and non-zero on any failure, so we use
    # the exit code as the primary success signal — parsing the text is
    # unreliable because Write-Host lines don't reach the output stream.
    & $Unified @($lang) -NoBundle 2>&1 | Tee-Object -FilePath $log | Out-Null
    $rc = $LASTEXITCODE
    if ($rc -eq 0) {
        Write-Ok $lang
        $ok++
    } else {
        Write-Err "$lang (exit=$rc, see $log)"
        $failed += $lang
        $fail++
    }
}

Write-Hdr "summary"
Write-Host "  ok      : $ok"
Write-Host "  failed  : $fail"
if ($fail -gt 0) {
    Write-Host "  failed languages: $($failed -join ', ')"
    Write-Host "  inspect logs:    Get-ChildItem '$LogDir'"
}
exit $fail
