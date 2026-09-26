"""标识符与错误文案的语言门：`docs/contributing.md`「命名与注释」那两条的机器版。

  规矩原文：「代码标识符一律英文，注释与文档用中文；**`thiserror` 的 `#[error]` 文案用英文**，
  日志与 UI 文案用中文」。

  为什么值得单独一道门：混进中文标识符（或中文错误文案）时，一切都还能编译、还能跑，但
  跨平台的终端/日志编码一出问题就是乱码，而且 `#[error]` 的文案是要被上层 match 与日志系统
  当**标识符**用的（分级、过滤、对接第三方），写成中文等于给自己埋雷。

  判据（**硬判**，存量 = 0 ⇒ 不设基线）：
    I1 代码里出现中文标识符（掩码后仍含中日韩字符 —— 注释与字符串已被抹掉，剩下的就是代码）；
    I2 `#[error("…")]` 的文案含中文（这条必须用**原始**文本判：掩码会把字符串一起抹掉）。

用法：python3 -X utf8 scripts/gates/ident_gate.py [--git-tracked] [--list]
"""
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gates_common as gc  # noqa: E402

CJK = re.compile(r"[一-鿿぀-ヿ가-힯]")
IDENT_CJK = re.compile(
    r"(?:fn|struct|enum|trait|let|const|static|type|mod|use|impl|pub)\s+\S*[一-鿿぀-ヿ가-힯]"
    r"|[A-Za-z0-9_][一-鿿぀-ヿ가-힯]|[一-鿿぀-ヿ가-힯][A-Za-z0-9_]")
# 只取属性**括号里**的内容再判中文：`#[error(transparent)] // 中文注释` 不该算命中。
# （早先用一条正则想一次匹配完，可选组会贪婪吃掉整个字符串，反而永远匹配不上。）
ERR_ATTR = re.compile(r"#\[error\((.*?)\)\]")


def scan(root, git_tracked):
    bad = []
    for rel in gc.rs_files(root, git_tracked):
        text = gc.read_text(root, rel)
        if not text:
            continue
        for i, ln in enumerate(gc.mask(text).splitlines(), 1):
            if IDENT_CJK.search(ln):
                bad.append(f"I1 {rel}:{i}: 标识符含中文（标识符一律英文，注释与文档才用中文）")
        for i, ln in enumerate(text.splitlines(), 1):
            if any(CJK.search(m.group(1)) for m in ERR_ATTR.finditer(ln)):
                bad.append(f"I2 {rel}:{i}: `#[error]` 文案含中文（thiserror 文案用英文）")
    return bad


def main() -> int:
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--git-tracked", action="store_true")
    a = ap.parse_args()
    bad = scan(gc.ROOT, a.git_tracked)
    print(f"IDENT-GATE 命中={len(bad)}（**硬判**：存量 = 0，无基线）")
    for line in bad[:25]:
        print(f"  ✗ {line}")
    if len(bad) > 25:
        print(f"  …另有 {len(bad) - 25} 条")
    print(f"IDENT-GATE {'FAIL' if bad else 'OK'}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
