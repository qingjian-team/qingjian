"""休眠门：产品代码里 `std::thread::sleep` / `thread::sleep`。

  Rust 的 sleep 跟 Go 一样，轮询等某事发生、循环里 sleep 节流、启动顺序靠 sleep 凑——都会让程序
  在 CI / 弱机器上 flake、在延迟敏感路径上卡顿。正确做法是 channel / 条件变量 / `?` 取消 /
  `tokio::time::sleep` + `select!`。测试里 sleep 等异步就绪是正当的 ⇒ 排除测试面。

  X1 产品代码（排除测试面）出现 `thread::sleep(` —— 棘轮：只准减（新增即红）。

用法：python3 -X utf8 scripts/gates/sleep_gate.py [--git-tracked] [--list] [--write]
"""
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gates_common as gc  # noqa: E402

SLEEP = re.compile(r"(?:std::)?thread::sleep\s*\(")

if __name__ == "__main__":
    sys.exit(gc.run_count_gate("SLEEP-GATE", "docs/review/sleep-baseline.json",
                               gc.regex_scan(SLEEP), "处 thread::sleep"))
