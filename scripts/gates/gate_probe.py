r"""门是真门吗？——给每一道门注入一次最小违规，它**必须**转红。

为什么要有它：`gate_selftest.py` 查的是门**在不在**（workflow 里有没有那一行、阈值有没有放宽），
查不出门**是不是空的**。空门的成因很实在：

  - 正则写错。实测：`#\[error\(\s*(?:"[^"]*")?[^")]*[一-鿿]` 的可选组会贪婪吃掉整个字符串，
    之后再要求 `[^")]*` + 中文 ⇒ 永远匹配不上。门看着在跑、CI 挂着绿勾，其实一次都没判过。
  - 路径过滤没匹配到任何文件（相对路径 vs 带前导斜杠的片段）。
  - 判据依赖的名单/基线取错了，永远取到空集。

这三类都不会让任何测试变红。本文件用**动态**办法补上：造一个最小违规，跑门，断言它红。
注入前那道门是绿的、注入后红了，才算「真门」。

用法：python -X utf8 scripts/gates/gate_probe.py [--only god-gate,…] [--keep]
退出码：0 = 每道门都能被触发；1 = 有门注入了也不红（空门或判据失效）。
"""
import re
import shutil
import subprocess
import sys
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
GATES = ROOT / "scripts" / "gates"
PROBE = GATES / "_probe"
PY = sys.executable

HDR = "//! 探针（由 gate_probe.py 生成，跑完即删）。\n"


def rs(text):
    return HDR + text


def long_line(n):
    return rs('pub const LONG: &str = "' + "a" * n + '";\n')


def many_methods(n):
    return rs("pub struct Probe;\nimpl Probe {\n"
              + "".join(f"    pub fn m{i}(&self) {{}}\n" for i in range(n)) + "}\n")


def deep(n):
    body, ind = "", "    "
    for i in range(n):
        body += f"{ind}if true {{\n"
        ind += "    "
    body += f"{ind}let _ = 1;\n"
    for i in range(n):
        ind = ind[:-4]
        body += f"{ind}}}\n"
    return rs("pub fn deep() {\n" + body + "}\n")


# (门名, 脚本, 探针文件 {文件名: 内容}) —— 内容只放**该门自己的**最小违规，便于定位。
PROBES = [
("god-gate", "god_gate.py",
     {"big.rs": rs("pub fn f() {\n" + "".join(f"    let x{i} = {i};\n" for i in range(820)) + "}\n")}),
("type-span", "type_span_gate.py", {"span.rs": many_methods(45)}),
]

# 需要复制一份现有文件才能触发的（雷同门）
COPIES = [

]

# 需要往**已有**文件里注入的（写 .agents/claims 下的声明）
EXTRA = [

]

# 需要**临时改坏**已跟踪文件的（跑完按内存备份原样写回，不走 git checkout ⇒
# 不会把别人未提交的改动一起还原掉）
MUTATIONS = [
("god-debt", "god_debt.py", ["--check"], "docs/review/god-debt.md",
     lambda t: re.sub(r"<!-- debt-total: \d+ -->", "<!-- debt-total: 1 -->", t, count=1)),
("selftest", "gate_selftest.py", [], "scripts/gates/god.gate.json",
     lambda t: re.sub(r'"max_file_lines": \d+', '"max_file_lines": 9000', t, count=1)),
]


def run(script, args=()):
    cp = subprocess.run([PY, "-X", "utf8", str(GATES / script), *args], cwd=str(ROOT),
                        capture_output=True, text=True, encoding="utf-8", errors="replace")
    return cp.returncode


def write_probe(files):
    PROBE.mkdir(parents=True, exist_ok=True)
    for name, text in files.items():
        (PROBE / name).write_text(text, encoding="utf-8")


def main(argv) -> int:
    only = None
    for i, a in enumerate(argv):
        if a == "--only" and i + 1 < len(argv):
            only = {s.strip() for s in argv[i + 1].split(",")}
    shutil.rmtree(PROBE, ignore_errors=True)        # 先清掉上次崩溃残留的探针

    rows, bad, unsure = [], [], []
    try:
        for name, script, files in PROBES:
            if only and name not in only:
                continue
            before = run(script)                     # 不带 --git-tracked：探针是新文件，本来就不在索引里
            write_probe(files)
            after = run(script)
            shutil.rmtree(PROBE, ignore_errors=True)
            ok = after != 0
            rows.append((name, before, after, ok))
            if not ok:
                bad.append(name)
            elif before != 0:
                unsure.append(name)

        for name, script, src in COPIES:
            if only and name not in only:
                continue
            before = run(script)
            PROBE.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / src, PROBE / (pathlib.Path(src).stem + "_copy.rs"))
            after = run(script)
            shutil.rmtree(PROBE, ignore_errors=True)
            rows.append((name, before, after, after != 0))
            if after == 0:
                bad.append(name)
            elif before != 0:
                unsure.append(name)

        for name, script, path, text in EXTRA:
            if only and name not in only:
                continue
            before = run(script)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            try:
                after = run(script)
            finally:
                path.unlink(missing_ok=True)
            rows.append((name, before, after, after != 0))
            if after == 0:
                bad.append(name)
            elif before != 0:
                unsure.append(name)

        for name, script, args, rel, mutate in MUTATIONS:
            if only and name not in only:
                continue
            p = ROOT / rel
            original = p.read_text(encoding="utf-8")
            before = run(script, args)
            p.write_text(mutate(original), encoding="utf-8")
            try:
                after = run(script, args)
            finally:
                p.write_text(original, encoding="utf-8")   # 原样写回，不动别人的未提交改动
            rows.append((name, before, after, after != 0))
            if after == 0:
                bad.append(name)
            elif before != 0:
                unsure.append(name)
    finally:
        shutil.rmtree(PROBE, ignore_errors=True)

    for name, before, after, ok in rows:
        flag = "OK  " if ok else "空门"
        star = "  ⚠ 注入前已红，验证不充分" if (ok and before != 0) else ""
        print(f"{flag} {name:16s} 注入前 exit={before} → 注入后 exit={after}{star}")
    if bad:
        print(f"GATE-PROBE FAIL 注入了也不红的门 = {bad}（空门或判据失效，见文件头三类成因）")
        return 1
    if unsure:
        print(f"GATE-PROBE OK（但 {unsure} 注入前就是红的 ⇒ 工作区不干净，建议先提交再跑）")
        return 0
    print(f"GATE-PROBE OK 全部 {len(rows)} 道门都能被触发（注入即红 ⇒ 不是空门）")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
