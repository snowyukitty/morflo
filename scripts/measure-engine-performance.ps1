param(
  [string]$FixtureDirectory = "fixtures/generated",
  [string]$OutputPath = "work/morflo/performance-engine.json"
)

$ErrorActionPreference = "Stop"
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$fixtureRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $FixtureDirectory))
$ffmpeg = (Get-Command ffmpeg -ErrorAction Stop).Source
$ffprobe = (Get-Command ffprobe -ErrorAction Stop).Source

function New-ProcessStartInfo {
  param([string]$Executable, [string[]]$Arguments)
  $info = [System.Diagnostics.ProcessStartInfo]::new()
  $info.FileName = $Executable
  $info.UseShellExecute = $false
  $info.CreateNoWindow = $true
  $info.RedirectStandardOutput = $true
  $info.RedirectStandardError = $true
  # Windows PowerShell 5.1 predates ProcessStartInfo.ArgumentList. These arguments are
  # fixed by this repository; quoting keeps fixture paths with spaces intact without a shell.
  $info.Arguments = ($Arguments | ForEach-Object {
    '"' + $_.Replace('"', '\"') + '"'
  }) -join ' '
  return $info
}

function Measure-Process {
  param([string]$Executable, [string[]]$Arguments)
  $timer = [System.Diagnostics.Stopwatch]::StartNew()
  $process = [System.Diagnostics.Process]::Start((New-ProcessStartInfo $Executable $Arguments))
  $stdout = $process.StandardOutput.ReadToEndAsync()
  $stderr = $process.StandardError.ReadToEndAsync()
  [long]$peakBytes = 0
  while (-not $process.HasExited) {
    $process.Refresh()
    $peakBytes = [Math]::Max($peakBytes, $process.WorkingSet64)
    Start-Sleep -Milliseconds 5
  }
  $process.WaitForExit()
  $timer.Stop()
  if ($process.ExitCode -ne 0) {
    throw "Measured process failed: $($stderr.Result)"
  }
  return [pscustomobject]@{
    Milliseconds = $timer.Elapsed.TotalMilliseconds
    PeakBytes = $peakBytes
  }
}

function Get-Sha256 {
  param([string]$Path)
  $stream = [System.IO.File]::OpenRead($Path)
  try {
    $algorithm = [System.Security.Cryptography.SHA256]::Create()
    try {
      return [System.BitConverter]::ToString($algorithm.ComputeHash($stream)).Replace("-", "")
    } finally {
      $algorithm.Dispose()
    }
  } finally {
    $stream.Dispose()
  }
}

$video = Join-Path $fixtureRoot "short-1080p.mp4"
$largeImage = Join-Path $fixtureRoot "large-image.png"
if (-not (Test-Path -LiteralPath $video) -or -not (Test-Path -LiteralPath $largeImage)) {
  throw "Generate the fixture corpus before measuring performance."
}

$probeArguments = @(
  "-v", "error", "-print_format", "json", "-show_format", "-show_streams", $video
)
$coldProbe = Measure-Process $ffprobe $probeArguments
$warmProbes = 1..5 | ForEach-Object { (Measure-Process $ffprobe $probeArguments).Milliseconds }

$progressInfo = New-ProcessStartInfo $ffmpeg @(
  "-hide_banner", "-loglevel", "error", "-nostdin", "-i", $video,
  "-t", "2", "-an", "-c:v", "libx264", "-preset", "ultrafast", "-f", "null",
  "-progress", "pipe:1", "-stats_period", "0.1", "NUL"
)
$progressTimer = [System.Diagnostics.Stopwatch]::StartNew()
$progressProcess = [System.Diagnostics.Process]::Start($progressInfo)
$progressError = $progressProcess.StandardError.ReadToEndAsync()
$firstProgressMilliseconds = $null
while (($line = $progressProcess.StandardOutput.ReadLine()) -ne $null) {
  if ($null -eq $firstProgressMilliseconds -and $line.StartsWith("out_time=")) {
    $firstProgressMilliseconds = $progressTimer.Elapsed.TotalMilliseconds
  }
}
$progressProcess.WaitForExit()
$progressTimer.Stop()
if ($progressProcess.ExitCode -ne 0 -or $null -eq $firstProgressMilliseconds) {
  throw "Progress measurement failed: $($progressError.Result)"
}

$tempBase = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$tempDirectory = [System.IO.Path]::GetFullPath(
  (Join-Path $tempBase ("morflo-perf-" + [System.Guid]::NewGuid().ToString("N")))
)
if ([System.IO.Path]::GetDirectoryName($tempDirectory) -ne $tempBase.TrimEnd('\')) {
  throw "Refusing to use an unexpected performance temporary directory."
}
[System.IO.Directory]::CreateDirectory($tempDirectory) | Out-Null
try {
  $imageHashBefore = Get-Sha256 $largeImage
  $imageOutput = Join-Path $tempDirectory "large-image.webp"
  $imageRun = Measure-Process $ffmpeg @(
    "-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-i", $largeImage,
    "-frames:v", "1", "-c:v", "libwebp", "-quality", "82", "-compression_level", "5",
    $imageOutput
  )
  $imageHashAfter = Get-Sha256 $largeImage
  if ($imageHashBefore -ne $imageHashAfter) {
    throw "The performance run changed its source fixture."
  }

  $result = [ordered]@{
    environment = "Windows 11 x64; FFmpeg child processes measured with System.Diagnostics.Process"
    engine = (& $ffmpeg -hide_banner -version | Select-Object -First 1)
    coldProbeMs = [Math]::Round($coldProbe.Milliseconds, 1)
    warmProbeMeanMs = [Math]::Round(($warmProbes | Measure-Object -Average).Average, 1)
    warmProbeMaxMs = [Math]::Round(($warmProbes | Measure-Object -Maximum).Maximum, 1)
    firstUsefulProgressMs = [Math]::Round($firstProgressMilliseconds, 1)
    largeImage = "4096x3072 PNG to WebP"
    largeImageConversionMs = [Math]::Round($imageRun.Milliseconds, 1)
    largeImageEnginePeakMiB = [Math]::Round($imageRun.PeakBytes / 1MB, 1)
    sourceHashPreserved = $true
  }
  $absoluteOutput = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $OutputPath))
  [System.IO.Directory]::CreateDirectory([System.IO.Path]::GetDirectoryName($absoluteOutput)) | Out-Null
  $result | ConvertTo-Json | Set-Content -LiteralPath $absoluteOutput -Encoding utf8
  $result | ConvertTo-Json
} finally {
  if ((Test-Path -LiteralPath $tempDirectory) -and
      [System.IO.Path]::GetDirectoryName($tempDirectory) -eq $tempBase.TrimEnd('\')) {
    Remove-Item -LiteralPath $tempDirectory -Recurse -Force
  }
}
