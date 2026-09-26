"""碰了就得减门（touch tax）：比棘轮更严厉的那一步。

  棘轮只说「不许变胖」，于是存量能永远躺着：一个 5000 行的上帝文件，只要没人给它加行，它就永远
  是 5000 行，而每个人都在绕着它走。本门规定：**改了已经超阈的文件，就必须把它变小**——
  连「原样不动」都不行（否则正确的重构会被棘轮逼着去关门，而真正的债务永远不动）。

  判据：这次改动碰到的、且在 `god-baseline.json` 里**已超过任一硬阈**的文件，其当前规模
  （file_lines / max_fn_lines / max_type_members 任一维度）必须严格小于基线——没变小就红。
  未超阈的文件不受这条管（仍受 god-gate 棘轮管）。

  CI 传 PR 的 base sha 只判本次改动；本地缺省比 `git diff HEAD`。

用法：python3 -X utf8 scripts/gates/god_touch.py [--base <sha>] [--git-tracked]
"""
import argparse
import importlib.util
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent


def load_god():
    spec = importlib.util.spec_from_file_location(
        "god_gate_for_touch", ROOT / "scripts" / "gates" / "god_gate.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def changed_files(base: str | None) -> set[str]:
    if base:
        cp = subprocess.run(["git", "diff", "--name-only", f"{base}...HEAD"],
                            cwd=str(ROOT), capture_output=True, text=True,
                            encoding="utf-8", errors="replace", shell=False)
    else:
        cp = subprocess.run(["git", "diff", "--name-only", "HEAD"],
                            cwd=str(ROOT), capture_output=True, text=True,
                            encoding="utf-8", errors="replace", shell=False)
    return {l for l in cp.stdout.split() if l.strip()}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default=None,
                    help="对比基准 sha（CI 传 PR base；本地缺省比 HEAD）")
    ap.add_argument("--git-tracked", action="store_true")
    a = ap.parse_args()

    gg = load_god()
    cfg = gg.load_cfg(ROOT, None)
    bpath = ROOT / cfg["baseline"]
    base = gg.load_baseline(bpath)
    files = gg.scan(ROOT, cfg)

    changed = changed_files(a.base)
    lim_f, lim_fn, lim_t = cfg["max_file_lines"], cfg["max_fn_lines"], cfg["max_type_members"]
    bad = []
    touched = 0
    for rel in sorted(changed):
        b = base.get(rel)
        if b is None:
            continue
        over = (b.get("file_lines", 0) > lim_f or b.get("max_fn_lines", 0) > lim_fn
                or b.get("max_type_members", 0) > lim_t)
        if not over:
            continue
        cur = files.get(rel)
        if cur is None:
            continue
        shrank = (cur["file_lines"] < b["file_lines"] or cur["max_fn_lines"] < b["max_fn_lines"]
                  or cur["max_type_members"] < b["max_type_members"])
        touched += 1
        if not shrank:
            bad.append(f"{rel}: 碰了超阈文件却没变小（file {b['file_lines']}→{cur['file_lines']} / "
                       f"fn {b['max_fn_lines']}→{cur['max_fn_lines']} / "
                       f"type {b['max_type_members']}→{cur['max_type_members']}）—必须减小")
    print(f"GOD-TOUCH 改动中超阈文件={touched} 命中={len(bad)}")
    for line in bad[:25]:
        print(f"  ✗ {line}")
    if len(bad) > 25:
        print(f"  …另有 {len(bad) - 25} 条")
    print(f"GOD-TOUCH {'FAIL' if bad else 'OK'}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
