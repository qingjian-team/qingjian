"""unwrap/expect 门：产品代码里 `.unwrap()` / `.expect()` / `.unwrap_err()` / `.expect_err()` /
`.unwrap_unchecked()` / `.expect_unchecked()` 这类「把 Result/Option 当必定成功」的调用。

  Rust 里 `?` 才是正确传播错误的方式；`unwrap`/`expect` 一旦遇到 Err/None 就直接 panic，把库里的
  一个可恢复错误变成整个进程崩溃。编译器不报，但这是 Rust 头号坏味道（clippy 只是 warn，不拦）。
  测试里 `assert!(x.unwrap())` 是正当写法 ⇒ 排除测试面。

  X1 产品代码（排除测试面）出现 unwrap/expect 系列 —— 棘轮：只准减（新增即红）。

用法：python3 -X utf8 scripts/gates/unwrap_gate.py [--git-tracked] [--list] [--write]
"""
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gates_common as gc  # noqa: E402

UNWRAP = re.compile(
    r"\.(?:unwrap|expect|unwrap_err|expect_err|unwrap_unchecked|expect_unchecked)\s*\(")

if __name__ == "__main__":
    sys.exit(gc.run_count_gate("UNWRAP-GATE", "docs/review/unwrap-baseline.json",
                               gc.regex_scan(UNWRAP), "处 unwrap/expect"))
