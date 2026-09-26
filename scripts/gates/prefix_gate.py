"""文件名前缀分组门：`docs/contributing.md`「**绝不用文件名前缀分组**」的机器版。

  规矩原文：`key_event.rs` + `key_outcome.rs` → `key/mod.rs` + `key/{event,outcome}.rs`，
  哪怕没有 `key.rs` 这个共同父文件；判断标准是「两个及以上文件名共享一段前缀且同属一个概念，
  就收进以那段前缀命名的目录」。

  为什么值得单独一道门：前缀分组是**目录结构**上的债，跟文件多大、函数多长完全无关——
  `god_gate` 三项全绿的一个目录，也可能是一堆 `foo_*.rs` 摊平的垃圾抽屉。按文件统计的门
  永远抓不到它，得按**目录 + 文件名**看。

  判据：同一目录下，去掉 `mod.rs`/`lib.rs`/`main.rs`/`tests.rs` 之后，两个及以上 `.rs` 文件的
  文件名以同一段下划线前缀开头，而**该目录本身不叫那段前缀** ⇒ 命中。

  存量实测 = 0 ⇒ **硬判，没有基线**（规矩原文就是「绝不用」，留基线等于给它发豁免）。

用法：python3 -X utf8 scripts/gates/prefix_gate.py [--git-tracked] [--list] [--write]
"""
import os
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gates_common as gc  # noqa: E402

SKIP = {"mod", "lib", "main", "tests"}


def scan(root, a):
    by_dir: dict[str, list[str]] = {}
    for rel in gc.rs_files(root, a.git_tracked):
        if rel.startswith("tests/") or gc.skip_hit(rel, gc.TESTISH):
            continue
        by_dir.setdefault(os.path.dirname(rel), []).append(os.path.basename(rel)[:-3])
    cur = {}
    for d, stems in sorted(by_dir.items()):
        groups: dict[str, list[str]] = {}
        for s in stems:
            if s in SKIP or "_" not in s:
                continue
            groups.setdefault(s.split("_")[0], []).append(s)
        base = os.path.basename(d)
        for prefix, ss in sorted(groups.items()):
            if len(ss) >= 2 and base != prefix:
                cur.setdefault(d, []).append(f"{prefix}_* → {len(ss)} 个（{', '.join(sorted(ss))}）")
    return cur


def main() -> int:
    import argparse
    ap = gc.add_args(argparse.ArgumentParser())
    a = ap.parse_args()
    cur = scan(gc.ROOT, a)
    total = sum(len(v) for v in cur.values())
    print(f"PREFIX-GATE 前缀分组目录={len(cur)} 组={total}（**硬判**：存量 = 0，无基线）")
    if a.list:
        for d, items in sorted(cur.items(), key=lambda kv: -len(kv[1]))[:a.top]:
            for it in items:
                print(f"  {it}   [{d}]")
        return 0
    # 存量本来就是 0 ⇒ 走硬规则，不设基线：`--write` 重记基线不该能把这条规矩祖父化。
    bad = [f"{d}: {it}（收进 {it.split('_')[0]}/ 目录）" for d, items in sorted(cur.items())
           for it in items]
    return gc.report("PREFIX-GATE", bad, [])


if __name__ == "__main__":
    sys.exit(main())
