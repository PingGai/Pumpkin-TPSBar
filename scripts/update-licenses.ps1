[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$OutputEncoding = [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()

$projectRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$licensesDirectory = [IO.Path]::GetFullPath((Join-Path $projectRoot 'LICENSES'))
$projectPrefix = $projectRoot.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
$toolchainPath = Join-Path $projectRoot 'rust-toolchain.toml'
$toolchainConfiguration = [IO.File]::ReadAllText($toolchainPath, [Text.Encoding]::UTF8)
$toolchainMatch = [regex]::Match($toolchainConfiguration, '(?m)^channel\s*=\s*"(?<channel>[^"]+)"')
if (-not $toolchainMatch.Success) {
    throw 'rust-toolchain.toml 中未找到 channel。'
}
$toolchain = $toolchainMatch.Groups['channel'].Value

if (-not $licensesDirectory.StartsWith($projectPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "拒绝操作项目目录之外的许可证目录：$licensesDirectory"
}

if (-not (Get-Command cargo-about -ErrorAction SilentlyContinue)) {
    throw '未找到 cargo-about。请先执行：cargo install --locked cargo-about'
}

$aboutConfig = Join-Path $projectRoot 'about.toml'
$summaryTemplate = Join-Path $projectRoot 'build/third_party.hbs'
$licensesTemplate = Join-Path $projectRoot 'build/licenses.hbs'
$summaryOutput = Join-Path $projectRoot 'THIRD-PARTY-NOTICES.md'
$temporaryOutput = Join-Path ([IO.Path]::GetTempPath()) ("tpsbar-licenses-{0}.txt" -f [guid]::NewGuid())

$lockPath = Join-Path $projectRoot 'Cargo.lock'
$lockContent = [IO.File]::ReadAllText($lockPath, [Text.Encoding]::UTF8)
$pumpkinApiMatch = [regex]::Match($lockContent, '(?m)^\[\[package\]\]\r?\nname = "pumpkin-plugin-api"\r?\nversion = "[^"]+"\r?\nsource = "git\+https://github\.com/Pumpkin-MC/Pumpkin\.git\?rev=(?<revision>[0-9a-f]{40})#(?<commit>[0-9a-f]{40})"')
if (-not $pumpkinApiMatch.Success) {
    throw 'Cargo.lock 中未找到固定 revision 的 pumpkin-plugin-api。'
}

$pumpkinSourceRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot '../SOURCE/Pumpkin'))
if (-not (Test-Path -LiteralPath $pumpkinSourceRoot -PathType Container)) {
    throw "未找到 Pumpkin 源码仓库：$pumpkinSourceRoot"
}
$pumpkinSourceRootForGit = $pumpkinSourceRoot.Replace('\', '/')
$witLicenseObject = "$($pumpkinApiMatch.Groups['commit'].Value):crates/pumpkin-plugin-wit/LICENSE-APACHE"
$witLicenseLines = & git -c "safe.directory=$pumpkinSourceRootForGit" -C $pumpkinSourceRoot show $witLicenseObject
if ($LASTEXITCODE -ne 0) {
    throw "无法从锁定的 Pumpkin revision 读取 WIT 许可证：$witLicenseObject"
}
$lineFeed = [string][char]10
$witLicenseText = ($witLicenseLines -join $lineFeed).TrimEnd() + $lineFeed

if (Test-Path -LiteralPath $licensesDirectory) {
    Remove-Item -LiteralPath $licensesDirectory -Recurse -Force
}
New-Item -ItemType Directory -Path $licensesDirectory | Out-Null

Push-Location -LiteralPath $projectRoot
try {
    & rustup run $toolchain cargo about generate --locked --target wasm32-wasip2 --config $aboutConfig --output-file $summaryOutput $summaryTemplate
    if ($LASTEXITCODE -ne 0) { throw "cargo-about 汇总生成失败，退出代码：$LASTEXITCODE" }
    & rustup run $toolchain cargo about generate --locked --target wasm32-wasip2 --config $aboutConfig --output-file $temporaryOutput $licensesTemplate
    if ($LASTEXITCODE -ne 0) { throw "cargo-about 许可证正文生成失败，退出代码：$LASTEXITCODE" }

    $currentLicenseId = $null
    $buffer = [Collections.Generic.List[string]]::new()
    foreach ($line in Get-Content -LiteralPath $temporaryOutput -Encoding UTF8) {
        if ($line -match '^---BEGIN LICENSE (.+)---$') {
            $currentLicenseId = $Matches[1]
            $buffer.Clear()
            continue
        }
        if ($line -match '^---END LICENSE (.+)---$') {
            if ($currentLicenseId) {
                $safeId = $currentLicenseId -replace '[^A-Za-z0-9._+-]', '_'
                $licensePath = Join-Path $licensesDirectory ("$safeId.txt")
                $text = ($buffer -join "`n").TrimEnd() + "`n"
                [IO.File]::WriteAllText($licensePath, $text, [Text.UTF8Encoding]::new($false))
            }
            $currentLicenseId = $null
            $buffer.Clear()
            continue
        }
        if ($currentLicenseId) {
            [void]$buffer.Add($line)
        }
    }

    # pumpkin-plugin-wit 是绑定生成输入而不是独立 Cargo 包，cargo-about 不会枚举它。
    # 许可证正文从 Cargo.lock 固定的 Pumpkin revision 读取。
    $witLicense = Join-Path $licensesDirectory 'Pumpkin-plugin-WIT-Apache-2.0.txt'
    [IO.File]::WriteAllText($witLicense, $witLicenseText, [Text.UTF8Encoding]::new($false))
}
finally {
    Pop-Location
    if (Test-Path -LiteralPath $temporaryOutput) {
        Remove-Item -LiteralPath $temporaryOutput -Force
    }
}

Write-Host '第三方许可证已更新。'
