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

## 一、上帝对象门（`god_gate.py` + `god.gate.json`）

**纪律来源**：`docs/contributing.md`「代码组织」——单文件 ≤800 行（目标 500）、一个类型一个文件、
大类型 `impl` 按职责拆子模块。门把那三条从「写给人看」变成「机器判」。

三个指标、每条都有**棘轮**与**硬阈**两档（`god.gate.json` 里分别开关）：

| 指标 | 阈值 | 棘轮 | 硬阈 | 存量 |
| --- | --- | --- | --- | --- |
| `file_lines` 文件行数 | 800 | 只准减 | **开**（`file_hard_threshold: true`） | 存量全绿（最大 738） |
| `max_fn_lines` 最长函数 | 100 | 只准减 | 待开 | 46 个文件欠账，见台账 |
| `max_type_members` 最大类型成员 | 20 | 只准减 | 待开 | 同上 |

- **棘轮**：任何指标超过基线值 ⇒ 红。低于基线 ⇒ 绿，并提示可收紧基线（只准减）。
- **硬阈**：`true` 时**存量超阈也红**（不接受基线祖父化）。三项分开开闸——
  `file_lines` 落地当天就开（存量已全绿）；另两项等 `docs/review/god-debt.md` 的欠账清零后再翻，
  翻转动作由 `gate_selftest.py` 的 S3 锁住：**只许 false→true，不许改回**。
- **拆函数的合法交换**：把长函数拆成 helper 会让文件变长。若 `max_fn_lines` 严格下降、
  且涨幅 ≤10%（或函数降幅 ≥ 文件涨幅），判合法交换并逐条打印；函数没变短就别想涨行数。
- **新文件**：没有基线可依赖，直接对阈值判。
- Rust 用花括号配平启发式（先掩掉字符串/注释/生命周期再配平），输出标注 `H`。

改基线前先看差：`python -X utf8 scripts/gates/god_diff.py`（`--write-baseline` 会一次性重记
所有值 = 放松棘轮，先看差才能分清「纠正测量」与「真变胖」）。

## 二、欠账台账（`god_debt.py` + `docs/review/god-debt.md`）

棘轮只保证「不许变胖」，管不住「历史上就这么胖」。台账把存量欠账列出来、排好序、可认领：

- 欠账 = Σ max(0, 指标 − 阈)，加权 行数×1 + 函数×3 + 成员×2（函数最难读，权重最高）；
- `--check`：台账里的总数必须与 `god-baseline.json` 算出来的一致 ⇒ **手改数字 = 红**，
  想让数字变小只能真的去拆；
- `--touched <ref>`：列出「这次改动碰到的欠账文件」（提示，不判红）——碰了就顺手减一点。

拆一块的流程：先认领 → 拆 → `gate.py --write` → `gate.py --fast` → 提交。

## 三、类型跨度门（`type_span_gate.py`）

**为什么还要单独一道**：`docs/contributing.md` 要求「大类型的 `impl` 按职责拆成子模块」。
拆完之后每个文件都只有一两百行，`god_gate` 的三项指标全绿——但那个**类型本身**可能还是个
庞然大物。按文件量规模**永远**抓不到它，必须按类型名把 `impl` 块聚合起来看。

判据（每个 crate 内按类型名聚合）：

| 指标 | 阈值 | 棘轮 | 硬阈 |
| --- | --- | --- | --- |
| `methods` 该类型所有 impl 的方法总数 | 40 | 只准减 | 待开 |
| `files` 这些 impl 散在几个文件 | 8 | 只准减 | 待开 |

当前最重的一处：`crates/qingjian-core|Engine` —— **207 个方法、散在 16 个文件**。
这是本仓最典型的上帝对象，而它在按文件统计的门里是全绿的。

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
