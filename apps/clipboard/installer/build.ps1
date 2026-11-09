<#
.SYNOPSIS
    打青简剪贴板的独立安装包：release 构建 qingjian-clipboard.exe，再用 Inno Setup 编 clipboard.iss。
.DESCRIPTION
    版本号读 apps\clipboard\Cargo.toml（-dev 版接 git 短哈希，工作区有改动加 +）；
    成品在 target\installer\qingjian-clipboard-<版本>-windows-x86_64-setup.exe。找 ISCC 的顺序与输入法的 build.ps1 相同。
.PARAMETER SkipBuild
    跳过 cargo build（只改了 .iss 时用）。
#>
[CmdletBinding()]
param([switch]$SkipBuild)

$ErrorActionPreference = 'Stop'

$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$Iss  = Join-Path $PSScriptRoot 'clipboard.iss'

if (-not $SkipBuild) {
    Push-Location $Repo
    try {
        cargo build --release --locked -p qingjian-clipboard-app
        if ($LASTEXITCODE -ne 0) { throw "cargo build 失败（退出码 $LASTEXITCODE）" }
    } finally { Pop-Location }
}
$exe = Join-Path $Repo 'target\release\qingjian-clipboard.exe'
if (-not (Test-Path $exe)) { throw "缺产物 $exe，先跑一次不带 -SkipBuild 的构建" }

$verLine = Get-Content (Join-Path $Repo 'apps\clipboard\Cargo.toml') |
    Where-Object { $_ -match '^\s*version\s*=\s*"(.+)"' } | Select-Object -First 1
if (-not ($verLine -match '"(.+)"')) { throw '在 apps\clipboard\Cargo.toml 里没找到 version' }
$Version = $Matches[1]
if ($Version.EndsWith('-dev')) {
    Push-Location $Repo
    try {
        $rev = (git rev-parse --short HEAD 2>$null)
        if ($LASTEXITCODE -eq 0 -and $rev) {
            if (git status --porcelain 2>$null) { $rev = "$rev+" }
            $Version = "$Version-$rev"
        }
    } finally { Pop-Location }
}
$VersionNumeric = $Version -replace '-.*$', ''
Write-Host "版本 $Version" -ForegroundColor Cyan

$iscc = $env:QINGJIAN_ISCC
if (-not $iscc) {
    $iscc = @("${env:ProgramFiles}\Inno Setup 7\ISCC.exe", "${env:ProgramFiles(x86)}\Inno Setup 7\ISCC.exe") |
        Where-Object { Test-Path $_ } | Select-Object -First 1
}
if (-not $iscc) { $iscc = (Get-Command iscc.exe -ErrorAction SilentlyContinue).Source }
if (-not $iscc) { throw '找不到 ISCC.exe：装 Inno Setup 7 或用 QINGJIAN_ISCC 指定' }

& $iscc "/DAppVersion=$Version" "/DAppVersionNumeric=$VersionNumeric" $Iss
if ($LASTEXITCODE -ne 0) { throw "iscc 失败（退出码 $LASTEXITCODE）" }
Write-Host "完成：$(Join-Path $Repo "target\installer\qingjian-clipboard-$Version-windows-x86_64-setup.exe")" -ForegroundColor Green
