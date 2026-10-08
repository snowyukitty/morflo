param(
  [string]$OutputDirectory = "fixtures/generated"
)

$ErrorActionPreference = "Stop"

$ffmpeg = Get-Command ffmpeg -ErrorAction Stop
$ffprobe = Get-Command ffprobe -ErrorAction Stop
$root = Split-Path -Parent $PSScriptRoot
$output = [System.IO.Path]::GetFullPath((Join-Path $root $OutputDirectory))
[System.IO.Directory]::CreateDirectory($output) | Out-Null

function Invoke-Ffmpeg {
  param([string[]]$Arguments)
  & $ffmpeg.Source @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "FFmpeg fixture generation failed with exit code $LASTEXITCODE"
  }
}

function Assert-Probeable {
  param([string]$Path)
  & $ffprobe.Source -v error -show_entries format=format_name -of default=nw=1 $Path | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw "Generated fixture is not probeable: $Path"
  }
}

function Add-ExifOrientation {
  param(
    [string]$Source,
    [string]$Destination,
    [ValidateRange(1, 8)][int]$Orientation
  )
  $jpeg = [System.IO.File]::ReadAllBytes($Source)
  if ($jpeg.Length -lt 2 -or $jpeg[0] -ne 0xFF -or $jpeg[1] -ne 0xD8) {
    throw "EXIF orientation source is not a JPEG"
  }
  [byte[]]$segment = @(
    0xFF, 0xE1, 0x00, 0x22,
    0x45, 0x78, 0x69, 0x66, 0x00, 0x00,
    0x49, 0x49, 0x2A, 0x00, 0x08, 0x00, 0x00, 0x00,
    0x01, 0x00,
    0x12, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00,
    [byte]$Orientation, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00
  )
  $result = [byte[]]::new($jpeg.Length + $segment.Length)
  [System.Buffer]::BlockCopy($jpeg, 0, $result, 0, 2)
  [System.Buffer]::BlockCopy($segment, 0, $result, 2, $segment.Length)
  [System.Buffer]::BlockCopy($jpeg, 2, $result, 2 + $segment.Length, $jpeg.Length - 2)
  [System.IO.File]::WriteAllBytes($Destination, $result)
}

$transparentPng = Join-Path $output "transparent-grid.png"
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y",
  "-f", "lavfi", "-i", "color=c=black@0.0:s=640x420:d=0.04,format=rgba,drawbox=x=80:y=60:w=480:h=300:color=0x347364@0.82:t=fill:replace=1,drawbox=x=220:y=130:w=200:h=160:color=0xE7A66D@0.94:t=fill:replace=1",
  "-frames:v", "1", $transparentPng
)

$opaqueJpeg = Join-Path $output "warm-landscape.jpg"
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y",
  "-f", "lavfi", "-i", "testsrc2=s=720x480:d=0.04",
  "-frames:v", "1", "-q:v", "2", $opaqueJpeg
)

$orientedBase = Join-Path $output "orientation-base.jpg"
$orientedJpeg = Join-Path $output "exif-orientation-6.jpg"
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y",
  "-f", "lavfi", "-i", "testsrc2=s=360x240:d=0.04",
  "-frames:v", "1", "-q:v", "2", $orientedBase
)
Add-ExifOrientation -Source $orientedBase -Destination $orientedJpeg -Orientation 6
Remove-Item -LiteralPath $orientedBase

$transparentWebp = Join-Path $output "transparent-grid.webp"
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y", "-i", $transparentPng,
  "-frames:v", "1", "-c:v", "libwebp", "-quality", "82", "-compression_level", "5", $transparentWebp
)

$largePng = Join-Path $output "large-image.png"
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y",
  "-f", "lavfi", "-i", "testsrc2=s=4096x3072:d=0.04",
  "-frames:v", "1", "-compression_level", "8", $largePng
)

$unusualPng = Join-Path $output "unusual-17x2049.png"
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y",
  "-f", "lavfi", "-i", "testsrc2=s=17x2049:d=0.04",
  "-frames:v", "1", $unusualPng
)

$bmp = Join-Path $output "sample.bmp"
$tiff = Join-Path $output "sample.tiff"
Invoke-Ffmpeg @("-hide_banner", "-loglevel", "error", "-y", "-i", $opaqueJpeg, "-frames:v", "1", $bmp)
Invoke-Ffmpeg @("-hide_banner", "-loglevel", "error", "-y", "-i", $opaqueJpeg, "-frames:v", "1", $tiff)

$ico = Join-Path $output "multi-resolution.ico"
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y", "-i", $transparentPng,
  "-filter_complex", "[0:v]split=4[m16][m32][m48][m256];[m16]scale=16:16:flags=lanczos[o16];[m32]scale=32:32:flags=lanczos[o32];[m48]scale=48:48:flags=lanczos[o48];[m256]scale=256:256:flags=lanczos[o256]",
  "-map", "[o16]", "-map", "[o32]", "-map", "[o48]", "-map", "[o256]",
  "-c:v", "png", "-frames:v:0", "1", "-frames:v:1", "1", "-frames:v:2", "1", "-frames:v:3", "1", "-f", "ico", $ico
)

$hasAvif = [bool]((& $ffmpeg.Source -hide_banner -formats) -match "\bE\s+avif\b")
if ($hasAvif) {
  $avif = Join-Path $output "sample.avif"
  Invoke-Ffmpeg @(
    "-hide_banner", "-loglevel", "error", "-y", "-i", $opaqueJpeg,
    "-frames:v", "1", "-c:v", "libaom-av1", "-still-picture", "1", "-crf", "30", "-b:v", "0", "-cpu-used", "6", "-f", "avif", $avif
  )
}

$animatedPng = Join-Path $output "animated-input.png"
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y",
  "-f", "lavfi", "-i", "testsrc2=s=320x200:r=8:d=1",
  "-plays", "0", "-f", "apng", $animatedPng
)

$damagedWebp = Join-Path $output "damaged-image.webp"
$webpBytes = [System.IO.File]::ReadAllBytes($transparentWebp)
$damagedLength = [Math]::Min(72, $webpBytes.Length)
$damagedBytes = [byte[]]::new($damagedLength)
[System.Buffer]::BlockCopy($webpBytes, 0, $damagedBytes, 0, $damagedLength)
[System.IO.File]::WriteAllBytes($damagedWebp, $damagedBytes)

$shortMp4 = Join-Path $output "short-1080p.mp4"
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y",
  "-f", "lavfi", "-i", "testsrc2=s=1920x1080:r=30:d=3",
  "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000:d=3",
  "-map", "0:v:0", "-map", "1:a:0", "-c:v", "libx264", "-preset", "ultrafast", "-crf", "25", "-pix_fmt", "yuv420p",
  "-c:a", "aac", "-b:a", "128k", "-shortest", $shortMp4
)

$portraitMov = Join-Path $output "portrait-phone.mov"
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y",
  "-f", "lavfi", "-i", "testsrc2=s=720x1280:r=24:d=3",
  "-f", "lavfi", "-i", "sine=frequency=523:sample_rate=48000:d=3",
  "-map", "0:v:0", "-map", "1:a:0", "-c:v", "libx264", "-preset", "ultrafast", "-crf", "25", "-pix_fmt", "yuv420p",
  "-c:a", "aac", "-b:a", "128k", "-shortest", $portraitMov
)

$webm = Join-Path $output "sample.webm"
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y",
  "-f", "lavfi", "-i", "testsrc2=s=640x360:r=24:d=2",
  "-f", "lavfi", "-i", "sine=frequency=330:sample_rate=48000:d=2",
  "-map", "0:v:0", "-map", "1:a:0", "-c:v", "libvpx-vp9", "-deadline", "realtime", "-cpu-used", "6", "-crf", "36", "-b:v", "0",
  "-c:a", "libopus", "-b:a", "96k", "-shortest", $webm
)

$subtitle = Join-Path $output "sample-subtitle.srt"
$chapterMetadata = Join-Path $output "sample-chapters.ffmeta"
[System.IO.File]::WriteAllText($subtitle, "1`r`n00:00:00,200 --> 00:00:01,200`r`nLocal synthetic subtitle`r`n`r`n2`r`n00:00:01,400 --> 00:00:02,500`r`nNothing leaves this device`r`n")
[System.IO.File]::WriteAllText($chapterMetadata, ";FFMETADATA1`n[CHAPTER]`nTIMEBASE=1/1000`nSTART=0`nEND=1500`ntitle=Opening`n[CHAPTER]`nTIMEBASE=1/1000`nSTART=1500`nEND=3000`ntitle=Finish`n")
$mkv = Join-Path $output "multi-stream.mkv"
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y",
  "-f", "lavfi", "-i", "testsrc2=s=640x360:r=30:d=3",
  "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000:d=3",
  "-f", "lavfi", "-i", "sine=frequency=660:sample_rate=48000:d=3",
  "-i", $subtitle, "-f", "ffmetadata", "-i", $chapterMetadata,
  "-map", "0:v:0", "-map", "1:a:0", "-map", "2:a:0", "-map", "3:s:0", "-map_metadata", "4", "-map_chapters", "4",
  "-metadata:s:a:0", "language=eng", "-metadata:s:a:1", "language=jpn",
  "-c:v", "libx264", "-preset", "ultrafast", "-crf", "25", "-pix_fmt", "yuv420p",
  "-c:a", "aac", "-b:a", "96k", "-c:s", "srt", $mkv
)
Remove-Item -LiteralPath $subtitle, $chapterMetadata

$silentMp4 = Join-Path $output "silent-video.mp4"
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi", "-i", "testsrc2=s=640x360:r=24:d=2",
  "-an", "-c:v", "libx264", "-preset", "ultrafast", "-crf", "25", "-pix_fmt", "yuv420p", $silentMp4
)

$vfrMkv = Join-Path $output "variable-frame-rate.mkv"
$fpsArgs = if ((& $ffmpeg.Source -hide_banner -h) -match "\bfps_mode\b") { @("-fps_mode", "vfr") } else { @("-vsync", "vfr") }
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi", "-i", "testsrc2=s=480x270:r=30:d=3",
  "-vf", "select='not(mod(n,2))+not(mod(n,5))'", $fpsArgs[0], $fpsArgs[1], "-an",
  "-c:v", "libx264", "-preset", "ultrafast", "-crf", "25", "-pix_fmt", "yuv420p", $vfrMkv
)

$gifSource = Join-Path $output "high-motion-gif-source.mp4"
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi", "-i", "testsrc2=s=640x360:r=30:d=4",
  "-an", "-c:v", "libx264", "-preset", "ultrafast", "-crf", "22", "-pix_fmt", "yuv420p", $gifSource
)

$damagedVideo = Join-Path $output "damaged-video.mp4"
$videoBytes = [System.IO.File]::ReadAllBytes($shortMp4)
$truncatedLength = [Math]::Max(1024, [Math]::Floor($videoBytes.Length / 3))
$truncatedVideo = [byte[]]::new($truncatedLength)
[System.Buffer]::BlockCopy($videoBytes, 0, $truncatedVideo, 0, $truncatedLength)
[System.IO.File]::WriteAllBytes($damagedVideo, $truncatedVideo)

$unicodeDirectory = Join-Path $output "paths with spaces 日本語"
[System.IO.Directory]::CreateDirectory($unicodeDirectory) | Out-Null
Copy-Item -LiteralPath $transparentPng -Destination (Join-Path $unicodeDirectory "轉換 🧳 O'Reilly.png") -Force

Assert-Probeable $transparentPng
Assert-Probeable $opaqueJpeg
Assert-Probeable $orientedJpeg
Assert-Probeable $transparentWebp
Assert-Probeable $largePng
Assert-Probeable $unusualPng
Assert-Probeable $bmp
Assert-Probeable $tiff
Assert-Probeable $ico
if ($hasAvif) {
  Assert-Probeable $avif
}
Assert-Probeable $animatedPng
Assert-Probeable $shortMp4
Assert-Probeable $portraitMov
Assert-Probeable $webm
Assert-Probeable $mkv
Assert-Probeable $silentMp4
Assert-Probeable $vfrMkv
Assert-Probeable $gifSource
Assert-Probeable (Join-Path $unicodeDirectory "轉換 🧳 O'Reilly.png")

Write-Output "Generated image and video fixtures in $output"
if (-not $hasAvif) {
  Write-Output "AVIF fixture: skipped (the detected FFmpeg build has no AVIF muxer)"
}
Write-Output "HEIC/HEIF fixture: skipped (the detected FFmpeg build has no HEIF muxer)"
