"""架构约束门：把 `docs/contributing.md` 里那一页「架构约束 / 代码组织」从写给人看变成机器判。

为什么要有它：那些规矩（一个类型一个文件、子模块用目录、不用 glob 导入、Core 不许依赖平台）
每一条都写得很清楚，但没有一条有人或机器在查——只在 review 时靠记忆想起来。
`god_gate.py` 管的是「单点过大」，这个门管的是「放错了地方 / 写法越界」。

判据（R1/R2 只管**新增文件**，R5 是硬规则不走基线，其余走只准减的棘轮）：
  R1 新 .rs 文件必须有 `//!` 文件头（`docs/contributing.md`：新文件都要有）；
  R2 新文件（非 `mod.rs` / 非测试）顶层类型声明 ≤ 1（「一个类型一个文件」）；
  R3 `use …::*` glob 导入 —— 棘轮（`#[cfg(test)]` 与普通测试文件豁免）；
  R4 `foo.rs` 与 `foo/` 并列 —— 棘轮（「子模块用目录」）；
  R5 **硬**：`crates/*` 不许依赖 `apps/*` 的 crate，也不许依赖 OS 特有 crate
     （objc2 / windows / cocoa / gtk / x11 / ibus / fcitx 等）——Core 必须平台无关；
  R6 产品代码里的 `dbg!` / `todo!` / `unimplemented!` / `unreachable!` / 裸 `panic!` —— 棘轮。
     **为什么还要单独扫**：workspace lints 已经 deny 这些，但 clippy 只跑编得到的 target——
     macOS 壳在 Linux runner 上不编、Windows 三件套在 macOS runner 上不编 ⇒ 那部分文件的
     `panic!` 谁也抓不到。这里是纯文本扫描，不看能不能编译，正好补上那个平台盲区。
  R7 装饰性分隔注释（`// ====`）—— 棘轮（pre-commit 只查暂存区，这里补全仓）。

用法：
  python -X utf8 scripts/gates/arch_gate.py --list          # 看看现在违反多少
  python -X utf8 scripts/gates/arch_gate.py --write         # 记基线
  python -X utf8 scripts/gates/arch_gate.py                 # 门
退出码：0 = 通过；1 = 命中；2 = 配置/IO 错。
"""
import argparse
import fnmatch
import json
import os
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
BASELINE = "docs/review/arch-baseline.json"
GOD_BASELINE = "docs/review/god-baseline.json"       # 借它当「已登记文件」名单（判新增）
RS = "**/*.rs"

TYPE_RE = re.compile(r"^(?:#\[[^\]]*\]\s*)?(?:pub(?:\([^)]*\))?\s+)?(struct|enum|trait|union)\s+[A-Za-z_]")
GLOB_RE = re.compile(r"^\s*use\s+[^;{]*::\*\s*;")
DECOR_RE = re.compile(r"^\s*//[/!]?\s*[=─-]{3,}\s*$")
BANNED_RE = re.compile(r"\b(dbg!|todo!|unimplemented!|unreachable!|panic!)\s*[(!{]")
# 平台特有依赖（Core 一侧出现即是「平台无关」这条约束破了）
OS_CRATES = {"objc2", "objc2-foundation", "cocoa", "core-foundation", "core-graphics",
             "windows", "windows-sys", "winapi", "gtk", "gtk4", "x11", "x11-dl", "ibus",
             "fcitx5", "dbus", "winit", "tao", "objc"}
APP_CRATES_PREFIX = ("qingjian-macos", "qingjian-windows", "qingjian-linux", "qingjian-cli")
# 测试面天然用 panic!/unimplemented!，且 clippy 那边也豁免了
TESTISH = ("/tests/", "/examples/", "/benches/", "/tests.rs")


def git(*args):
    return subprocess.run(["git", *args], cwd=str(ROOT), capture_output=True, text=True,
                          encoding="utf-8", errors="replace", shell=False).stdout


def rs_files(git_tracked: bool):
    tracked = set(git("ls-files").split()) if git_tracked else None
    out = []
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in {".git", "target", "node_modules", "__pycache__"}]
        for fn in filenames:
            if not fn.endswith(".rs"):
                continue
            rel = (pathlib.Path(dirpath) / fn).relative_to(ROOT).as_posix()
            if tracked is not None and rel not in tracked:
                continue
            out.append(rel)
    return sorted(out)


def is_testish(rel: str) -> bool:
    return any(t in rel for t in TESTISH) or rel.startswith("tests/")


def scan(rels):
    """返回 (glob, banned, decor, types, header) 各 {文件: 计数/是否}，以及并列清单。"""
    glob, banned, decor, types, nheader = {}, {}, {}, {}, {}
    dirs = {os.path.dirname(r) for r in rels}
    siblings = []
    for rel in rels:
        try:
            lines = (ROOT / rel).read_text(encoding="utf-8", errors="replace").splitlines()
        except OSError:
            continue
        if not rel.endswith("mod.rs"):
            stem = rel[:-3]
            if (ROOT / (stem + ".rs")).is_file() and os.path.isdir(ROOT / stem):
                siblings.append(rel)
        head = next((ln for ln in lines if ln.strip() and not ln.lstrip().startswith("#![")), "")
        if not head.lstrip().startswith("//!"):
            nheader[rel] = 1
        if not is_testish(rel) and not rel.endswith("mod.rs"):
            n = 0
            for ln in lines:
                if TYPE_RE.match(ln):
                    n += 1
            if n:
                types[rel] = n
        g = b = d = 0
        for i, ln in enumerate(lines):
            ctx = "\n".join(lines[max(0, i - 3):i])
            if GLOB_RE.match(ln) and "#[cfg(test)]" not in ctx and not is_testish(rel):
                g += 1
            if BANNED_RE.search(ln) and not is_testish(rel):
                near = "\n".join(lines[max(0, i - 2):i + 1])
                if "allow(" not in near:
                    b += 1
            if DECOR_RE.match(ln):
                d += 1
        if g:
            glob[rel] = g
        if b:
            banned[rel] = b
        if d:
            decor[rel] = d
    return {"glob": glob, "banned": banned, "decor": decor, "types": types,
            "no_header": nheader, "siblings": siblings}


def dep_violations():
    """R5（硬）：crates/* 不许依赖 apps/* 的 crate，也不许依赖 OS 特有 crate。"""
    out = []
    app_crates = set()
    for line in git("ls-files", "apps/*/Cargo.toml", "apps/*/*/Cargo.toml").split():
        txt = (ROOT / line).read_text(encoding="utf-8", errors="replace") if (ROOT / line).is_file() else ""
        for m in re.finditer(r"^name\s*=\s*\"([^\"]+)\"", txt, re.M):
            app_crates.add(m.group(1))
    for line in git("ls-files", "crates/*/Cargo.toml").split():
        p = ROOT / line
        if not p.is_file():
            continue
        # 只查**无条件**的 `[dependencies]` 段：`[target.'cfg(windows)'.dependencies]` 是
        # 平台后端（例如 render 用 DirectWrite 列字族），crate 本身仍然跨平台，
        # 把条件依赖也算违规会逼人把正当写法改成绕过。`[dev-dependencies]` 同理不算。
        txt = p.read_text(encoding="utf-8", errors="replace")
        sec = ""
        for blk in re.split(r"^\[", txt, flags=re.M)[1:]:
            if blk.startswith("dependencies]"):
                sec = blk
                break
        for m in re.finditer(r"^(?:qingjian-)?([A-Za-z0-9_.-]+)\s*=", sec, re.M):
            name = m.group(1)
            if name in OS_CRATES or name in app_crates:
                out.append(f"{line}: 无条件依赖 {name}")
    return sorted(set(out))


def ratchet(cur: dict, base: dict, label: str, bad, shrank):
    for rel, v in sorted(cur.items()):
        bv = base.get(rel, 0)
        if v > bv:
            bad.append(f"R·{label} {rel}: {bv} → {v}（新增 {v - bv} 处，只准减）")
        elif v < bv:
            shrank.append(f"{label} {rel}: {bv} → {v}（可收紧）")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--git-tracked", action="store_true")
    ap.add_argument("--write", action="store_true")
    ap.add_argument("--list", action="store_true")
    a = ap.parse_args()
    bpath = ROOT / BASELINE
    files = rs_files(a.git_tracked)
    cur = scan(files)
    dep = dep_violations()

    known = set()
    gb = ROOT / GOD_BASELINE
    if gb.is_file():
        known = set(json.loads(gb.read_text(encoding="utf-8")))
    new_files = [f for f in files if f not in known]

    print(f"ARCH-GATE 文件={len(files)}（新增 {len(new_files)}）glob={sum(cur['glob'].values())} "
          f"禁词={sum(cur['banned'].values())} 装饰注释={sum(cur['decor'].values())} "
          f"并列={len(cur['siblings'])} 无文件头={len(cur['no_header'])} 依赖越界={len(dep)}")
    if a.list:
        for k in ("glob", "banned", "decor", "types"):
            top = sorted(cur[k].items(), key=lambda kv: -kv[1])[:8]
            for r, v in top:
                print(f"  {k:8s} {v:3d}  {r}")
        for r in cur["siblings"][:8]:
            print(f"  sibling      {r}")
        for r in dep[:8]:
            print(f"  dep         {r}")
        return 0

    if a.write:
        bpath.parent.mkdir(parents=True, exist_ok=True)
        payload = json.dumps(cur, ensure_ascii=False, indent=1) + "\n"
        tmp = bpath.with_suffix(bpath.suffix + ".tmp")
        tmp.write_text(payload, encoding="utf-8")
        os.replace(tmp, bpath)
        print(f"已写基线 {BASELINE}——此后只准减")
        return 0

    base = json.loads(bpath.read_text(encoding="utf-8")) if bpath.is_file() else {}
    if not base:
        print(f"警告：无 {BASELINE} ⇒ 只对新增文件与依赖方向判；跑 --write 才会管住存量")
    bad, shrank = [], []
    for label in ("glob", "banned", "decor"):
        ratchet(cur[label], base.get(label, {}), label, bad, shrank)
    bsib = set(base.get("siblings", []))
    for r in cur["siblings"]:
        if r not in bsib:
            bad.append(f"R4 {r}: `foo.rs` 与 `foo/` 并列（子模块用目录，别两套并存）")
    if dep:
        for d in dep:
            bad.append(f"R5 {d} —— Core 必须平台无关，不许依赖壳或 OS 特有 crate")
    # 新文件的两条硬规矩
    for r in new_files:
        if r in cur["no_header"]:
            bad.append(f"R1 {r}: 新文件缺 `//!` 文件头")
        if cur["types"].get(r, 0) > 1:
            bad.append(f"R2 {r}: 新文件有 {cur['types'][r]} 个顶层类型（一个类型一个文件）")
    for line in bad[:25]:
        print(f"  ✗ {line}")
    if len(bad) > 25:
        print(f"  …另有 {len(bad) - 25} 条")
    if shrank:
        print(f"  （{len(shrank)} 条可收紧，跑 --write 更新）")
    print(f"ARCH-GATE {'FAIL' if bad else 'OK'} 命中={len(bad)}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
