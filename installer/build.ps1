$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repositoryRoot

$metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$package = $metadata.packages | Where-Object { $_.name -eq "espresso" } | Select-Object -First 1
if ($null -eq $package) {
    throw "Could not find the espresso package metadata."
}

$version = $package.version
$releaseDirectory = Join-Path $repositoryRoot "target\release"
$releaseExecutable = Join-Path $releaseDirectory "espresso.exe"
$installerDirectory = Join-Path $repositoryRoot "target\installers"

New-Item -ItemType Directory -Force -Path $installerDirectory | Out-Null

cargo build --release
if (-not (Test-Path -LiteralPath $releaseExecutable)) {
    throw "Release executable was not produced: $releaseExecutable"
}

$makensis = Get-Command makensis.exe -ErrorAction SilentlyContinue
if ($null -eq $makensis) {
    $makensisCandidates = @(
        "${env:ProgramFiles(x86)}\NSIS\makensis.exe",
        "${env:ProgramFiles}\NSIS\makensis.exe"
    )
    $makensisPath = $makensisCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    if ($null -eq $makensisPath) {
        throw "makensis.exe was not found. Install NSIS and ensure makensis.exe is available."
    }
    $makensis = @{ Source = $makensisPath }
}

& $makensis.Source "/DVERSION=$version" (Join-Path $PSScriptRoot "espresso.nsi")
if ($LASTEXITCODE -ne 0) {
    throw "NSIS failed with exit code $LASTEXITCODE."
}

$wix = Get-Command wix.exe -ErrorAction SilentlyContinue
if ($null -eq $wix) {
    $wixPath = Join-Path $env:USERPROFILE ".dotnet\tools\wix.exe"
    if (-not (Test-Path -LiteralPath $wixPath)) {
        throw "wix.exe was not found. Install the WiX CLI with 'dotnet tool install --global wix'."
    }
    $wix = @{ Source = $wixPath }
}

$wixVersion = (& $wix.Source --version | Out-String).Trim()
$isWix7 = $wixVersion -match "^7\."

$msiPath = Join-Path $installerDirectory "espresso-$version.msi"
$wixArguments = @(
    "build"
)
if ($isWix7) {
    $wixArguments += @("-acceptEula", "wix7")
}
$wixArguments += @(
    "-d", "ProductVersion=$version",
    "-d", "SourceDir=$releaseDirectory",
    "-o", $msiPath,
    (Join-Path $PSScriptRoot "espresso.wxs")
)
& $wix.Source @wixArguments
if ($LASTEXITCODE -ne 0) {
    throw "WiX failed with exit code $LASTEXITCODE."
}

Write-Host "Created installers in $installerDirectory"
