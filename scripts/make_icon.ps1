# DiskBytes icon regeneration (doc 03 M11.1): draws the brand source
# PNG programmatically, then fans it out to every platform size via
# `npx tauri icon`. Run from app/ (or pass -AppRoot).
param(
    [string]$AppRoot = (Split-Path $PSScriptRoot -Parent)
)

$ErrorActionPreference = "Stop"
Set-Location (Join-Path $AppRoot "src-tauri")

Write-Host "== DiskBytes icon: generating the 1024x1024 source =="
cargo run --example make_icon
if ($LASTEXITCODE -ne 0) { throw "make_icon example failed" }

Write-Host "== DiskBytes icon: fanning out sizes (npx tauri icon) =="
Set-Location $AppRoot
npx tauri icon (Join-Path $AppRoot "packaging/icon/icon-source.png")
if ($LASTEXITCODE -ne 0) { throw "tauri icon failed" }

Write-Host "Done — src-tauri/icons/ regenerated from the brand source."
