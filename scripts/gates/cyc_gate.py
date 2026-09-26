"""圈复杂度门：函数内分支点数量（纯文本启发式，Rust 感知掩码）。

  决策点 = 1（函数本身）+ `if`/`for`/`while`/`loop`/`match` 各 +1 + `&&`/`||` 各 +1 +
  `?` 试运算符各 +1 + `match` 每个分支 `=>` 各 +1。只判产品代码（测试 setup 函数复杂度高是
  常态，不算产品风险）——与 clippy 的 cyclomatic_complexity 同思路，但用纯文本、不依赖编译。

  X1 函数圈复杂度 >15 —— 棘轮；>50（几乎无法单测覆盖）—— 棘轮。按文件记「超 15 的函数数 /
  超 50 的函数数」。

用法：python3 -X utf8 scripts/gates/cyc_gate.py [--git-tracked] [--list] [--write]
"""
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gates_common as gc  # noqa: E402

PARTS = [("over15", "cyc>15"), ("over50", "cyc>50")]
KW = re.compile(r"\b(if|for|while|loop|match)\b")
ANDOR = re.compile(r"&&|\|\|")
QUEST = re.compile(r"(?<![:\w<])\?(?![A-Za-z_])")
ARMS = re.compile(r"=>")


def fn_cc(body):
    src = "\n".join(body)
    return 1 + len(KW.findall(src)) + len(ANDOR.findall(src)) \
        + len(QUEST.findall(src)) + len(ARMS.findall(src))


if __name__ == "__main__":
    sys.exit(gc.run_dicts_gate(
        "CYC-GATE", "docs/review/cyc-baseline.json",
        gc.fn_metric_scan([(15, "over15"), (50, "over50")], fn_cc), PARTS))
