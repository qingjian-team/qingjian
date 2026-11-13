# Linux 五笔接入实现计划

> 面向 AI 实现者：使用 executing-plans 内联执行；不创建分支、工作树或提交。

**目标：** Linux 三个前端通过共用 Server 支持五笔 86、形码方案与拼音混输。

**架构：** Router 复用 `SchemeSlot`，Core 继续负责形码查询与上屏判定。共用安装清单携带方案资源，设置程序只开放已有入口。

**技术栈：** Rust、Fcitx C++ 插件、IBus D-Bus、Python 安装测试。

**规格：** `docs/design/linux-wubi.md`，2026-11-05 已确认。

## 约束与审查重点

- 不修改 Core 排序与 Windows/macOS 行为，不新增协议或运行依赖。
- 旧组词与迟到的异步结果不能跨方案继续提交；无关配置更新不能清空输入。
- 顶字提交当前高亮，提交与下一词预编辑允许同时存在。
- 码表失效不能让输入法完全失去输入能力。
- 坏配置时保留有效配置，用户词库仍按上次有效开关更新。
- 安装及卸载不能覆盖或删除用户方案、词库和配置。

## 任务 1：按键上屏接入

文件：`apps/linux/server/src/dispatch/key/input.rs`、`key/tests/mod.rs`、新增 `key/tests/code.rs`。

- [x] 为纯五笔、混输、顶字、自动上屏、重码、退格、数字与空格选词编写失败测试。
- [x] 运行 `cargo test -p qingjian-linux-server --locked dispatch::key::tests::code`，确认失败对应缺失行为。
- [x] 参照 Windows `push_letter`，复用 `code_tops` 与 `commit_unique_code`，不在壳中查询码表。
- [x] 重跑针对性测试与现有按键测试。

## 任务 2：启动、方案与热更新

文件：Linux `dispatch/mod.rs`、新增 `dispatch/schemes/`、`dispatch/reload/{mod,state}.rs`、`main.rs`，新增方案测试。

- [x] 为启动配置、混输切换、方案缺失回退、用户优先级、自定义双拼、规则更新、多会话清理编写测试。
- [x] 增加用户方案和导入五笔词库不修改配置即可更新、坏配置保持有效状态的失败测试。
- [x] Router 保存 `SchemeSlot` 与有效方案配置/目录快照；启动和轮询统一应用。
- [x] 实际方案变化时取消各会话组词并失效展示身份、翻译和重排结果；其他配置保持组词。
- [x] 重跑 Linux Server 全部测试。

## 任务 3：安装资源与设置能力

文件：`apps/linux/scripts/files.py`、`apps/linux/tests/installation.py`、`apps/settings/src/support/mod.rs` 及支持状态测试。

- [x] 为源码/打包资源中的码表、描述和许可证，以及卸载保留个人方案增加失败断言。
- [x] 随包五笔沿用 `dicts.lock` 校验和安装，运行 `python3 apps/linux/tests/installation.py`。
- [x] 开放 Linux `Feature::Schemes`，保持辅码与字根提示状态不变。
- [x] 在独立进程中验证设置侧能发现临时资源目录的随包五笔方案及许可证并加载码表。

## 任务 4：跨前端与真实输入

文件：Fcitx5/Fcitx4 现有 CTest、IBus `tests/ibus_engine.rs` 与直接相关测试夹具。

- [x] 扩展真实 Server 链路测试，验证五笔候选、上屏与下一段预编辑共存。
- [x] 构建 Server 与 IBus，运行 Fcitx5 CTest 和隔离的真实 IBus daemon 测试；Rust 信号测试受本机 IBus / zbus 组合阻碍，单独的 GTK3 输入通过，不把前者写成通过。
- [x] 评估并补齐 Fcitx4 依赖，运行其 CTest；运行时改为解包隔离使用。
- [x] 使用隔离配置、词库和学习目录，在本机 Xvfb 的真实 GTK3 文本框验证 Fcitx5、Fcitx4 与 IBus 输入；没有操作用户当前桌面输入框。
- [x] 验证临时发行资源输出；deb/rpm 缺少构建环境，明确记录未验证。

## 任务 5：文档、门禁与交付

文件：Linux/五笔用户页及英文对应页、Linux 工程说明、crate-notes、待办和设计状态。

- [x] 更新支持范围、切换方法、热更新及未覆盖环境，保留字根图等已知限制。
- [x] 运行 `cargo fmt --all --check`、相关 crate clippy/test 与非 macOS workspace 门禁；脚本通过 ShellCheck warning 门槛。
- [x] 自行审查最终 diff；补充自制码表嵌套路径监听的失败 / 通过证据，不修复无关依赖失败。
- [x] 记录实际测试命令、系统与应用，交付修改和未验证清单；不自动提交或发布。
