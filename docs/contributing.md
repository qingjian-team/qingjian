# 开发约定

改代码前先看这一页：架构上不能越的线、代码怎么组织、版本号与提交信息怎么写、改了行为要同步哪些文档。
设计的来龙去脉在 [design/architecture.md](design/architecture.md)，各 crate 的实现要点在 [notes/crate-notes.md](notes/crate-notes.md)。

## 架构约束（不要违反）

长版与理由见 [design/architecture.md](design/architecture.md)。

- **Core 与平台层严格解耦。** `qingjian-core` 及其兄弟 crate 必须平台无关：词库、拼音解析、候选生成、排序、学习、翻译、文本变换全部属于 Core。
  平台层（IMK / TSF / IBus-Fcitx）只做两件事：把系统输入事件翻译成 Core 的输入，把 Core 返回的帧画到候选窗口。
  **平台层里不允许出现排序逻辑、词库访问、翻译调用或文本变换。** 判断标准：把 IMK 换成 TSF，不应该需要改 Core 的任何一行。
- **一个候选词只显示一种辅助语言。** 用户配置 Primary Language + 单个 Learning Language。不要设计成 `translations: Vec<Translation>` 或
  `HashMap<Lang, String>` 这类多语言并列的数据结构，那会在 API 层面把「一次只学一种语言」这条产品原则给破坏掉。翻译是候选词的 annotation（可选、单条）。
- **输入优先于学习。** 任何为学习功能增加的延迟、弹窗、UI 干扰都是设计错误。翻译查询不能阻塞候选生成，Core 必须能在翻译尚未就绪时先返回候选。
- **输入方案是配置项，不是模式。** 双拼、注音这类键盘方案放 `[general]` 里当设置，中 / 英切换始终是布尔；新方案不能改变别的方案的既定按键行为（[user/getting-started/keys.md](user/getting-started/keys.md)）。
- **显示面自绘、控件面原生。** 候选窗、拼音行、状态条这类显示面由渲染器出位图各平台贴图（主题靠它）；偏好设置、菜单、安装器用各平台原生控件。见 [design/rendering.md](design/rendering.md)。

## 代码组织

- **一个类型一个文件。** 一个 struct / enum / trait 及其 impl 单独一个文件；模块文件只做 `mod` 声明、re-export 与自由函数，不把一个 crate 平铺在 `lib.rs` 里。
- **子模块用目录。** 有子模块的模块用 `foo/mod.rs`，**不用** `foo.rs` + `foo/` 并列。
- **同词干的兄弟文件收进目录，绝不用文件名前缀分组。** `key_event.rs` + `key_outcome.rs` → `key/mod.rs` + `key/{event,outcome}.rs`，哪怕没有 `key.rs` 这个共同父文件；
  `query/` + `querying.rs` 这种也不行。判断：两个及以上文件名共享一段前缀且同属一个概念，就收进以那段前缀命名的目录。
- **一个职责连带它专用的类型收进一个目录。** `engine/commit/mod.rs` 放上屏部分，`chain.rs` / `last.rs` 放只有它用的类型。
- **大类型的 `impl` 按职责拆成子模块。** 每个文件一个 `impl Foo { … }`（`host/settings.rs` 这样），结构体与构造留在 `mod.rs`，跨文件用到的私有方法标 `pub(super)`。
- **文件长度。** 单文件不超过 800 行，目标 500 行以内；测试超过 200 行搬到 `tests.rs`（多时 `tests/` 按主题分文件）。
- **导入写精确路径。** 不用 `use super::*` / `use foo::*`（`#[cfg(test)] mod tests` 里的 `use super::*` 除外）。
  子文件从父模块拿的类型写 `use super::{Engine, Candidate};`，父模块没定义的直接 `use crate::…` 或 `use <crate>::…`，不绕 `super::` 转手。
  存量的 glob 改到那个文件时顺手换掉，不专门清扫。

## 命名与注释

- 代码标识符一律英文，注释与文档用中文；`thiserror` 的 `#[error]` 文案用英文，日志与 UI 文案用中文。
- 新文件都要有 `//!` 文件头；结构体 / 枚举字段之间空一行，字段名说不清的加 `///`（`r` / `g` / `b`、`width` 这类不用，`a`「255 为不透明」这类要）。
- 注释只写维护时用得上的：文件头一两句说它是什么、怎么用；代码里只解释读代码看不出来的约束与原因（平台怪癖、协议约定、踩过的坑），一两句说完。
  不写复述代码的教学性注释，不写改动经过、调试过程这类日志型注释；较长的设计理由写进 `docs/`，注释里留一句指过去。
- 不写装饰性分隔注释（`// ====`），提交钩子会拦。

## 依赖与配置

- 依赖用 `cargo add` 加，共用包提到根 `[workspace.dependencies]`；错误用 `thiserror` 不用 `anyhow`；日志用 `tracing` 门面。
- 快捷键一律进 `[shortcut]` 可配置，不写死键码。

## 版本号

- `crates/*` 用 `version.workspace = true`；**`apps/*` 各壳是独立发布的产品，写死自己的 `version`**（Windows 读 `server/Cargo.toml`）。
- 发版之间带 `-dev`（两端都是 `0.1.3-dev`），打包脚本再接 git 短哈希成 `0.1.3-dev-1a2b3c4`（脏加 `+`，Cargo.toml 里只写 `-dev`）。
- 发版提交去掉 `-dev` 打标签 `v<版本>`（三个平台共用一个 Release；单平台补丁用 `macos-v` / `windows-v` / `linux-v<版本>`），标签后再改成下一个 `-dev`；带 `-dev` 的标签 CI 拒绝；pkg / Inno 只认数字点号。

## 提交信息

- [Conventional Commits](https://www.conventionalcommits.org/zh-hans/)：第一行 `<类型>(<范围>): <说明>`，类型与范围英文小写，说明用中文，例如
  `fix(core): 修自绘输入框吞数字`、`feat(windows): 三进程日志统一到 %LOCALAPPDATA%\Qingjian\logs`、`docs(changelog): 补 0.1.3 条目`。
  - 类型：`feat` 新功能 / `fix` 修 bug / `docs` 只改文档 / `refactor` 不改行为的整理 / `perf` 性能 / `test` 只改测试 /
    `build` 打包与构建脚本 / `ci` 工作流 / `chore` 版本号、依赖、仓库杂务 / `style` 只改格式 / `revert` 还原。
  - 范围：crate 或壳的名字——`core` `platform` `render` `dictionary` `translate` `learning` `predict` `lm` `neural` `format` `cli`
    `macos` `windows`（Server / DLL / 设置程序细分时用 `server` `tsf` `settings`）`installer` `linux` `tools` `docs` `ci` `deps` `release`；
    跨好几处的可以省略。不兼容的改动在范围后加 `!`。
  - 正文写「为什么」与取舍，一行一条；不加 AI 署名。`.githooks/commit-msg` 会拦第一行不合格式的提交。
  - 2026-09-16 之前的历史是「`macOS：……` / `Core：……`」的中文冒号格式，不重写。

## 文档同步

- 实现与规划分歧时以代码为准并改文档。技术方向写 `docs/design/`，计划写 `docs/plan/`，工程记录写 `docs/notes/`；不往 README 里加技术内容。
- **用户能感知的行为改了（按键、菜单、偏好设置、配置文件、数据文件），同一个提交里改 `docs/user/` 对应的页**，按键改动同时改 `keys.md`。
- 改了某个 crate 的实现要点（数据文件、常数、生成命令），同步 `docs/notes/crate-notes.md`。

## 提交前检查

- 钩子：`.githooks/pre-commit`（禁装饰性分隔注释 + fmt + clippy）、`.githooks/commit-msg`（提交信息格式）、`.githooks/pre-push`（全 workspace 测试）；`git config core.hooksPath .githooks` 启用一次。
  两个跑编译的钩子按 `uname` 划平台范围：**非 Apple 平台排除 `qingjian-macos`**（IMK 壳依赖 objc2，在别的平台上是硬 `compile_error!`，
  排除不掉就整条命令失败），与 [ci.yml](../.github/workflows/ci.yml) 三个 job 的划分一致；Windows 上 pre-push 另设 `QINGJIAN_UIACCESS=0`
  （Server 的 build.rs 嵌 uiAccess manifest，没签名的测试二进制起不来，os error 740）。
- 排序 / 整句 / 纠错的改动先跑 `apps/cli` 再合。
- **结构与协作门禁**：`python3 scripts/gates/gate.py --fast`（pre-commit 同款，秒级、不需 cargo）跑
  上帝对象规模棘轮 + 欠账台账对账 + 雷同代码 + 产品代码坏味道 + 多智能体认领 + 门禁自检；
  合入前跑 `python3 scripts/gates/gate.py`（多跑 fmt / clippy / 全量测试）。说明见 [GATES.md](GATES.md)。
- **产品代码不许有这些坏味道**（全是棘轮，只准减）：`.unwrap()` / `.expect()` 系列（该用 `?`）、
  单行 > 200 字符、`thread::sleep`（该用 channel / 条件变量）、`let _ = …` 吞掉 `Result`、
  非 FFI 代码里的 `unsafe` 块、函数圈复杂度 > 15、嵌套深度 > 5（> 8 硬禁）、
  `println!`/`eprintln!`（该用 `tracing`；`apps/cli` 与 `tools/` 除外）、trait 方法数 > 15（> 40 硬禁）。
  测试面（`tests/` `examples/` `benches/`）不算产品代码，不进这些门的口径。
- **存量 = 0 的规矩直接硬判**（没有基线，不许被 `--write` 祖父化）：函数形参 > 7、
  同目录下用文件名前缀分组（`foo_a.rs` + `foo_b.rs` ⇒ 收进 `foo/`）、`crates/*` 必须
  `version.workspace = true`、各壳必须写死自己的 `version`、全仓不许 `anyhow`、
  不许 `Vec<Translation>` / `HashMap<Lang, _>` 这类多语言并列结构（一次只学一种语言）、
  标识符一律英文且 `#[error]` 文案用英文。
- **另有两道棘轮**：`use super::super::…` 绕父模块转手（该直接 `use crate::…`）、
  单文件内嵌测试 > 200 行（该搬到 `tests.rs`）。
- **不许有上帝对象**：单文件 ≤800 行、最长函数 ≤100 行、最大类型 ≤20 成员；另有一个类型所有
  `impl` 加起来方法 ≤40、散在文件 ≤8（按类型名聚合，拆子模块后每个文件都很小也照样抓得到）。
  全是棘轮（只准减，涨了就红）。存量欠账在 `docs/review/god-debt.md`，谁改到谁认领、顺手减；
  拆完跑 `gate.py --write` 重记基线（**要 git diff 过目**：基线变松 = 门变松）。
  **碰了就得减**：改动已超阈的欠账文件时，必须把它变小——棘轮只管「不许变胖」，这条连
  「原样不动」都不放行，否则存量能永远躺着没人碰。
- **架构约束同样有门**：新文件必须有 `//!` 文件头、一个文件一个类型、不用 `use …::*`、
  子模块用目录（`foo/mod.rs` 不与 `foo.rs` 并列）、`crates/*` 不许无条件依赖壳或 OS 特有 crate。
- **多个智能体并行**：动代码前先认领（`.agents/claims/<id>.json`，见 [`.agents/CLAIMS.md`](../.agents/CLAIMS.md)），
  域与他人重叠或改到别人域里 = 红。

## CI 与发版

- CI 三个 job（Linux 全量 / macOS 壳 / Windows 三 crate）都 `--locked`；Dependabot 升 actions；每周 `cargo audit`。
  另有 `gates.yml` 跑结构与协作门禁（上帝对象 / 架构约束 / 雷同代码 / 产品代码坏味道 /
  多智能体认领 / 门禁自检），
  并在 ci.yml 里挂了一道「门禁自检」——把 gates.yml 或门脚本删掉，ci.yml 那道先红。
- 发版：推 `<平台>-v<版本>` 标签触发 `release.yml`，门禁是版本号 = 标签且不带 -dev、标签在 main 上、产品数据按 SHA256SUMS 校验。
- CHANGELOG 手写、发版时由维护者统一改（PR 不动它）。流程与 Secrets 见 [notes/release.md](notes/release.md)。

## 外部 PR

- 从 main 开分支，一个 PR 只做一件事、只碰一个平台（Core 改动单独一个）。
- 维护者对着 main 审，squash 合并保留作者署名。PR 模板里的合并前清单就是审核标准。
- **修 bug、改壳里行为（按键、上屏、候选窗位置）的 PR 必须真机验过**：先在自己机器上复现问题，改完在同一个应用里确认修好，
  PR 的「怎么验证的」写明系统版本、应用与操作步骤。编译与 CI 通过不算验证。没验过的修复发出去，报 issue 的人升级后还得再报一次。
  复现不了的（没有那个应用或系统）不提修复：把分析写在 issue 里，或者提只加日志、不改行为的 PR。
  不改行为的改动（日志、注释、文档）与有测试 / 回放兜底的 Core 逻辑不受此限。
- 主题与自绘渲染器（`crates/qingjian-render`、各壳的贴图路径、主题文件）还在测试，这部分暂不接受 PR；稳定一版后再开。


## CI 与发版

- CI 三个 job（Linux 全量 / macOS 壳 / Windows 三 crate）都 `--locked`；Dependabot 升 actions；每周 `cargo audit`。
- 发版：推 `<平台>-v<版本>` 标签触发 `release.yml`，门禁是版本号 = 标签且不带 -dev、标签在 main 上、产品数据按 SHA256SUMS 校验。
- CHANGELOG 手写、发版时由维护者统一改（PR 不动它）。流程与 Secrets 见 [notes/release.md](notes/release.md)。

## 外部 PR

- 从 main 开分支，一个 PR 只做一件事、只碰一个平台（Core 改动单独一个）。
- 维护者对着 main 审，squash 合并保留作者署名。PR 模板里的合并前清单就是审核标准。
- **修 bug、改壳里行为（按键、上屏、候选窗位置）的 PR 必须真机验过**：先在自己机器上复现问题，改完在同一个应用里确认修好，
  PR 的「怎么验证的」写明系统版本、应用与操作步骤。编译与 CI 通过不算验证。没验过的修复发出去，报 issue 的人升级后还得再报一次。
  复现不了的（没有那个应用或系统）不提修复：把分析写在 issue 里，或者提只加日志、不改行为的 PR。
  不改行为的改动（日志、注释、文档）与有测试 / 回放兜底的 Core 逻辑不受此限。
- 主题与自绘渲染器（`crates/qingjian-render`、各壳的贴图路径、主题文件）还在测试，这部分暂不接受 PR；稳定一版后再开。
