"""门禁脚本的共用件（零依赖，纯 stdlib）。

为什么抽出来：新加的多道门（unwrap / 复杂度 / god trait / 行长 / 忽略 / 休眠 / unsafe /
碰了就得减）判据各不相同，但「扫哪些文件、基线怎么读写、棘轮怎么判、结果怎么报」完全一样。
各写一遍就会有 N 份各差一点的副本——判据口径一旦漂移，同一个文件在不同门里算出来的数
不一样，比没有门更糟。

本文件**不是一道门**（不进 `gate.py` 的 STEPS、不出现在 workflow 里），只被各门 import。
Rust 感知的掩码复用 `god_gate._mask`（它认 Rust 字符字面量/生命周期，括号配平才准）。
基线统一落在 `docs/review/`（与 god / dupe / type-span / arch 一致）。
"""
import argparse
import fnmatch
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
EXCLUDE = ("**/target/**", "**/.git/**", "**/node_modules/**", "**/__pycache__/**",
           "**/dist/**", "**/build/**", "**/data/generated/**", "**/assets/**")
TESTISH = ("/tests/", "/examples/", "/benches/", "/tests.rs")


def skip_hit(rel: str, parts) -> bool:
    """`rel` 是否命中任一片段——片段带不带前导斜杠都认，单个文件名也认。

    `rel` 是**相对**路径（`apps/cli/src/x.rs`），写 `"/apps/cli/"` 永远匹配不上，
    这类静默失效的排除面比没有排除更糟（门看着在跑，其实豁免了一大片）。
    """
    base = rel.rsplit("/", 1)[-1]
    for d in parts:
        s = d.strip("/")
        if not s:
            continue
        if base == s or rel.startswith(s + "/") or f"/{s}/" in rel:
            return True
    return False


def matched(rel: str, extra_exclude=()) -> bool:
    def hit(pat):
        return fnmatch.fnmatch(rel, pat) or (pat.startswith("**/") and fnmatch.fnmatch(rel, pat[3:]))
    return not any(hit(p) for p in EXCLUDE + tuple(extra_exclude))


def rs_files(root: pathlib.Path, git_tracked: bool, suffix=".rs", extra_exclude=()):
    """受扫文件清单（**相对路径，正斜杠**）；`--git-tracked` 时只算已跟踪的。"""
    tracked = None
    if git_tracked:
        cp = subprocess.run(["git", "ls-files"], cwd=str(root), capture_output=True,
                            text=True, encoding="utf-8", errors="replace", shell=False)
        tracked = set(cp.stdout.split())
    out = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in {".git", "target", "node_modules",
                                                        "__pycache__", "dist", "build"}]
        for fn in filenames:
            if not fn.endswith(suffix):
                continue
            rel = (pathlib.Path(dirpath) / fn).relative_to(root).as_posix()
            if not matched(rel, extra_exclude):
                continue
            if tracked is not None and rel not in tracked:
                continue
            out.append(rel)
    return sorted(out)


FN_DECLARE = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+|async\s+|unsafe\s+|extern\s+)*fn\s+[A-Za-z_]")


def iter_fns(lines):
    """遍历掩码后的每一行，产出每个函数的 `(起始行, 结束行)`（花括号配平到函数体结束）。

    cyc / nest 两道门都要「逐函数看一段代码」，各写一遍配平就会有两份各差一点的副本；
    抽出来后两门共用同一份口径，也顺手躲开雷同门。
    """
    for i, line in enumerate(lines):
        if not FN_DECLARE.match(line):
            continue
        depth, started, end = 0, False, len(lines) - 1
        for j in range(i, len(lines)):
            depth += lines[j].count("{") - lines[j].count("}")
            if "{" in lines[j]:
                started = True
            if started and depth <= 0:
                end = j
                break
        yield i, end


def read_text(root: pathlib.Path, rel: str) -> str:
    try:
        return (root / rel).read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


_GOD = None


def load_god():
    """`god_gate` 只 import 一次——逐文件 exec_module 会在上千文件上把门拖成分钟级。"""
    global _GOD
    if _GOD is None:
        spec = importlib.util.spec_from_file_location(
            "god_gate_common", ROOT / "scripts" / "gates" / "god_gate.py")
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        _GOD = mod
    return _GOD


def mask(text: str) -> str:
    """Rust 感知掩码（注释/字符串/字符字面量/生命周期），供括号配平与声明识别。"""
    return load_god()._mask(text, js=False)


def load_baseline(path: pathlib.Path) -> dict:
    if not path.is_file():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except ValueError as e:
        sys.exit(f"基线 {path.name} 不是合法 JSON（{e}）——若上次写入被中断，重跑 --write 即可")


def write_baseline(path: pathlib.Path, obj) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(obj, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    os.replace(tmp, path)          # 原子替换：中断也不留半截基线


def ratchet(cur: dict, base: dict, label: str, bad: list, shrank: list) -> None:
    """只准减：新增即红，减少则提示可收紧。"""
    for rel, v in sorted(cur.items()):
        bv = base.get(rel, 0)
        if v > bv:
            bad.append(f"{label} {rel}: {bv} → {v}（新增 {v - bv} 处，只准减）")
        elif v < bv:
            shrank.append(f"{label} {rel}: {bv} → {v}")


def report(name: str, bad: list, shrank: list, limit: int = 25) -> int:
    for line in bad[:limit]:
        print(f"  ✗ {line}")
    if len(bad) > limit:
        print(f"  …另有 {len(bad) - limit} 条")
    if shrank:
        print(f"  （{len(shrank)} 条可收紧，跑 --write 更新）")
    print(f"{name} {'FAIL' if bad else 'OK'} 命中={len(bad)}")
    return 1 if bad else 0


def add_args(ap):
    ap.add_argument("--git-tracked", action="store_true")
    ap.add_argument("--write", action="store_true")
    ap.add_argument("--list", action="store_true")
    ap.add_argument("--top", type=int, default=8)
    return ap


def run_count_gate(name, baseline_rel, scan, label, extra_args=None, head=None,
                   hard=False) -> int:
    """计数门的 main 流程：解析参数 → 扫描 → 写基线 / 棘轮比对 → 报告。

    五道计数门（unwrap / linelen / sleep / ignore / unsafe）只有「数什么」不一样，
    其余（扫哪些文件、基线怎么读写、棘轮怎么判、怎么报）完全相同。流程搬到这里，
    各门文件只剩自己的判据与文档字符串——否则七份样板互相雷同，dupe 门第一个不答应。
    """
    ap = argparse.ArgumentParser()
    add_args(ap)
    if extra_args:
        extra_args(ap)
    a = ap.parse_args()
    cur = scan(ROOT, a)
    total = sum(cur.values())
    if head:
        head(cur, a)
    else:
        print(f"{name} count={total}")
    if a.list:
        for r, n in sorted(cur.items(), key=lambda kv: -kv[1])[:a.top]:
            print(f"  {n:3d}  {r}")
        return 0
    bpath = ROOT / baseline_rel
    if a.write:
        write_baseline(bpath, cur)
        print(f"已写基线 {baseline_rel}（{total} {label}）——此后只准减")
        return 0
    if hard:
        # 存量 = 0 的规矩开硬阈：`--write` 不许把它祖父化，否则「重记基线」就成了合法削弱。
        bad = [f"{label} {rel}: {n} 处（硬阈，基线不放行）" for rel, n in sorted(cur.items())]
        return report(name, bad, [])
    base = load_baseline(bpath)
    if not base:
        print("警告：无基线 ⇒ 不判；跑 --write 才会管住存量")
    bad, shrank = [], []
    ratchet(cur, base, label, bad, shrank)
    return report(name, bad, shrank)


def fn_metric_scan(thresholds, metric):
    """按**函数**统计指标：`metric(函数体各行) -> int`，超过每档阈值就给该文件记一笔。

    cyc（圈复杂度）与 nest（嵌套深度）都是「逐函数量一个数」，配平与遍历共用 `iter_fns`，
    各门只剩下自己的量法。
    """
    def scan(root, a):
        out = {key: {} for _lim, key in thresholds}
        for rel in rs_files(root, a.git_tracked):
            if rel.startswith("tests/") or skip_hit(rel, TESTISH):
                continue
            text = read_text(root, rel)
            if not text:
                continue
            lines = mask(text).splitlines()
            for i, end in iter_fns(lines):
                v = metric(lines[i:end + 1])
                for lim, key in thresholds:
                    if v > lim:
                        out[key][rel] = out[key].get(rel, 0) + 1
        return out
    return scan


def run_dicts_gate(name, baseline_rel, scan, parts, head=None, hard=()) -> int:
    """分档计数门（多个 {文件: 计数} 字典）的 main 流程。`parts = [(key, 标签), …]`。"""
    ap = argparse.ArgumentParser()
    add_args(ap)
    a = ap.parse_args()
    cur = scan(ROOT, a)
    summary = " ".join(f"{k}={sum(cur.get(k, {}).values())}" for k, _l in parts)
    if head:
        head(cur, a)
    else:
        print(f"{name} {summary}")
    if a.list:
        for k, _l in parts:
            for r, n in sorted(cur.get(k, {}).items(), key=lambda kv: -kv[1])[:a.top]:
                print(f"  {k:7s} {n:3d}  {r}")
        return 0
    bpath = ROOT / baseline_rel
    if a.write:
        write_baseline(bpath, cur)
        print(f"已写基线 {baseline_rel}（{summary}）——此后只准减")
        return 0
    base = load_baseline(bpath)
    if not base:
        print("警告：无基线 ⇒ 不判；跑 --write 才会管住存量")
    bad, shrank = [], []
    for k, label in parts:
        if k in hard:
            for rel, n in sorted(cur.get(k, {}).items()):
                bad.append(f"{label} {rel}: {n} 处（硬阈，基线不放行）")
            continue
        ratchet(cur.get(k, {}), base.get(k, {}), label, bad, shrank)
    return report(name, bad, shrank)


def regex_scan(rx, per_line=False, prod_only=True, extra_skip=(), masked=True):
    """「每个文件里某模式出现几次」的扫描器——返回可直接交给 `run_count_gate` 的 scan。"""
    def scan(root, a):
        cur = {}
        for rel in rs_files(root, a.git_tracked):
            if prod_only and (rel.startswith("tests/") or skip_hit(rel, TESTISH)):
                continue
            if extra_skip and skip_hit(rel, extra_skip):
                continue
            text = read_text(root, rel)
            if not text:
                continue
            src = mask(text) if masked else text
            n = (sum(1 for ln in src.splitlines() if rx.search(ln)) if per_line
                 else len(rx.findall(src)))
            if n:
                cur[rel] = n
        return cur
    return scan
