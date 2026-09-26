"""super 转手门：`use super::super::…` 绕父模块拿类型。

  `docs/contributing.md`：「子文件从父模块拿的类型写 `use super::{Engine, Candidate};`，
  **父模块没定义的直接 `use crate::…` 或 `use <crate>::…`，不绕 `super::` 转手**」。

  `a/b/c.rs` 里写 `use super::super::X`，意思是「往上两级拿 X」——读者得先搞清楚 `a/mod.rs`
  到底 re-export 了什么才能知道 X 从哪来；父模块一改 re-export，中间这一层就断。
  直接从 `crate::` 或 crate 名起步，路径是绝对的，读的人不用在脑子里做相对路径运算。

  X1 产品代码出现 `use super::super::` —— 棘轮：只准减（新增即红）。

用法：python3 -X utf8 scripts/gates/super_gate.py [--git-tracked] [--list] [--write]
"""
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gates_common as gc  # noqa: E402

SUPER2 = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?use\s+super::super::")

if __name__ == "__main__":
    sys.exit(gc.run_count_gate("SUPER-GATE", "docs/review/super-baseline.json",
                               gc.regex_scan(SUPER2, per_line=True), "处 super::super:: 转手"))
