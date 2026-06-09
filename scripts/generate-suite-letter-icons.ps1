$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class NativeIconMethods {
  [DllImport("user32.dll", SetLastError = true)]
  public static extern bool DestroyIcon(IntPtr hIcon);
}
"@

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$controlCenterRoot = Split-Path -Parent $scriptDir
$suiteRoot = Split-Path -Parent $controlCenterRoot

$apps = @(
  @{ Code = 'BL'; Name = 'BatchLapse'; Root = Join-Path $suiteRoot 'BatchLapse'; Tauri = 'src-tauri\icons' },
  @{ Code = 'DM'; Name = 'DepthMapAIGenerator'; Root = Join-Path $suiteRoot 'DepthMapAIGenerator'; Tauri = 'src-tauri\icons' },
  @{ Code = 'OS'; Name = 'OpenSplit'; Root = Join-Path $suiteRoot 'OpenSplit'; Tauri = 'src-tauri\icons' },
  @{ Code = 'VM'; Name = 'VeniceMediaLocal'; Root = Join-Path $suiteRoot 'VeniceMediaLocal'; Tauri = 'src-tauri\icons' },
  @{ Code = 'MR'; Name = 'MarkRush'; Root = Join-Path $suiteRoot 'MarkRush'; MarkRush = $true },
  @{ Code = 'PP'; Name = 'PurplePlanet'; Root = Join-Path $suiteRoot 'PurplePlanet\PurplePlanet\src\PurplePlanet'; DotNetIcon = 'Assets\PurplePlanet.ico'; DotNetPng = 'Assets\PurplePlanet-256.png' },
  @{ Code = 'SG'; Name = 'StarGaze'; Root = Join-Path $suiteRoot 'StarGaze\StarGaze\src\StarGaze'; DotNetIcon = 'Assets\StarGaze.ico'; DotNetPng = 'Assets\StarGaze-256.png' },
  @{ Code = 'A3'; Name = 'ImageToASCII3D'; Root = Join-Path $suiteRoot 'ImageToASCII3D\apps\web'; WebIcon = 'public\icon.png' }
)

function New-RoundedRectanglePath {
  param([System.Drawing.RectangleF]$Rect, [float]$Radius)

  $path = New-Object System.Drawing.Drawing2D.GraphicsPath
  $path.AddArc($Rect.X, $Rect.Y, $Radius, $Radius, 180, 90)
  $path.AddArc(($Rect.Right - $Radius), $Rect.Y, $Radius, $Radius, 270, 90)
  $path.AddArc(($Rect.Right - $Radius), ($Rect.Bottom - $Radius), $Radius, $Radius, 0, 90)
  $path.AddArc($Rect.X, ($Rect.Bottom - $Radius), $Radius, $Radius, 90, 90)
  $path.CloseFigure()
  return $path
}

function New-LetterBitmap {
  param(
    [string]$Code,
    [int]$Size
  )

  $bmp = New-Object System.Drawing.Bitmap $Size, $Size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
  $g.Clear([System.Drawing.Color]::Transparent)

  $scale = $Size / 512.0
  function S([float]$value) { return [float]($value * $scale) }

  $black = [System.Drawing.ColorTranslator]::FromHtml('#050505')
  $surface = [System.Drawing.ColorTranslator]::FromHtml('#140903')
  $orange = [System.Drawing.ColorTranslator]::FromHtml('#ff6a00')
  $amber = [System.Drawing.ColorTranslator]::FromHtml('#ffb000')
  $text = [System.Drawing.ColorTranslator]::FromHtml('#ffd08a')

  $outer = New-Object System.Drawing.RectangleF (S 34), (S 34), (S 444), (S 444)
  $inner = New-Object System.Drawing.RectangleF (S 54), (S 54), (S 404), (S 404)
  $outerPath = New-RoundedRectanglePath -Rect $outer -Radius (S 88)
  $innerPath = New-RoundedRectanglePath -Rect $inner -Radius (S 68)

  $g.FillPath((New-Object System.Drawing.SolidBrush $black), $outerPath)
  $g.FillEllipse((New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(42, $orange))), (S 92), (S 80), (S 328), (S 328))
  $g.FillPath((New-Object System.Drawing.SolidBrush $surface), $innerPath)
  $g.DrawPath((New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(180, $orange)), (S 10)), $outerPath)
  $g.DrawPath((New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(90, $amber)), (S 3)), $innerPath)

  $fontFamily = New-Object System.Drawing.FontFamily 'Segoe UI'
  $fontStyle = [System.Drawing.FontStyle]::Bold
  $fontSize = if ($Code.Length -gt 2) { S 188 } else { S 218 }
  $font = New-Object System.Drawing.Font -ArgumentList $fontFamily, $fontSize, $fontStyle, ([System.Drawing.GraphicsUnit]::Pixel)
  $format = New-Object System.Drawing.StringFormat
  $format.Alignment = [System.Drawing.StringAlignment]::Center
  $format.LineAlignment = [System.Drawing.StringAlignment]::Center
  $textRect = New-Object System.Drawing.RectangleF (S 34), (S 28), (S 444), (S 444)

  foreach ($offset in @(-8, -4, 4, 8)) {
    $shadowRect = New-Object System.Drawing.RectangleF ($textRect.X + (S $offset)), $textRect.Y, $textRect.Width, $textRect.Height
    $g.DrawString($Code, $font, (New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(54, $orange))), $shadowRect, $format)
  }
  $g.DrawString($Code, $font, (New-Object System.Drawing.SolidBrush $text), $textRect, $format)

  $font.Dispose()
  $fontFamily.Dispose()
  $format.Dispose()
  $g.Dispose()
  return $bmp
}

function Get-PngBytes {
  param([string]$Code, [int]$Size)

  $stream = New-Object System.IO.MemoryStream
  $bmp = New-LetterBitmap -Code $Code -Size $Size
  $bmp.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  $bytes = $stream.ToArray()
  $stream.Dispose()
  return $bytes
}

function Save-Png {
  param([string]$Code, [int]$Size, [string]$Path)

  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Path) | Out-Null
  [System.IO.File]::WriteAllBytes($Path, (Get-PngBytes -Code $Code -Size $Size))
}

function Save-Ico {
  param([string]$Code, [string]$Path)

  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Path) | Out-Null
  $bmp = New-LetterBitmap -Code $Code -Size 256
  $handle = $bmp.GetHicon()
  try {
    $icon = [System.Drawing.Icon]::FromHandle($handle)
    $fs = [System.IO.File]::Create($Path)
    try {
      $icon.Save($fs)
    }
    finally {
      $fs.Dispose()
      $icon.Dispose()
    }
  }
  finally {
    [NativeIconMethods]::DestroyIcon($handle) | Out-Null
    $bmp.Dispose()
  }
}

function Write-Ascii {
  param([System.IO.BinaryWriter]$Writer, [string]$Value)
  $Writer.Write([System.Text.Encoding]::ASCII.GetBytes($Value))
}

function Write-BigEndianUInt32 {
  param([System.IO.BinaryWriter]$Writer, [UInt32]$Value)
  $Writer.Write([byte](($Value -shr 24) -band 255))
  $Writer.Write([byte](($Value -shr 16) -band 255))
  $Writer.Write([byte](($Value -shr 8) -band 255))
  $Writer.Write([byte]($Value -band 255))
}

function Save-Icns {
  param([string]$Code, [string]$Path)

  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Path) | Out-Null
  $chunks = @(
    @{ Type = 'ic07'; Bytes = (Get-PngBytes -Code $Code -Size 128) },
    @{ Type = 'ic08'; Bytes = (Get-PngBytes -Code $Code -Size 256) },
    @{ Type = 'ic09'; Bytes = (Get-PngBytes -Code $Code -Size 512) },
    @{ Type = 'ic10'; Bytes = (Get-PngBytes -Code $Code -Size 1024) }
  )
  $totalLength = 8
  foreach ($chunk in $chunks) {
    $totalLength += 8 + $chunk.Bytes.Length
  }

  $fs = [System.IO.File]::Create($Path)
  $writer = New-Object System.IO.BinaryWriter $fs
  Write-Ascii $writer 'icns'
  Write-BigEndianUInt32 $writer ([UInt32]$totalLength)
  foreach ($chunk in $chunks) {
    Write-Ascii $writer $chunk.Type
    Write-BigEndianUInt32 $writer ([UInt32](8 + $chunk.Bytes.Length))
    $writer.Write($chunk.Bytes)
  }
  $writer.Dispose()
  $fs.Dispose()
}

function Save-TauriIconSet {
  param([hashtable]$App)

  $iconDir = Join-Path $App.Root $App.Tauri
  Save-Png $App.Code 32 (Join-Path $iconDir '32x32.png')
  Save-Png $App.Code 64 (Join-Path $iconDir '64x64.png')
  Save-Png $App.Code 128 (Join-Path $iconDir '128x128.png')
  Save-Png $App.Code 256 (Join-Path $iconDir '128x128@2x.png')
  Save-Png $App.Code 512 (Join-Path $iconDir 'icon.png')
  Save-Ico $App.Code (Join-Path $iconDir 'icon.ico')
  Save-Icns $App.Code (Join-Path $iconDir 'icon.icns')

  $logoSizes = @{
    'Square30x30Logo.png' = 30
    'Square44x44Logo.png' = 44
    'Square71x71Logo.png' = 71
    'Square89x89Logo.png' = 89
    'Square107x107Logo.png' = 107
    'Square142x142Logo.png' = 142
    'Square150x150Logo.png' = 150
    'Square284x284Logo.png' = 284
    'Square310x310Logo.png' = 310
    'StoreLogo.png' = 50
  }
  foreach ($entry in $logoSizes.GetEnumerator()) {
    $target = Join-Path $iconDir $entry.Key
    if (Test-Path -LiteralPath $target) {
      Save-Png $App.Code $entry.Value $target
    }
  }
}

foreach ($app in $apps) {
  if (-not (Test-Path -LiteralPath $app.Root)) {
    Write-Warning "Skipping $($app.Name); root not found: $($app.Root)"
    continue
  }

  if ($app.Tauri) {
    Save-TauriIconSet $app
  }
  if ($app.MarkRush) {
    Save-Png $app.Code 256 (Join-Path $app.Root 'assets\icons\MarkRush-256.png')
    Save-Ico $app.Code (Join-Path $app.Root 'assets\icons\MarkRush.ico')
    foreach ($size in @(16, 24, 32, 48, 64, 128, 256)) {
      Save-Png $app.Code $size (Join-Path $app.Root "reference\wpf\Assets\MarkRush-$size.png")
    }
    Save-Png $app.Code 256 (Join-Path $app.Root 'reference\wpf\Assets\Extracted-AppIcon-256.png')
    Save-Ico $app.Code (Join-Path $app.Root 'reference\wpf\Assets\MarkRush.ico')
  }
  if ($app.DotNetIcon) {
    Save-Ico $app.Code (Join-Path $app.Root $app.DotNetIcon)
    Save-Png $app.Code 256 (Join-Path $app.Root $app.DotNetPng)
  }
  if ($app.WebIcon) {
    Save-Png $app.Code 512 (Join-Path $app.Root $app.WebIcon)
  }

  Write-Host "Generated $($app.Code) icon assets for $($app.Name)"
}
