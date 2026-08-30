<#
.SYNOPSIS
  AlphaTA — one-click QUICK multi-language package builder (Windows / PowerShell).

.DESCRIPTION
  Fast one-command builder for all 7 usage packages
  (python/node/java/go/c/dotnet/wasm). Mirrors build-quick.sh:
    1. one combined `cargo build --release` of every native cdylib
       (the shared core is compiled exactly once),
    2. the 7 per-language packaging scripts are fanned out in parallel
       (their internal cargo builds are no-ops, so packaging overlaps).
  Verify and bundle are OFF by default for speed. Opt in with switches.

.PARAMETER Languages
  Optional subset of: python, node, java, go, c, dotnet, wasm.
  When omitted, all 7 languages are built.

.PARAMETER Verify
  Also install + smoke-test each produced artifact (delegates to
  scripts/build-usage-packages.sh --verify).

.PARAMETER Bundle
  Also zip dist/ into a usage-bundle (.zip).

.PARAMETER Clean
  Wipe dist/ before building.

.PARAMETER Help
  Print this synopsis.

.EXAMPLE
  pwsh ./build-quick.ps1
  pwsh ./build-quick.ps1 c go node
  pwsh ./build-quick.ps1 -Bundle
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string[]] $Languages = @(),

    [switch] $Verify,
    [switch] $Bundle,
    [switch] $Clean,
    [switch] $Help
)

$ErrorActionPreference = "Continue"   # we collect per-language failures, not abort

# ---- paths ----------------------------------------------------------------
$root  = (Resolve-Path (Split-Path $MyInvocation.MyCommand.Path)).Path
$dist  = Join-Path $root "dist"
$logDir = Join-Path $dist ".build-quick"
$ver   = (Select-String -Path (Join-Path $root "Cargo.toml") -Pattern '^version' | Select-Object -First 1) -replace '.*"([^"]+)".*', '$1'
$platform = "windows-x64"

# bash used to drive the existing *.sh per-language builders (no PS1 twins exist)
$bash = if ($env:GIT_BASH) { $env:GIT_BASH }
        elseif (Get-Command bash -ErrorAction SilentlyContinue) { (Get-Command bash).Source }
        else { "C:\Program Files\Git\bin\bash.exe" }

# ---- CLI ------------------------------------------------------------------
$all = @("python","node","java","go","c","dotnet","wasm")
if ($Help) {
    Get-Help $MyInvocation.MyCommand.Path -Full
    exit 0
}
if ($Languages.Count -eq 0) { $WANT = $all } else { $WANT = $Languages }

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "[build-quick] FATAL: cargo not found on PATH"; exit 1
}

if ($Clean) {
    Write-Host "`n=== clean ===" -ForegroundColor Cyan
    Remove-Item -Recurse -Force $dist -ErrorAction SilentlyContinue
    Write-Host "  removed $dist"
}
New-Item -ItemType Directory -Force -Path $dist, $logDir | Out-Null

function Ok   { param($m) Write-Host "  [OK]   $m" -ForegroundColor Green }
function Err  { param($m) Write-Host "  [FAIL] $m" -ForegroundColor Red }
function Info { param($m) Write-Host "  [INFO] $m" -ForegroundColor Cyan }
function Hdr  { param($m) Write-Host "`n=== $m ===" -ForegroundColor Cyan }

# ===================== STEP 1: combined cargo pre-build =====================
Hdr "step 1/3 - combined cargo pre-build (core compiled once)"
$nativeCrates = @()
$wantPython = $false
$wantWasm   = $false
foreach ($lang in $WANT) {
    switch ($lang) {
        "python" { $wantPython = $true }
        "wasm"   { $wantWasm   = $true }
        "node"    { $nativeCrates += "alpha-ta-node" }
        "go"      { $nativeCrates += "alpha-ta-go" }
        "c"       { $nativeCrates += "alpha-ta-ffi" }
        "dotnet"  { $nativeCrates += "alpha-ta-dotnet" }
        "java"    { $nativeCrates += "alpha-ta-java" }
    }
}
$t0 = Get-Date
if ($nativeCrates.Count -gt 0) {
    Info "cargo build --release $($nativeCrates | ForEach-Object { "-p $_" } | Join-String -Separator ' ')"
    & cargo build --release $( $nativeCrates | ForEach-Object { "-p", $_ } )
    if ($LASTEXITCODE -ne 0) { Write-Warning "[build-quick] native pre-build failed; per-language scripts will rebuild" }
}
if ($wantPython) {
    Info "cargo build --release -p alpha-ta-python --features abi3"
    & cargo build --release -p alpha-ta-python --features abi3
    if ($LASTEXITCODE -ne 0) { Write-Warning "[build-quick] python pre-build failed; build-usage-python.sh will rebuild" }
}
if ($wantWasm) {
    Info "wasm: ensuring wasm32-unknown-unknown target"
    & rustup target add wasm32-unknown-unknown 2>$null
}
$t1 = Get-Date
Info ("pre-build took {0}s" -f [int]($t1 - $t0).TotalSeconds)

# ===================== STEP 2: parallel per-language packaging ==============
Hdr "step 2/3 - parallel per-language packaging"
$jobs = @()
foreach ($lang in $WANT) {
    $log = Join-Path $logDir "$lang.log"
    Info "launching build-usage-$lang.sh  (log: $log)"
    $p = Start-Process -FilePath $bash `
        -ArgumentList "$root/scripts/build-usage-$lang.sh" `
        -RedirectStandardOutput $log -RedirectStandardError $log `
        -PassThru -NoNewWindow
    $jobs += @{ lang = $lang; proc = $p }
}
$fail = 0; $ok = 0
foreach ($j in $jobs) {
    $j.proc.WaitForExit()
    if ($j.proc.ExitCode -eq 0) { Ok "$($j.lang) packaged"; $ok++ }
    else { Err "$($j.lang) packaging FAILED -- see $logDir/$($j.lang).log"; $fail++ }
}

# ===================== STEP 3: verify / bundle / manifest ===================
Hdr "step 3/3 - verify / bundle / manifest"
if ($Verify) {
    Info "verify requested -- delegating to build-usage-packages.sh --verify"
    & $bash "$root/scripts/build-usage-packages.sh" $WANT --verify --no-bundle
    if ($LASTEXITCODE -ne 0) { Err "verify step reported failures" }
}

# manifest
Info "writing dist/manifest.json"
$LANGS_CSV = $WANT -join ','
$py = @"
import hashlib, json, os, pathlib
root = pathlib.Path(r"$dist")
langs = "$LANGS_CSV".split(",")
components = []
for lang in langs:
    base = root / lang / "$platform"
    if not base.exists():
        continue
    for f in base.rglob("*"):
        if not f.is_file():
            continue
        if any(part.startswith('.') for part in f.parts):
            continue
        if f.suffix == ".pyc":
            continue
        components.append({
            "language": lang,
            "platform": "$platform",
            "path": str(f.relative_to(root)).replace(os.sep, "/"),
            "size_bytes": f.stat().st_size,
            "sha256": hashlib.sha256(f.read_bytes()).hexdigest(),
        })
manifest = {"name": "AlphaTA", "version": "$ver", "platform": "$platform", "quick_build": True, "components": components}
out = root / "manifest.json"
out.write_text(json.dumps(manifest, indent=2))
print(f"wrote {out} ({len(components)} components)")
"@
& python3 -c $py

# bundle
if ($Bundle -and $fail -eq 0) {
    $BUNDLE = Join-Path $dist "alpha-ta-$ver-$platform-quick-bundle.zip"
    Info "bundling into $BUNDLE"
    $srcs = @()
    foreach ($lang in $WANT) { $srcs += (Join-Path $dist $lang $platform) }
    $srcs += (Join-Path $dist "manifest.json")
    Compress-Archive -Path $srcs -DestinationPath $BUNDLE -Force
    if (Test-Path $BUNDLE) { Ok "bundle: $BUNDLE" } else { Err "Compress-Archive failed" }
}

# summary
Hdr "summary"
Write-Host "  platform : $platform"
Write-Host "  version  : $ver"
Write-Host "  packaged : $ok"
Write-Host "  failed   : $fail"
Write-Host "  verify   : $($Verify.IsPresent)"
Write-Host "  bundle   : $($Bundle.IsPresent)"
Write-Host "  logs     : $logDir/<lang>.log"
exit $(if ($fail -eq 0) { 0 } else { 1 })
