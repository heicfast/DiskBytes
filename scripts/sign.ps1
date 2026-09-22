# DiskBytes optional signing pass (doc 03 M11.2): signs every artifact
# in dist-release/ when signtool + a cert are available; otherwise
# prints the SHA256 manifest and exits 0 (unsigned private testing is
# legitimate — SmartScreen guidance lives in docs/DISTRIBUTION.md).
#
#   scripts/sign.ps1                              # EV/OV cert from store
#   scripts/sign.ps1 -CertFile .\diskbytes.pfx    # file-based cert
param(
    [string]$AppRoot = (Split-Path $PSScriptRoot -Parent),
    [string]$CertFile,
    [string]$TimestampUrl = "http://timestamp.digicert.com"
)

$ErrorActionPreference = "Stop"
$outDir = Join-Path $AppRoot "dist-release"
if (-not (Test-Path $outDir)) { throw "no dist-release/ — run scripts/build.ps1 first" }

$signtool = Get-Command signtool.exe -ErrorAction SilentlyContinue
if (-not $signtool) {
    Write-Warning "signtool.exe not on PATH (Windows SDK). Artifacts stay unsigned; hashes:"
    Get-Content (Join-Path $outDir "SHA256SUMS.txt") | Write-Host
    return
}

$files = Get-ChildItem $outDir -Include "*.exe", "*.zip" -File -Recurse
foreach ($f in $files) {
    $args = @("sign", "/fd", "SHA256", "/td", "SHA256", "/tr", $TimestampUrl)
    if ($CertFile) {
        $args += @("/f", $CertFile)
    }
    $args += $f.FullName
    & $signtool.Source @args
    if ($LASTEXITCODE -ne 0) { throw "signing failed for $($f.Name)" }
}

# Refresh the manifest against the signed bytes.
Get-ChildItem $outDir -File | ForEach-Object {
    $h = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower()
    "$h  $($_.Name)"
} | Set-Content (Join-Path $outDir "SHA256SUMS.txt")
Write-Host "Signed $($files.Count) artifact(s); SHA256SUMS.txt refreshed."
