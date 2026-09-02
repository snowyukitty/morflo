param(
    [ValidateRange(1, 80)]
    [int]$MaxRounds = 40,

    [ValidateRange(1, 120)]
    [int]$MaxMinutes = 45,

    [Alias("dir")]
    [string]$ReviewedEngineDir,

    [Alias("expected-manifest-sha256")]
    [string]$ExpectedManifestSha256,

    [switch]$RequireReviewedEngine
)

# Windows App Control can reject a freshly linked Cargo build helper by hash.
# This wrapper retries only after removing the exact refused generated artifact.
# It never changes policy, trusts a path, elevates, or touches source files.

$ErrorActionPreference = "Continue"
$repoRoot = Split-Path $PSScriptRoot -Parent
$tauriRoot = Join-Path $repoRoot "src-tauri"
$targetRoot = [IO.Path]::GetFullPath((Join-Path $tauriRoot "target"))
$releaseRoot = [IO.Path]::GetFullPath((Join-Path $targetRoot "release"))
$allowedBuildRoot = [IO.Path]::GetFullPath((Join-Path $releaseRoot "build"))
$allowedDepsRoot = [IO.Path]::GetFullPath((Join-Path $releaseRoot "deps"))
$bundleRoot = [IO.Path]::GetFullPath((Join-Path $releaseRoot "bundle\nsis"))
$reviewedStageRoot = [IO.Path]::GetFullPath((Join-Path $targetRoot "reviewed-engine-package"))
$reviewedStageBundle = [IO.Path]::GetFullPath((Join-Path $reviewedStageRoot "engines"))
$reviewedConfig = [IO.Path]::GetFullPath((Join-Path $reviewedStageRoot "tauri.reviewed-engine.json"))
$previousCargoTarget = $env:CARGO_TARGET_DIR
$previousManifestPin = $env:MORFLO_REVIEWED_ENGINE_MANIFEST_SHA256
$timer = [Diagnostics.Stopwatch]::StartNew()

function Assert-DirectChild {
    param(
        [Parameter(Mandatory)]
        [string]$Candidate,

        [Parameter(Mandatory)]
        [string]$AllowedParent
    )

    $resolvedCandidate = [IO.Path]::GetFullPath($Candidate)
    $resolvedParent = [IO.Path]::GetFullPath((Split-Path $resolvedCandidate -Parent))
    if (-not $resolvedParent.Equals($AllowedParent, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing generated-artifact recovery outside ${AllowedParent}: $resolvedCandidate"
    }
    return $resolvedCandidate
}

function Remove-RefusedBuildDirectory {
    param([Parameter(Mandatory)][string]$Executable)

    $directory = Split-Path $Executable -Parent
    $resolvedDirectory = Assert-DirectChild -Candidate $directory -AllowedParent $allowedBuildRoot
    if (-not (Test-Path -LiteralPath $resolvedDirectory -PathType Container)) {
        throw "Refused build directory no longer exists: $resolvedDirectory"
    }
    Write-Host "Relinking generated build helper: $(Split-Path $resolvedDirectory -Leaf)"
    Remove-Item -LiteralPath $resolvedDirectory -Recurse -Force
}

function Remove-RefusedDependencyArtifacts {
    param([Parameter(Mandatory)][string]$Library)

    $resolvedLibrary = Assert-DirectChild -Candidate $Library -AllowedParent $allowedDepsRoot
    if (-not (Test-Path -LiteralPath $resolvedLibrary -PathType Leaf)) {
        throw "Refused dependency artifact no longer exists: $resolvedLibrary"
    }

    $stem = [IO.Path]::GetFileNameWithoutExtension($resolvedLibrary)
    $relatedArtifacts = @(
        Get-ChildItem -LiteralPath $allowedDepsRoot -File |
            Where-Object {
                $_.Name.Equals("${stem}.dll", [StringComparison]::OrdinalIgnoreCase) -or
                $_.Name.StartsWith("${stem}.", [StringComparison]::OrdinalIgnoreCase)
            }
    )
    if ($relatedArtifacts.Count -eq 0) {
        throw "No generated dependency artifacts matched the refused library: $resolvedLibrary"
    }

    foreach ($artifact in $relatedArtifacts) {
        $resolvedArtifact = Assert-DirectChild -Candidate $artifact.FullName -AllowedParent $allowedDepsRoot
        Remove-Item -LiteralPath $resolvedArtifact -Force
    }
    Write-Host "Relinking generated dependency: $stem"
}

function Remove-ReviewedEngineStage {
    if (-not (Test-Path -LiteralPath $reviewedStageRoot)) {
        return
    }
    $resolvedStage = [IO.Path]::GetFullPath($reviewedStageRoot)
    $resolvedParent = [IO.Path]::GetFullPath((Split-Path $resolvedStage -Parent))
    if (
        -not $resolvedStage.Equals($reviewedStageRoot, [StringComparison]::OrdinalIgnoreCase) -or
        -not $resolvedParent.Equals($targetRoot, [StringComparison]::OrdinalIgnoreCase)
    ) {
        throw "Refusing reviewed-engine cleanup outside the exact Cargo target directory."
    }
    Remove-Item -LiteralPath $resolvedStage -Recurse -Force
}

Push-Location $repoRoot
try {
    # Keep the recovery boundary deterministic even when a parent shell defines
    # a different Cargo target directory.
    $env:CARGO_TARGET_DIR = $targetRoot

    $hasReviewedDirectory = -not [string]::IsNullOrWhiteSpace($ReviewedEngineDir)
    $hasExpectedDigest = -not [string]::IsNullOrWhiteSpace($ExpectedManifestSha256)
    if ($hasReviewedDirectory -ne $hasExpectedDigest) {
        throw "Reviewed packaging requires both -ReviewedEngineDir and -ExpectedManifestSha256."
    }
    if ($RequireReviewedEngine -and -not $hasReviewedDirectory) {
        throw "Reviewed packaging requires one explicit bundle directory and its independently reviewed manifest digest."
    }

    $tauriArguments = @("exec", "tauri", "build", "--bundles", "nsis")
    if ($hasReviewedDirectory) {
        if (-not [IO.Path]::IsPathRooted($ReviewedEngineDir)) {
            throw "ReviewedEngineDir must be an explicit absolute directory."
        }
        if ($ExpectedManifestSha256 -cnotmatch '^[0-9a-f]{64}$') {
            throw "ExpectedManifestSha256 must be 64 lowercase hexadecimal characters."
        }
        $reviewedSource = [IO.Path]::GetFullPath($ReviewedEngineDir)
        $sourcePrefix = $reviewedSource.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
        $stagePrefix = $reviewedStageRoot.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
        if (
            $reviewedSource.Equals($reviewedStageRoot, [StringComparison]::OrdinalIgnoreCase) -or
            $sourcePrefix.StartsWith($stagePrefix, [StringComparison]::OrdinalIgnoreCase) -or
            $stagePrefix.StartsWith($sourcePrefix, [StringComparison]::OrdinalIgnoreCase)
        ) {
            throw "ReviewedEngineDir cannot overlap the controlled staging directory."
        }
        Remove-ReviewedEngineStage
        New-Item -ItemType Directory -Path $reviewedStageRoot | Out-Null
        & node scripts/engine-verify.mjs `
            --dir $reviewedSource `
            --expected-manifest-sha256 $ExpectedManifestSha256 `
            --stage $reviewedStageBundle
        if ($LASTEXITCODE -ne 0) {
            throw "Reviewed media-engine staging failed closed."
        }

        $env:MORFLO_REVIEWED_ENGINE_MANIFEST_SHA256 = $ExpectedManifestSha256
        $resourceSource = $reviewedStageBundle.Replace("\", "/") + "/"
        $configuration = @{
            bundle = @{
                resources = @{
                    $resourceSource = "engines/"
                }
            }
        }
        $configuration | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $reviewedConfig -Encoding utf8NoBOM
        $tauriArguments += @("--config", $reviewedConfig)
        Write-Host "Reviewed engine input verified and staged from one explicit directory."
    }
    else {
        Remove-Item Env:MORFLO_REVIEWED_ENGINE_MANIFEST_SHA256 -ErrorAction SilentlyContinue
        Write-Host "Normal engine-free package mode."
    }

    for ($round = 1; $round -le $MaxRounds; $round++) {
        if ($timer.Elapsed.TotalMinutes -ge $MaxMinutes) {
            throw "Windows package recovery exceeded the ${MaxMinutes}-minute limit."
        }

        Write-Host "Windows package build round $round of $MaxRounds"
        $capturedOutput = @()
        & pnpm @tauriArguments 2>&1 |
            Tee-Object -Variable capturedOutput |
            ForEach-Object {
                $line = $_.ToString()
                if ($line -ne "System.Management.Automation.RemoteException") {
                    Write-Host $line
                }
            }
        $buildCode = $LASTEXITCODE
        $output = @($capturedOutput | ForEach-Object { $_.ToString() }) -join [Environment]::NewLine

        if ($buildCode -eq 0) {
            $installers = @(Get-ChildItem -LiteralPath $bundleRoot -Filter "Morflo_*_x64-setup.exe" -File)
            if ($installers.Count -ne 1) {
                throw "Expected exactly one Morflo NSIS installer under $bundleRoot; found $($installers.Count)."
            }
            Write-Host "WINDOWS PACKAGE: succeeded after $round round(s)."
            Write-Host "INSTALLER: $($installers[0].FullName)"
            exit 0
        }

        $blockedBuildHelpers = @(
            [regex]::Matches(
                $output,
                'could not execute process `([^`]+\\build-script-build(?:\.exe)?)`'
            ) |
                ForEach-Object { $_.Groups[1].Value } |
                Sort-Object -Unique
        )
        $blockedLibraries = @(
            [regex]::Matches(
                $output,
                '(?<path>[A-Za-z]:\\[^\r\n]+?\\target\\release\\deps\\[^:\r\n]+\.dll): LoadLibraryExW failed'
            ) |
                ForEach-Object { $_.Groups["path"].Value } |
                Sort-Object -Unique
        )
        if ($blockedBuildHelpers.Count -eq 0 -and $blockedLibraries.Count -eq 0) {
            if ($output -match "An Application Control policy has blocked this file") {
                throw "Application Control refused an artifact outside the bounded build-helper recovery contract."
            }
            throw "Native Windows package failed for a reason unrelated to the recoverable App Control artifacts."
        }

        foreach ($helper in $blockedBuildHelpers) {
            Remove-RefusedBuildDirectory -Executable $helper
        }
        foreach ($library in $blockedLibraries) {
            Remove-RefusedDependencyArtifacts -Library $library
        }
    }
}
finally {
    Remove-ReviewedEngineStage
    Pop-Location
    if ($null -eq $previousCargoTarget) {
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    }
    else {
        $env:CARGO_TARGET_DIR = $previousCargoTarget
    }
    if ($null -eq $previousManifestPin) {
        Remove-Item Env:MORFLO_REVIEWED_ENGINE_MANIFEST_SHA256 -ErrorAction SilentlyContinue
    }
    else {
        $env:MORFLO_REVIEWED_ENGINE_MANIFEST_SHA256 = $previousManifestPin
    }
}

throw "Windows package did not converge after $MaxRounds bounded recovery rounds."
