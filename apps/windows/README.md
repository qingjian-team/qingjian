# 青简 Windows 输入法

Windows 端是**一个产品、两个产物**，各自一个 package，同放本目录：

| 目录 | package | 产物 | 职责 |
| --- | --- | --- | --- |
| `server/` | `qingjian-windows-server` | `qingjian-server.exe` | 持有唯一的输入内核 `qingjian-core::Engine`，跑在所有应用进程之外 |
| `tsf/` | `qingjian-windows-tsf` | `qingjian_tsf.dll` | TSF 文本服务，被加载进每个应用进程，只做按键转发与候选绘制 |

```
应用进程 A ── qingjian_tsf.dll ─┐
应用进程 B ── qingjian_tsf.dll ─┼─ 命名管道 \\.\pipe\qingjian ─▶ qingjian-server（唯一的 Engine）
应用进程 C ── qingjian_tsf.dll ─┘
```

## 为什么核心逻辑要在进程外

Windows 的文本服务（TSF，Text Services Framework）是一个 COM DLL（`ITfTextInputProcessor`），
系统会把它加载进**每一个**接受文本输入的应用进程。因此输入内核不能待在 DLL 里（会被复制进几十个进程、
状态无法共享、崩溃会连累宿主应用）。青简照 Weasel（WeaselServer + WeaselTSF）、水杉（Server 进程）的做法：
Engine 只此一份，跑在独立的 Server 进程；每个应用进程里的 TSF DLL 只做两件事——把系统按键翻成协议消息发来、
把 Server 回的候选画到候选窗口。

## 为什么是两个 package 而不是一个

两个产物的依赖集合刻意不同：DLL 只依赖 `qingjian-core`、`qingjian-platform` 与官方 `windows` crate（COM
`implement` 宏），Server 才依赖词库 / 学习 / 翻译 / 云联想（含 tokio）整棵树。合成一个 package 后，DLL 的
编译单元会拉进 Server 的依赖；用 feature 区分也不行，workspace 一起构建时 feature 会统一。crate 边界就是
「DLL 不含 Engine」这条约束的强制手段。判断标准与 macOS 的 IMK 壳一致：换掉平台适配层，不应该需要改
Core 的任何一行。

## 三个部分

- **Server 进程**（`server/`）：装配并持有 Engine（词库 / 语言模型 / 翻译 / 学习），按 `SessionId` 为每个
  应用会话维护各自的组句状态，处理按键、产出候选与上屏文本，把云联想 / 本地整句模型的异步结果主动推给
  对应会话。
- **IPC 协议**（`qingjian-platform::protocol`，两端共用）：`ClientMessage`（DLL → Server：开 / 关会话、按键、
  上屏、回上下文）与 `ServerMessage`（Server → DLL：按键结果、异步重绘、请求上下文），一次要绘制的状态是
  `Frame`（preedit 分段 + 候选页）。长度前缀 JSON 帧的编解码与缺省管道名也在这里，DLL 不必依赖整个 Server 库。
- **TSF DLL**（`tsf/`）：分「引擎层」`client`（平台无关的管道客户端 `EngineClient`，泛型在任意 `Read + Write`
  上，本机就能接真 Server 端到端测）与「COM 层」`com`（`cfg(windows)`：`DllGetClassObject` → `IClassFactory`
  → `#[implement(ITfTextInputProcessor, ITfKeyEventSink)]`，编辑会话上屏，Win32 自绘候选窗口，
  `DllRegisterServer` 注册文本服务）。

设计细节见 `docs/design/architecture.md`「Windows：TSF」。

## 构建

本机（macOS / Linux）只做交叉 `check`，产出不了可用二进制，但协议层的端到端测试能跑：

```bash
cargo check --target x86_64-pc-windows-gnu -p qingjian-windows-server -p qingjian-windows-tsf
cargo test -p qingjian-windows-server -p qingjian-windows-tsf
```

真正编译与试用都在 Windows 机器上（MSVC 工具链）：

```bat
:: 1) 编译出 DLL 与 Server
cargo build -p qingjian-windows-tsf -p qingjian-windows-server

:: 2) 注册文本服务（改 HKEY_CLASSES_ROOT，图标写到 %ProgramData%\Qingjian\qingjian.ico，要管理员）
regsvr32 target\debug\qingjian_tsf.dll

:: 3) 起 Server（引擎在这里；没起时 DLL 吃掉字母键但没有候选，起来后下一键 / 下次聚焦自动重连）
cargo run -p qingjian-windows-server

:: 4) 在系统「语言 / 输入法」里应能看到「青简」，切到它，在任意输入框敲字
::    DLL 侧日志在 %LOCALAPPDATA%\Qingjian\tsf.log

:: 反注册
regsvr32 /u target\debug\qingjian_tsf.dll
```

## 版本与发布

各平台壳版本号独立（见 `docs/notes/release.md`）：两个 package 的 `version` 各自写在自己的 `Cargo.toml`，
不跟 workspace 走；发布标签用 `windows-v<版本>`。
