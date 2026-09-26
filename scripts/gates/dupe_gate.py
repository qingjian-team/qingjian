"""重复代码门：同类代码不许**新增**，只报新增、可收紧基线（与 `god_gate.py` 同形态）。

为什么要有它：上帝对象门管「单点过大」，这个门管「多处雷同」——两件不同的事，但都是维护成本的
源头，而且拆上帝对象时最容易顺手复制出一批雷同的 helper（所以两个门要配套）。

**与上游 unified-rx-mcp 的差异（必须记）**：上游调用 Rust 侧 `rx-scan.exe` 出 MinHash 指纹，
那个 exe 在本仓与 CI 上都不存在 ⇒ 若照抄，门会以「引擎不可用 FAIL」把所有人卡死，或被人加个
`try/except` 静默判绿（更糟）。这里改成**纯 stdlib 的 bottom-k MinHash**（`hashlib.blake2b`），
语义与上游一致（同 ng / k / 阈值口径），零依赖、CI 免编译即可跑。

判据：文件对 Jaccard ≥ `--threshold`（默认 0.80）即雷同；**基线里没有的新对 ⇒ 红**。

用法：
  python -X utf8 scripts/gates/dupe_gate.py --list            # 只列当前雷同对
  python -X utf8 scripts/gates/dupe_gate.py --write-baseline   # 记基线（人工过目后提交）
  python -X utf8 scripts/gates/dupe_gate.py                    # 门：新增即红
退出码：0 = 通过；1 = 有新增雷同；2 = 配置/IO 错。
"""
import argparse
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
BASELINE = "docs/review/dupe-baseline.json"
NG, K = 5, 128
DEFAULT_THRESHOLD = 0.80
EXTS = (".rs", ".py")
EXCLUDE_DIRS = {".git", "target", "node_modules", "__pycache__", "dist", "build",
                ".venv", "venv", ".local", "generated"}
# 测试夹具面天然「长得像」（engine/tests/*、平台壳的同类用例）：本门看的是**产品代码**的雷同。
# 但 `crates/*/src/**` 里的 `#[cfg(test)] mod tests` 仍会入册——那部分靠 god_gate 管体量。
EXCLUDE_PREFIX = ("docs/", "assets/", "data/")
LINE_COMMENT = re.compile(r"//.*$", re.M)
BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.S)
STR_LIT = re.compile(r'"(?:\\.|[^"\\])*"')
TOKEN = re.compile(r"[A-Za-z_][A-Za-z0-9_]*|\d+")


def collect(root: pathlib.Path, git_tracked: bool):
    tracked = None
    if git_tracked:
        cp = subprocess.run(["git", "ls-files"], cwd=str(root), capture_output=True, text=True,
                            encoding="utf-8", errors="replace", shell=False)
        tracked = set(cp.stdout.split())
    out = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in EXCLUDE_DIRS]
        for fn in filenames:
            if not fn.endswith(EXTS):
                continue
            fp = pathlib.Path(dirpath) / fn
            rel = fp.relative_to(root).as_posix()
            if rel.startswith(EXCLUDE_PREFIX):
                continue
            if tracked is not None and rel not in tracked:
                continue
            out.append(rel)
    return sorted(out)


def normalize(text: str) -> str:
    """去掉注释与字符串字面量：**字面量不同、骨架相同**才是要抓的雷同。"""
    text = BLOCK_COMMENT.sub(" ", text)
    text = LINE_COMMENT.sub(" ", text)
    return STR_LIT.sub(' "" ', text)


def fingerprint(text: str):
    """bottom-k MinHash：n-gram 哈希取最小的 K 个（与上游 rx-scan sketch 同口径）。"""
    toks = TOKEN.findall(normalize(text))
    if len(toks) < NG:
        return set()
    seen = set()
    for i in range(len(toks) - NG + 1):
        g = " ".join(toks[i:i + NG])
        h = hashlib.blake2b(g.encode("utf-8"), digest_size=8).digest()
        seen.add(int.from_bytes(h, "little"))
    return set(sorted(seen)[:K])


def jaccard(a: set, b: set) -> float:
    if not a or not b:
        return 0.0
    return len(a & b) / len(a | b)


def pairs_above(fps, thr):
    out = {}
    items = [(p, s) for p, s in fps if len(s) >= 16]      # 太小的样本不判（噪声）
    for i in range(len(items)):
        for j in range(i + 1, len(items)):
            sim = jaccard(items[i][1], items[j][1])
            if sim >= thr:
                a, b = sorted((items[i][0], items[j][0]))
                out[f"{a}|{b}"] = round(sim, 3)
    return out


def main(argv):
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=str(ROOT))
    ap.add_argument("--threshold", type=float, default=DEFAULT_THRESHOLD)
    ap.add_argument("--write-baseline", action="store_true")
    ap.add_argument("--list", action="store_true")
    ap.add_argument("--git-tracked", action="store_true")
    a = ap.parse_args()
    root = pathlib.Path(a.root).resolve()
    bpath = root / BASELINE

    rels = collect(root, a.git_tracked)
    fps = []
    for rel in rels:
        try:
            text = (root / rel).read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        s = fingerprint(text)
        if s:
            fps.append((rel, s))
    cur = pairs_above(fps, a.threshold)
    print(f"DUPE-GATE root={root} 文件={len(fps)} 阈值={a.threshold}（ng={NG} k={K}）当前雷同对={len(cur)}")
    for key, sim in sorted(cur.items(), key=lambda kv: -kv[1])[:12]:
        print(f"  {sim:.3f}  {key}")
    if len(cur) > 12:
        print(f"  …另有 {len(cur) - 12} 对")

    if a.list:
        return 0
    if a.write_baseline:
        bpath.parent.mkdir(parents=True, exist_ok=True)
        payload = json.dumps({"policy": "雷同对基线：新增即红（scripts/gates/dupe_gate.py，纯 stdlib "
                                        "bottom-k MinHash）；只登记产品代码面",
                              "threshold": a.threshold, "ng": NG, "k": K,
                              "pairs": sorted(cur)},
                             ensure_ascii=False, indent=1) + "\n"
        tmp = bpath.with_suffix(bpath.suffix + ".tmp")
        tmp.write_text(payload, encoding="utf-8")
        os.replace(tmp, bpath)                      # 原子替换（同 god_gate：中断不留半截）
        print(f"已写基线 {BASELINE}（{len(cur)} 对）——此后只准减")
        return 0
    if not bpath.is_file():
        print(f"警告：无 {BASELINE} ⇒ 先跑 --write-baseline 才会管住存量")
        return 0
    base = set(json.loads(bpath.read_text(encoding="utf-8"))["pairs"])
    new = sorted(set(cur) - base)
    gone = sorted(base - set(cur))
    for k in new:
        print(f"  ✗ 新增雷同对 {cur[k]:.3f}  {k}")
    if gone:
        print(f"  （{len(gone)} 对已消失，可收紧基线）")
    if new:
        print("DUPE-GATE FAIL 新增雷同对 —— 抽公共模块/复用已有实现，别再复制一份")
        return 1
    print("DUPE-GATE OK 无新增雷同对")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
