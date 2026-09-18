# Linux Fcitx5 第一阶段修改方案（PR #90）

状态：已完成第一阶段代码与文档实现，定向测试、完整本地检查、声明范围的桌面输入验证及独立代码审查均已通过。原始自绘、GNOME、打包工作保留在备份分支，不属于本阶段交付。

依据：[维护者对 PR #90 的回复](https://github.com/qingjian-team/qingjian/pull/90#issuecomment-5718766925)（2026-09-18，北京时间）。#90 承载第一阶段，后续自绘、GNOME 与发布工作按下述边界拆分；实际验证范围见 [Linux 工程说明](../notes/linux-fcitx5.md)。

## 1. 交付目标与范围

将 #90 收敛为可独立安装、运行、验证的 Linux 最小输入链路：

```text
应用输入 → Fcitx5 薄插件 → Unix socket → Rust Server / Engine
应用上屏 ← Fcitx5 薄插件 ← 上屏结果、预编辑、候选内容
                         ↓
                  Fcitx5 自带候选面板
```

| 本阶段保留 | 本阶段移出或暂缓 |
|---|---|
| Rust Server、必要的配置与资源加载、Unix socket 会话 | Server 公共 crate 抽取，由维护者在合并后处理 |
| Fcitx5 插件、默认预编辑与候选面板、点击与按键转发 | 插件内渲染、X11 / XWayland 自绘、GNOME 扩展、render-ffi |
| 普通输入、现有本地候选与学习能力、隐私边界 | 神经模型重排、模型加载后的当轮重排，由 #100 贡献者后续补充 |
| 用户级安装、卸载、手动启动说明 | Debian 打包、服务自动启动、桌面会话管理、旧安装迁移 |
| Linux 构建、输入链路测试与必要文档 | 自绘探针、上游补丁、审查流水与原始验收产物 |

不新增云服务接入、设置界面或排序算法。保留已有输入能力时，不顺带扩展功能。共享 crate 仅保留接通 Linux 必需的适配，不携带无关 Core 或渲染改动。

维护者明确要求第一阶段使用 Fcitx5 默认面板，这是本阶段对仓库长期自绘方向的具体安排。原 GNOME 与双 UI 方案中超出本阶段的工作后移，不能继续以原先“全部阶段完成”为 #90 的验收条件。

## 2. 分支与拆分方式

1. 保存当前完整实现到独立备份分支，确保后续自绘与打包工作可以找回。
2. 获取最新上游，从 `origin/main` 建立干净的第一阶段工作分支，按职责提取需要的文件与改动。
3. 分别核对当前本地主线与 #90 远端分支：两边存在不同的后续修复，不能只取其中一个分支，也不能直接搬入包含全部阶段的大提交。
4. 保留仍适用的上游兼容修复，例如配置访问方式、候选帧字段；核对 PR 分支已有的 CI 依赖与无会话总线修复，仅带入第一阶段仍使用的部分。
5. 以相对最新上游的最终差异为审查对象，确认没有混入已在上游存在的改动，也没有移除上游原有功能。

本阶段继续使用 #90 承载最小输入链路；后续自绘、GNOME、打包分别提交 PR。备份分支不合回第一阶段分支。

## 3. 按顺序修改

### 3.1 裁剪构建、配置和显示路径

| 位置 | 修改要求 |
|---|---|
| `apps/linux/fcitx5/CMakeLists.txt` | 只编译插件、IPC、按键映射、默认候选 UI 和对应测试；移除本次引入的自绘开关、源文件、FFI 与 XCB 链接，以及 GNOME / Wayland 探针目标 |
| `apps/linux/fcitx5/src/qingjian.{h,cpp}`、`session.h` | 移除自绘控制器、后端选择、GNOME bridge、外观订阅和仅服务于自绘的 watcher；保留默认面板所需事件处理 |
| `apps/linux/fcitx5/src/panel/` | 自绘实现移出第一阶段；若其中有默认面板仍需的通用类型，只提取确实使用的最小部分 |
| `apps/linux/render-ffi/`、`apps/linux/gnome/`、`apps/linux/probes/`、`apps/linux/upstream/` | 从第一阶段交付中移出；依赖 Fcitx5 本地补丁的功能同时撤出，不能仅删除补丁文件 |
| 根 `Cargo.toml`、`Cargo.lock` | 移除本次新增的 render-ffi workspace 条目与不再使用的依赖，重新生成一致的锁文件；保留上游已有渲染 crate |
| `crates/qingjian-platform/src/config/linux_ui/` 及引用 | 撤出仅用于 Linux 自绘的 renderer、UI 缩放等配置、导出、模板与测试；保留基本输入配置及默认面板需要的预编辑设置 |
| `apps/linux/server/src/dispatch/config.rs`、`dispatch/display.rs`、`protocol/` | 收缩 Linux 显示扩展，移除自绘协商字段；保留会话校验，以及默认面板仍需要的最小显示反馈 |
| `crates/qingjian-render/` 等共享目录 | 本次 PR 为后续自绘新增的改动后移，不删除上游已有代码 |

完成后，插件构建不需要 render-ffi archive，也不直接链接 Rust 渲染器、字体栈或自绘 XCB 后端。Fcitx5 自身的系统依赖不属于本项目自绘依赖。

### 3.2 明确插件与 Server 的职责

| 插件负责 | Server / Engine 负责 |
|---|---|
| 转换 Fcitx5 按键、修饰键、必要的按下/释放信息和上下文事件 | 输入模式、快捷键、组句、候选选择、翻页与提交决策 |
| 报告上下文能力与焦点事实 | 根据能力决定私密、禁用及会话清理行为 |
| 建立连接、校验消息长度和会话标识、处理连接失败 | 会话状态、配置、资源、候选生成与学习 |
| 将返回内容映射到 Fcitx5 预编辑、候选列表和上屏 API | 决定候选内容、辅助语言、提交文本与按键是否消费 |
| 转发候选点击、翻页请求、框架是否已提交预编辑等事实 | 解释这些事件，防止重复提交与跨上下文应用旧结果 |

重点检查插件现有 Shift 处理、候选回调和 `commitRaw` / `deactivate` 路径。插件可以保留框架适配与协议防御，不能通过移动函数或改名继续保留业务策略。

协议需要补充能力信息时，优先使用小范围、版本明确的 Linux 扩展；不要为了 Linux 单方面改变 Windows 现有消息格式。版本不匹配或连接失败时应清理候选与预编辑并放行输入，不重复提交旧文本。

默认候选的展示反馈不能误删：如果学习依赖实际交给面板展示的释义，应只报告当前会话、当前页中实际提供的内容。隐藏面板、失焦、私密会话和过期响应不能产生有效展示记录；不要把 Server 生成过的全部候选直接视为已展示。

### 3.3 修复 capability 与隐私行为

入口：`apps/linux/fcitx5/src/qingjian.cpp` 的 `syncPrivacy`，以及 Server 的会话与消息分派。

| 能力状态 | 目标行为 |
|---|---|
| capability 全空 | 普通应用，正常组句与候选；学习和输入日志遵循用户配置 |
| 普通非空能力，无下列标记 | 同普通应用 |
| Sensitive | 私密输入，可保留组句；不记录输入文本，不更新词频、个人 n-gram 等学习数据 |
| Password 或 Disable | 禁用输入法处理，交还应用；丢弃旧组句与显示状态，不学习、不补交旧文本 |
| 多个标记同时存在 | Password / Disable 的禁用行为优先 |

删除 `caps == CapabilityFlags()` 导致私密的判断，并调整 `shift-unknown` 等现有测试预期。能力转换放在插件，业务判定收敛到 Server；框架因 Password / Disable 停止交付输入时，不额外采集或补发按键。

保留两种不同边界：

- 同一会话的隐私能力发生变化：丢弃旧组句、暂存透传文本、补全与学习链，旧响应不可再次上屏。
- 普通与私密上下文之间切换焦点：按各自状态恢复或按既有框架规则结束输入，不能把一方的文本、学习状态带到另一方。

重点回归 `apps/linux/server/tests/privacy.rs`、`router.rs` 和插件 `tests/lifecycle.cpp`。保留 capability 变化触发 deactivate 时的清理，避免在框架仍暴露旧能力的一瞬间补交文本。

### 3.4 修复 Tab，并核对三端一致性

入口：`apps/linux/server/src/dispatch/key/input.rs`。目标采用现有 macOS 行为：

| 场景 | 行为 |
|---|---|
| 未组句 | Tab / Shift+Tab 交还应用 |
| 英文组句 + Tab | 提交当前高亮候选 |
| 中文组句 + Tab，有整句补全 | 接受整句补全 |
| 中文组句 + Tab，无整句补全 | 下一页，不按单个候选移动高亮 |
| 组句中 Shift+Tab | 上一页，不接受整句补全；与 macOS `insertBacktab:` 一致 |
| PageUp / PageDown | 保持原有上一页 / 下一页行为 |

复用现有 `page` 路径，处理首页、末页、不足一页、空候选、不同 page size 和翻页后的选择索引；保留裸问号等已有特殊输入规则。改动后注释与行为必须一致。

macOS 已在无整句补全时翻页，Linux 在本阶段对齐。Windows 对应逻辑、TSF 注释、回归测试和用户说明通过独立 PR 修复，#90 仅包含 Linux 平台改动。两个 PR 分别基于主线，Linux 输入链路不依赖 Windows 修复；公共 dispatch 抽取仍由维护者后续处理。

Linux 当前没有接入云补全服务，整句补全分支通过可控测试状态验证，不为了测试增加云服务功能。按键变化同一提交更新 `docs/user/getting-started/keys.md`。

### 3.5 简化用户级安装与卸载

修改 `apps/linux/scripts/install.sh`、`uninstall.sh`，并同步默认安装路径与插件注册配置。

- 只构建并安装 Rust Server、Fcitx5 插件、注册文件、图标和运行所需资源；保留必要的数据完整性检查。
- 在用户目录安装，不修改系统 Fcitx5，不应用补丁；确保 Fcitx5 能从安装后的配置找到插件库。
- 提供明确的手动启动 Server 和在 Fcitx5 中添加青简的步骤；首次安装即可按文档完成一次输入。
- 移除 `--gnome`、`--startup` 等后续阶段选项，以及其构建、复制和调用路径，避免保留无效参数。
- `apps/linux/management/`、`scripts/deploy.py`、`scripts/package-deb.sh`、`packaging/` 与启动模板移出第一阶段；仅复用基本安装确实需要的小段逻辑，不带入迁移与服务管理框架。
- 重复安装结果可预测；卸载只移除本安装拥有的文件，保留用户配置、词库与学习数据，不影响其他输入法。

自动启动、服务重启策略、linger、会话环境导入、代际回滚与旧包迁移留到第四阶段。第一阶段只要求手动启动后的完整输入链路可用。

### 3.6 收缩 CI 与测试依赖

修改 `.github/workflows/ci.yml`：

1. 保留 Linux Rust fmt、clippy、workspace 测试，以及插件构建和默认面板 CTest；Rust 命令继续使用仓库规定的 `--locked`。
2. 删除 render-ffi、XCB 自绘、独立 Wayland 探针、GNOME JS、服务管理与迁移的专属检查。
3. 按保留的目标精简 apt 依赖，复核 Fcitx5 最低版本与 runner。只声明在未打补丁的发行版 Fcitx5 上验证过的范围。
4. 若仍保留真实 Fcitx5 DBus/frontend 测试，显式安装其运行模块，包括适用的 `fcitx5-modules`；不能只装开发包。
5. 默认插件初始化必须支持没有桌面会话总线的环境；确需 D-Bus 的测试通过 `dbus-run-session` 创建独立总线。移出的 GNOME 测试无需为第一阶段保留依赖。
6. 保留上游 macOS / Windows 检查，三端 Tab 的平台验证由对应 runner 承担；不因 Linux 拆分删除现有平台任务。

旧版本 CI 通过不能替代裁剪后新提交的检查结果。

### 3.7 清理并同步文档

- 从本次提交移出 `docs/notes/` 新增的 coder/reviewer 记录、截图、CSV、JSON、日志和原始验收目录。验证输出保存在本地或 CI artifacts，PR 正文只写必要结论与复现步骤。
- 保留并精简 `docs/notes/linux-fcitx5.md` 等可持续使用的构建、协议和排错说明；不清空整个 `docs/notes/`，不删除上游原有工程文档。
- 更新 `docs/user/getting-started/linux.md`：只介绍默认面板、实际支持的环境、用户级安装、手动启动与卸载。
- 同步 `CLAUDE.md` 的 Linux 目录地图、`docs/README.md` 索引，以及实际涉及的 roadmap、todo、crate-notes，避免继续描述已移出的模块为当前交付。
- 原自绘与 GNOME 方案后移并收缩；若保留未来架构说明，必须改成 Server 渲染与管理 X11 窗口、Server 向 GNOME 扩展发送位图，避免继续指导在 Fcitx5 进程中渲染。
- 不修改 `CHANGELOG.md`，不加入新的审查流水文档。

## 4. 验收矩阵

| 验收项 | 通过条件 |
|---|---|
| 干净构建 | 无本地 Fcitx5 补丁、无 render-ffi archive、无 GNOME 扩展，仍可构建并加载插件 |
| 基本输入 | 中文输入、空格/数字选词、退格、取消、回车、现有中英切换正常；候选点击与键盘选择一致 |
| 预编辑 | 客户端内联与默认面板回退符合设置和能力；失焦只提交一次，没有丢字或重复上屏 |
| Tab / 翻页 | Linux 符合上表，首页/末页边界正确，翻页后选词对应当前页；Windows 差异由独立 PR 处理 |
| 空 capability | 输入、候选及配置允许的学习正常，不再被当作私密应用 |
| 隐私 | Sensitive 不学习、不记录文本；Password / Disable 放行；切换、关闭、断线不泄漏暂存内容 |
| 会话与故障 | 多输入框不串状态；Server 未启动、退出、重连与协议错误不挂住 Fcitx5，不重放旧提交 |
| 展示反馈 | 隐藏、失焦、私密和过期结果不计入有效展示；默认候选内容与记录一致 |
| 安装与卸载 | 干净用户环境可安装、手动启动并输入；重复安装可用；卸载保留用户数据及其他输入法 |
| 桌面验证 | 在未打补丁的支持环境验证默认面板；至少覆盖所声明支持的 GTK / Qt、X11 或 Wayland 路径，未测环境明确列出 |
| CI 与范围 | 当前提交相关检查通过，最终差异不含后续阶段源码、补丁和原始验收产物 |

先运行定向回归测试，再运行仓库要求的完整检查。裁剪完成后的 Linux 基本命令：

```bash
cargo test -p qingjian-linux-server --locked
cargo fmt --all --check
cargo clippy --workspace --exclude qingjian-macos --all-targets --locked -- -D warnings
cargo test --workspace --exclude qingjian-macos --locked
cmake -S apps/linux/fcitx5 -B target/fcitx5-stage1 -DCMAKE_BUILD_TYPE=Debug -DBUILD_TESTING=ON
cmake --build target/fcitx5-stage1 --parallel 2
ctest --test-dir target/fcitx5-stage1 --output-on-failure
bash -n apps/linux/scripts/install.sh apps/linux/scripts/uninstall.sh
git diff --check
```

以上命令面向完成裁剪后的结构，使用新的 CMake 构建目录，避免旧缓存掩盖依赖。Shell 语法检查不能替代安装/卸载实测。Windows 真机与编译测试交给 Windows 环境或 CI，Linux 本机检查通过不能记作 Windows 已验收。

## 5. 提交组织与完成清单

建议按可审查职责组织提交：最小链路与依赖裁剪、隐私修复、Tab 一致性修复、用户级安装、CI 与文档。行为变化对应的用户文档随该行为提交。提交信息遵循仓库 Conventional Commits，不将备份实现整批合回。

- [x] 当前完整实现已保留，第一阶段基于最新上游整理。
- [x] 默认面板输入链路独立可用，插件内不含自绘渲染路径。
- [x] 插件业务判断已收敛到 Server，协议与失效处理有回归覆盖。
- [x] 空 capability、Sensitive、Password、Disable 及切换行为符合方案。
- [x] Linux Tab / Shift+Tab 与翻页索引修复，Windows 修复拆为独立 PR。
- [x] 用户级安装与手动运行可用，卸载保留用户数据。
- [x] 后续阶段代码、Fcitx5 补丁、探针、审查流水和原始证据已移出本次差异。
- [x] 用户文档、目录说明、配置模板和 CI 与最终实现一致。
- [x] 定向回归、完整检查和声明支持环境的输入实测通过，未验证范围已列明。
- [x] 相对最新上游审阅最终 diff，未混入无关共享层或其他平台改动；Windows Tab 修复不进入 #90。

本地审查通过后，由贡献者另行更新 #90 的分支、标题和正文；本轮实现与验证不自动推送或修改外部 PR。建议标题：`feat(linux): 接入 Rust Server 与 Fcitx5 默认候选输入链路`。正文按仓库 PR 模板说明交付范围、隐私与 Tab 行为、安装方式、实际验证结果和后续拆分安排。

完成上述门槛后，#90 可进入正式审查；X11 自绘、GNOME 扩展与 Debian 发布验收不作为本阶段转为 Ready for review 的前置条件。是否合并仍由维护者审查决定。
