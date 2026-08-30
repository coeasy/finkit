<#
.SYNOPSIS
  AlphaTA — toolchain preflight check (PowerShell 7+).

.DESCRIPTION
  Verifies that the host has the toolchain needed for a one-click build.
  Returns 0 on success, 1 if any hard requirement is missing.

.PARAMETER Json
  Emit the result as a JSON object on stdout (for CI consumption).
#>
[CmdletBinding()]
param([switch] $Json)

$ErrorActionPreference = "Continue"

$ScriptDir = (Resolve-Path "$PSScriptRoot").Path
$Root      = (Resolve-Path "$PSScriptDir\..\..").Path

$ok_n = 0
$warn_n = 0
$miss_n = 0
$results = @()

function Add-Result {
    param(
        [string] $Name,
        [string] $Status,   # OK / WARN / MISS
        [string] $Version,
        [string] $Why,
        [string] $Install
    )
    $script:results += [pscustomobject]@{
        name    = $Name
        status  = $Status
        version = $Version
        why     = $Why
        install = $Install
    }
    switch ($Status) {
        "OK"   { $script:ok_n++;   Write-Host "  [OK]   $Name $Version -- $Why" -ForegroundColor Green }
        "WARN" { $script:warn_n++; Write-Host "  [WARN] $Name missing -- $Why" -ForegroundColor Yellow }
        "MISS" { $script:miss_n++; Write-Host "  [MISS] $Name MISSING -- $Why" -ForegroundColor Red }
    }
    if ($Install) { Write-Host "        install: $Install" -ForegroundColor DarkGray }
}

function Test-Tool {
    param(
        [string] $Name,
        [string] $Why,
        [string] $Install,
        [string] $Required = "soft"
    )
    $cmd = Get-Command $Name -ErrorAction SilentlyContinue
    if ($cmd) {
        $ver = & $Name --version 2>$null | Select-Object -First 1
        if (-not $ver) { $ver = "(found)" }
        Add-Result -Name $Name -Status "OK" -Version "$ver" -Why $Why -Install $Install
    } else {
        $st = if ($Required -eq "hard") { "MISS" } else { "WARN" }
        Add-Result -Name $Name -Status $st -Version "" -Why $Why -Install $Install
    }
}

function Write-Hdr ([string]$m) { Write-Host ""; Write-Host "=== $m ===" -ForegroundColor DarkCyan }

# ---- hard requirements ---------------------------------------------------
Write-Hdr "hard requirements"
Test-Tool -Name "cargo"   -Why "Rust compiler & build tool"        -Install "https://rustup.rs"                    -Required hard
Test-Tool -Name "python"  -Why "Python interpreter + pip"          -Install "https://python.org"                   -Required hard
Test-Tool -Name "pip"     -Why "Python package manager"            -Install "python -m ensurepip"                 -Required hard
Test-Tool -Name "maturin" -Why "Python wheel builder"              -Install "pip install maturin"                 -Required hard

# ---- per-language toolchains --------------------------------------------
Write-Hdr "per-language toolchains"
Test-Tool -Name "node"      -Why "Node.js 20+ (for tgz build)"   -Install "https://nodejs.org"                   -Required hard
Test-Tool -Name "npm"       -Why "Node package manager"          -Install "bundled with node"                   -Required hard
Test-Tool -Name "mvn"       -Why "Maven (Java build)"            -Install "https://maven.apache.org"             -Required hard
Test-Tool -Name "javac"     -Why "Java compiler (JDK 17+)"       -Install "https://adoptium.net"                 -Required hard
Test-Tool -Name "go"        -Why "Go 1.22+ (Go package build)"   -Install "https://go.dev/dl/"                   -Required hard
Test-Tool -Name "cmake"     -Why "CMake (C/C++ FFI install)"     -Install "https://cmake.org"                    -Required hard
Test-Tool -Name "cl"        -Why "MSVC C compiler (C FFI build)" -Install "Visual Studio Build Tools"           -Required hard
Test-Tool -Name "dotnet"    -Why ".NET SDK 8.0+ (NuGet build)"   -Install "https://dotnet.microsoft.com"         -Required hard
Test-Tool -Name "wasm-pack" -Why "WASM packager"                 -Install "cargo install wasm-pack"              -Required soft

Test-Tool -Name "talib"     -Why "TA-Lib PyPI (only for precision step)" -Install "pip install TA-Lib"           -Required soft
Test-Tool -Name "git"       -Why "git"                           -Install "https://git-scm.com"                  -Required soft

# ---- platform info --------------------------------------------------------
Write-Hdr "platform"
$Platform = switch ($env:OS) {
    "Windows" { "windows-x64" }
    "Linux"   { if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "linux-arm64" } else { "linux-x64" } }
    "Darwin"  { if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "macos-arm64" } else { "macos-x64" } }
    default   { "unknown" }
}
Write-Host "  os    : $($env:OS)"
Write-Host "  arch  : $($env:PROCESSOR_ARCHITECTURE)"
Write-Host "  plat  : $Platform"

# ---- summary -------------------------------------------------------------
Write-Hdr "summary"
if ($miss_n -eq 0) {
    Write-Host "  ready to run ./build-usage.ps1" -ForegroundColor Green
} else {
    Write-Host "  $miss_n hard requirement(s) missing" -ForegroundColor Red
    Write-Host "  Install the tools marked [MISS] above, then re-run." -ForegroundColor Red
}

if ($Json) {
    [pscustomobject]@{
        ready       = ($miss_n -eq 0)
        hard_missing = $miss_n
        warnings    = $warn_n
        ok          = $ok_n
        platform    = $Platform
        results     = $results
    } | ConvertTo-Json -Depth 4
}

exit $miss_n
