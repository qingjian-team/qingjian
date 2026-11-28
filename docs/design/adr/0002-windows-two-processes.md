# 0002 Windows 拆 TSF DLL 与 Server 两个进程

- 状态：已采纳
- 日期：2026-09（`docs/design/architecture.md` 的「Windows：TSF」一节成形），本文件 2026-11-28 追记
- 相关：[../architecture.md](../architecture.md)、`apps/windows/server`、`apps/windows/tsf`、
  `crates/qingjian-platform/src/protocol`

## 背景

Windows 的输入法必须实现为 TSF 文本服务：`ITfTextInputProcessor` 由输入法管理器加载到**每一个**用到输入法的
应用进程里。而青简的引擎一侧带着完整的依赖树：词库 mmap、拼音解析、整句模型、神经模型、翻译表、渲染器 ——
几万行加几十 MB 的静态数据。三件事同时压在拆与不拆之间：

1. **加载成本**：DLL 被注入进每个进程，每个进程都要为这份依赖树付出镜像与内存
2. **崩溃隔离**：引擎一次 panic 不应该让用户正在打字的那个应用一起崩（TSF 里崩宿主是用户能立刻感觉到的）
3. **候选窗的 z-band**：普通顶层窗会被开始屏幕、任务栏搜索、Microsoft Store 应用这些更高 z-band 的宿主盖住，
   只有带 `uiAccess="true"` 清单、代码签名的 exe 才能把窗口升到 UIAccess 高带

## 备选

**A. 全部放进 DLL**：实现最简单，一次调用一次返回。否掉：上面三条全踩。尤其是第三点无解——DLL 在宿主进程里，
没有任何办法把窗口升到比宿主更高的 z-band。

**B. DLL + 独立 Server 进程（当前）**：DLL 只做 TSF 协议、量光标矩形、把按键报给 Server；Server 持有 Engine、
自绘候选窗与状态条，通过命名管道通信。

**C. DLL + 后台线程里的引擎**：不换进程，把 Engine 放进 DLL 的工作线程。否掉：内存与崩溃隔离都还在宿主进程里，
z-band 问题同样无解。

## 决定

走 **B**。`apps/windows/server`（Engine + IPC + 自绘窗 + 状态条）与 `apps/windows/tsf`（TSF DLL，不链 `qingjian-render`、
不链引擎）是两个 package，协议类型放 `qingjian-platform` 的 `protocol`。

补充决定，都是被现场逼出来的：

- **系统界面宿主例外**：开始屏幕 / 任务栏搜索 / AppContainer 应用里，Server 的无 uiAccess 浮层会被盖住，
  于是 DLL 在这些宿主里自己画一套轻量候选窗（`tsf/src/com/candidates/`，纯 GDI，不链渲染库），
  行为与 Server 窗口一致（选词、翻页、高亮、矩阵），外观允许不同
- **启动方式不能是 `CreateProcess`**：uiAccess 的 exe 由无权进程拉起会报 740，所以安装器完成页用
  `ShellExecAsOriginalUser`、登录时靠 `{commonstartup}` 的快捷方式由 Explorer 拉起

## 后果

好的：

- 引擎只在一处、只装一次；DLL 保持很小，被注入进任何应用都无所谓
- 引擎 panic 只死 Server，宿主应用不受影响，下次按键自动重连
- 候选窗在 UIAccess 带里，盖得住任务栏搜索与商店应用

坏的 / 必须一直守住的：

- **候选窗的绘制在 Windows 上有两份**（Server 的 `ui/candidates`、DLL 的 `com/candidates`）。这是硬约束的代价：
  DLL 不能链渲染库。守住的规则是**行为一致、外观可以不同**；尺寸与布局的通用计算要尽量沉到共用层，
  不许两边各写一套新规则
- 进程间协议是有版本的：`protocol` 的帧结构改动要同时考虑旧 DLL + 新 Server 的组合（升级过程中的常见状态）
- 任何「只在 Server 侧能做、DLL 侧做不到」的能力，都要想清楚系统界面宿主怎么办

## 复审触发条件

- 微软改变 TSF 的加载模型，或 UIAccess 的授权方式（那会让 z-band 这条理由消失）
- 出现无需注入宿主进程的 Windows 输入法形态（例如系统级外挂面板成为主流）
- 两份绘制逻辑的行为差异开始被用户当成 bug 报（说明「行为一致」这条守不住了，要重新设计）
