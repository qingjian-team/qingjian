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

## 六、多智能体协作门（`agent_gate.py` + `.agents/CLAIMS.md`）

本仓同时有好几个智能体在开发。认领写在你自己那一个文件里（`.agents/claims/<id>.json`），
文件名互不相同 ⇒ 天然没有写冲突；冲突判定交给机器（A3 域重叠）。

| 编号 | 规则 | 命中 |
| --- | --- | --- |
| A1 | 文件名 stem == `agent`；`agent`/`task`/`scope`/`expires` 必填 | 红 |
| A2 | 一个智能体只许一份声明 | 红 |
| A3 | 两份未过期声明的 scope 有交集 | 红 |
| A4 | 改了别人活跃域里的文件 | 红 |
| A5 | 设了 `QJ_AGENT_ID` 却没有对应声明 | 红 |

完整协议、声明模板、文件地图见 [`.agents/CLAIMS.md`](../.agents/CLAIMS.md)。

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
