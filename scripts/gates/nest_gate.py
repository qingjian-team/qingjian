"""嵌套深度门：函数体里花括号的最大层数（箭头代码 / Arrow Code）。

  圈复杂度数的是「分支多少」，这一门数的是「分支套了几层」。同样复杂的一段逻辑，写成
  卫语句平铺（`if !ok { return }`）与写成五层 `if let` 套 `match`，复杂度可能一样，可读性
  差一个量级——后者 review 时人眼配平不过来，改一处要同时记住五层上下文。
  深度从函数体的 `{` 起算为 1，所以「>5」≈ 函数体里又套了四层。

  X1 函数最大嵌套深度 >5 —— 棘轮；>8（基本无法在 review 里跟住）—— 棘轮。
  按文件记「超 5 的函数数 / 超 8 的函数数」。只判产品代码。

用法：python3 -X utf8 scripts/gates/nest_gate.py [--git-tracked] [--list] [--write]
"""
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gates_common as gc  # noqa: E402

PARTS = [("over5", "nest>5"), ("over8", "nest>8")]


def max_depth(body):
    d = mx = 0
    for line in body:
        for ch in line:
            if ch == "{":
                d += 1
                mx = max(mx, d)
            elif ch == "}":
                d -= 1
    return mx


if __name__ == "__main__":
    sys.exit(gc.run_dicts_gate(
        "NEST-GATE", "docs/review/nest-baseline.json",
        gc.fn_metric_scan([(5, "over5"), (8, "over8")], max_depth), PARTS,
        hard=("over8",)))   # >8 存量本来就是 0 ⇒ 开硬阈，不许被 --write 祖父化
