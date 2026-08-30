<#
.SYNOPSIS
    Check version consistency across all FTA package configuration files.
.PARAMETER Fix
    Automatically sync all versions to the workspace version.
#>
param(
    [switch]$Fix
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot

function Get-WorkspaceVersion {
    $cargoToml = Join-Path $Root "Cargo.toml"
    $content = Get-Content $cargoToml -Raw
    if ($content -match 'version\s*=\s*"([^"]+)"') {
        return $Matches[1]
    }
    throw "Cannot parse workspace version from $cargoToml"
}

function Get-CargoVersions {
    $results = @()
    $cargoFiles = Get-ChildItem -Path $Root -Recurse -Filter "Cargo.toml" |
        Where-Object { $_.FullName -ne (Join-Path $Root "Cargo.toml") }

    foreach ($file in $cargoFiles) {
        $content = Get-Content $file.FullName -Raw
        $relPath = $file.FullName.Substring($Root.Length + 1).Replace("\", "/")

        if ($content -match 'version\.workspace\s*=\s*true') {
            $results += [PSCustomObject]@{
                File    = $relPath
                Version = "workspace"
                Type    = "cargo"
                Raw     = $file.FullName
            }
        }
        elseif ($content -match '\[package\][\s\S]*?version\s*=\s*"([^"]+)"') {
            $results += [PSCustomObject]@{
                File    = $relPath
                Version = $Matches[1]
                Type    = "cargo"
                Raw     = $file.FullName
            }
        }
    }
    return $results
}

function Get-PyprojectVersions {
    $results = @()
    $files = Get-ChildItem -Path $Root -Recurse -Filter "pyproject.toml"
    foreach ($file in $files) {
        $content = Get-Content $file.FullName -Raw
        $relPath = $file.FullName.Substring($Root.Length + 1).Replace("\", "/")
        if ($content -match 'version\s*=\s*"([^"]+)"') {
            $results += [PSCustomObject]@{
                File    = $relPath
                Version = $Matches[1]
                Type    = "pyproject"
                Raw     = $file.FullName
            }
        }
    }
    return $results
}

function Get-PackageJsonVersions {
    $results = @()
    $files = Get-ChildItem -Path $Root -Recurse -Filter "package.json" |
        Where-Object { $_.FullName -notmatch "node_modules" }
    foreach ($file in $files) {
        $content = Get-Content $file.FullName -Raw | ConvertFrom-Json
        $relPath = $file.FullName.Substring($Root.Length + 1).Replace("\", "/")
        if ($content.version) {
            $results += [PSCustomObject]@{
                File    = $relPath
                Version = $content.version
                Type    = "npm"
                Raw     = $file.FullName
            }
        }
    }
    return $results
}

function Fix-Version {
    param(
        [string]$FilePath,
        [string]$Type,
        [string]$TargetVersion
    )

    $content = Get-Content $FilePath -Raw

    switch ($Type) {
        "pyproject" {
            $content = $content -replace '(version\s*=\s*)"[^"]+"', "`${1}`"$TargetVersion`""
        }
        "npm" {
            $json = $content | ConvertFrom-Json
            $json.version = $TargetVersion
            if ($json.optionalDependencies) {
                $props = $json.optionalDependencies | Get-Member -MemberType NoteProperty
                foreach ($prop in $props) {
                    $json.optionalDependencies.$($prop.Name) = $TargetVersion
                }
            }
            $content = $json | ConvertTo-Json -Depth 10
        }
        "cargo" {
            $content = $content -replace '(\[package\][\s\S]*?version\s*=\s*)"[^"]+"', "`${1}`"$TargetVersion`""
        }
    }

    Set-Content -Path $FilePath -Value $content -NoNewline
}

$wsVersion = Get-WorkspaceVersion
Write-Host "=== FTA Version Consistency Check ===" -ForegroundColor Cyan
Write-Host "Workspace version: $wsVersion" -ForegroundColor Green
Write-Host ""

$allVersions = @()
$allVersions += Get-CargoVersions
$allVersions += Get-PyprojectVersions
$allVersions += Get-PackageJsonVersions

$mismatches = @()
$consistent = @()

foreach ($entry in $allVersions) {
    $effectiveVersion = if ($entry.Version -eq "workspace") { $wsVersion } else { $entry.Version }
    $status = if ($effectiveVersion -eq $wsVersion) { "OK" } else { "MISMATCH" }

    if ($status -eq "MISMATCH") {
        $mismatches += $entry
        Write-Host "  MISMATCH  $($entry.File): $($entry.Version) (expected $wsVersion)" -ForegroundColor Red
    }
    else {
        $consistent += $entry
        Write-Host "  OK        $($entry.File): $($entry.Version)" -ForegroundColor Green
    }
}

Write-Host ""
Write-Host "--- Summary ---"
Write-Host "Total files checked: $($allVersions.Count)"
Write-Host "Consistent: $($consistent.Count)"
Write-Host "Mismatches: $($mismatches.Count)"

if ($mismatches.Count -gt 0 -and $Fix) {
    Write-Host ""
    Write-Host "Fixing mismatches..." -ForegroundColor Yellow
    foreach ($entry in $mismatches) {
        Fix-Version -FilePath $entry.Raw -Type $entry.Type -TargetVersion $wsVersion
        Write-Host "  Fixed $($entry.File) -> $wsVersion" -ForegroundColor Green
    }
    Write-Host "All versions synced to $wsVersion" -ForegroundColor Green
    exit 0
}
elseif ($mismatches.Count -gt 0) {
    Write-Host ""
    Write-Host "Run with -Fix to automatically sync versions." -ForegroundColor Yellow
    exit 1
}
else {
    Write-Host ""
    Write-Host "All versions are consistent!" -ForegroundColor Green
    exit 0
}
