"""日志门面门：产品代码里写死 `println!` / `eprintln!` 而不用 `tracing`。

  `docs/contributing.md`：日志用 `tracing` 门面。裸 `println!` 在壳里是看不见的——
  macOS 的 IMK 插件、Windows 的 TSF DLL 都没有控制台，写出去的东西没人收得到，等到真要查
  问题时才发现日志早丢光了；它同时也没有级别、没有 span、没法按模块开关。

  排除面（这些地方打印到 stdout 就是本分）：
  `apps/cli`（命令行工具就是要往终端打）、`tools/`（构建/转换脚本）、`build.rs`、
  以及测试面（`tests/` `examples/` `benches/`）。

  X1 其余代码出现 `println!` / `eprintln!` —— 棘轮：只准减（新增即红）。

用法：python3 -X utf8 scripts/gates/log_gate.py [--git-tracked] [--list] [--write]
"""
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gates_common as gc  # noqa: E402

PRINT = re.compile(r"\b(?:println!|eprintln!)\s*[(!]")
PRINT_OK = ("apps/cli/", "tools/", "build.rs", "examples/")

if __name__ == "__main__":
    sys.exit(gc.run_count_gate("LOG-GATE", "docs/review/log-baseline.json",
                               gc.regex_scan(PRINT, extra_skip=PRINT_OK),
                               "处 println!/eprintln!"))
