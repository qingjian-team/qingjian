# 启动离线翻译后台；只启动本机模型服务，不修改系统环境变量。
param([Parameter(Mandatory=$true)][string]$Root)
$ErrorActionPreference = 'Stop'
$qjOfflineRoot = (Resolve-Path -LiteralPath $Root).Path
$qjOfflinePython = Join-Path $qjOfflineRoot 'runtime\pythonw.exe'
$qjOfflineArguments = '-X utf8 -m qingjian_offline --models "' + (Join-Path $qjOfflineRoot 'models') + '" --token-file "' + (Join-Path $qjOfflineRoot 'local-token.txt') + '"'
try {
    $qjOfflineHealth = Invoke-RestMethod -Uri 'http://127.0.0.1:17863/health' -TimeoutSec 2
    if ($qjOfflineHealth.offline -and $qjOfflineHealth.ready) { return }
} catch {}
$qjOfflineProcess = Start-Process -FilePath $qjOfflinePython -ArgumentList $qjOfflineArguments -WorkingDirectory $qjOfflineRoot -WindowStyle Hidden -PassThru
Set-Content -LiteralPath (Join-Path $qjOfflineRoot 'service.pid') -Value $qjOfflineProcess.Id -Encoding ascii
for ($qjOfflineAttempt = 0; $qjOfflineAttempt -lt 40; $qjOfflineAttempt++) {
    Start-Sleep -Milliseconds 250
    try {
        $qjOfflineHealth = Invoke-RestMethod -Uri 'http://127.0.0.1:17863/health' -TimeoutSec 1
        if ($qjOfflineHealth.offline -and $qjOfflineHealth.ready) { return }
    } catch {}
    if ($qjOfflineProcess.HasExited) { throw '离线翻译服务启动失败，请检查本地模型和运行时。' }
}
throw '离线翻译服务未能在 10 秒内就绪。'
