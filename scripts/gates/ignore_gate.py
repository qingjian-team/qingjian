"""忽略结果门：产品代码里显式丢弃值——`let _ = expr;` 或 `let (a, _) = f()` / `let (_, b) = f()`
这类含 `_` 的解构赋值。

  Rust 里 `let _ = result;` 是明确「我不在乎这个结果」——绝大多数情况是吞掉了 `Result` 里的错误
  （与 Go 的 errcheck、本仓 arch 门的 panic!/todo! 同一类「静默忽略」）。编译器不报。测试里
  丢弃无害 ⇒ 排除测试面。`let _x = …`（把值绑给 `_x`）是正常命名，不算丢弃。

  X1 产品代码（排除测试面）出现 `let _ =` / 含 `_` 的解构赋值 —— 棘轮：只准减（新增即红）。

用法：python3 -X utf8 scripts/gates/ignore_gate.py [--git-tracked] [--list] [--write]
"""
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gates_common as gc  # noqa: E402

# `let _ =` 或 `let (... _ ... ) =`（解构里含丢弃位）。
# **不锚行首**：`pub fn f() { let _ = g(); }` 这种单行函数体里的丢弃同样该算——
# 早先写成 `^\s*let…`，单行函数体就整条漏过去了（gate_probe 注入时才暴露）。
IGNORE = re.compile(r"\blet\s+(?:_\s*=|\([^;]*\b_\b[^;]*\)\s*=)")

if __name__ == "__main__":
    sys.exit(gc.run_count_gate("IGNORE-GATE", "docs/review/ignore-baseline.json",
                               gc.regex_scan(IGNORE, per_line=True), "处忽略结果"))
