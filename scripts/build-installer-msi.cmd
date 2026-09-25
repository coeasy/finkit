@echo off
REM ----------------------------------------------------------------------------
REM AlphaTA MSI builder (Windows).
REM
REM Pre-requisites:
REM   * WiX Toolset 3.x on PATH (candle.exe, light.exe, heat.exe)
REM   * PowerShell (for the LICENSE → RTF conversion)
REM
REM Usage:
REM   scripts\build-installer-msi.cmd
REM ----------------------------------------------------------------------------

setlocal EnableDelayedExpansion
set ROOT=%~dp0..
set VERSION=
for /f "delims=" %%v in ('findstr /R "^version" "%ROOT%\Cargo.toml"') do (
  set LINE=%%v
  if not defined VERSION (
    for /f "tokens=2 delims= " %%a in ("!LINE!") do set VERSION=%%~a
  )
)
echo [build-installer-msi] AlphaTA version: %VERSION%

set WIX=%~dp0..\packaging\wix
set WIXOBJ=%WIX%\obj
if not exist "%WIXOBJ%" mkdir "%WIXOBJ%"

REM --- Preflight ------------------------------------------------------------
REM Fail here, with a precise message, rather than mid-harvest with a confusing
REM WiX error. Two inputs are easy to be missing: the release DLL (needs a
REM release build first) and the WiX product definition.
set DLL=%ROOT%\target\release\finkit_ffi.dll
if not exist "%DLL%" (
  echo [build-installer-msi] missing %DLL% >&2
  echo [build-installer-msi] run: cargo build --release -p finkit-ffi >&2
  exit /b 1
)
if not exist "%WIX%\Product.wxs" (
  echo [build-installer-msi] missing %WIX%\Product.wxs >&2
  echo [build-installer-msi] this repository has no WiX product definition; >&2
  echo [build-installer-msi] the MSI target cannot be built from a clean checkout. >&2
  exit /b 1
)

REM --- Stage the native binaries + headers into packaging\wix\stage ----
REM The crate is `finkit-ffi` with no explicit [lib] name, so the artifacts are
REM finkit_ffi.* -- not AlphaTA_ffi.*, which is what this script used to look
REM for. `copy` does not fail the script, so the stale name silently produced an
REM MSI whose bin\ was empty.
set STAGE=%WIX%\stage
if exist "%STAGE%" rmdir /s /q "%STAGE%"
mkdir "%STAGE%\bin" "%STAGE%\lib" "%STAGE%\include" "%STAGE%\share\finkit"
copy /y "%ROOT%\ffi\c-binding\include\*.h"   "%STAGE%\include\"  >nul
copy /y "%ROOT%\ffi\c-binding\include\*.hpp" "%STAGE%\include\"  >nul
copy /y "%DLL%"                                     "%STAGE%\bin\"  >nul
copy /y "%ROOT%\target\release\finkit_ffi.dll.lib"  "%STAGE%\lib\"  >nul
copy /y "%ROOT%\target\release\finkit_ffi.lib"      "%STAGE%\lib\"  >nul
if exist "%ROOT%\LICENSE" copy /y "%ROOT%\LICENSE" "%STAGE%\share\finkit\"

REM --- Convert LICENSE to RTF for the WixUI_Minimal licence page ----
powershell -NoProfile -Command ^
  "Get-Content -Raw '%ROOT%\LICENSE' | Out-File -Encoding ascii '%WIX%\License.txt';" ^
  "if (-not (Test-Path '%WIX%\License.rtf')) {" ^
  "  $rtf = '{\rtf1\ansi\ansicpg1252\deff0\nouicompat\deflang1033' + " ^
  "        [System.IO.File]::ReadAllText('%WIX%\License.txt').Replace('\','\\\\').Replace('{','\{').Replace('}','\}').Replace('`n','\par ') + '}';" ^
  "  [System.IO.File]::WriteAllText('%WIX%\License.rtf', $rtf)" ^
  "}"

REM --- Harvest the staged payload ----
heat.exe dir "%STAGE%" -cg FinkitComponentGroup -dr INSTALLDIR ^
  -srd -scom -sreg -sfrag -sb -out "%WIXOBJ%\harvested.wxs"
if errorlevel 1 goto :err

candle.exe -ext WixUIExtension -out "%WIXOBJ%\\" "%WIX%\Product.wxs" "%WIXOBJ%\harvested.wxs"
if errorlevel 1 goto :err

set MSI_OUT=%ROOT%\dist\installer\finkit-%VERSION%-x86_64-pc-windows-msvc.msi
if not exist "%ROOT%\dist\installer" mkdir "%ROOT%\dist\installer"
light.exe -ext WixUIExtension ^
  -out "%MSI_OUT%" ^
  "%WIXOBJ%\Product.wixobj" "%WIXOBJ%\harvested.wixobj"
if errorlevel 1 goto :err

echo [build-installer-msi] OK: %MSI_OUT%
exit /b 0

:err
echo [build-installer-msi] FAILED >&2
exit /b 1
