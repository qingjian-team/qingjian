# qingjian-tsf

青简输入法的 **Windows TSF 文本服务 DLL**（产物 `qingjian_tsf.dll`）。Windows 会把它加载进每一个接受
文本输入的应用进程；它只做适配，不含任何输入逻辑——把系统按键翻成协议消息发给独立的
`qingjian-server` 进程（输入内核 `qingjian-core::Engine` 在那边），再把 Server 回的候选画出来。

```
应用进程 ── qingjian_tsf.dll ── 命名管道 \\.\pipe\qingjian ─▶ qingjian-server（唯一的 Engine）
```

为什么内核要在进程外，见 `../windows/README.md` 与 `docs/design/architecture.md`「Windows：TSF」。

## 两层

- **引擎层**（`client`，平台无关）：`EngineClient` 把开 / 关会话、按键、上屏编排成
  `qingjian-platform::protocol` 的消息，在一条双工字节流上收发。它泛型在任意 `Read + Write` 上，所以能用
  `UnixStream::pair` 接上真正的 `qingjian-server` 端到端测（`tests/protocol_loop.rs`，本机就能跑）。
  `cfg(windows)` 的 `client::pipe` 用 `CreateFileW` 把这条流接到命名管道。
- **COM 层**（`com`，`cfg(windows)`）：实现 TSF 要求的 COM 接口。`DllGetClassObject` → `IClassFactory` →
  `#[implement(ITfTextInputProcessor, ITfKeyEventSink)]`；`Activate` 时把自己挂到击键管理器上收键、并连
  Server；`OnKeyDown` 转发按键。`DllRegisterServer` 写 InprocServer32 并经 `ITfInputProcessorProfiles` /
  `ITfCategoryMgr` 把青简登记成键盘类文本服务（这样它出现在系统输入法列表里）。

## 现状

最小链路已通：系统能造出对象、`Activate`、经 `ITfKeyEventSink` 收到按键并转发给 Server。**尚未**把候选画成
窗口、也**尚未**经 `ITfContext` 编辑会话把选中的字上屏——收键结果先记进日志
（`%LOCALAPPDATA%\Qingjian\tsf.log`），用来验证「真实应用 → TSF → 管道 → Server → 引擎」整条链路。
候选窗口（Win32 / Direct2D）与 preedit / commit 上屏是下一步。

## 构建与试用（都在 Windows 上，需 MSVC 工具链）

本机（macOS / Linux）只能交叉 `check`，链接不出可用 DLL：

```bash
cargo check --target x86_64-pc-windows-gnu -p qingjian-tsf
```

在 Windows 机器上：

```bat
:: 1) 编译出 DLL 与 Server
cargo build -p qingjian-tsf -p qingjian-windows

:: 2) 注册文本服务（改 HKEY_CLASSES_ROOT，要管理员）
regsvr32 target\debug\qingjian_tsf.dll

:: 3) 起 Server（引擎在这里；不起的话 DLL 连不上管道，键会放行）
cargo run -p qingjian-windows

:: 4) 在系统「语言 / 输入法」里应能看到「青简」，切到它，在任意输入框敲字
::    观察 %LOCALAPPDATA%\Qingjian\tsf.log 看按键与候选是否流到了引擎

:: 反注册
regsvr32 /u target\debug\qingjian_tsf.dll
```

版本号与发布标签（`windows-v<版本>`）独立于其他平台，见 `docs/notes/release.md`。
