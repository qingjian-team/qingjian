"""行长门：单行超过阈值（默认 200 字符）即记一笔。语言无关，纯行长度，无需掩码。

  Go/Rust 官方都不强制行长，但超长行在 review / diff / 终端里都难读，也常是「一个表达式塞了
  太多东西」或「超长字符串字面量 / 宏」的信号。本门不要求立刻修存量，只拦**新增**的超长行——
  改文件时顺手断行即可。

  X1 任意 .rs 文件里存在超过 `MAX`（默认 200）字符的行 —— 棘轮：只准减。

用法：python3 -X utf8 scripts/gates/linelen_gate.py [--git-tracked] [--list] [--write] [--max 200]
"""
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gates_common as gc  # noqa: E402

MAX = 200


def scan(root, a):
    """行长是**唯一**不过 `regex_scan` 的计数门：它量的是原始文本长度，
    过掩码会把注释与字符串抹掉——那量出来的就不是行长。测试文件同样受管。"""
    cur = {}
    for rel in gc.rs_files(root, a.git_tracked):
        text = gc.read_text(root, rel)
        if not text:
            continue
        n = sum(1 for ln in text.splitlines() if len(ln) > a.max)
        if n:
            cur[rel] = n
    return cur


if __name__ == "__main__":
    sys.exit(gc.run_count_gate(
        "LINELEN-GATE", "docs/review/linelen-baseline.json", scan, f"行>{MAX} 字符",
        extra_args=lambda ap: ap.add_argument("--max", type=int, default=MAX),
        head=lambda cur, a: print(f"LINELEN-GATE(>{a.max}) count={sum(cur.values())}")))
