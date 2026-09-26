"""测试体积门：单个 `.rs` 文件里内嵌的测试（`#[cfg(test)]`）超过阈值（默认 200 行）。

  `docs/contributing.md`：「文件长度。单文件不超过 800 行，目标 500 行以内；**测试超过 200 行
  搬到 `tests.rs`**（多时 `tests/` 按主题分文件）」。

  为什么单独一道：内嵌测试不长在 `god_gate` 的口径里——一个 700 行的文件，其中 400 行是测试，
  产品代码只有 300 行，`god_gate` 看着还「没超 800」。但读这个文件的人要翻过 400 行测试才能
  看到产品逻辑，改一行产品代码要在 400 行测试里找对应的断言。测试该有自己的地方。

  口径：从 `#[cfg(test)]` 那行起到**文件末尾**的行数（本仓的惯例是把测试放在文件最后，
  这么量就够；若哪天测试夹在中间，量出来的数会偏大，方向保守——更容易提醒你搬走）。

  X1 单文件内测试 >200 行 —— 棘轮：只准减（新增即红）。

用法：python3 -X utf8 scripts/gates/testsize_gate.py [--git-tracked] [--list] [--write] [--max 200]
"""
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gates_common as gc  # noqa: E402

MAX = 200
CFG_TEST = re.compile(r"#\[cfg\(test\)\]")


def scan(root, a):
    """测试体积量的是**原始**行数，不过掩码（掩码会把注释与字符串抹掉，行数就不准了）。"""
    cur = {}
    for rel in gc.rs_files(root, a.git_tracked):
        text = gc.read_text(root, rel)
        if not text:
            continue
        lines = text.splitlines()
        for i, ln in enumerate(lines):
            if CFG_TEST.search(ln):
                n = len(lines) - i
                if n > a.max:
                    cur[rel] = n
                break
    return cur


if __name__ == "__main__":
    sys.exit(gc.run_count_gate(
        "TESTSIZE-GATE", "docs/review/testsize-baseline.json", scan, f"个文件测试>{MAX} 行",
        extra_args=lambda ap: ap.add_argument("--max", type=int, default=MAX),
        head=lambda cur, a: print(f"TESTSIZE-GATE(>{a.max}) 文件={len(cur)} "
                                  f"超阈行数合计={sum(cur.values())}")))
