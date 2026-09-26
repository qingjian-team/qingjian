"""统一门禁入口（本仓版；形态取自 unified-rx-mcp 的 `scripts/local_gate.py`）。

一条命令跑完**与 CI 同一套**审核（CI 逐条显式调用同一批脚本 ⇒ 漂移由
`gate_selftest.py` 的 S1 锁死）。本 PR 版本只挂自己的门——其余门由同系列配套 PR 逐批加入（拆分自一个捆绑 PR，见其关闭说明）：

  python -X utf8 scripts/gates/gate.py            # 全门（合入前跑；含 cargo）
  python -X utf8 scripts/gates/gate.py --fast     # 快门（秒级，pre-commit 用）
  python -X utf8 scripts/gates/gate.py --list     # 列出步骤
  python -X utf8 scripts/gates/gate.py --only god-gate,dupe-gate
  python -X utf8 scripts/gates/gate.py --no-cargo # 无 Rust 工具链时（**显式**，不静默）
  python -X utf8 scripts/gates/gate.py --write    # 重记全部基线（拆完一块后跑；要人工过目）
  QJ_GATE_FORCE_FAIL=god-gate …                   # 自检：注入失败，验证门是真门

退出码：0 = 全绿；1 = 有门红；2 = 用法/环境错。
纪律：cargo 缺失默认 **FAIL 不静默**——确需跳过必须显式 `--no-cargo`。
"""
import os
import shutil
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
PY = sys.executable
CARGO = shutil.which("cargo") or "cargo"
G = "scripts/gates/"

# (名字, argv, 档位, 说明)——与 .github/workflows/gates.yml 同源，漂移由 gate_selftest.py 拦。
STEPS = [
    ("arch-gate", [PY, "-X", "utf8", G + "arch_gate.py", "--git-tracked"], "fast",
     "架构约束（新文件文件头与一类型一文件 / glob 导入 / 目录并列 / Core 依赖方向 / 禁词）"),
    ("dupe-gate", [PY, "-X", "utf8", G + "dupe_gate.py", "--git-tracked"], "fast",
     "重复代码（雷同对新增即红；纯 stdlib MinHash）"),
    ("unwrap-gate", [PY, "-X", "utf8", G + "unwrap_gate.py", "--git-tracked"], "fast",
     "unwrap/expect 滥用（产品代码，棘轮只准减）"),
    ("linelen-gate", [PY, "-X", "utf8", G + "linelen_gate.py", "--git-tracked"], "fast",
     "超长行 >200 字符（棘轮只准减）"),
    ("sleep-gate", [PY, "-X", "utf8", G + "sleep_gate.py", "--git-tracked"], "fast",
     "thread::sleep（产品代码，棘轮只准减）"),
    ("ignore-gate", [PY, "-X", "utf8", G + "ignore_gate.py", "--git-tracked"], "fast",
     "忽略结果 let _ =（产品代码，棘轮只准减）"),
    ("unsafe-gate", [PY, "-X", "utf8", G + "unsafe_gate.py", "--git-tracked"], "fast",
     "unsafe 块/声明（排除 FFI 与平台壳，棘轮只准减）"),
    ("cyc-gate", [PY, "-X", "utf8", G + "cyc_gate.py", "--git-tracked"], "fast",
     "函数圈复杂度 >15 / >50（棘轮只准减）"),
    ("nest-gate", [PY, "-X", "utf8", G + "nest_gate.py", "--git-tracked"], "fast",
     "嵌套深度 >5 棘轮 / >8 硬禁（箭头代码）"),
    ("args-gate", [PY, "-X", "utf8", G + "args_gate.py", "--git-tracked"], "fast",
     "函数形参 >7（硬禁，存量 0）"),
    ("log-gate", [PY, "-X", "utf8", G + "log_gate.py", "--git-tracked"], "fast",
     "println!/eprintln! 而非 tracing（排除 cli/tools，棘轮）"),
    ("trait-gate", [PY, "-X", "utf8", G + "trait_gate.py", "--git-tracked"], "fast",
     "上帝接口 trait 方法数（>15 棘轮 / >40 硬禁）"),
    ("prefix-gate", [PY, "-X", "utf8", G + "prefix_gate.py", "--git-tracked"], "fast",
     "文件名前缀分组（硬禁，存量 0；该收进目录）"),
    ("cargo-gate", [PY, "-X", "utf8", G + "cargo_gate.py", "--git-tracked"], "fast",
     "Cargo.toml 约定（crates 版本统一 / 壳写死版本 / 不用 anyhow，全硬判）"),
    ("lang-gate", [PY, "-X", "utf8", G + "lang_gate.py", "--git-tracked"], "fast",
     "单辅助语言（不许 Vec<Translation>/HashMap<Lang,_>，硬判）"),
    ("ident-gate", [PY, "-X", "utf8", G + "ident_gate.py", "--git-tracked"], "fast",
     "标识符英文 / #[error] 文案英文（硬判）"),
    ("super-gate", [PY, "-X", "utf8", G + "super_gate.py", "--git-tracked"], "fast",
     "use super::super:: 绕父模块转手（棘轮只准减）"),
    ("testsize-gate", [PY, "-X", "utf8", G + "testsize_gate.py", "--git-tracked"], "fast",
     "单文件内嵌测试 >200 行（该搬到 tests.rs，棘轮）"),
    ("selftest", [PY, "-X", "utf8", G + "gate_selftest.py"], "fast",
     "门禁自检（门不许被悄悄削弱/绕过）"),
    # 自检查的是门「在不在」，这道查的是门「是不是空的」：给每道门注入一次最小违规，它必须红。
    # 归 **full** 档：每道门要跑两遍（注入前 / 注入后）≈ 24 秒，放进 pre-commit 会把每次提交
    # 拖到 36 秒。CI 的 gates.yml 显式跑它，合入前的 `gate.py`（全门）也会跑。
    ("gate-probe", [PY, "-X", "utf8", G + "gate_probe.py"], "full",
     "门是真门（逐门注入违规，不红 = 空门）"),
    # 下面两步的脚本**不在本 PR 里**（属另一条 CI 分支的工作，尚未合入 main）⇒ 脚本不存在时
    # 显式 SKIP 并说明，不静默判绿、也不把环境差异当红。
    ("unsafe", [PY, "-X", "utf8", "scripts/unsafe_audit.py"], "fast", "unsafe 增量门"),
    ("todos", [PY, "-X", "utf8", "scripts/detect_todos.py"], "fast", "TODO/FIXME 残留门"),
    # 注：CI 形状锁（`scripts/ci_shape_lock.sh`）**不在**这里跑——它是 bash 脚本，
    # Windows 上从 PowerShell 起的子进程里没有 bash ⇒ 门会以「环境问题」误红。
    # 它已在 `.githooks/pre-commit` 与 `quality.yml` 各跑一次，不缺覆盖。
    ("cargo-fmt", [CARGO, "fmt", "--all", "--check"], "full", "rustfmt"),
    ("clippy", [CARGO, "clippy", "--workspace", "--exclude", "qingjian-macos",
                "--all-targets", "--locked", "--", "-D", "warnings"], "full",
     "clippy 零告警（非 Apple 平台排除 IMK 壳，与 ci.yml 同款）"),
    ("cargo-test", [CARGO, "test", "--workspace", "--exclude", "qingjian-macos", "--locked"], "full",
     "Rust 全量测试"),
]

# 写基线步骤（--write）：顺序有讲究——先 god 基线，再据此重算欠账台账。
WRITE_STEPS = [
    ("dupe-baseline", [PY, "-X", "utf8", G + "dupe_gate.py", "--git-tracked", "--write-baseline"]),
    ("arch-baseline", [PY, "-X", "utf8", G + "arch_gate.py", "--git-tracked", "--write"]),
    ("unwrap-baseline", [PY, "-X", "utf8", G + "unwrap_gate.py", "--git-tracked", "--write"]),
    ("linelen-baseline", [PY, "-X", "utf8", G + "linelen_gate.py", "--git-tracked", "--write"]),
    ("sleep-baseline", [PY, "-X", "utf8", G + "sleep_gate.py", "--git-tracked", "--write"]),
    ("ignore-baseline", [PY, "-X", "utf8", G + "ignore_gate.py", "--git-tracked", "--write"]),
    ("unsafe-baseline", [PY, "-X", "utf8", G + "unsafe_gate.py", "--git-tracked", "--write"]),
    ("cyc-baseline", [PY, "-X", "utf8", G + "cyc_gate.py", "--git-tracked", "--write"]),
    ("nest-baseline", [PY, "-X", "utf8", G + "nest_gate.py", "--git-tracked", "--write"]),
    ("log-baseline", [PY, "-X", "utf8", G + "log_gate.py", "--git-tracked", "--write"]),
    ("super-baseline", [PY, "-X", "utf8", G + "super_gate.py", "--git-tracked", "--write"]),
    ("testsize-baseline", [PY, "-X", "utf8", G + "testsize_gate.py", "--git-tracked", "--write"]),
    ("trait-baseline", [PY, "-X", "utf8", G + "trait_gate.py", "--git-tracked", "--write"]),
]


def run(argv, name, force_fail=None):
    if force_fail == name:
        return False, 0.0, f"[自检注入] QJ_GATE_FORCE_FAIL={name}"
    env = dict(os.environ)
    env["PYTHONUTF8"] = "1"
    t0 = time.time()
    cp = subprocess.run(argv, cwd=ROOT, env=env, shell=False, capture_output=True,
                        text=True, encoding="utf-8", errors="replace", timeout=3600)
    out = (cp.stdout or "") + (("\n" + cp.stderr) if cp.stderr else "")
    return cp.returncode == 0, time.time() - t0, out


def main(argv):
    if "--list" in argv:
        for n, _a, tier, why in STEPS:
            print(f"{tier:4s} {n:12s} {why}")
        return 0
    write = "--write" in argv
    want_fast = "--fast" in argv
    no_cargo = "--no-cargo" in argv
    only = None
    for i, a in enumerate(argv):
        if a == "--only" and i + 1 < len(argv):
            only = {s.strip() for s in argv[i + 1].split(",") if s.strip()}
    force_fail = os.environ.get("QJ_GATE_FORCE_FAIL")

    if write:
        print("== 重记基线（跑完请 git diff 过目：基线变松 = 门变松） ==")
        bad = []
        for name, cmd in WRITE_STEPS:
            ok, secs, out = run(cmd, name, force_fail)
            print(f"{'OK  ' if ok else 'FAIL'} {name:14s} {secs:6.1f}s")
            if not ok:
                bad.append(name)
                print("  " + "\n  ".join(out.strip().splitlines()[-12:]))
        print(f"GATE-WRITE {'OK' if not bad else 'FAIL'} {bad}")
        return 1 if bad else 0

    rows, failed, skipped = [], [], []
    for name, cmd, tier, why in STEPS:
        if want_fast and tier != "fast":
            continue
        if only is not None and name not in only:
            continue
        if name in ("unsafe", "todos") and not os.path.isfile(os.path.join(ROOT, cmd[-1])):
            print(f"SKIP {name:12s} {cmd[-1]} 不在本仓（另一条 CI 分支的工作，尚未合入）——"
                  f"不静默判绿，合入后自动生效")
            skipped.append(name)
            continue
        if no_cargo and name in ("cargo-fmt", "clippy", "cargo-test"):
            print(f"SKIP {name:12s} --no-cargo（显式跳过；红线语义：不算全绿）")
            skipped.append(name)
            continue
        if name in ("cargo-fmt", "clippy", "cargo-test") and not shutil.which("cargo"):
            print(f"FAIL {name:12s} cargo 不可用——不静默降级（要跳过请显式 --no-cargo）")
            failed.append(name)
            continue
        ok, secs, out = run(cmd, name, force_fail)
        rows.append((name, ok, secs))
        print(f"{'OK  ' if ok else 'FAIL'} {name:12s} {secs:6.1f}s {why}")
        if not ok:
            failed.append(name)
            tail = out.strip().splitlines()[-12:]
            print("  " + "\n  ".join(tail))
    total = sum(r[2] for r in rows)
    print(f"GATE {'OK' if not failed else 'FAIL'} steps={len(rows)} "
          f"skipped={skipped or '[]'} failed={failed} total={total:.1f}s")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
