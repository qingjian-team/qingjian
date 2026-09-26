# 门禁（结构与协作）

`ci.yml` 管编译与测试，`quality.yml` 管依赖、秘密与仓库卫生，**这一组管结构与协作**：
不许有上帝对象、不许新增雷同代码、多个智能体并行时不许互相踩、门本身不许被悄悄削弱。

移植自 `D:/开发/unified-rx-mcp`（那边是一整套 Python 门禁），按本仓（Rust workspace、多智能体并行）
做了适配：门脚本全部**零依赖、纯 stdlib**，CI 上 `python3` 直接跑，不需要装任何东西。

一条命令：

```bash
python -X utf8 scripts/gates/gate.py --fast     # 快门（pre-commit 同款，秒级，不需 cargo）
python -X utf8 scripts/gates/gate.py            # 全门（含 fmt / clippy / 全量测试；合入前跑）
python -X utf8 scripts/gates/gate.py --list     # 看看都有哪几道
python -X utf8 scripts/gates/gate.py --only god-gate,dupe-gate
python -X utf8 scripts/gates/gate.py --write    # 拆完一块后重记基线（**要 git diff 过目**）
```

## 四、架构约束门（`arch_gate.py`）

把 `docs/contributing.md` 那一页「架构约束 / 代码组织」从写给人看变成机器判。
那些规矩每一条都写得很清楚，但没有一条有人或机器在查。

| 编号 | 规则 | 命中 |
| --- | --- | --- |
| R1 | 新 `.rs` 文件缺 `//!` 文件头 | 红（只管新增） |
| R2 | 新文件（非 `mod.rs` / 非测试）顶层类型 > 1 | 红（只管新增） |
| R3 | `use …::*` glob 导入 | 棘轮（`#[cfg(test)]` 与测试文件豁免） |
| R4 | `foo.rs` 与 `foo/` 并列 | 棘轮 |
| R5 | `crates/*` 无条件依赖 `apps/*` 的 crate 或 OS 特有 crate（objc2 / windows / gtk …） | **红（硬）** |
| R6 | 产品代码里的 `dbg!` / `todo!` / `unimplemented!` / `unreachable!` / 裸 `panic!` | 棘轮 |
| R7 | 装饰性分隔注释 `// ====` | 棘轮 |

R5 只查**无条件**的 `[dependencies]` 段：`[target.'cfg(windows)'.dependencies]` 是平台后端
（render 用 DirectWrite 列字族），crate 本身仍跨平台，把条件依赖也算违规会逼人改写成绕过。

R6 为什么单独扫一遍：workspace lints 已经 deny 这些，但 clippy 只跑编得到的 target——
macOS 壳在 Linux runner 上不编、Windows 三件套在 macOS runner 上不编 ⇒ 那部分文件的 `panic!`
谁也抓不到。这里是纯文本扫描，不看能不能编译，正好补上那个**平台盲区**。

## 五、重复代码门（`dupe_gate.py`）

上帝对象门管「单点过大」，这个管「多处雷同」——拆上帝对象时最容易顺手复制出一批雷同的
helper，所以两个门配套。

- 判据：文件对 Jaccard ≥ 0.80（bottom-k MinHash，ng=5 / k=128）即雷同；**基线里没有的新对 ⇒ 红**。
- **与上游的差异（重要）**：上游调用 Rust 侧 `rx-scan.exe` 出指纹，那个 exe 在本仓与 CI 上都没有，
  照抄会以「引擎不可用」把所有人卡死。这里改成**纯 stdlib** 实现（同口径），零依赖。
- 当前基线 9 对，全是 `apps/linux/server` 与 `apps/windows/server` 之间的复制——真欠账，不是误报。
  真要消掉得抽公共 crate，而不是再复制一份。

## 八、产品代码坏味道门（七道棘轮 + 一道「碰了就得减」）

前面几道门管的是**规模**（多大）与**雷同**（几份）。这一组管的是「编译器不报、clippy 只 warn
不拦、但代码评审每次都会挑」的坏味道。共性：

- 全部**零依赖纯 stdlib**，复用 `scripts/gates/gates_common.py`——扫描面 / 基线读写 / 棘轮判定，
  连 `main` 流程（`run_count_gate`）与「每文件数某模式几次」（`regex_scan`）都只有一份实现
  ⇒ 口径不会在各门之间漂移，各门文件只剩自己的判据与文档字符串；Rust 感知掩码复用
  `god_gate._mask`；
- 全部**排除测试面**（`/tests/`、`/examples/`、`/benches/`）——测试里 `unwrap()`、`sleep()` 是
  正当写法，算进去只会逼人把测试写歪；
- 全部是**棘轮**：存量入基线，只准减，新增即红。不要求一次性还清历史债，但一个数都不许再涨。

| 门 | 判据 | 存量基线 |
| --- | --- | --- |
| `unwrap_gate.py` | 产品代码 `.unwrap()` / `.expect()` / `.unwrap_err()` / `.expect_err()` / `.unwrap_unchecked()` / `.expect_unchecked()` | 398 处 |
| `linelen_gate.py` | 单行 > 200 字符 | 5 行 |
| `sleep_gate.py` | `thread::sleep(` | 6 处 |
| `ignore_gate.py` | `let _ = …` / 解构里含 `_` 的丢弃位 | 155 处 |
| `unsafe_gate.py` | `unsafe {` / `unsafe fn` / `unsafe impl`（排除 FFI 与平台壳目录） | 8 处 |
| `cyc_gate.py` | 函数圈复杂度 > 15 / > 50 | 71 / 9 |
| `nest_gate.py` | 函数嵌套深度 > 5 棘轮、**> 8 硬禁** | 16 / 0 |
| `log_gate.py` | `println!` / `eprintln!` 而非 `tracing`（排除 `apps/cli`、`tools/`、`build.rs`） | 16 |
| `trait_gate.py` | 单个 trait 定义的方法数 > 15 棘轮、**> 40 硬禁** | 14 个 trait 入册 |

各自的理由：

- **unwrap/expect**：Rust 里 `?` 才是正确传播错误的方式；`unwrap` 一旦遇到 `Err`/`None` 就
  panic，把库里一个可恢复错误变成整个进程崩溃。这是 Rust 头号坏味道，clippy 只 warn。
- **超长行**：官方不强制行长，但超长行在 review / diff / 终端里都难读，也常是「一个表达式
  塞太多东西」或超长字符串字面量的信号。
- **`thread::sleep`**：轮询等某事、循环里 sleep 节流、启动顺序靠 sleep 凑——都会让程序在
  CI 与弱机器上 flake、在延迟敏感路径上卡顿。正解是 channel / 条件变量 / `tokio::time::sleep` + `select!`。
- **忽略结果**：`let _ = result;` 绝大多数是吞掉了 `Result` 里的错误，即 Rust 版的 errcheck。
  `let _x = …`（绑给 `_x`）是正常命名，不算丢弃。
- **unsafe**：用得越多，未定义行为面越大，reviewer 要逐行盯。FFI / 平台后端（`ffi` / `sys` /
  `platform` / `bindings` / 各 OS 壳目录）里的 unsafe 是正当的 ⇒ 排除，其余只拦新增。
- **圈复杂度**：决策点 = 1（函数本身）+ `if`/`for`/`while`/`loop`/`match` + `&&`/`||` + `?` +
  `match` 每分支 `=>`。与 clippy 的 `cyclomatic_complexity` 同思路，但纯文本、不依赖编译，
  因此连编不到的 target 也能扫。
- **嵌套深度**：圈复杂度数的是「分支多少」，这一门数的是「分支套了几层」。同样复杂的一段逻辑，
  写成卫语句平铺与写成五层 `if let` 套 `match`，复杂度可能一样，**可读性差一个量级**——
  后者 review 时人眼配平不过来。深度从函数体的 `{` 起算为 1，所以「> 5」≈ 函数体里又套了四层。
- **日志门面**：壳里没有控制台（macOS 的 IMK 插件、Windows 的 TSF DLL），`println!` 写出去
  **没人收得到**，等真要查问题才发现日志早丢光了；它同时没有级别、没有 span、没法按模块开关。
  `apps/cli` 与 `tools/` 打印到 stdout 是本分 ⇒ 排除。
- **上帝接口**：一个 trait 方法越多，实现方要填的坑越多，越像「胖接口」。按 **crate 内 trait 名**
  聚合（同名小 trait 在不同 crate 里很常见，不合并）。> 40 是**硬禁**——基线不放行。

### 硬判的四条（存量 = 0 ⇒ 不设基线）

存量本来就是 0 的规矩，留基线等于给它发豁免——`--write` 重记基线就能把它祖父化。这四条**硬判**：

| 门 | 判据 | 为什么值得硬判 |
| --- | --- | --- |
| `args_gate.py` | 函数形参 > 7 | 相邻的同类型参数随时能被对调，编译器不吭声；正解是收进结构体/配置对象 |
| `prefix_gate.py` | 同目录下 ≥2 个 `.rs` 共享下划线前缀，而目录本身不叫那段前缀 | 用文件名当前缀分组是**目录结构**上的债，跟文件多大无关 ⇒ 按文件统计的门永远抓不到 |
| `cargo_gate.py` C1 | `crates/*/Cargo.toml` 必须 `version.workspace = true` | 写死就会在发版时对不上 |
| `cargo_gate.py` C2 | `apps/{macos,linux,windows}*/Cargo.toml` 必须写死自己的 `version` | 各壳独立发布，Windows 读 `server/Cargo.toml` 取版本号 ⇒ 写错就是发错版 |
| `cargo_gate.py` C3 | 全仓不许 `anyhow`（依赖表与代码都判） | 库里抛 `anyhow::Error` 会抹掉具体错误类型，调用方只能 `downcast` 猜 |
| `lang_gate.py` | 不许 `Vec<Translation>` / `[Translation; N]` / `HashMap<Lang, _>` / 复数 `translations:` | 「一个候选词只显示一种辅助语言」是**产品原则**：写成多语言并列照样编译得过、测试也过，等要加第二种语言时才发现整个数据结构得推翻。`Option<Translation>`（可选单条）是合规写法 |
| `ident_gate.py` | 标识符一律英文；`#[error]` 文案用英文 | 中文标识符在跨平台终端/日志编码下随时乱码；`#[error]` 的文案要被上层当**标识符**用（分级、过滤、对接第三方）。只判属性括号里的内容 ⇒ `#[error(transparent)] // 中文注释` 不算命中 |

`apps/cli` 不参与 C2：它是 workspace 里的工具，不是独立发布的壳。

### 代码组织与命名的两条（棘轮）

| 门 | 判据 | 存量 |
| --- | --- | --- |
| `super_gate.py` | `use super::super::…` 绕父模块转手 | 4 |
| `testsize_gate.py` | 单文件内嵌测试（`#[cfg(test)]`）> 200 行 | 7 个文件 |

- **super 转手**：`a/b/c.rs` 里写 `use super::super::X`，读者得先搞清楚 `a/mod.rs` re-export 了
  什么才知道 X 从哪来；父模块一改 re-export，中间这层就断。规矩要求直接 `use crate::…`
  （路径是绝对的，不用在脑子里做相对路径运算）。
- **测试体积**：contributing 写的是「测试超过 200 行搬到 `tests.rs`」。内嵌测试**不长在 god_gate
  的口径里**——一个 700 行的文件里 400 行是测试，产品代码只有 300 行，god_gate 看着还「没超 800」，
  但读的人要翻过 400 行测试才看到产品逻辑。口径：从 `#[cfg(test)]` 那行起到文件末尾
  （本仓惯例是测试放最后；万一夹在中间，量出来偏大，方向保守）。

`unsafe_gate.py` 与 `gate.py` 里那条条件步 `unsafe`（调 `scripts/unsafe_audit.py`）不重复：
后者是**增量**门（属另一条 CI 分支，尚未合入），前者是**存量棘轮**，合入后两道并行。

`cyc_gate.py` 与 `nest_gate.py` 都是「逐函数量一个数」，配平与遍历共用 `gates_common.iter_fns`
与 `fn_metric_scan` ⇒ 两门口径一致，也不会互相雷同（雷同门同样盯着门脚本自己）。

### 碰了就得减（`god_touch.py`）

棘轮只说「不许变胖」，于是存量能永远躺着：一个 5000 行的上帝文件，只要没人给它加行，它就
永远是 5000 行，而每个人都在绕着它走。本门规定：**改了已经超阈的文件，就必须把它变小**——
连「原样不动」都不行。

- 判据：本次改动碰到的、且在 `god-baseline.json` 里已超过任一硬阈的文件，其当前规模
  （`file_lines` / `max_fn_lines` / `max_type_members` 任一维度）必须**严格小于**基线；
- 未超阈的文件不受这条管（仍受上帝对象门的棘轮管）；
- 没有这条，正确的重构反而会被棘轮逼着去关门，而真正的债务永远不动。

CI 传 PR 的 base sha；本地缺省比 `git diff HEAD`。base 拿不到时自动退回比 HEAD。

## 七、门禁自检（`gate_selftest.py`）

门全是仓库里的文本文件——删掉 workflow 一行、调大阈值、把硬阈改回 false、把钩子里的
`--fast` 去掉，都不会让任何测试变红，门却已经没了。**门静默变弱比没有门更危险**（它还挂着绿勾）。

- S1 本地 `gate.py` 的 STEPS 与 `.github/workflows/gates.yml` **双向同源**（少一步即红）；
- S2 `.githooks/pre-commit` 必须调用 `gate.py --fast`；
- S3 `god.gate.json` 相对 HEAD：阈值只许收紧、硬阈只许 false→true、`include` 不许少、`exclude` 不许多；
- S4 `god_gate.py` 的两处本仓适配仍在（上游重抄整文件时最容易抄丢）；
- S5 基线在位且是合法 JSON；
- S6 `.agents/CLAIMS.md` 在位。
- S7 `ci.yml` 的 `gate-shape` 锚在位（两处互盯，见下）。

CI 里额外跑一次**注入自检**：`QJ_GATE_FORCE_FAIL=god-gate` 时门必须红，绿了说明这一步根本没生效。


> **拆分说明**：本文件随门禁系列 PR 逐批扩充——本 PR 只含上述章节，其余章节见同系列配套 PR（上帝对象族 / 质量棘轮族 / 协作基建）。
