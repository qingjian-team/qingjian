"""类型跨度门：抓**跨文件的上帝类型**（`god_gate.py` 抓不到的那一半）。

为什么还要一个门：`docs/contributing.md` 要求「大类型的 `impl` 按职责拆成子模块」，拆完之后
每个文件都只有一两百行，`god_gate` 的三项指标全绿——但那个**类型本身**可能有 80 个方法、
散在 12 个文件里。这恰恰是最典型的「上帝对象」：职责发散、改一处要读懂十几个文件。
按文件量规模永远抓不到它，必须按**类型名聚合**。

判据（每个 crate 内按类型名聚合 `impl` 块）：
  · `methods` —— 该类型所有 `impl` 块里的方法总数（含 trait impl）；
  · `files`   —— 这些 `impl` 块散在几个文件里（职责发散度）；
  · 两项各自有阈值 + 棘轮基线（只准减）+ 可单独开硬阈。

用法：
  python -X utf8 scripts/gates/type_span_gate.py --list            # 看看谁最发散
  python -X utf8 scripts/gates/type_span_gate.py --write-baseline   # 记基线
  python -X utf8 scripts/gates/type_span_gate.py --git-tracked      # 门
退出码：0 = 通过；1 = 超标/变胖；2 = 配置错。
"""
import argparse
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
CFG_REL = "scripts/gates/god.gate.json"          # 与上帝对象门共用一份配置（阈值同族）
DEFAULT = {
    "max_type_methods_total": 40,
    "max_type_impl_files": 8,
    "min_methods_to_track": 10,      # 方法太少的不入册（噪声；基线体积也靠它压住）
    "type_span_hard_threshold": False,
    "include": ["**/*.rs"],
    "exclude": ["**/target/**", "**/.git/**"],
    # 键名**必须**叫 `type_span_baseline` 而不是 `baseline`：配置是从 god.gate.json 合并来的，
    # 那边有一份 `baseline`（上帝对象基线）。共用键名会把类型基线写进 god-baseline.json，
    # 把上帝对象基线整份覆盖掉（实测已踩：随后 god-gate 全量误红、arch 的「已登记文件」名单
    # 也跟着失效）。宁可多一个键，也不要两个门共用一份基线文件。
    "type_span_baseline": "docs/review/type-span-baseline.json",
}
IMPL_RE = re.compile(r"^\s*impl(?P<gen>\s*<[^>]*>)?\s+"
                     r"(?:(?P<trait>[A-Za-z_][A-Za-z0-9_:]*)(?:\s*<[^>]*>)?\s+for\s+)?"
                     r"(?P<ty>[A-Za-z_][A-Za-z0-9_]*)")
FN_RE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+|async\s+|unsafe\s+|extern\s+)*fn\s+[A-Za-z_]")


def load_gate():
    spec = importlib.util.spec_from_file_location("god_gate_for_span", ROOT / "scripts" / "gates" / "god_gate.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def load_cfg(root: pathlib.Path) -> dict:
    cfg = dict(DEFAULT)
    p = root / CFG_REL
    if p.is_file():
        cfg.update({k: v for k, v in json.loads(p.read_text(encoding="utf-8")).items() if k in cfg})
    return cfg


def unit_of(rel: str) -> str:
    """聚合单元：crate 或壳（不跨 crate 聚合——同名小类型在不同 crate 里很常见）。"""
    parts = rel.split("/")
    if parts[0] in ("crates", "apps") and len(parts) > 1:
        return "/".join(parts[:2])
    return parts[0]


def scan(root: pathlib.Path, cfg: dict, git_tracked: bool):
    gg = load_gate()
    import fnmatch

    def hit(pat, rel):
        return fnmatch.fnmatch(rel, pat) or (pat.startswith("**/") and fnmatch.fnmatch(rel, pat[3:]))

    tracked = None
    if git_tracked:
        cp = subprocess.run(["git", "ls-files"], cwd=str(root), capture_output=True, text=True,
                            encoding="utf-8", errors="replace", shell=False)
        tracked = set(cp.stdout.split())
    out: dict[str, dict] = {}
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in {".git", "target", "node_modules", "__pycache__"}]
        for fn in filenames:
            if not fn.endswith(".rs"):
                continue
            fp = pathlib.Path(dirpath) / fn
            rel = fp.relative_to(root).as_posix()
            if any(hit(p, rel) for p in cfg["exclude"]):
                continue
            if not any(hit(p, rel) for p in cfg["include"]):
                continue
            if tracked is not None and rel not in tracked:
                continue
            try:
                src = fp.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            masked = gg._mask(src)
            lines = masked.splitlines()
            for i, line in enumerate(lines):
                m = IMPL_RE.match(line)
                if not m:
                    continue
                depth, started, end = 0, False, i
                for j in range(i, len(lines)):
                    depth += lines[j].count("{") - lines[j].count("}")
                    if "{" in lines[j]:
                        started = True
                    if started and depth <= 0:
                        end = j
                        break
                depth, methods = 0, 0
                for j in range(i, end + 1):
                    d0 = depth
                    depth += lines[j].count("{") - lines[j].count("}")
                    if j == i:
                        continue
                    if d0 == 1 and FN_RE.match(lines[j]):
                        methods += 1
                if methods == 0:
                    continue
                key = f"{unit_of(rel)}|{m.group('ty')}"
                e = out.setdefault(key, {"methods": 0, "impls": 0, "files": []})
                e["methods"] += methods
                e["impls"] += 1
                if rel not in e["files"]:
                    e["files"].append(rel)
    return {k: v for k, v in out.items()
            if v["methods"] >= cfg["min_methods_to_track"] or len(v["files"]) >= 3}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=str(ROOT))
    ap.add_argument("--git-tracked", action="store_true")
    ap.add_argument("--write-baseline", action="store_true")
    ap.add_argument("--list", action="store_true")
    ap.add_argument("--top", type=int, default=15)
    a = ap.parse_args()
    root = pathlib.Path(a.root).resolve()
    cfg = load_cfg(root)
    bpath = root / cfg["type_span_baseline"]
    cur = scan(root, cfg, a.git_tracked)
    lim_m, lim_f = cfg["max_type_methods_total"], cfg["max_type_impl_files"]
    print(f"TYPE-SPAN root={root} 入册类型={len(cur)} 阈值: methods>{lim_m} files>{lim_f}"
          f"（追踪下限 methods≥{cfg['min_methods_to_track']}）")
    ranked = sorted(cur.items(), key=lambda kv: (-kv[1]["methods"], -len(kv[1]["files"])))
    for k, v in ranked[:a.top]:
        print(f"  methods {v['methods']:4d}  impl {v['impls']:3d}块  散在 {len(v['files']):2d} 文件  {k}")
    if a.list:
        return 0
    if a.write_baseline:
        bpath.parent.mkdir(parents=True, exist_ok=True)
        payload = json.dumps({k: {"methods": v["methods"], "files": len(v["files"])}
                              for k, v in sorted(cur.items())},
                             ensure_ascii=False, indent=1) + "\n"
        tmp = bpath.with_suffix(bpath.suffix + ".tmp")
        tmp.write_text(payload, encoding="utf-8")
        os.replace(tmp, bpath)
        print(f"已写基线 {cfg['type_span_baseline']}（{len(cur)} 个类型）——此后只准减")
        return 0
    base = json.loads(bpath.read_text(encoding="utf-8")) if bpath.is_file() else {}
    hard = cfg["type_span_hard_threshold"]
    bad, shrank = [], []
    for k, v in sorted(cur.items()):
        m, f = v["methods"], len(v["files"])
        b = base.get(k)
        if b is None:
            if m > lim_m:
                bad.append(f"{k}: methods={m} > {lim_m}（新增类型，无基线）")
            if f > lim_f:
                bad.append(f"{k}: files={f} > {lim_f}（新增类型，无基线）")
            continue
        for name, val, lim, bv in (("methods", m, lim_m, b.get("methods", 0)),
                                   ("files", f, lim_f, b.get("files", 0))):
            if hard and val > lim:
                bad.append(f"{k}: {name} {bv} → {val} > 硬阈 {lim}（基线不放行）")
            elif val > bv:
                bad.append(f"{k}: {name} {bv} → {val}（不许变胖）")
            elif val < bv:
                shrank.append(f"{k}: {name} {bv} → {val}")
    if not base and not a.write_baseline:
        print("警告：尚无基线 ⇒ 只对新增类型判阈值；跑 --write-baseline 才会管住存量")
    for line in bad[:25]:
        print(f"  ✗ {line}")
    if len(bad) > 25:
        print(f"  …另有 {len(bad) - 25} 条")
    if shrank:
        print(f"  （{len(shrank)} 条可收紧，跑 --write-baseline 更新）")
    print(f"TYPE-SPAN {'FAIL' if bad else 'OK'} 超标/变胖={len(bad)}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
