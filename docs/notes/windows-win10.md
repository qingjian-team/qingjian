# Windows 10 上的设置程序（2026-09-13）

## 起因

Windows 10 22H2 上点开始菜单的「青简设置」，弹的是系统错误框：

> qingjian-settings.exe - 无法找到入口
> 无法定位程序输入点 TryCreatePackageDependency 于动态链接库 C:\Program Files\Qingjian\qingjian-settings.exe 上。

同一台机器上 Server 与 TSF DLL 一切正常（候选、上屏、学习都在跑），只有设置程序起不来。

## 定位

1. 读 `qingjian-settings.exe` 的 PE 导入表：`api-ms-win-appmodel-runtime-l1-1-5.dll` 下挂着
   `TryCreatePackageDependency` 与 `AddPackageDependency`，而且是**静态导入**（延迟导入段是空的）。
2. 在 Windows 10 22H2（19045.5011）上逐个探导出：`kernel32.dll` / `kernelbase.dll` / `AppXDeploymentClient.dll` /
   `Kernel.Appcore.dll` 与 `api-ms-win-appmodel-runtime-l1-1-{0,1,2,3}` 全都没有这两个函数，符号名在 System32 里
   也搜不到。微软文档写明 `TryCreatePackageDependency` 的 Minimum supported client 是
   **Windows 11（10.0.22000）**——Windows 10 的任何版本都没有这个 API，也没有可再发行组件能补上
   （补 DLL 那一类野路子对它无效）。
3. 名字只能来自依赖：`apps/windows/settings` 用 `windows-reactor = "0.100"`，它的
   `src/native/winui/bindings.rs` 用 `windows_core::link!("api-ms-win-appmodel-runtime-l1-1-5.dll" …)` 早期绑定
   这两个函数，`src/native/winui/bootstrap.rs` 拿它们把 `Microsoft.WindowsAppRuntime.2` 框架包加进进程包图
   （框架依赖部署的标准做法，但那条路只有 Windows 11 24H1+ 走得通）。
4. 上游有同样的问题与讨论：microsoft/windows-rs issue #4866（Reactor 5419f50 之后，Windows 10 上以
   0xc0000139 STATUS_ENTRYPOINT_NOT_FOUND 退出）、PR #4873（改成运行时解析；**已关闭未合并**，master 至今
   仍是早期绑定，0.100.0 也是）、issue #4692（自包含 + `bootstrap()` 会缺 `MddBootstrapInitialize2`）。
5. 关键事实：机器上装的 Windows App Runtime **自己**的 MSIX 清单写的是
   `<TargetDeviceFamily Name="Windows.Universal" MinVersion="10.0.17763.0" />`——运行时支持 Windows 10 1809+，
   不支持的只是 Reactor 的引导**方式**。

## 结论

- 框架依赖部署在 Windows 10 上无解：它必须先靠那两个 Windows 11 才有的 API 把框架包加进进程包图。
- **自包含部署**（exe 旁边自带一份 Windows App Runtime）在 Windows 10 上可行：Reactor 见到清单里有
  `windows-reactor-self-contained` 标记、且 exe 旁边有 `Microsoft.WindowsAppRuntime.dll`，就整段跳过引导。
- 但光换部署方式不够：IAT 里那两个名字会让 Windows 10 在**加载期**就拒绝启动，连自包含分支都进不去。

## 改法

- `apps/windows/settings/build.rs`：① `windows_reactor_setup::as_self_contained()` 铺运行时并嵌自包含清单；
  ② 链接参数加 `/DELAYLOAD:api-ms-win-appmodel-runtime-l1-1-5.dll` + `delayimp.lib`，把这两个名字从 IAT 挪到
  延迟加载描述符——自包含部署下它们永远不会被调用，Windows 10 也就不会去找它们。
- `apps/windows/installer/`：`settings-runtime.txt` 记要跟着装的运行时清单，`build.ps1` 按清单从
  `target\release` 挑进 `target\installer\settings-runtime`，`qingjian.iss` 整个目录装进 `{app}`。
  代价：安装包多约 56 MB 的运行时（185 个文件，LZMA2 压完更小），换来的是不依赖机器上装没装框架包。

## 验证

在本机（Windows 10 22H2，19045.5011）跑：

```powershell
cargo build --release -p qingjian-windows-settings
target\release\qingjian-settings.exe
```

设置窗口应当正常打开（不再弹入口错误），改一项设置能写进 `%APPDATA%\Qingjian\config.toml`。
反证：把 `build.rs` 里那两行 `/DELAYLOAD` 去掉重编，同一台机器上会重新以 0xc0000139 在加载期失败。
CI 侧：`windows` job 编三个 Windows 产物；`release.yml` 打 Inno 包时缺任何一项运行时文件会在 `build.ps1` 里直接失败。

## 经验

- 「无法定位程序输入点 X 于**动态链接库 <自己的 exe>** 上」是 API-set 早期绑定缺失时的典型措辞：报出来的模块名
  可能是宿主 exe，而不是真正缺函数的那份 DLL；顺着 PE 导入表找，比按报错文案猜快得多。
- 依赖里的 `#[link]` / `windows_core::link!` 会成为**你的** exe 的导入表条目，哪怕那条代码路径在运行时根本走不到。
  「上游 crate 支持多老的系统」与「你的 exe 能在多老的系统上**启动**」是两件事。
