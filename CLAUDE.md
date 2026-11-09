# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 仓库现状

跨平台输入法，Core 平台无关，各平台只做壳。已发版 0.1.3（macOS 与 Windows 共用版本号，2026-09-17）；Linux 提供 Rust Server + Fcitx5 默认候选面板（源码安装，0.1.4 起另有 Ubuntu 26.04 预编译包），另有 Ubuntu 22.04 的 Fcitx4 源码适配。
阶段与已完成项见 `docs/plan/roadmap.md`，待办见 `docs/plan/todo.md`。

## 目录地图

一行一个，只说它是什么、入口在哪；实现要点（数据文件、常数、生成命令）在 `docs/notes/crate-notes.md`，改了实现要同步那里。

- `crates/qingjian-core`：引擎。`Engine` 是对外唯一门面，`Translator` / `Learner` 等 trait 在 `engine` 模块；拼音解析、纠错、候选、排序、整句、双拼、注音、九宫格（`nine_key`）、英文模式、繁体输出、emoji 与符号都在这里。
- `crates/qingjian-dictionary`：词库（TSV 或 `.qj` mmap），按音节位置二分查询。
- `crates/qingjian-translate`：释义表 `Glossary`、词汇等级表 `LevelTable`。
- `crates/qingjian-learning`：用户侧落盘：词频 / 用户词 / 个人 n-gram / 敲错表（`FrequencyLearner`）、输入日志（`InputLog`）、输入统计（`UsageStats`）、词汇记录（`VocabularyBook`）。
- `crates/qingjian-predict`：云联想 `CloudPredictor`（OpenAI 兼容接口）与释义兜底 `CloudGlossFiller`；`PredictConfig` 是 `[predict]` 分节。
- `crates/qingjian-voice`：云端语音识别，实现 Core 的 `Transcriber`；现在是火山引擎豆包流式语音识别 `VolcengineTranscriber`（WebSocket + rustls，后台线程）。
- `crates/qingjian-lm`：整句转换的 bigram 语言模型 `BigramModel`，另有读知意 `zh.ngram` 的 `TrigramModel` 与插值用的 `MixedModel`。
- `crates/qingjian-neural`：本地神经模型推理，`hanzhang/`（含章字级 Transformer：`CharScorer` 打分、`P2c` 按拼音生成整句）与 `zhiyi/`（知意选词模型）。
- `crates/qingjian-format`：`.qj` 数据容器（mmap 读、零拷贝视图、写入器、哈希索引）。
- `crates/qingjian-platform`：平台层共用：`Config`（TOML 配置）、`extra_dictionaries`、`protocol` 线上类型（Windows Server ↔ DLL，以及 `protocol::linux` 的 Server ↔ 各前端）。
- `crates/qingjian-render`：候选窗与状态条：帧数据 + 主题 → 场景树，画在 bamboo-slip 上，各平台只贴图。见 `docs/design/rendering.md`。
- `crates/bamboo-slip`：自绘 UI 引擎（场景树、布局、文字、动画 → 位图，另有控件库与窗口宿主），候选窗与设置程序共用；不放任何青简的概念，约定见该目录的 `CLAUDE.md`。双许可，见该目录的 `LICENSE.md`。
- `crates/bamboo-slip-i18n`：界面文案多语言：各 crate 的 `locales/*.toml` 编译时嵌入，`tr` 按进程级的界面语言查，回退英文再回退中文。双许可，见该目录的 `LICENSE.md`。
- `crates/qingjian-update`：检查更新：读官网的 `releases.json`、验 ed25519 签名、按平台与渠道挑新版本，只提示不安装；发版侧的签名工具是 `tools/release-sign`。见 `docs/design/update.md`。
- `crates/qingjian-code`：认出一段文字像不像代码并切高亮片段，输入法与剪贴板共用；core 里转出为 `qingjian_core::code`。
- `crates/qingjian-clipboard`：剪贴板历史存储（SQLite，留好同步要用的字段）与面板位置，平台无关，不放窗口与界面。
- `crates/qingjian-clipboard-sync`：剪贴板的设备间同步：配对、加密通道、合并、发现与扫描，不依赖具体存储；Windows 与 Android 共用。
- `apps/settings`：设置程序（三平台共用，画在 bamboo-slip 上，读写同一份配置文件），页面与设置项按 `docs/design/settings.md`；macOS 装在 `Qingjian.app/Contents/Applications/`，Windows、Linux 与 Server 同目录。`--png <目录>` 拷一份配置按脚本截图。
- `apps/clipboard`：青简剪贴板（独立进程，Windows 与 macOS）：监听复制、全局快捷键呼出卡片面板、粘贴回原窗口，画在 bamboo-slip 上；系统相关的在 `src/system/{windows,macos}/`，霓虹灯的节奏与像素在共用的 `src/system/neon/`。macOS 版是输入法包里的 `QingjianClipboard.app`，由输入法按配置拉起。
- `apps/cli`：Core 的验证工具：查询、逐键计时、输入日志回放、整句评测、常数扫描。排序 / 整句 / 纠错的改动先跑它再合。
- `apps/macos`：IMK 壳，按 `app / host / imk / candidates / menubar / preferences` 分目录（`preferences` 是旧版设置，新版 `apps/settings` 为主、旧版只作兜底）；`scripts/bundle.sh --install` 装到本机，`--pkg` 出分发包。
- `apps/windows`：`server`（Server 进程：Engine + IPC + 自绘候选窗与状态条）+ `tsf`（TSF DLL）+ `settings`（旧版设置，WinUI 3，只作兜底，新版 `apps/settings` 为主）+ `installer`（Inno）。DLL 不能带 Engine 的依赖树，所以是两个 package。
- `apps/linux`：`server`（Engine + Unix socket）+ `fcitx5` / `fcitx4`（框架事件、默认面板）+ `ibus`（第二个前端：IBus，按键 / 焦点 / 面板已通，对着真 daemon 验过）+ `scripts`（用户级安装卸载、`package.sh` 打 Fcitx5 预编译包）+ `packaging`（`build.sh` 在容器里打 deb / rpm，见 `docs/notes/linux-packaging.md`）+ `appimage`（`build.sh` 在 Debian 13 容器里打 AppImage，`test/test.sh` 各发行版容器实测，见 `docs/notes/linux-appimage.md`）；Server 手动启动或装成 systemd 用户服务，见 `docs/notes/linux-fcitx5.md` 和 `docs/notes/linux-fcitx4.md`。
- `apps/android`：`core`（Rust crate：Engine 装配 + `Session` + JNI，编成 `libqingjian_android.so`）+ `app`（Kotlin `InputMethodService` 壳，自绘键盘，候选条贴渲染器位图）；Gradle 构建时自动跑 `cargo ndk`，见 `apps/android/README.md`。
- `tools/dict-convert`、`tools/gloss-gen`、`tools/corpus`：产品数据生成（词库 / 语言模型 / 释义表 / emoji / 英文词表），输出到 `data/generated/`（gitignore）。
- `assets/`：随包数据源与样例，各目录有 README 写来源与许可。雾凇拼音（GPL）已彻底移除，不要再引入。

`docs/` 分四类（索引在 `docs/README.md`）：`design/` 设计与决定、`plan/` 路线与待办、`notes/` 工程记录（性能、复盘、踩坑、crate 实现要点）、
`user/` 用户文档（官网构建时拉取渲染，约定见 `docs/user/README.md`，措辞面向用户、不出现实现词）。

## 常用命令

```bash
cargo build                                   # 整个 workspace
cargo test                                    # 全部测试；-p <crate> 单个，加测试名过滤
cargo clippy --all-targets -- -D warnings
cargo fmt --all
cargo run -p qingjian-cli -- <拼音>...          # Core 的主要验证方式；--replay / --eval-text 见 crate-notes
apps/macos/scripts/bundle.sh --install        # mac 壳装到 ~/Library/Input Methods/（IMK 不能 cargo run 验证）
```

Windows 本机只 `cargo check --target x86_64-pc-windows-gnu`，真编译与真机测试在 Windows 机器上做（部署方式见 `apps/windows/README.md`）；
端到端验证 mac 可用 `osascript` 往 TextEdit 发按键再读回文本。

## 约定

架构约束、代码组织、版本号、提交信息、文档同步、提交前检查与发版都在 `docs/contributing.md`，随本文件一起载入：

@docs/contributing.md

交流用中文。
