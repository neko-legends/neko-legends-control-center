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
  @{ Code = 'CC'; Name = 'CutsceneConverter'; Root = Join-Path $suiteRoot 'CutsceneConverter'; Tauri = 'src-tauri\icons' },
  @{ Code = 'DM'; Name = 'DepthMapAIGenerator'; Root = Join-Path $suiteRoot 'DepthMapAIGenerator'; Tauri = 'src-tauri\icons' },
  @{ Code = 'A3'; Name = 'ImageToASCII3D'; Root = Join-Path $suiteRoot 'ImageToASCII3D'; Tauri = 'src-tauri\icons'; WebIcon = 'apps\web\public\icon.png' },
  @{ Code = 'MR'; Name = 'MarkRush'; Root = Join-Path $suiteRoot 'MarkRush'; MarkRush = $true },
  @{ Code = 'OS'; Name = 'OpenSplit'; Root = Join-Path $suiteRoot 'OpenSplit'; Tauri = 'src-tauri\icons' },
  @{ Code = 'SI'; Name = 'SeamlessImageEdit'; Root = Join-Path $suiteRoot 'SeamlessImageEdit'; Tauri = 'src-tauri\icons' },
  @{ Code = 'VM'; Name = 'VeniceMediaLocal'; Root = Join-Path $suiteRoot 'VeniceMediaLocal'; Tauri = 'src-tauri\icons' },
  @{ Code = 'PP'; Name = 'PurplePlanet'; Root = Join-Path $suiteRoot 'PurplePlanet\PurplePlanet\src\PurplePlanet'; DotNetIcon = 'Assets\PurplePlanet.ico'; DotNetPng = 'Assets\PurplePlanet-256.png' },
  @{ Code = 'SG'; Name = 'StarGaze'; Root = Join-Path $suiteRoot 'StarGaze\StarGaze\src\StarGaze'; DotNetIcon = 'Assets\StarGaze.ico'; DotNetPng = 'Assets\StarGaze-256.png' }
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
  $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
  $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $g.Clear([System.Drawing.Color]::Transparent)

  $scale = $Size / 512.0
  function S([float]$value) { return [float]($value * $scale) }
  function W([float]$value) { return [float][Math]::Max(1.0, ($value * $scale)) }

  $black = [System.Drawing.ColorTranslator]::FromHtml('#050505')
  $surface = [System.Drawing.ColorTranslator]::FromHtml('#120602')
  $orange = [System.Drawing.ColorTranslator]::FromHtml('#ff6a00')
  $amber = [System.Drawing.ColorTranslator]::FromHtml('#ffb000')
  $text = [System.Drawing.ColorTranslator]::FromHtml('#fff0bd')

  $smallIcon = $Size -le 64
  $outer = if ($smallIcon) {
    New-Object System.Drawing.RectangleF (S 18), (S 18), (S 476), (S 476)
  }
  else {
    New-Object System.Drawing.RectangleF (S 30), (S 30), (S 452), (S 452)
  }
  $inner = if ($smallIcon) {
    New-Object System.Drawing.RectangleF (S 32), (S 32), (S 448), (S 448)
  }
  else {
    New-Object System.Drawing.RectangleF (S 52), (S 52), (S 408), (S 408)
  }
  $outerRadius = if ($smallIcon) { S 72 } else { S 88 }
  $innerRadius = if ($smallIcon) { S 58 } else { S 68 }
  $glowAlpha = if ($smallIcon) { 34 } else { 48 }
  $outerPenWidth = if ($smallIcon) { W 12 } else { S 10 }
  $textGlowWide = if ($smallIcon) { W 16 } else { S 18 }
  $textGlowMid = if ($smallIcon) { W 10 } else { S 10 }
  $textGlowTight = if ($smallIcon) { W 4 } else { S 5 }
  $outerPath = New-RoundedRectanglePath -Rect $outer -Radius $outerRadius
  $innerPath = New-RoundedRectanglePath -Rect $inner -Radius $innerRadius

  $g.FillPath((New-Object System.Drawing.SolidBrush $black), $outerPath)
  $g.FillEllipse((New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb($glowAlpha, $orange))), (S 86), (S 76), (S 340), (S 340))
  $g.FillPath((New-Object System.Drawing.SolidBrush $surface), $innerPath)
  $g.DrawPath((New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(220, $orange), $outerPenWidth)), $outerPath)
  if (-not $smallIcon) {
    $g.DrawPath((New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(90, $amber), (S 3))), $innerPath)
  }

  try {
    $fontFamily = New-Object System.Drawing.FontFamily 'Arial Black'
  }
  catch {
    $fontFamily = New-Object System.Drawing.FontFamily 'Segoe UI'
  }
  $fontStyle = [System.Drawing.FontStyle]::Regular
  $format = [System.Drawing.StringFormat]::GenericTypographic.Clone()
  $format.FormatFlags = $format.FormatFlags -bor [System.Drawing.StringFormatFlags]::NoClip
  $baseTextSize = if ($Code.Length -gt 2) { S 360 } else { S 380 }
  $textBox = if ($smallIcon) {
    New-Object System.Drawing.RectangleF (S 40), (S 42), (S 432), (S 416)
  }
  else {
    New-Object System.Drawing.RectangleF (S 48), (S 62), (S 416), (S 360)
  }

  $textPath = New-Object System.Drawing.Drawing2D.GraphicsPath
  $textPath.AddString($Code, $fontFamily, [int]$fontStyle, $baseTextSize, (New-Object System.Drawing.PointF 0, 0), $format)
  $textBounds = $textPath.GetBounds()
  $fitScale = [Math]::Min(($textBox.Width / $textBounds.Width), ($textBox.Height / $textBounds.Height))
  $matrix = New-Object System.Drawing.Drawing2D.Matrix
  $matrix.Translate(-$textBounds.X, -$textBounds.Y)
  $matrix.Scale($fitScale, $fitScale, [System.Drawing.Drawing2D.MatrixOrder]::Append)
  $fittedWidth = $textBounds.Width * $fitScale
  $fittedHeight = $textBounds.Height * $fitScale
  $matrix.Translate(
    ($textBox.X + (($textBox.Width - $fittedWidth) / 2)),
    ($textBox.Y + (($textBox.Height - $fittedHeight) / 2)),
    [System.Drawing.Drawing2D.MatrixOrder]::Append
  )
  $textPath.Transform($matrix)

  $g.DrawPath((New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(86, $orange), $textGlowWide)), $textPath)
  $g.DrawPath((New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(156, $orange), $textGlowMid)), $textPath)
  $g.DrawPath((New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(205, $amber), $textGlowTight)), $textPath)
  $g.FillPath((New-Object System.Drawing.SolidBrush $text), $textPath)

  $matrix.Dispose()
  $textPath.Dispose()
  $fontFamily.Dispose()
  $format.Dispose()
  $outerPath.Dispose()
  $innerPath.Dispose()
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
  return ,$bytes
}

function Save-Png {
  param([string]$Code, [int]$Size, [string]$Path)

  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Path) | Out-Null
  [System.IO.File]::WriteAllBytes($Path, (Get-PngBytes -Code $Code -Size $Size))
}

function Save-Ico {
  param([string]$Code, [string]$Path)

  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Path) | Out-Null
  $images = @(16, 24, 32, 48, 64, 128, 256) | ForEach-Object {
    @{ Size = $_; Bytes = ([byte[]](Get-PngBytes -Code $Code -Size $_)) }
  }

  $fs = [System.IO.File]::Create($Path)
  $writer = New-Object System.IO.BinaryWriter $fs
  try {
    $writer.Write([UInt16]0)
    $writer.Write([UInt16]1)
    $writer.Write([UInt16]$images.Count)

    $offset = 6 + ($images.Count * 16)
    foreach ($image in $images) {
      $dimension = if ($image.Size -ge 256) { [byte]0 } else { [byte]$image.Size }
      $writer.Write($dimension)
      $writer.Write($dimension)
      $writer.Write([byte]0)
      $writer.Write([byte]0)
      $writer.Write([UInt16]1)
      $writer.Write([UInt16]32)
      $writer.Write([UInt32]$image.Bytes.Length)
      $writer.Write([UInt32]$offset)
      $offset += $image.Bytes.Length
    }

    foreach ($image in $images) {
      $writer.Write($image.Bytes)
    }
  }
  finally {
    $writer.Dispose()
    $fs.Dispose()
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
