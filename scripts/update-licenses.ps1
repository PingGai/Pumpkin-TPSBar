[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$OutputEncoding = [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()

$projectRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$licensesDirectory = [IO.Path]::GetFullPath((Join-Path $projectRoot 'LICENSES'))
$projectPrefix = $projectRoot.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar

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

if (Test-Path -LiteralPath $licensesDirectory) {
    Remove-Item -LiteralPath $licensesDirectory -Recurse -Force
}
New-Item -ItemType Directory -Path $licensesDirectory | Out-Null

Push-Location -LiteralPath $projectRoot
try {
    cargo about generate --locked --target wasm32-wasip2 --config $aboutConfig --output-file $summaryOutput $summaryTemplate
    cargo about generate --locked --target wasm32-wasip2 --config $aboutConfig --output-file $temporaryOutput $licensesTemplate

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
    # 直接归档上游随 WIT 分发的原始许可证，避免把本项目版权声明误写入上游许可证。
    $witSourceLicense = [IO.Path]::GetFullPath((Join-Path $projectRoot '../SOURCE/Pumpkin/crates/pumpkin-plugin-wit/LICENSE-APACHE'))
    if (-not (Test-Path -LiteralPath $witSourceLicense -PathType Leaf)) {
        throw "未找到 Pumpkin Plugin WIT 许可证：$witSourceLicense"
    }
    $witLicense = Join-Path $licensesDirectory 'Pumpkin-plugin-WIT-Apache-2.0.txt'
    [IO.File]::Copy($witSourceLicense, $witLicense, $true)
}
finally {
    Pop-Location
    if (Test-Path -LiteralPath $temporaryOutput) {
        Remove-Item -LiteralPath $temporaryOutput -Force
    }
}

Write-Host '第三方许可证已更新。'
