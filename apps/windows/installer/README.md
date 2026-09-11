# 青简 Windows 安装包

用 [Inno Setup](https://jrsoftware.org/isinfo.php) 打的安装包，把 TSF DLL、Server、设置程序与随包数据一起装进
`C:\Program Files\Qingjian`，注册文本服务，并设登录自启。对应 macOS 的 pkg。

## 安装布局

```
C:\Program Files\Qingjian\
    qingjian_tsf.dll          TSF 文本服务（被加载进每个应用进程）
    qingjian-server.exe       输入内核 Server（跑在应用进程外）
    qingjian-settings.exe     设置界面
    data\generated\           dict.qj / lm.qj / glossary-{en,ja,zh}.qj / english.tsv / dicts\*.qj
    assets\                   emoji\ levels\ sample\
```

Server 与设置程序按 **exe 相对**定位随包资源（`qingjian_platform::resources`）：装机时资源与 exe 同级，
开发时是仓库 `ime\`（exe 在 `target\{debug,release}\` 下往上三层）。相对写法两套布局一致，只有根不同。

用户数据仍在 `%APPDATA%\Qingjian`（config.toml、密钥 .env、学习数据、统计），日志在 `%APPDATA%\Qingjian\logs`；
卸载不动这些。图标由 `regsvr32` 写到 `%ProgramData%\Qingjian\qingjian.ico`（DLL 里 include_bytes 内嵌）。

## 安装程序做的四件事

1. **应用容器权限**：`icacls` 给安装目录加 `ALL APPLICATION PACKAGES`（SID `*S-1-15-2-1`）读+执行。
   不加的话 UWP/AppContainer 应用（任务栏搜索、设置）读不到 DLL，切不到青简。
2. **注册文本服务**：`regsvr32 /s qingjian_tsf.dll`（写 HKCR、图标，要管理员——安装程序本就提权）。
3. **登录自启**：建计划任务 `Qingjian Server`，登录时以普通权限（`/rl limited`，中完整性，配合命名管道）起 Server。
4. **立即启动**：以当前非提升用户起一次 Server，装完就能用，不必先注销。

卸载反向：删任务 → 杀 Server → 反注册 DLL → 删文件。

## 打包（在编译机上）

```powershell
# 需要 MSVC 工具链 + Inno Setup。数据取自仓库 data\generated 与 assets，打包前先确保 .qj 是最新的。
powershell -ExecutionPolicy Bypass -File apps\windows\installer\build.ps1
```

脚本 release 构建三个产物、从 `apps\windows\server\Cargo.toml` 读版本、找 `ISCC.exe`、编 `qingjian.iss`，
成品在 `target\installer\Qingjian-<版本>-Setup.exe`。改了数据 / 脚本但二进制没变时加 `-SkipBuild`。

也可手动：`iscc /DAppVersion=0.1.0 apps\windows\installer\qingjian.iss`。

## 注意

- **升级覆盖在用的 DLL**：`qingjian_tsf.dll` 被加载进各应用进程时文件被锁，覆盖安装删不掉旧 DLL。
  重装前让用户注销一次（或安装程序会安排重启后替换）。这条约束和开发期一致。
- **Inno 版本**：`ArchitecturesAllowed=x64compatible` 需 Inno Setup 6.3+；更老的版本把它改成 `x64`。
- **签名**：目前未签名，用户首次运行有 SmartScreen 提示。证书就绪后在这里加 `SignTool`（对应 mac 的 Developer ID）。
