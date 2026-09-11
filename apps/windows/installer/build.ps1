<#
.SYNOPSIS
    在 Windows 上打青简安装包：release 构建三个产物 + 用 Inno Setup 编 qingjian.iss。
.DESCRIPTION
    在编译机（MSVC 工具链 + Inno Setup）上跑。步骤：
      1) cargo build --release 出 DLL / Server / 设置程序；
      2) 从 apps\windows\server\Cargo.toml 读版本号；
      3) 找 ISCC.exe（PATH 或常见安装位置）；
      4) iscc /DAppVersion=<版本> 编脚本，成品在 target\installer\Qingjian-<版本>-Setup.exe。
    随包数据（.qj / .tsv）直接由 .iss 从仓库 data\generated 与 assets 里取，不另建暂存目录；
    确保打包前 data\generated 里的 .qj 是最新的（bundle 流程见仓库 CLAUDE.md）。
.PARAMETER SkipBuild
    跳过 cargo build（数据或 .iss 改了、二进制没变时重编安装包用）。
.PARAMETER Sign
    打包前用自签证书给产物代码签名（sign-local.ps1）。uiAccess=true 的 Server 必须签名 + 装 Program Files
    才拿到高 z-band 权限；不加此开关打出的包，Server 在商店 / 任务栏搜索里仍被盖住（桌面程序不受影响）。
#>
[CmdletBinding()]
param([switch]$SkipBuild, [switch]$Sign)

$ErrorActionPreference = 'Stop'

# 仓库根：本脚本在 apps\windows\installer 下，往上三层是 ime\。
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$Iss  = Join-Path $PSScriptRoot 'qingjian.iss'

# 1) 构建三个产物。
if (-not $SkipBuild) {
    Write-Host '构建 release 产物…' -ForegroundColor Cyan
    Push-Location $Repo
    try {
        cargo build --release -p qingjian-windows-server -p qingjian-windows-tsf -p qingjian-windows-settings
        if ($LASTEXITCODE -ne 0) { throw "cargo build 失败（退出码 $LASTEXITCODE）" }
    } finally { Pop-Location }
}

# 缺一个产物就早报错。
$targets = @('qingjian_tsf.dll', 'qingjian-server.exe', 'qingjian-settings.exe')
foreach ($t in $targets) {
    $p = Join-Path $Repo "target\release\$t"
    if (-not (Test-Path $p)) { throw "缺产物 $p，先跑一次不带 -SkipBuild 的构建" }
}

# 1.5) 签名（必须在 iscc 打包前：Inno 把已签的文件原样拷进安装包）。
if ($Sign) {
    Write-Host '自签产物（uiAccess 要求 Server 代码签名）…' -ForegroundColor Cyan
    $binaries = $targets | ForEach-Object { Join-Path $Repo "target\release\$_" }
    & (Join-Path $PSScriptRoot 'sign-local.ps1') -Path $binaries
}

# 2) 从 server 的 Cargo.toml 读版本（apps\* 各自写死版本，不跟 workspace）。
$cargoToml = Get-Content (Join-Path $Repo 'apps\windows\server\Cargo.toml')
$verLine = $cargoToml | Where-Object { $_ -match '^\s*version\s*=\s*"(.+)"' } | Select-Object -First 1
if (-not ($verLine -match '"(.+)"')) { throw '在 server\Cargo.toml 里没找到 version' }
$Version = $Matches[1]
Write-Host "版本 $Version" -ForegroundColor Cyan

# 3) 找 ISCC.exe。
$iscc = (Get-Command iscc.exe -ErrorAction SilentlyContinue).Source
if (-not $iscc) {
    $candidates = @(
        "${env:ProgramFiles(x86)}\Inno Setup 7\ISCC.exe",
        "${env:ProgramFiles}\Inno Setup 7\ISCC.exe",
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "${env:ProgramFiles}\Inno Setup 6\ISCC.exe"
    )
    $iscc = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
}
if (-not $iscc) { throw '找不到 ISCC.exe：把 Inno Setup 装了或加进 PATH' }
Write-Host "用 $iscc" -ForegroundColor Cyan

# 4) 编安装包。
& $iscc "/DAppVersion=$Version" $Iss
if ($LASTEXITCODE -ne 0) { throw "iscc 失败（退出码 $LASTEXITCODE）" }

$out = Join-Path $Repo "target\installer\Qingjian-$Version-Setup.exe"
Write-Host "完成：$out" -ForegroundColor Green
