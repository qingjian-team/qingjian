"""门禁自检（selftest）：**门本身不许被悄悄削弱或绕过**。

为什么要有它：门禁全是仓库里的文本文件——删掉 workflow 里的一行、把阈值调大、把硬阈从
true 改回 false、把钩子里的 `--fast` 去掉，都不会让任何测试变红，门却已经没了。
这类「门静默变弱」比没有门更危险（它还挂着绿勾）。本文件把这些变成机器判据。

判据：
  S1 **同源**：`gate.py` 的 STEPS 里每个 `scripts/gates/*.py` 必须在
     `.github/workflows/gates.yml` 里被显式调用，反之亦然（防 workflow 少跑一步）。
  S2 **钩子在**：`.githooks/pre-commit` 必须调用 `gate.py --fast`。
  S3 **阈值只许收紧**：`god.gate.json` 相对 HEAD 的版本——
     `max_file_lines` / `max_fn_lines` / `max_type_members` 只许减小；
     `*_hard_threshold` 只许 false→true；`include` 不许少、`exclude` 不许多。
  S4 **本仓适配仍在**：`god_gate.py` 里两处针对本仓的改动（脚本同目录取配置、
     三档硬阈）必须还在——上游重抄整文件时最容易把这两处抄丢。
  S5 **基线在位**：`docs/review/` 下各门的基线存在且是合法 JSON（没基线 = 只对新文件生效，
     存量管不住 ⇒ 删基线等于把门卸了）。
  S6 **协议在位**：`.agents/CLAIMS.md` 存在（多智能体协作协议不许删）。
  S7 **ci.yml 的锚在位**：`gate-shape` job 必须还在——gates.yml 被整个删掉时它自己不会跑，
     只有 ci.yml 这道能发现；这道 S7 又反过来盯住「有人把 ci.yml 的锚摘掉」。两处互盯。

用法：python -X utf8 scripts/gates/gate_selftest.py
退出码：0 = 通过；1 = 门被削弱/绕过。
"""
import importlib.util
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
GATES = ROOT / "scripts" / "gates"
WORKFLOW = ROOT / ".github" / "workflows" / "gates.yml"
HOOK = ROOT / ".githooks" / "pre-commit"
CFG = GATES / "god.gate.json"
BASELINES = [
]

CI = ROOT / ".github" / "workflows" / "ci.yml"
CLAIMS = ROOT / ".agents" / "CLAIMS.md"


def load_steps():
    spec = importlib.util.spec_from_file_location("gate_for_selftest", GATES / "gate.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.STEPS


def head_text(rel: str):
    cp = subprocess.run(["git", "show", f"HEAD:{rel}"], cwd=str(ROOT), capture_output=True,
                        text=True, encoding="utf-8", errors="replace", shell=False)
    return cp.stdout if cp.returncode == 0 else None


def main() -> int:
    fails: list[str] = []

    # S1 同源
    if not WORKFLOW.is_file():
        fails.append(f"S1 workflow 不存在：{WORKFLOW.relative_to(ROOT)}（门禁没接进 CI）")
        wf = ""
    else:
        wf = WORKFLOW.read_text(encoding="utf-8")
    steps = load_steps()
    ENTRY = "scripts/gates/gate.py"          # 入口本身不是「一道门」，不参与同源对账
    step_scripts = {a for _n, cmd, _t, _w in steps for a in cmd
                    if isinstance(a, str) and a.startswith("scripts/gates/")
                    and a.endswith(".py") and a != ENTRY}
    for s in sorted(step_scripts):
        if s not in wf:
            fails.append(f"S1 门脚本 {s} 不在 gates.yml 里（CI 少跑这一步 ⇒ 门形同虚设）")
    for s in sorted(set(re.findall(r"scripts/gates/[A-Za-z_]+\.py", wf))):
        if s == ENTRY or s in step_scripts:
            continue
        fails.append(f"S1 gates.yml 里的 {s} 不在 gate.py 的 STEPS 里（本地与 CI 漂移）")

    # S2 钩子
    if not HOOK.is_file():
        fails.append("S2 .githooks/pre-commit 不存在")
    else:
        hook = HOOK.read_text(encoding="utf-8")
        if "gate.py" not in hook or "--fast" not in hook:
            fails.append("S2 pre-commit 没有调用 `gate.py --fast`（本地钩子上少一步，"
                         "别人改代码时门不响）")

    # S3 阈值只许收紧
    if not CFG.is_file():
        print("  god.gate.json not landed yet (god-family PR) => skip S3")
        cur = None
    else:
        cur = json.loads(CFG.read_text(encoding="utf-8"))
    old_txt = head_text("scripts/gates/god.gate.json")
    if old_txt is None or cur is None:
        print("  （god.gate.json 还没进 HEAD ⇒ 跳过与历史的对比；下一次跑起生效）")
    else:
        old = json.loads(old_txt)
        for k in ("max_file_lines", "max_fn_lines", "max_type_members",
                  "max_type_methods_total", "max_type_impl_files"):
            # 老配置里没有的键 = 新引入的阈值，没有「放宽」可言（拿 0 比会误红）
            if k not in old:
                continue
            if cur.get(k, 0) > old.get(k, 0):
                fails.append(f"S3 阈值放宽：{k} {old.get(k)} → {cur.get(k)}（只许收紧）")
        for k in ("file_hard_threshold", "fn_hard_threshold", "type_hard_threshold",
                  "type_span_hard_threshold"):
            if old.get(k) is True and cur.get(k) is not True:
                fails.append(f"S3 硬阈被关：{k} true → {cur.get(k)}（开闸后不许改回）")
        for k, verb in (("include", "变少"), ("exclude", "变多")):
            miss = set(old.get(k, [])) - set(cur.get(k, []))
            add = set(cur.get(k, [])) - set(old.get(k, []))
            if k == "include" and miss:
                fails.append(f"S3 include {verb}：少了 {sorted(miss)}（扫描面不许缩小）")
            if k == "exclude" and add:
                fails.append(f"S3 exclude {verb}：多了 {sorted(add)}（豁免面不许扩大）")

    # S4 本仓适配仍在（god_gate.py 随上帝对象族 PR 落地；未落地前跳过）
    if not (GATES / "god_gate.py").is_file():
        gg = None
    else:
        gg = (GATES / "god_gate.py").read_text(encoding="utf-8")
    if gg is not None and 'parent / "god.gate.json"' not in gg:
        fails.append("S4 god_gate.py 丢了「配置取脚本同目录」的适配（会静默落回默认阈值 ⇒ 门变弱）")
    if gg is not None and ("file_hard_threshold" not in gg or "type_hard_threshold" not in gg):
        fails.append("S4 god_gate.py 丢了三档硬阈（只剩 fn 一档 ⇒ 文件与类型规模管不住）")

    # S5 基线在位
    for b in BASELINES:
        if not b.is_file():
            fails.append(f"S5 基线缺失：{b.relative_to(ROOT)}（没基线 = 只对新文件生效，存量管不住）")
            continue
        try:
            json.loads(b.read_text(encoding="utf-8"))
        except ValueError as e:
            fails.append(f"S5 基线 {b.name} 不是合法 JSON（{e}）")

    # S6 协议在位
    if not CLAIMS.is_file():
        fails.append("S6 .agents/CLAIMS.md 缺失（多智能体协作协议没了 ⇒ 并行改动没人认领）")

    # S7 **ci.yml 上那道锚还在**：gates.yml 被整个删掉时它自己不会跑，只有 ci.yml 的
    # `gate-shape` job 能发现；反过来，这道 S7 又能发现有人把 ci.yml 的锚摘掉。两处互盯。
    if not CI.is_file():
        fails.append("S7 ci.yml 不存在")
    else:
        ci = CI.read_text(encoding="utf-8")
        if "gate-shape" not in ci or "gate_selftest.py" not in ci:
            fails.append("S7 ci.yml 里的 `gate-shape` 锚没了——gates.yml 被删时就没人报警了")

    if fails:
        for f in fails:
            print(f"  ✗ {f}")
        print(f"GATE-SELFTEST FAIL {len(fails)} 条（门禁被削弱或绕过）")
        return 1
    print("GATE-SELFTEST OK 门禁自洽（CI/本地同源、钩子在位、阈值未放宽、基线在位）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
