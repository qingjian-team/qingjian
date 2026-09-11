# qingjian-windows-server

青简 Windows 输入法的 **Server 进程**（bin `qingjian-server`）：持有唯一的输入内核
`qingjian-core::Engine`，跑在所有应用进程之外，通过命名管道给 TSF DLL（`../tsf`）提供候选。
Windows 端的整体结构、为什么内核要在进程外、构建与注册步骤，见 `../README.md`。

## 模块

- `assembly`：装配 Engine（词库 / 释义 / 学习）。
- `dispatch::Router`：按 `SessionId` 分派多会话，处理按键、上屏、异步结果推送。
- `session`：单个应用会话的组句状态。
- `ipc`：长度前缀帧的收发循环；`ipc::pipe`（`cfg(windows)`）在 `\\.\pipe\qingjian` 上起命名管道服务。

bin `src/main.rs` 只做配置读取、Engine 装配与启动；逻辑都在库部分，`tests/` 里的集成测试直接用库。
