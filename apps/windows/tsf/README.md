# qingjian-windows-tsf

青简 Windows 输入法的 **TSF 文本服务 DLL**（产物 `qingjian_tsf.dll`）。Windows 会把它加载进每一个接受
文本输入的应用进程；它只做适配，不含任何输入逻辑——把系统按键翻成协议消息发给独立的
`qingjian-server` 进程（`../server`），再把 Server 回的候选画出来。
Windows 端的整体结构、为什么是两个 package、构建与注册步骤，见 `../README.md`。

## 两层

- **引擎层**（`client`，平台无关）：`EngineClient` 把开 / 关会话、按键、上屏编排成
  `qingjian-platform::protocol` 的消息，在一条双工字节流上收发。它泛型在任意 `Read + Write` 上，所以能用
  `UnixStream::pair` 接上真正的 Server 端到端测（`tests/protocol_loop.rs`，本机就能跑）。
  `cfg(windows)` 的 `client::pipe` 用 `std::fs::File` 打开命名管道当这条流。
- **COM 层**（`com`，`cfg(windows)`）：实现 TSF 要求的 COM 接口。`DllGetClassObject` → `IClassFactory` →
  `#[implement(ITfTextInputProcessor, ITfKeyEventSink)]`；`Activate` 时把自己挂到击键管理器上收键、并连
  Server；`OnKeyDown` 转发按键，`edit_session` / `composition` 经 `ITfContext` 做 preedit 内联与上屏，
  `anchor` 把组句 / 选区的屏幕矩形报给 Server 摆候选窗口（窗口在 Server 进程自绘），`poll` 定时拉取云联想的异步
  结果。`DllRegisterServer`（`registry`）写 InprocServer32 并经 `ITfInputProcessorProfiles` /
  `ITfCategoryMgr` 把青简登记成键盘类文本服务。

DLL 侧日志在 `%LOCALAPPDATA%\Qingjian\tsf.<日期>.log`，按天一个文件、只留最近 7 天。
