[CmdletBinding()]
param(
    [switch]$Release,
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$versionPath = Join-Path $projectRoot 'build/version.txt'
$manifestPath = Join-Path $projectRoot 'Cargo.toml'
$toolchainPath = Join-Path $projectRoot 'rust-toolchain.toml'
$releaseLine = ([IO.File]::ReadAllText($versionPath, [Text.Encoding]::UTF8)).Trim()

if ($releaseLine -notmatch '^\d+\.\d+$') {
    throw "无效发布线版本：$releaseLine（build/version.txt 只能写 major.minor）"
}

$toolchainConfiguration = [IO.File]::ReadAllText($toolchainPath, [Text.Encoding]::UTF8)
$toolchainMatch = [regex]::Match($toolchainConfiguration, '(?m)^channel\s*=\s*"(?<channel>[^"]+)"')
if (-not $toolchainMatch.Success) {
    throw 'rust-toolchain.toml 中未找到 channel。'
}
$toolchain = $toolchainMatch.Groups['channel'].Value

$manifest = [IO.File]::ReadAllText($manifestPath, [Text.Encoding]::UTF8)
$packageVersionMatch = [regex]::Match($manifest, '(?m)^version\s*=\s*"(?<version>[^"]+)"')
if (-not $packageVersionMatch.Success) {
    throw 'Cargo.toml 中未找到 package version。'
}

$expectedPackageVersion = "$releaseLine.0"
if ($packageVersionMatch.Groups['version'].Value -ne $expectedPackageVersion) {
    throw "Cargo.toml 版本必须为 $expectedPackageVersion，与 build/version.txt 不匹配。"
}

$lockPath = Join-Path $projectRoot 'Cargo.lock'
$lockContent = [IO.File]::ReadAllText($lockPath, [Text.Encoding]::UTF8)
$pumpkinVersionMatch = [regex]::Match($lockContent, '(?m)^\[\[package\]\]\r?\nname = "pumpkin-plugin-api"\r?\nversion = "(?<version>[^"]+)"\r?\nsource = "git\+https://github\.com/Pumpkin-MC/Pumpkin\.git')
if (-not $pumpkinVersionMatch.Success) {
    throw 'Cargo.lock 中未找到 Pumpkin Plugin API 版本。'
}
$pumpkinSemver = $pumpkinVersionMatch.Groups['version'].Value -replace '\+.*$', ''
if ($pumpkinSemver -notmatch '^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$') {
    throw "无效 Pumpkin Plugin API 版本：$pumpkinSemver"
}

if ($Release) {
    $artifactVersion = "v$releaseLine+$pumpkinSemver"
} else {
    $timestamp = Get-Date -Format 'yyyyMMddHHmm'
    $artifactVersion = "$releaseLine.0-dev.$timestamp+$pumpkinSemver"
}

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $destinationDirectory = Join-Path $projectRoot 'publish'
} elseif ([IO.Path]::IsPathRooted($OutputDirectory)) {
    $destinationDirectory = $OutputDirectory
} else {
    $destinationDirectory = Join-Path $projectRoot $OutputDirectory
}
$destinationDirectory = [IO.Path]::GetFullPath($destinationDirectory)

Push-Location -LiteralPath $projectRoot
try {
    & rustup run $toolchain cargo build --locked --release --target wasm32-wasip2
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo 构建失败，退出代码：$LASTEXITCODE"
    }
} finally {
    Pop-Location
}

$sourceArtifact = Join-Path $projectRoot 'target/wasm32-wasip2/release/tpsbar.wasm'
if (-not (Test-Path -LiteralPath $sourceArtifact -PathType Leaf)) {
    throw "未找到构建产物：$sourceArtifact"
}

[IO.Directory]::CreateDirectory($destinationDirectory) | Out-Null
$artifactName = "tpsbar-$artifactVersion.wasm"
$destinationPath = Join-Path $destinationDirectory $artifactName
if (Test-Path -LiteralPath $destinationPath -PathType Leaf) {
    throw "目标产物已存在，拒绝覆盖：$destinationPath"
}
[IO.File]::Copy($sourceArtifact, $destinationPath)
Write-Host "已生成：$destinationPath"
