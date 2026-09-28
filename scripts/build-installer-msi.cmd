@echo off
REM ----------------------------------------------------------------------------
REM Finkit MSI builder (Windows) - thin launcher.
REM
REM The MSI is built by `scripts/build-installer.sh --target msi`, which stages
REM the payload, renders the license RTF and invokes candle.exe/light.exe.
REM This launcher exists so Windows users have something to double-click; it
REM deliberately contains **no build logic of its own**, because a second copy
REM of that logic is how this repository ended up with two installers whose
REM payloads disagreed.
REM
REM Pre-requisites:
REM   * WiX Toolset 3.x on PATH (candle.exe, light.exe)
REM   * bash (Git for Windows provides one at %ProgramFiles%\Git\bin\bash.exe)
REM
REM Usage:
REM   scripts\build-installer-msi.cmd
REM ----------------------------------------------------------------------------

setlocal EnableDelayedExpansion
set ROOT=%~dp0..

REM Work out the version only to print it; the real reader lives in the shell
REM script (and in scripts/build_native_archive.py) so there is one parser.
for /f "tokens=3" %%v in ('findstr /R "^version" "%ROOT%\Cargo.toml"') do (
  if not defined VERSION set VERSION=%%~v
)

if not defined VERSION set VERSION=unknown
echo [build-installer-msi] Finkit version: %VERSION%

set INSTALLER_SCRIPT=%ROOT%\scripts\build-installer.sh

REM Prefer whichever bash is present. Naming each candidate is worth it: a bare
REM `bash` fails with an unhelpful message on machines that have Git installed
REM but not added to PATH.
set BASH_EXE=
for %%c in (
  "%ProgramFiles%\Git\bin\bash.exe"
  "%ProgramFiles(x86)%\Git\bin\bash.exe"
  "%LocalAppData%\Programs\Git\bin\bash.exe"
) do (
  if not defined BASH_EXE if exist %%c set BASH_EXE=%%~c
)
if not defined BASH_EXE (
  for %%c in (bash.exe) do if not defined BASH_EXE set BASH_EXE=%%~$PATH:c
)

if not defined BASH_EXE (
  echo [build-installer-msi] no bash found. >&2
  echo [build-installer-msi] install Git for Windows, or build the MSI with: >&2
  echo [build-installer-msi]   bash "%INSTALLER_SCRIPT%" --target msi >&2
  exit /b 1
)

"%BASH_EXE%" "%INSTALLER_SCRIPT%" --target msi
if errorlevel 1 (
  echo [build-installer-msi] FAILED >&2
  exit /b 1
)

echo [build-installer-msi] OK
exit /b 0
