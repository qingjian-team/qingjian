"""上帝接口（trait）门：单个 trait **定义**的方法数。

  一个 trait 方法越多，实现方要填的坑越多、越像「胖接口」，违反「小契约」原则。按 crate 内
  trait 名聚合（同名小 trait 在不同 crate 里很常见，不合并）。只判 trait **定义**本身——
  它的 impl 散落在多少文件由本仓已有的 type-span 门管，这里不重复，故与本仓其它门不重叠。

  X1 trait 方法数 >15 —— 棘轮（只准减）；>40 —— 硬禁止（失去小契约意义，出现即红）。

用法：python3 -X utf8 scripts/gates/trait_gate.py [--git-tracked] [--list] [--write]
"""
import argparse
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gates_common as gc  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
BASELINE = "docs/review/trait-baseline.json"
LIM_RATCHET = 15
LIM_HARD = 40
TRAIT_RE = re.compile(r"^\s*(?:pub\s+)?(?:unsafe\s+)?trait\s+([A-Za-z_]\w*)")
FN_RE = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+|async\s+|unsafe\s+|extern\s+)*fn\s+[A-Za-z_]")


def unit_of(rel: str) -> str:
    parts = rel.split("/")
    if parts[0] in ("crates", "apps") and len(parts) > 1:
        return "/".join(parts[:2])
    return parts[0]


def scan(root: pathlib.Path, git_tracked: bool):
    out = {}
    for rel in gc.rs_files(root, git_tracked):
        text = gc.read_text(root, rel)
        if not text:
            continue
        lines = gc.mask(text).splitlines()
        for i, line in enumerate(lines):
            m = TRAIT_RE.match(line)
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
            methods = 0
            for j in range(i, end + 1):
                if FN_RE.match(lines[j]):
                    methods += 1
            if methods == 0:
                continue
            key = f"{unit_of(rel)}|{m.group(1)}"
            e = out.setdefault(key, {"methods": 0, "files": []})
            e["methods"] = max(e["methods"], methods)   # 同一 trait 可能分块定义，取最大
            if rel not in e["files"]:
                e["files"].append(rel)
    return out


def main() -> int:
    ap = gc.add_args(argparse.ArgumentParser())
    a = ap.parse_args()
    bpath = ROOT / BASELINE
    cur = scan(ROOT, a.git_tracked)
    print(f"TRAIT-GATE 入册={len(cur)} 阈值: >{LIM_RATCHET} 棘轮 / >{LIM_HARD} 硬禁")
    ranked = sorted(cur.items(), key=lambda kv: -kv[1]["methods"])
    for k, v in ranked[:a.top]:
        print(f"  methods {v['methods']:4d}  散在 {len(v['files']):2d} 文件  {k}")
    if a.list:
        return 0
    if a.write:
        gc.write_baseline(bpath, {k: v["methods"] for k, v in sorted(cur.items())})
        print(f"已写基线 {BASELINE}（{len(cur)} 个 trait）——此后只准减")
        return 0
    base = gc.load_baseline(bpath)
    if not base:
        print("警告：无基线 ⇒ 只对新增 trait 判阈值；跑 --write 才会管住存量")
    bad, shrank = [], []
    for k, v in sorted(cur.items()):
        m = v["methods"]
        b = base.get(k)
        if b is None:
            if m > LIM_RATCHET:
                bad.append(f"{k}: methods={m} > {LIM_RATCHET}（新增 trait，无基线）")
            continue
        if m > LIM_HARD:
            bad.append(f"{k}: methods {b} → {m} > 硬阈 {LIM_HARD}（基线不放行）")
        elif m > b:
            bad.append(f"{k}: methods {b} → {m}（不许变胖）")
        elif m < b:
            shrank.append(f"{k}: methods {b} → {m}")
    if not base:
        print("警告：尚无基线 ⇒ 只拦新增；跑 --write 才会管住存量")
    for line in bad[:25]:
        print(f"  ✗ {line}")
    if len(bad) > 25:
        print(f"  …另有 {len(bad) - 25} 条")
    if shrank:
        print(f"  （{len(shrank)} 条可收紧，跑 --write 更新）")
    print(f"TRAIT-GATE {'FAIL' if bad else 'OK'} 超标/变胖={len(bad)}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
