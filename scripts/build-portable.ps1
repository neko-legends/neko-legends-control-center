$ErrorActionPreference = 'Stop'

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$rootDir = Split-Path -Parent $scriptDir
$targetDir = Join-Path $rootDir 'src-tauri\target-portable'
$releaseDir = Join-Path $targetDir 'release'
$portablePackageDir = Join-Path $rootDir 'release\NekoLegendsControlCenter'
$cargoExe = (Get-Command cargo -ErrorAction SilentlyContinue).Source

if (-not $cargoExe) {
  $cargoExe = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
}

if (-not (Test-Path -LiteralPath $cargoExe)) {
  throw 'Cargo was not found. Install Rust or add cargo.exe to PATH.'
}

Push-Location $rootDir
try {
  $previousCargoTargetDir = $env:CARGO_TARGET_DIR
  $previousPath = $env:PATH
  $env:CARGO_TARGET_DIR = $targetDir
  $env:PATH = "$(Split-Path -Parent $cargoExe);$env:PATH"

  try {
    npx tauri build --no-bundle --runner $cargoExe
  } finally {
    $env:PATH = $previousPath
    if ($null -eq $previousCargoTargetDir) {
      Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    } else {
      $env:CARGO_TARGET_DIR = $previousCargoTargetDir
    }
  }

  $sourceExe = Join-Path $releaseDir 'neko-legends-control-center.exe'
  $staleTargetPortableExe = Join-Path $releaseDir 'neko-legends-control-center-portable.exe'
  $portableExe = Join-Path $portablePackageDir 'neko-legends-control-center-portable.exe'

  if (-not (Test-Path -LiteralPath $sourceExe)) {
    throw "Portable build failed because $sourceExe was not created."
  }

  if (Test-Path -LiteralPath $staleTargetPortableExe) {
    Remove-Item -LiteralPath $staleTargetPortableExe -Force
  }

  New-Item -ItemType Directory -Force -Path $portablePackageDir | Out-Null
  New-Item -ItemType Directory -Force -Path (Join-Path $portablePackageDir 'apps') | Out-Null
  if (Test-Path -LiteralPath $portableExe) {
    Remove-Item -LiteralPath $portableExe -Force
  }
  Move-Item -LiteralPath $sourceExe -Destination $portableExe -Force
  Set-Content -LiteralPath (Join-Path $portablePackageDir 'README.txt') -Value @(
    'Neko Legends Control Center Portable',
    '',
    'Keep the apps folder beside the launcher. Downloaded Neko Legends apps install there by default.'
  )
  Write-Host "Portable app created: $portableExe"
  Write-Host "Portable package created: $portablePackageDir"
} finally {
  Pop-Location
}
