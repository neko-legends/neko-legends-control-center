$ErrorActionPreference = 'Stop'

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$rootDir = Split-Path -Parent $scriptDir
$portableExe = Join-Path $rootDir 'release\NekoLegendsControlCenter\neko-legends-control-center-portable.exe'

if (-not (Test-Path -LiteralPath $portableExe)) {
  throw 'Portable app was not found. Run npm run build:portable first.'
}

Write-Host "Starting portable build: $portableExe"
Start-Process -FilePath $portableExe -WorkingDirectory (Split-Path -Parent $portableExe)
