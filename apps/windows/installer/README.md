# 青简 Windows 安装包

用 [Inno Setup](https://jrsoftware.org/isinfo.php) 打的安装包，把 TSF DLL、Server、设置程序与随包数据一起装进
`C:\Program Files\Qingjian`，注册文本服务，并设登录自启。对应 macOS 的 pkg。

## 安装布局

```
C:\Program Files\Qingjian\
    qingjian_tsf-<版本>.dll   TSF 文本服务（被加载进每个应用进程；按版本起名，见「升级」）
    qingjian-server.exe       输入内核 Server（跑在应用进程外）
    qingjian-settings.exe     设置界面
    qingjian.ico              开始菜单 / 启动项快捷方式的图标（exe 里也嵌了一份）
    data\generated\           dict.qj / lm.qj / glossary-{en,ja,zh}.qj / english.tsv / dicts\*.qj
    assets\                   emoji\ levels\ sample\
```

Server 与设置程序按 **exe 相对**定位随包资源（`qingjian_platform::resources`）：装机时资源与 exe 同级，
开发时是仓库 `ime\`（exe 在 `target\{debug,release}\` 下往上三层）。相对写法两套布局一致，只有根不同。

用户数据仍在 `%APPDATA%\Qingjian`（config.toml、密钥 .env、学习数据、统计），日志在 `%APPDATA%\Qingjian\logs`；
卸载不动这些。图标由 `regsvr32` 写到 `%ProgramData%\Qingjian\qingjian.ico`（DLL 里 include_bytes 内嵌）。

## 安装程序做的几件事

1. **结束旧进程**：`PrepareToInstall` 里 `taskkill` Server 与设置程序（只有这两个 exe 要覆盖）。
2. **应用容器权限**：`icacls` 给安装目录加 `ALL APPLICATION PACKAGES`（SID `*S-1-15-2-1`）读+执行。
   不加的话 UWP/AppContainer 应用（任务栏搜索、设置）读不到 DLL，切不到青简。
3. **注册文本服务**：`regsvr32 /s qingjian_tsf-<版本>.dll`（写 HKCR、图标，要管理员——安装程序本就提权）。
4. **清旧 DLL**：装完删历次版本留下的 `qingjian_tsf*.dll`，仍被应用占用的登记成重启后删（`RestartReplace`）。
5. **登录自启**：「启动」文件夹放 Server 快捷方式（Explorer 走 ShellExecute 拉起才拿到 uiAccess；计划任务拿不到）。
6. **立即启动**：完成页以当前非提升用户 ShellExecute 起一次 Server，装完就能用，不必先注销。

卸载反向：杀 Server / 设置程序 → 反注册当前版本 DLL → 删文件（占用中的 DLL 重启后删，`[UninstallDelete]` 兜住旧版本的）。

## 升级：DLL 被占用怎么办

`qingjian_tsf.dll` 被加载进每一个有文本框的应用进程，文件锁着覆盖不了；Inno 缺省的 `CloseApplications`
用 Restart Manager 找出所有占用者要求关闭——对输入法 DLL 就是「关掉一切」，所以关掉它（`CloseApplications=no`），改成：

- DLL **按版本起名并排装**（`DestName: qingjian_tsf-<版本>.dll`），新文件从不与旧文件撞名；
- 只 `regsvr32` 新文件（InprocServer32 指向它）。**不要**对旧 DLL `regsvr32 /u`：那会把整个 CLSID / profile 注销掉；
- 已开着的应用继续用进程里的旧 DLL 直到重启，Server 两个版本都服务（`OpenSession` 带协议版本，对不上只记警告）；
- 装完删旧 DLL，删不掉的登记成重启后删。

## 打包（在编译机上）

```powershell
# 需要 MSVC 工具链 + Inno Setup。数据取自仓库 data\generated 与 assets，打包前先确保 .qj 是最新的。
powershell -ExecutionPolicy Bypass -File apps\windows\installer\build.ps1
```

脚本 release 构建三个产物、从 `apps\windows\server\Cargo.toml` 读版本、找 `ISCC.exe`、编 `qingjian.iss`，
成品在 `target\installer\Qingjian-<版本>-Setup.exe`。改了数据 / 脚本但二进制没变时加 `-SkipBuild`；`-Sign` 用自签证书签产物
（uiAccess 要求 Server 签名 + 装 Program Files）。

也可手动：`iscc /DAppVersion=0.1.0 apps\windows\installer\qingjian.iss`。

## 注意

- **Inno 版本**：开发机与 CI 统一用 Inno Setup **7.1.0**（CI 从 jrsoftware/issrc 的 GitHub Release 钉死下载）。它自带简体中文翻译；
  6.x 的安装包不带 `Languages\ChineseSimplified.isl`，Chocolatey 也只有 6.x，别用。`ArchitecturesAllowed=x64compatible` 需 6.3+。
- **签名**：发版证书就绪后在这里加 `SignTool`（对应 mac 的 Developer ID）；开发期用 `-Sign` 的自签证书。
- **没证书的包**（CI 内测版）：先设 `$env:QINGJIAN_UIACCESS = '0'` 再打，Server 不嵌 uiAccess——没签名的 exe 带 uiAccess=true 会起不来。
  代价是候选窗在 UWP 宿主里可能被盖住。`release.yml` 的 `windows` job 就是这么打的。
