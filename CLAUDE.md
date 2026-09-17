# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 仓库现状

跨平台输入法，Core 平台无关，各平台只做壳。已发版 0.1.3（macOS 与 Windows 共用版本号，2026-09-17）；Linux 壳开发中（Fcitx5，本机日用验证阶段，未发版）。
阶段与已完成项见 `docs/plan/roadmap.md`，待办见 `docs/plan/todo.md`。

## 目录地图

一行一个，只说它是什么、入口在哪；实现要点（数据文件、常数、生成命令）在 `docs/notes/crate-notes.md`，改了实现要同步那里。

- `crates/qingjian-core`：引擎。`Engine` 是对外唯一门面，`Translator` / `Learner` 等 trait 在 `engine` 模块；拼音解析、纠错、候选、排序、整句、双拼、注音、英文模式都在这里。
- `crates/qingjian-dictionary`：词库（TSV 或 `.qj` mmap），按音节位置二分查询。
- `crates/qingjian-translate`：释义表 `Glossary`、词汇等级表 `LevelTable`。
- `crates/qingjian-learning`：用户侧落盘：词频 / 用户词 / 个人 n-gram / 敲错表（`FrequencyLearner`）、输入日志（`InputLog`）、输入统计（`UsageStats`）、词汇记录（`VocabularyBook`）。
- `crates/qingjian-predict`：云联想 `CloudPredictor`（OpenAI 兼容接口）与释义兜底 `CloudGlossFiller`；`PredictConfig` 是 `[predict]` 分节。
- `crates/qingjian-lm`：整句转换的 bigram 语言模型 `BigramModel`。
- `crates/qingjian-neural`：字级 Transformer 本地推理 `CharScorer`（candle），给整句前几条路径重打分。
- `crates/qingjian-format`：`.qj` 数据容器（mmap 读、零拷贝视图、写入器、哈希索引）。
- `crates/qingjian-platform`：平台层共用：`Config`（TOML 配置）、`extra_dictionaries`、Windows Server ↔ DLL 的 `protocol` 类型。
- `crates/qingjian-render`：自绘渲染器（spike 中，分支 renderer-spike）：候选窗一帧 + 主题 → 位图，各平台只贴图。见 `docs/design/rendering.md`。
- `apps/cli`：Core 的验证工具：查询、逐键计时、输入日志回放、整句评测、常数扫描。排序 / 整句 / 纠错的改动先跑它再合。
- `apps/macos`：IMK 壳，按 `app / host / imk / candidates / menubar / preferences` 分目录；`scripts/bundle.sh --install` 装到本机，`--pkg` 出分发包。
- `apps/linux`：Fcitx5 壳：`fcitx5-shim`（薄 C++ 插件，每件事一两行转发）+ `host`（Rust staticlib：Engine 装配与键路由），同进程 C ABI 边界；`install.sh` 用户级安装（双模式），`pack.sh` 出自包含 tarball。设计见 `docs/design/linux-fcitx5.md`。
- `apps/windows`：`server`（Server 进程：Engine + IPC + 自绘候选窗与状态条）+ `tsf`（TSF DLL）+ `settings`（WinUI 3）+ `installer`（Inno）。DLL 不能带 Engine 的依赖树，所以是两个 package。
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
