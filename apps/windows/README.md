# qingjian-windows

青简输入法的 **Windows Server 进程**：持有输入内核 `qingjian-core::Engine`，跑在所有应用进程之外，
通过 IPC 协议给 TSF DLL 提供候选，是青简在 Windows 上的核心逻辑所在。

## 为什么核心逻辑要在进程外

Windows 的文本服务（TSF，Text Services Framework）是一个 COM DLL（`ITfTextInputProcessor`），
系统会把它加载进**每一个**接受文本输入的应用进程。因此输入内核不能待在 DLL 里（会被复制进几十个进程、
状态无法共享、崩溃会连累宿主应用）。青简照 Weasel（WeaselServer）、水杉（Server 进程）的做法：Engine
只此一份，跑在这个独立的 Server 进程；每个应用进程里的 TSF DLL 只做两件事——把系统按键翻成协议消息发来、
把 Server 回的候选画到候选窗口。

```
应用进程 A ── TSF DLL ─┐
应用进程 B ── TSF DLL ─┼─ 命名管道 ─▶ qingjian-server（本 crate，唯一的 Engine）
应用进程 C ── TSF DLL ─┘
```

## 三个部分

- **Server 进程**（本 crate，bin `qingjian-server`）：装配并持有 Engine（词库 / 语言模型 / 翻译 / 学习），
  按 `SessionId` 为每个应用会话维护各自的组句状态，处理按键、产出候选与上屏文本，把云联想 / 本地整句模型的
  异步结果主动推给对应会话。
- **IPC 协议**（`qingjian-platform::protocol`，与 DLL 共用）：`ClientMessage`（DLL → Server：开 / 关会话、
  按键、上屏、回上下文）与 `ServerMessage`（Server → DLL：按键结果、异步重绘、请求上下文），一次要绘制的
  状态是 `Frame`（preedit 分段 + 候选页）。全部 serde 可序列化。
- **TSF DLL**（单独的 `cdylib` crate，依赖 `windows` crate 的 COM `implement` 宏）：实现 TSF 接口，
  `OnKeyDown` 转成协议消息，用 Win32 / Direct2D 画候选窗口。只做 IPC 与绘制，不含任何排序 / 词库逻辑。

判断标准与 macOS 的 IMK 壳一致：换掉平台适配层，不应该需要改 Core 的任何一行。

## 构建

各平台壳版本号独立（见 `docs/notes/release.md`）：本 crate 的 `version` 在自己的 `Cargo.toml`，
不跟 workspace 走；发布标签用 `windows-v<版本>`。

本机（macOS / Linux）只做交叉 `check`，产出不了可用二进制：

```bash
cargo check --target x86_64-pc-windows-gnu -p qingjian-windows -p qingjian-platform -p qingjian-core
```

真正编译在 Windows 机器上做（装 MSVC 工具链后）：

```bat
cargo build -p qingjian-windows
```
