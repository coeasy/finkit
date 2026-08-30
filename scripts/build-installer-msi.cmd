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

REM --- Stage the native binaries + headers into packaging\wix\stage ----
set STAGE=%WIX%\stage
if exist "%STAGE%" rmdir /s /q "%STAGE%"
mkdir "%STAGE%\lib" "%STAGE%\include" "%STAGE%\share\AlphaTA"
copy /y "%ROOT%\ffi\c-binding\include\*.h"   "%STAGE%\include\"  >nul
copy /y "%ROOT%\ffi\c-binding\include\*.hpp" "%STAGE%\include\"  >nul
copy /y "%ROOT%\target\release\AlphaTA_ffi.dll"      "%STAGE%\bin\"  >nul
copy /y "%ROOT%\target\release\AlphaTA_ffi.dll.lib"  "%STAGE%\lib\"  >nul
copy /y "%ROOT%\target\release\AlphaTA_ffi.lib"      "%STAGE%\lib\"  >nul
if exist "%ROOT%\LICENSE" copy /y "%ROOT%\LICENSE" "%STAGE%\share\AlphaTA\"

REM --- Convert LICENSE to RTF for the WixUI_Minimal licence page ----
powershell -NoProfile -Command ^
  "Get-Content -Raw '%ROOT%\LICENSE' | Out-File -Encoding ascii '%WIX%\License.txt';" ^
  "if (-not (Test-Path '%WIX%\License.rtf')) {" ^
  "  $rtf = '{\rtf1\ansi\ansicpg1252\deff0\nouicompat\deflang1033' + " ^
  "        [System.IO.File]::ReadAllText('%WIX%\License.txt').Replace('\','\\\\').Replace('{','\{').Replace('}','\}').Replace('`n','\par ') + '}';" ^
  "  [System.IO.File]::WriteAllText('%WIX%\License.rtf', $rtf)" ^
  "}"

REM --- Harvest the staged payload ----
heat.exe dir "%STAGE%" -cg AlphaTAComponentGroup -dr INSTALLDIR ^
  -srd -scom -sreg -sfrag -sb -out "%WIXOBJ%\harvested.wxs"
if errorlevel 1 goto :err

candle.exe -ext WixUIExtension -out "%WIXOBJ%\\" "%WIX%\Product.wxs" "%WIXOBJ%\harvested.wxs"
if errorlevel 1 goto :err

set AlphaTA_VERSION=%VERSION%
light.exe -ext WixUIExtension ^
  -out "%ROOT%\dist\installer\alpha-ta-%VERSION%-x86_64-pc-windows-msvc.msi" ^
  "%WIXOBJ%\Product.wixobj" "%WIXOBJ%\harvested.wixobj"
if errorlevel 1 goto :err

echo [build-installer-msi] OK: %ROOT%\dist\installer\alpha-ta-%VERSION%-x86_64-pc-windows-msvc.msi
exit /b 0

:err
echo [build-installer-msi] FAILED >&2
exit /b 1
