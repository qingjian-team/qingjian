"""单辅助语言门：`docs/contributing.md`「**一个候选词只显示一种辅助语言**」的机器版。

  规矩原文：不要设计成 `translations: Vec<Translation>` 或 `HashMap<Lang, String>` 这类多语言
  并列的数据结构——「那会在 API 层面把『一次只学一种语言』这条产品原则给破坏掉。
  翻译是候选词的 annotation（可选、单条）」。

  为什么值得单独一道门：这是**产品原则**，写坏了照样编译得过去、测试也过得去，等到要加第二
  种语言时才发现整个数据结构都得推翻。它是「数据结构形状」上的约束，跟规模、复杂度、命名都没
  关系，别的门一类都抓不到。

  判据（**硬判**，存量 = 0 ⇒ 不设基线）：
    L1 类型是多语言并列的容器：`Vec<Translation>` / `[Translation; N]` / `HashMap<Lang, _>` /
       `BTreeMap<Lang, _>`；
    L2 字段名是复数 `translations:`（单条 annotation 不该有复数）。

  `Option<Translation>`（可选单条）是**合规**写法，不判。

用法：python3 -X utf8 scripts/gates/lang_gate.py [--git-tracked] [--list]
"""
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gates_common as gc  # noqa: E402

MULTI = re.compile(
    r"Vec<\s*(?:&)?\w*Translation\b"          # Vec<Translation> / Vec<&Translation>
    r"|\[\s*(?:&)?\w*Translation\s*;"         # [Translation; N]
    r"|(?:$|[^A-Za-z_])(?:Hash|BTree)Map<\s*(?:&)?\w*Lang\b"   # HashMap<Lang, _>
    r"|(?:$|[^A-Za-z_])(?:Hash|BTree)Map<\s*[^,]*\bLang\b\s*,"  # HashMap<Lang, _>（键在后）
)
PLURAL = re.compile(r"\btranslations\s*:")


def scan(root, git_tracked):
    bad = []
    for rel in gc.rs_files(root, git_tracked):
        text = gc.read_text(root, rel)
        if not text:
            continue
        masked = gc.mask(text)
        for i, ln in enumerate(masked.splitlines(), 1):
            if MULTI.search(ln):
                bad.append(f"L1 {rel}:{i}: 多语言并列容器（一次只学一种语言 ⇒ 用单条 annotation）")
            elif PLURAL.search(ln):
                bad.append(f"L2 {rel}:{i}: 字段名 `translations` 是复数（翻译是可选单条）")
    return bad


def main() -> int:
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--git-tracked", action="store_true")
    a = ap.parse_args()
    bad = scan(gc.ROOT, a.git_tracked)
    print(f"LANG-GATE 命中={len(bad)}（**硬判**：存量 = 0，无基线）")
    for line in bad[:25]:
        print(f"  ✗ {line}")
    if len(bad) > 25:
        print(f"  …另有 {len(bad) - 25} 条")
    print(f"LANG-GATE {'FAIL' if bad else 'OK'}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
