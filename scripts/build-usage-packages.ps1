<#
.SYNOPSIS
  Finkit unified usage-package builder + verifier (PowerShell 7+).

.DESCRIPTION
  Windows-native equivalent of scripts/build-usage-packages.sh.
  For each enabled language this script:
    1. Invokes scripts/build-usage-<lang>.ps1 to produce the installable
       artifact in dist/<lang>/<platform>/.
    2. Invokes packaging/usage/<lang>/tests/* to verify the artifact by
       actually installing + running it.
    3. Emits a SHA256 + size manifest to dist/manifest.json.
    4. Bundles the full tree into
       dist/finkit-<version>-<plat>-usage-bundle.zip
       (via Compress-Archive).

.PARAMETER Languages
  Subset of: python, node, java, go, c, dotnet, wasm. Default = all.

.PARAMETER NoBundle
  Skip the Compress-Archive step.

.PARAMETER NoVerify
  Skip the install/verify step (build only).

.PARAMETER Json
  Emit the manifest as JSON to stdout in addition to the file.

.PARAMETER OutDir
  Override the default dist/ output directory.

.EXAMPLE
  pwsh ./scripts/build-usage-packages.ps1
  pwsh ./scripts/build-usage-packages.ps1 python node
  pwsh ./scripts/build-usage-packages.ps1 -NoBundle
#>
[CmdletBinding()]
param(
    [string[]] $Languages = @("python", "node", "java", "go", "c", "dotnet", "wasm"),
    [switch] $NoBundle,
    [switch] $NoVerify,
    [switch] $Json,
    [string] $OutDir
)

$ErrorActionPreference = "Stop"

$ScriptDir = (Resolve-Path "$PSScriptRoot").Path
$Root      = (Resolve-Path "$PSScriptRoot\..").Path
$Dist      = if ($OutDir) { (Resolve-Path $OutDir).Path } else { Join-Path $Root "dist" }
$null      = New-Item -ItemType Directory -Force -Path $Dist

# --- version ----------------------------------------------------------------
$versionLine = Select-String -Path (Join-Path $Root "Cargo.toml") -Pattern '^version' | Select-Object -First 1
$Version = if ($versionLine) { ($versionLine.ToString() | Select-String '"([^"]+)"').Matches[0].Groups[1].Value } else { "0.0.0" }

# --- platform ----------------------------------------------------------------
# Use PowerShell 7 idiomatic checks; $env:OS is "Windows_NT" in CMD, useless
# for distinguishing Windows / Linux / macOS.
if     ($IsWindows) { $Platform = "windows-x64" }
elseif ($IsLinux)   { $Platform = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "linux-arm64" } else { "linux-x64" } }
elseif ($IsMacOS)  { $Platform = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "macos-arm64" } else { "macos-x64" } }
else   { Write-Error "unsupported platform: $($PSVersionTable.OS)"; exit 1 }

# --- helpers ----------------------------------------------------------------
function Write-Ok   ([string]$m) { Write-Host "  [OK]   $m" -ForegroundColor Green }
function Write-Err  ([string]$m) { Write-Host "  [FAIL] $m" -ForegroundColor Red }
function Write-Info ([string]$m) { Write-Host "  [INFO] $m" -ForegroundColor Cyan }
function Write-Hdr  ([string]$m) { Write-Host ""; Write-Host "=== $m ===" -ForegroundColor DarkCyan }

$Py = "python"
$Pip = "pip"

# --- builders / verifiers ---------------------------------------------------
# Generic dispatch helper: try <lang>.ps1, fall back to <lang>.sh via bash.
# Lets us re-use the existing per-language bash scripts on Windows until
# the .ps1 wrappers are written.
function Invoke-LangBuilder {
    param([string] $Lang)
    $ps1 = Join-Path $ScriptDir "build-usage-$Lang.ps1"
    $sh  = Join-Path $ScriptDir "build-usage-$Lang.sh"
    if (Test-Path $ps1) {
        & $ps1
    } elseif (Test-Path $sh) {
        & bash $sh
    } else {
        throw "no builder for $Lang (looked for $ps1 and $sh)"
    }
}

$Builders = [ordered]@{
    python = { Invoke-LangBuilder 'python' }
    node   = { Invoke-LangBuilder 'node'   }
    java   = { Invoke-LangBuilder 'java'   }
    go     = { Invoke-LangBuilder 'go'     }
    c      = { Invoke-LangBuilder 'c'      }
    dotnet = { Invoke-LangBuilder 'dotnet' }
    wasm   = { Invoke-LangBuilder 'wasm'   }
}

$Verifiers = [ordered]@{
    python = {
        # Accept abi3 OR cp<XY>-cp<XY>-*.whl (maturin picks based on cargo
        # features; either is valid for a smoke test).
        $distPy = Join-Path $Dist "python\$Platform"
        $whl = Get-ChildItem -Path $distPy -Filter "finkit-*.whl" -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -match 'finkit-.*-abi3-.*\.whl|finkit-.*-cp\d+-.*\.whl' } |
            Select-Object -First 1
        if (-not $whl) { throw "no wheel to verify in $distPy" }
        $venv = Join-Path $Root ".test_venv\_usage_python"
        $null  = New-Item -ItemType Directory -Force -Path (Join-Path $Root ".test_venv")
        if (-not (Test-Path $venv)) { & $Py -m venv $venv }
        $pipExe = Join-Path $venv "Scripts\pip.exe"
        $pyExe  = Join-Path $venv "Scripts\python.exe"
        # The finkit Python binding is pyO3 + numpy-aware; install numpy
        # in the venv first (the wheel's first call panics otherwise).
        & $pipExe install --quiet numpy
        if ($LASTEXITCODE -ne 0) { throw "pip install numpy failed" }
        & $pipExe install --quiet $whl.FullName
        if ($LASTEXITCODE -ne 0) { throw "pip install wheel failed" }
        & $pyExe (Join-Path $Root "packaging\usage\python\verify_install.py")
        if ($LASTEXITCODE -ne 0) { throw "verify_install.py exited $LASTEXITCODE" }
    }
    node = {
        $tgz = Get-ChildItem -Path (Join-Path $Dist "node\$Platform") -Filter "finkit-*.tgz" -ErrorAction SilentlyContinue | Select-Object -First 1
        if (-not $tgz) { throw "no tgz to verify" }
        $scratch = Join-Path $Root ".test_venv\_usage_node"
        Remove-Item -Recurse -Force $scratch -ErrorAction SilentlyContinue
        $null = New-Item -ItemType Directory -Force -Path $scratch
        Push-Location $scratch
        try {
            npm init -y | Out-Null
            npm install --silent $tgz.FullName | Out-Null
            Copy-Item (Join-Path $Root "packaging\usage\node\verify_install.js") .
            node verify_install.js
        } finally { Pop-Location }
    }
    java = {
        $jar = Get-ChildItem -Path (Join-Path $Dist "java\$Platform") -Filter "finkit-*.jar" -ErrorAction SilentlyContinue | Select-Object -First 1
        if (-not $jar) { throw "no jar to verify" }
        $tmp = Join-Path $Root ".test_venv\_usage_java"
        Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
        $null = New-Item -ItemType Directory -Force -Path $tmp
        Copy-Item (Join-Path $Root "packaging\usage\java\verify_install.java") $tmp
        Push-Location $tmp
        try {
            javac -cp $jar.FullName verify_install.java
            java -cp ".;$($jar.FullName)" verify_install
        } finally { Pop-Location }
    }
    go = {
        $mod = Join-Path $Root "packaging\usage\go\tests\go.mod"
        $goDist = Join-Path $Dist "go\$Platform\finkit"
        if (-not (Test-Path $goDist)) { throw "no go module to verify" }
        (Get-Content $mod) -replace 'replace github.com/coeasy/finkit => .*', "replace github.com/coeasy/finkit => $goDist" | Set-Content $mod
        Push-Location (Join-Path $Root "packaging\usage\go\tests")
        try { & go run ..\verify_install.go } finally { Pop-Location }
    }
    c = {
        $bash = "bash"
        $test = Join-Path $Root "packaging\usage\c\tests\test_c_install.sh"
        if (Test-Path $test) { & $bash $test } else { Write-Warn "no c verifier — skipping" }
    }
    dotnet = {
        $nupkg = Get-ChildItem -Path (Join-Path $Dist "dotnet\$Platform") -Filter "Finkit.*.nupkg" -ErrorAction SilentlyContinue | Select-Object -First 1
        if (-not $nupkg) { throw "no nupkg to verify" }
        $feed = Join-Path $Root ".test_venv\_usage_dotnet_feed"
        Remove-Item -Recurse -Force $feed -ErrorAction SilentlyContinue
        $null = New-Item -ItemType Directory -Force -Path $feed
        Copy-Item $nupkg.FullName $feed
        Push-Location (Join-Path $Root "packaging\usage\dotnet\tests")
        try {
            # Build + run on the same line so we can inspect $LASTEXITCODE.
            # `dotnet run` swallows non-zero exit codes when piped, so
            # we explicitly capture and rethrow.
            $out = & dotnet run --source $feed 2>&1
            $rc  = $LASTEXITCODE
            $out | Select-Object -Last 10
            if ($rc -ne 0) { throw "dotnet run exited $rc" }
        } finally { Pop-Location }
    }
    wasm = {
        Push-Location (Join-Path $Root "packaging\usage\wasm\tests")
        try { & node verify_install.js } finally { Pop-Location }
    }
}

# --- main loop --------------------------------------------------------------
Write-Hdr "Finkit usage-package builder"
Write-Info "version : $Version"
Write-Info "platform: $Platform"
Write-Info "langs   : $($Languages -join ', ')"
Write-Info "verify  : $(-not $NoVerify)"
Write-Info "bundle  : $(-not $NoBundle)"

$okCount  = 0
$failCount = 0

foreach ($lang in $Languages) {
    Write-Hdr "[$lang] build"
    try {
        & $Builders[$lang]
        Write-Ok "$lang build"
    } catch {
        Write-Err "$lang build failed: $_"
        $failCount++
        continue
    }

    if (-not $NoVerify) {
        Write-Hdr "[$lang] verify (install + run)"
        try {
            & $Verifiers[$lang]
            Write-Ok "$lang verify"
            $okCount++
        } catch {
            Write-Err "$lang verify failed: $_"
            $failCount++
        }
    }
}

# --- manifest ---------------------------------------------------------------
Write-Hdr "manifest"
$langs = $Languages -join ","
$manifestPath = Join-Path $Dist "manifest.json"

# Write the inline Python to a temp script. PowerShell doesn't support
# bash-style heredocs, so we use a here-string instead.
$manifestPy = @'
import hashlib, json, os, pathlib
root = pathlib.Path(os.environ["DIST"])
langs = os.environ["LANGS"].split(",")
plat  = os.environ["PLATFORM"]
components = []
for lang in langs:
    base = root / lang / plat
    if not base.exists():
        continue
    for f in base.rglob("*"):
        if not f.is_file():
            continue
        if any(part.startswith(".") for part in f.parts):
            continue
        if f.suffix in {".pyc"}:
            continue
        components.append({
            "language": lang,
            "platform": plat,
            "path": str(f.relative_to(root)).replace(os.sep, "/"),
            "size_bytes": f.stat().st_size,
            "sha256": hashlib.sha256(f.read_bytes()).hexdigest(),
        })
manifest = {
    "name": "finkit",
    "version": os.environ["VERSION"],
    "platform": plat,
    "components": components,
}
pathlib.Path(os.environ["MANIFEST_OUT"]).write_text(json.dumps(manifest, indent=2))
print(f"wrote {os.environ['MANIFEST_OUT']} ({len(components)} components)")
'@

$env:DIST         = $Dist
$env:LANGS        = $langs
$env:PLATFORM     = $Platform
$env:VERSION      = $Version
$env:MANIFEST_OUT = $manifestPath

$tmpPy = [System.IO.Path]::GetTempFileName() + ".py"
[System.IO.File]::WriteAllText($tmpPy, $manifestPy, [System.Text.Encoding]::UTF8)
try {
    & $Py $tmpPy
} finally {
    Remove-Item $tmpPy -ErrorAction SilentlyContinue
}

if ($Json) { Get-Content $manifestPath }

# --- bundle -----------------------------------------------------------------
if (-not $NoBundle -and $failCount -eq 0) {
    Write-Hdr "bundle"
    $bundle = Join-Path $Dist "finkit-$Version-$Platform-usage-bundle.zip"
    $paths = @()
    foreach ($l in $Languages) {
        $p = Join-Path $Dist "$l\$Platform"
        if (Test-Path $p) { $paths += $p }
    }
    $wasmPath = Join-Path $Dist "wasm"
    if (Test-Path $wasmPath) { $paths += $wasmPath }
    $paths += $manifestPath
    try {
        Compress-Archive -Path $paths -DestinationPath $bundle -Force
        Write-Ok "bundle: $bundle"
    } catch {
        Write-Err "Compress-Archive failed: $_"
    }
}

Write-Hdr "summary"
Write-Host "  built+verified: $okCount"
Write-Host "  failed        : $failCount"

if ($failCount -ne 0) { exit 1 } else { exit 0 }
