# DiskBytes release build (doc 03 M11.2; spec §17): the full gate
# battery, the per-user NSIS installer, the portable zip and
# SHA256SUMS.txt. Windows only (MSVC + WebView2).
#
#   scripts/build.ps1                 # x64 (default)
#   scripts/build.ps1 -Arch x64
#
# Requires: Rust (stable-msvc), Node 20+, npm; optional signtool for
# scripts/sign.ps1 (un-signed builds are fine for private testing —
# SmartScreen guidance is in docs/DISTRIBUTION.md).
param(
    [ValidateSet("x64")]
    [string]$Arch = "x64",
    [string]$AppRoot = (Split-Path $PSScriptRoot -Parent),
    [switch]$SkipGates   # never use for releases; dev speed only
)

$ErrorActionPreference = "Stop"
$target = "x86_64-pc-windows-msvc"
$toml = Join-Path $AppRoot "src-tauri/Cargo.toml"
Set-Location $AppRoot

if (-not $SkipGates) {
    Write-Host "== Gates: cargo fmt / clippy / tests =="
    Push-Location (Join-Path $AppRoot "src-tauri")
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw "cargo fmt --check failed" }
    cargo clippy --target $target --workspace --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "clippy failed" }
    cargo test --workspace
    if ($LASTEXITCODE -ne 0) { throw "tests failed" }
    Pop-Location

    Write-Host "== Gates: typecheck / vitest / build =="
    npm run typecheck
    if ($LASTEXITCODE -ne 0) { throw "typecheck failed" }
    npx vitest run
    if ($LASTEXITCODE -ne 0) { throw "vitest failed" }
    npm run build
    if ($LASTEXITCODE -ne 0) { throw "frontend build failed" }

    Write-Host "== Gates: safety greps (R7.1 / R2.2) =="
    $r71 = git grep -nE "remove_file|remove_dir|remove_dir_all|DeleteFileW|RemoveDirectoryW|SHEmptyRecycleBin" -- src-tauri/src
    if ($LASTEXITCODE -eq 0) { throw "R7.1 violation: $r71" }
    $r22 = git grep -nE "TODO|FIXME|unimplemented!|todo!" -- src src-tauri/src
    if ($LASTEXITCODE -eq 0) { throw "R2.2 violation: $r22" }
}

Write-Host "== Icon (brand source -> all sizes) =="
& (Join-Path $PSScriptRoot "make_icon.ps1") -AppRoot $AppRoot

Write-Host "== Tauri build ($target, NSIS per-user) =="
npx tauri build --target $target
if ($LASTEXITCODE -ne 0) { throw "tauri build failed" }

# ---- Artifacts: installer + portable zip + SHA256SUMS --------------------
$version = (Select-String -Path $toml -Pattern '^version\s*=\s*"([^"]+)"' |
    Select-Object -First 1).Matches[0].Groups[1].Value
$bundle = Join-Path $AppRoot "src-tauri/target/$target/release/bundle"
$outDir = Join-Path $AppRoot "dist-release"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

$installer = Get-ChildItem (Join-Path $bundle "nsis") -Filter "*.exe" |
    Select-Object -First 1
if (-not $installer) { throw "NSIS installer not found" }
Copy-Item $installer.FullName -Destination (Join-Path $outDir "DiskBytes-$version-$Arch-setup.exe") -Force

# Portable zip: the bare exe + WebView2 bootstrapper note.
$exe = Join-Path $AppRoot "src-tauri/target/$target/release/diskbytes.exe"
if (-not (Test-Path $exe)) { throw "release exe not found" }
$portable = Join-Path $outDir "DiskBytes-$version-$Arch-portable"
New-Item -ItemType Directory -Force -Path $portable | Out-Null
Copy-Item $exe -Destination (Join-Path $portable "DiskBytes.exe") -Force
@(
    "DiskBytes portable. Runs without installing; WebView2 is required",
    "(the Windows bootstrapper is bundled with the setup exe instead).",
    "See docs/DISTRIBUTION.md for SmartScreen/Unblock guidance."
) | Set-Content (Join-Path $portable "README.txt")
Compress-Archive -Path $portable -DestinationPath (Join-Path $outDir "DiskBytes-$version-$Arch-portable.zip") -Force
Remove-Item $portable -Recurse -Force

Write-Host "== SHA256SUMS.txt =="
$sums = Get-ChildItem $outDir -File | ForEach-Object {
    $h = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower()
    "$h  $($_.Name)"
}
$sums | Set-Content (Join-Path $outDir "SHA256SUMS.txt")
$sums | Write-Host

Write-Host "Release artifacts in $outDir"
