"""多智能体并发协作门（agent_gate）——**本仓同时有好几个智能体在开发**，靠自觉必然撞车。

为什么要有它：文件级冲突合并起来最贵（两个智能体各改半个函数，git 能合并、语义不能）。
上游 unified-rx-mcp 没有这个门（单智能体项目），本仓新增。

机制（全部机器可判，零依赖）：
  `.agents/claims/<agent-id>.json` —— **每个智能体只写自己那一个文件**（文件名互不相同 ⇒
  天然不存在写冲突，这是选「一文件一智能体」而不是「共用一份 JSON」的唯一理由）。

判据：
  A1 声明格式：文件名 stem 必须等于 `agent` 字段；`agent`/`task`/`scope`/`expires` 必填；
     `scope` 非空；`expires` 是 ISO 日期。
  A2 一个智能体一份声明（按 `agent` 字段去重；重名即红）。
  A3 **域不重叠**：两份**未过期**声明的 scope 有交集 ⇒ 红，并打印交到的具体文件
     （模式级：相同 / 目录包含 / 通配互含；文件级：把 scope 投影到 `git ls-files`）。
  A4 **越界改动**：当前改动的文件若落在**别人的**活跃域里 ⇒ 红。执行者身份取
     `QJ_AGENT_ID` 环境变量，或 `git config qingjian.agentId`。
  A5 **未登记就动手**：`QJ_AGENT_ID` 设了却没有对应声明 ⇒ 红。
     CI 上没设 ⇒ 打印 SKIP（CI 是只读校验，靠 A3 挡住域重叠即可），不静默判绿。

用法：
  python -X utf8 scripts/gates/agent_gate.py                       # 门
  python -X utf8 scripts/gates/agent_gate.py --list                # 列当前认领
  python -X utf8 scripts/gates/agent_gate.py --changed-from main   # 指定比较基线
  python -X utf8 scripts/gates/agent_gate.py --agent <id>          # 覆盖身份（调试用）
退出码：0 = 通过；1 = 命中；2 = 声明文件坏了（JSON / 字段）。
"""
import argparse
import fnmatch
import json
import os
import pathlib
import subprocess
import sys
from datetime import date, datetime

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
CLAIMS_DIR = ROOT / ".agents" / "claims"
REQUIRED = ("agent", "task", "scope", "expires")
# 改门禁本身不需要为每个智能体单独登记域——这些是「门自己的地盘」
GATE_OWN = {".agents/**", "scripts/gates/**", ".github/workflows/gates.yml",
            "docs/review/god-*.md", "docs/review/god-*.json", "docs/review/dupe-baseline.json"}


def _git(*args):
    return subprocess.run(["git", *args], cwd=str(ROOT), capture_output=True, text=True,
                          encoding="utf-8", errors="replace", shell=False)


def _today() -> date:
    override = os.environ.get("QJ_FAKE_TODAY")
    return datetime.strptime(override, "%Y-%m-%d").date() if override else date.today()


def load_claims():
    """返回 [(path, claim, err)]；err 非空即该文件坏了（A1/A2 在调用处判）。"""
    out = []
    if not CLAIMS_DIR.is_dir():
        return out
    for fp in sorted(CLAIMS_DIR.glob("*.json")):
        try:
            doc = json.loads(fp.read_text(encoding="utf-8"))
        except ValueError as e:
            out.append((fp, None, f"{fp.name} 不是合法 JSON（{e}）"))
            continue
        if not isinstance(doc, dict):
            out.append((fp, None, f"{fp.name} 顶层必须是对象"))
            continue
        out.append((fp, doc, None))
    return out


def match(rel: str, pat: str) -> bool:
    """`**/` 前缀可省（fnmatch 的老坑：根级文件匹配不到 `**/x`）。"""
    if pat.endswith("/**"):
        d = pat[:-3]
        return rel.startswith(d + "/") or rel == d
    if fnmatch.fnmatch(rel, pat):
        return True
    return pat.startswith("**/") and fnmatch.fnmatch(rel, pat[3:])


def expand(scope, files):
    return {f for f in files if any(match(f, p) for p in scope)}


def patterns_overlap(p: str, q: str) -> bool:
    """模式级重叠：相同 / 目录包含 / 通配互含。够挡住绝大多数误登记。"""
    if p == q:
        return True
    a = p[:-3] if p.endswith("/**") else p
    b = q[:-3] if q.endswith("/**") else q
    if a == b:
        return True
    if a.endswith("/**") or b.endswith("/**"):
        return a.startswith(b + "/") or b.startswith(b + "/")
    if a.startswith(b + "/") or b.startswith(b + "/"):
        return True
    return fnmatch.fnmatch(a, b) or fnmatch.fnmatch(b, a)


def check_a1(entries):
    """A1 声明格式：坏文件、缺字段、文件名与 agent 不一致、scope 空、expires 非法。"""
    fails = []
    for fp, doc, err in entries:
        if err:
            fails.append(f"A1 {err}")
            continue
        missing = [k for k in REQUIRED if not doc.get(k)]
        if missing:
            fails.append(f"A1 {fp.name} 缺字段 {missing}（必填：{', '.join(REQUIRED)}）")
            continue
        if fp.stem != doc["agent"]:
            fails.append(f"A1 {fp.name}：文件名 stem 必须等于 agent 字段（{doc['agent']}）"
                         f"——文件名互不相同才不会有写冲突")
        if not isinstance(doc["scope"], list) or not doc["scope"]:
            fails.append(f"A1 {fp.name}：scope 必须是非空数组")
        try:
            datetime.strptime(doc["expires"], "%Y-%m-%d")
        except ValueError:
            fails.append(f"A1 {fp.name}：expires 必须是 YYYY-MM-DD（拿到 {doc['expires']!r}）")
    return fails


def check_a2(valid):
    """A2 一个智能体只许一份声明。"""
    by_agent: dict[str, list[str]] = {}
    for fp, doc in valid:
        by_agent.setdefault(str(doc.get("agent")), []).append(fp.name)
    return [f"A2 智能体 {agent} 有 {len(names)} 份声明 {names}——一个智能体只许一份"
            for agent, names in by_agent.items() if len(names) > 1]


def split_active(valid, today):
    """按 `expires` 分活跃/过期；过期的不参与域冲突判定。"""
    active, expired = [], []
    for fp, doc in valid:
        if not isinstance(doc.get("scope"), list) or not doc["scope"]:
            continue
        try:
            exp = datetime.strptime(doc["expires"], "%Y-%m-%d").date()
        except ValueError:
            continue
        (active if exp >= today else expired).append((fp, doc))
    return active, expired


def check_a3(active, tracked):
    """A3 域不重叠：两份活跃声明的 scope 有交集（模式级或实际文件级）即红。"""
    fails = []
    for i in range(len(active)):
        for j in range(i + 1, len(active)):
            f1, d1 = active[i]
            f2, d2 = active[j]
            hits = [f"{p} ↔ {q}" for p in d1["scope"] for q in d2["scope"] if patterns_overlap(p, q)]
            common = expand(d1["scope"], tracked) & expand(d2["scope"], tracked)
            if hits or common:
                detail = "；实际共同文件 " + "、".join(sorted(common)[:5]) if common else ""
                fails.append(f"A3 域重叠：{d1['agent']}（{f1.name}）× {d2['agent']}（{f2.name}）"
                             f" —— {', '.join(hits[:3])}{detail}"
                             f"　两个智能体不能同时动同一处，先谈清楚再改 scope")
    return fails


def changed_files(how: str):
    if how == "worktree":
        return set(_git("diff", "--name-only", "HEAD").stdout.split())
    if how == "none":
        return set()
    return set(_git("diff", "--name-only", how).stdout.split())


def check_identity(a, active):
    """A4 越界改动 / A5 未登记就动手。无身份时打印说明并跳过（CI 靠 A3）。"""
    who = (a.agent or os.environ.get("QJ_AGENT_ID")
           or _git("config", "--get", "qingjian.agentId").stdout.strip() or None)
    if who is None:
        print("  （未设 QJ_AGENT_ID / git config qingjian.agentId ⇒ 跳过越界检查；"
              "CI 靠 A3 挡域重叠。智能体动手前请先建 .agents/claims/<id>.json 并设 QJ_AGENT_ID）")
        return []
    fails = []
    mine = [d for _fp, d in active if d["agent"] == who]
    if not mine:
        fails.append(f"A5 身份 {who} 没有活跃声明——先在 .agents/claims/{who}.json 登记"
                     f"（格式见 .agents/CLAIMS.md），别直接改代码")
        mine = [{"scope": []}]
    my_scope = set(mine[0]["scope"])
    changed = {c for c in changed_files(a.changed_from) if c.strip()}
    others = [d for _fp, d in active if d["agent"] != who]
    for f in sorted(changed):
        for d in others:
            if any(match(f, p) for p in d["scope"]):
                fails.append(f"A4 越界：改了 {f}（在 {d['agent']} 的域内：{d['task']}）"
                             f"——要么让给它，要么先在 .agents/claims/ 里改 scope 并知会对方")
                break
    if changed and my_scope:
        allowed = my_scope | GATE_OWN
        for f in [f for f in sorted(changed) if not any(match(f, p) for p in allowed)][:10]:
            print(f"  ⚠ {f} 不在 {who} 声明的域内（提示，不判红：跨模块小改请补进 scope 再提交）")
    return fails


def do_list(valid, active):
    for fp, doc in valid:
        mark = "活跃" if any(x[0] == fp for x in active) else "过期"
        print(f"  [{mark}] {doc['agent']}  {doc.get('branch', '-')}  {doc['task']}")
        for p in doc["scope"]:
            print(f"        {p}")
    print(f"共 {len(valid)} 份声明（活跃 {len(active)}）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--list", action="store_true")
    ap.add_argument("--changed-from", default="worktree",
                    help="比较基线：`worktree`（默认，= git diff HEAD）、`none`（跳过 A4）、或某个 ref")
    ap.add_argument("--agent", default=None, help="覆盖执行者身份（调试用）")
    a = ap.parse_args()

    entries = load_claims()
    fails = check_a1(entries)
    valid = [(fp, doc) for fp, doc, err in entries if err is None and doc]
    fails += check_a2(valid)

    active, expired = split_active(valid, _today())
    for fp, doc in expired:
        print(f"  （声明 {fp.name} 已于 {doc['expires']} 过期，不参与域冲突判定；请删掉或续期）")

    if a.list:
        return do_list(valid, active)

    tracked = set(_git("ls-files").stdout.split())
    fails += check_a3(active, tracked)
    fails += check_identity(a, active)

    if fails:
        for f in fails[:20]:
            print(f"  ✗ {f}")
        if len(fails) > 20:
            print(f"  …另有 {len(fails) - 20} 条")
        print(f"AGENT-GATE FAIL {len(fails)} 条（声明目录 {CLAIMS_DIR.relative_to(ROOT).as_posix()}）")
        return 1
    print(f"AGENT-GATE OK 声明 {len(valid)} 份 / 活跃 {len(active)} 份，域无重叠、改动无越界")
    return 0


if __name__ == "__main__":
    sys.exit(main())
