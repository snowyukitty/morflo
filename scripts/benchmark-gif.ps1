param(
  [string]$Fixture = "fixtures/generated/high-motion-gif-source.mp4",
  [string]$OutputDirectory = "work/morflo/gif-benchmark"
)

$ErrorActionPreference = "Stop"

$ffmpeg = Get-Command ffmpeg -ErrorAction Stop
$root = Split-Path -Parent $PSScriptRoot
$source = [System.IO.Path]::GetFullPath((Join-Path $root $Fixture))
if (-not [System.IO.File]::Exists($source)) {
  throw "GIF benchmark fixture is missing. Run pnpm generate-fixtures first."
}

$benchmarkRoot = [System.IO.Path]::GetFullPath((Join-Path $root $OutputDirectory))
$run = Join-Path $benchmarkRoot ("run-" + [guid]::NewGuid().ToString("N"))
[System.IO.Directory]::CreateDirectory($run) | Out-Null
$palette = Join-Path $run "palette.png"
$optimized = Join-Path $run "palette.gif"
$direct = Join-Path $run "direct.gif"
$frames = "trim=start=0.5:end=2.7,setpts=PTS-STARTPTS,fps=12,scale=w='max(2,trunc(min(iw,540)/2)*2)':h=-2:flags=lanczos"

function Invoke-Ffmpeg {
  param([string[]]$Arguments)
  & $ffmpeg.Source @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "FFmpeg GIF benchmark failed with exit code $LASTEXITCODE"
  }
}

Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y", "-i", $source,
  "-vf", "$frames,palettegen=max_colors=192:stats_mode=diff:reserve_transparent=0",
  "-frames:v", "1", $palette
)
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y", "-i", $source, "-i", $palette,
  "-filter_complex", "[0:v]$frames[m];[m][1:v]paletteuse=dither=sierra2_4a:diff_mode=rectangle[v]",
  "-map", "[v]", "-loop", "0", $optimized
)
Invoke-Ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-y", "-i", $source,
  "-vf", $frames, "-loop", "0", $direct
)

function Measure-Ssim {
  param([string]$Candidate)
  $filter = "[0:v]$frames,format=rgb24[ref];[1:v]setpts=PTS-STARTPTS,format=rgb24[dist];[ref][dist]ssim"
  $previousErrorPreference = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  $diagnostics = & $ffmpeg.Source -hide_banner -i $source -i $Candidate -filter_complex $filter -f null NUL 2>&1
  $exitCode = $LASTEXITCODE
  $ErrorActionPreference = $previousErrorPreference
  if ($exitCode -ne 0) {
    throw "FFmpeg SSIM comparison failed for $Candidate"
  }
  $summary = $diagnostics | Select-String "All:" | Select-Object -Last 1
  if ($null -eq $summary -or $summary.Line -notmatch "All:([0-9.]+)") {
    throw "FFmpeg did not report an SSIM summary for $Candidate"
  }
  return [double]$Matches[1]
}

$optimizedBytes = (Get-Item -LiteralPath $optimized).Length
$directBytes = (Get-Item -LiteralPath $direct).Length
[pscustomobject]@{
  fixture = $Fixture
  ffmpeg = (& $ffmpeg.Source -version | Select-Object -First 1)
  durationSeconds = 2.2
  width = 540
  fps = 12
  palette = [pscustomobject]@{
    bytes = $optimizedBytes
    ssim = Measure-Ssim $optimized
  }
  direct = [pscustomobject]@{
    bytes = $directBytes
    ssim = Measure-Ssim $direct
  }
  outputDirectory = $run
} | ConvertTo-Json -Depth 3
